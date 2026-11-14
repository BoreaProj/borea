//! F04 end-to-end cell runner: create → plan → apply → boot → verify → collect.
//!
//! Each run owns `target/e2e/<family>/<profile>/<timestamp>/` and always leaves
//! `result.json`, even when a stage fails. Stages run in order and stop at the
//! first failure; later stages are recorded as `not_run`. Nothing here promotes
//! a matrix cell: `matrix verify --strict` (F05) reads the `pass` field.
//!
//! Create prepares image storage with the closed G02 program. Plan is made
//! against that image's identity and UUIDs. Apply runs the plan's steps with
//! the G05 image plan executor (D-040); the closed Arch bootstrap is not used.

use crate::vm;
use borea_core::{BoreaError, Config, DeviceIdentity, FamilyId, Filesystem, Loader, Plan, Result};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    process::Command,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

const IMAGE_BYTES: u64 = 20 * 1024 * 1024 * 1024;
const STAGES: &[&str] = &["create", "plan", "apply", "boot", "verify", "collect"];
/// TESTING.md §11: no artifact may contain the canary password.
const CANARIES: &[&str] = &[
    "canary-not-a-real-password",
    "borea-bootstrap-ci-canary-not-a-real-password",
];
/// The VM lane reads the kernel log and login prompt from the serial port.
const SERIAL_CONSOLE: &[&str] = &["console=tty0", "console=ttyS0,115200"];
/// `vm::boot` waits for this host's login prompt.
const VM_HOSTNAME: &str = "borea";

#[derive(Debug, PartialEq, Eq)]
pub struct Args {
    pub family: FamilyId,
    pub profile: String,
    pub root_fs: Filesystem,
    pub loader: Loader,
    pub keep: bool,
    pub out: Option<PathBuf>,
}

impl Args {
    pub fn parse(args: &[&str]) -> Result<Self> {
        let usage = || {
            BoreaError::invalid(
                "usage: cargo xtask e2e --family FAMILY --profile PROFILE [--loader auto|limine|grub] [--root-fs btrfs|ext4] [--keep] [--out ABSOLUTE_DIR]",
            )
        };
        let mut values = BTreeMap::new();
        let mut keep = false;
        let mut rest = args;
        while let [key, tail @ ..] = rest {
            if *key == "--keep" {
                keep = true;
                rest = tail;
                continue;
            }
            let key = key.strip_prefix("--").ok_or_else(usage)?;
            let [value, tail @ ..] = tail else {
                return Err(usage());
            };
            if !["family", "profile", "loader", "root-fs", "out"].contains(&key)
                || values.insert(key, *value).is_some()
            {
                return Err(usage());
            }
            rest = tail;
        }
        let family = FamilyId::parse(values.get("family").ok_or_else(usage)?)?;
        let profile = (*values.get("profile").ok_or_else(usage)?).to_string();
        let profile_fs = if profile == "minimal-ext4" {
            Filesystem::Ext4
        } else {
            Filesystem::Btrfs
        };
        let root_fs = match values.get("root-fs").copied() {
            None => profile_fs,
            Some("btrfs") => Filesystem::Btrfs,
            Some("ext4") => Filesystem::Ext4,
            Some(_) => return Err(usage()),
        };
        let loader = match values.get("loader").copied() {
            None if profile == "minimal-grub" => Loader::Grub,
            None | Some("auto") => Loader::Auto,
            Some("limine") => Loader::Limine,
            Some("grub") => Loader::Grub,
            Some(_) => return Err(usage()),
        };
        let out = values.get("out").map(PathBuf::from);
        if out.as_ref().is_some_and(|o| !o.is_absolute()) {
            return Err(BoreaError::invalid("--out must be an absolute directory"));
        }
        Ok(Self {
            family,
            profile,
            root_fs,
            loader,
            keep,
            out,
        })
    }

    /// Only cells with a disposable-image lane can run. Refuse the rest before
    /// any image is created rather than produce an artifact that proves nothing.
    fn gate(&self) -> Result<()> {
        if self.family != FamilyId::Arch {
            return Err(BoreaError::unsupported(format!(
                "e2e has no {} image lane: only the Arch runtime bootstraps images (G03); Debian/Fedora/openSUSE image bootstrap are later cards",
                self.family.as_str()
            )));
        }
        if !matches!(self.profile.as_str(), "minimal" | "minimal-ext4") {
            return Err(BoreaError::unsupported(format!(
                "e2e profile {} is not runnable: session cells have not been run through the image plan executor yet; only arch/minimal and arch/minimal-ext4 have an image lane",
                self.profile
            )));
        }
        if self.loader == Loader::Grub {
            return Err(BoreaError::unsupported(
                "e2e has no GRUB image lane: the Arch image bootloader executor installs Limine only (G03)",
            ));
        }
        Ok(())
    }

    fn fs(&self) -> &'static str {
        match self.root_fs {
            Filesystem::Btrfs => "btrfs",
            Filesystem::Ext4 => "ext4",
        }
    }
}

struct Run {
    dir: PathBuf,
    stages: Vec<serde_json::Value>,
    artifacts: BTreeMap<&'static str, String>,
    identity: serde_json::Value,
}

impl Run {
    fn stage<T>(
        &mut self,
        name: &'static str,
        f: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        let started = Instant::now();
        let result = f(self);
        let mut entry = serde_json::json!({
            "stage": name,
            "status": if result.is_ok() { "ok" } else { "failed" },
            "duration_s": started.elapsed().as_secs(),
        });
        if let Err(e) = &result {
            entry["error"] = serde_json::json!({"code": e.code, "message": e.message});
        }
        self.stages.push(entry);
        result
    }

    /// Record a file under its artifact name if it exists.
    fn artifact(&mut self, name: &'static str, file: &str) {
        if self.dir.join(file).is_file() {
            self.artifacts.insert(name, file.into());
        }
    }
}

pub fn run(args: &Args) -> Result<()> {
    args.gate()?;
    let started = Instant::now();
    let root = match &args.out {
        Some(out) => out.clone(),
        None => fs::canonicalize(crate::workspace())?.join("target/e2e"),
    };
    let cell = format!("{}/{}", args.family.as_str(), args.profile);
    let parent = root.join(&cell);
    fs::create_dir_all(&parent)?;
    let dir = parent.join(timestamp(SystemTime::now()));
    fs::create_dir(&dir)?;
    let mut run = Run {
        dir,
        stages: Vec::new(),
        artifacts: BTreeMap::new(),
        identity: serde_json::Value::Null,
    };
    let disk = run.dir.join("disk.img");
    let outcome = pipeline(args, &mut run, &disk);
    if !args.keep && disk.exists() {
        let _ = fs::remove_file(&disk);
    }
    let collected = run.stage("collect", |run| collect(run, args.keep));
    let outcome = outcome.and(collected);
    let failed = run
        .stages
        .iter()
        .find(|s| s["status"] == "failed")
        .map(|s| s["stage"].clone());
    for name in STAGES {
        if !run.stages.iter().any(|s| s["stage"] == *name) {
            run.stages
                .push(serde_json::json!({"stage": name, "status": "not_run"}));
        }
    }
    order_stages(&mut run.stages);
    let result = serde_json::json!({
        "schema": 1,
        "cell": cell,
        "family": args.family,
        "profile": args.profile,
        "root_fs": args.fs(),
        "loader": "limine",
        "commit": commit(),
        "pass": outcome.is_ok(),
        "failed_stage": failed,
        "stages": run.stages,
        "artifacts": run.artifacts,
        "identity": run.identity,
        "duration_s": started.elapsed().as_secs(),
        "release": borea_core::RELEASE,
    });
    let path = run.dir.join("result.json");
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?
        .write_all(format!("{}\n", serde_json::to_string_pretty(&result)?).as_bytes())?;
    println!("{}", serde_json::to_string(&result)?);
    eprintln!("e2e artifacts: {}", run.dir.display());
    outcome
}

fn pipeline(args: &Args, run: &mut Run, disk: &Path) -> Result<()> {
    let cache = std::env::var_os("BOREA_RUNTIME_CACHE").map(PathBuf::from);
    let lock = borea_core::runtime::RuntimeLock::embedded()?;
    let spec = lock.get(args.family)?;
    let (image, cache) = run.stage("create", |run| {
        let cache = cache.ok_or_else(|| {
            BoreaError::invalid("set BOREA_RUNTIME_CACHE to the fetched runtime cache")
        })?;
        let manifest = run.dir.join("image.json");
        let image = borea_run::image::create(disk, &manifest, args.fs(), IMAGE_BYTES, spec)?;
        run.artifact("image_manifest", "image.json");
        let journal = run.dir.join("storage.journal");
        let applied = borea_run::image::apply(&image, spec, &cache, &journal, false, true);
        run.artifact("storage_journal", "storage.journal");
        applied?;
        Ok((image, cache))
    })?;
    let (config, plan) = run.stage("plan", |run| {
        let (config, plan) = plan_for(args, &image)?;
        fs::write(run.dir.join("plan.json"), plan.json()?)?;
        run.artifact("plan", "plan.json");
        run.identity = identity_report(&plan, &image);
        Ok((config, plan))
    })?;
    run.stage("apply", |run| {
        let staging = stage_inputs(&config)?;
        let journal = run.dir.join("journal.jsonl");
        let applied = borea_run::image::apply_plan(
            &image,
            &plan,
            spec,
            &cache,
            &journal,
            borea_run::image::PlanOptions {
                staging: staging.path(),
                resume: false,
                yes: true,
            },
        );
        run.artifact("journal", "journal.jsonl");
        applied
    })?;
    let boot = vm::BootArgs {
        image: disk.to_path_buf(),
        filesystem: args.fs().into(),
        serial_log: run.dir.join("serial.log"),
        verify: Some((run.dir.join("plan.json"), run.dir.join("verify.json"))),
        expect: vm::Expect::Pass,
    };
    // vm::boot boots to login, checks the serial log, then runs `borea verify
    // --ssh`; an E_VERIFY error means the boot itself passed.
    let started = Instant::now();
    let result = vm::boot(&boot);
    let elapsed = started.elapsed().as_secs();
    let boot_ok = result.is_ok() || result.as_ref().is_err_and(|e| e.code == "E_VERIFY");
    for (name, ok, err) in [
        ("boot", boot_ok, result.as_ref().err().filter(|_| !boot_ok)),
        (
            "verify",
            result.is_ok(),
            result.as_ref().err().filter(|_| boot_ok),
        ),
    ] {
        if name == "verify" && !boot_ok {
            break;
        }
        let mut entry = serde_json::json!({
            "stage": name,
            "status": if ok { "ok" } else { "failed" },
            "duration_s": elapsed,
        });
        if let Some(e) = err {
            entry["error"] = serde_json::json!({"code": e.code, "message": e.message});
        }
        run.stages.push(entry);
    }
    run.artifact("serial_log", "serial.log");
    run.artifact("qemu_log", "serial.qemu.log");
    run.artifact("verify", "verify.json");
    result
}

/// Plan the cell against the image: its identity, its storage UUIDs, the VM
/// lane's hostname and a serial console. Everything else is the golden config.
fn plan_for(args: &Args, image: &borea_run::image::ImagePlan) -> Result<(Config, Plan)> {
    let mut config = crate::case_config(args.family, &args.profile, args.root_fs, args.loader)?;
    config.target.hostname = VM_HOSTNAME.into();
    let matrix = borea_core::matrix::Matrix::embedded()?;
    let family = borea_core::recipe::family(config.target.family)?;
    let loader = borea_distro::loader(&config, &family, &matrix)?;
    let session = borea_session::select(&config, &matrix, loader, false, false)?;
    let plan = borea_distro::planner::build_with(
        &config,
        DeviceIdentity {
            path: config.disk.device.clone(),
            serial: format!("borea-image-{}", &image.seed[..16]),
            size: image.bytes,
            sector_size: 512,
        },
        borea_distro::planner::Intent {
            components: session.components,
            greeter: session.greeter,
            session: session.session,
            autostart: session.autostart,
        },
        &borea_distro::planner::Overrides {
            storage_ids: Some(borea_distro::planner::StorageIds {
                esp: image.uuid("esp"),
                boot: image.uuid("boot"),
                root: image.uuid("root"),
            }),
            kernel_args: SERIAL_CONSOLE.iter().map(|s| s.to_string()).collect(),
        },
    )?;
    Ok((config, plan))
}

/// Root-private staging for the plan's host files: the account password (a
/// canary, so a leak into any artifact is detectable) and a throwaway SSH key.
fn stage_inputs(config: &Config) -> Result<tempfile::TempDir> {
    let staging = tempfile::Builder::new()
        .prefix("borea-e2e-staging-")
        .tempdir()?;
    fs::set_permissions(staging.path(), fs::Permissions::from_mode(0o700))?;
    let host = |plan_path: &Path| -> Result<PathBuf> {
        let rel = plan_path
            .strip_prefix("/")
            .map_err(|_| BoreaError::invalid("plan host paths must be absolute"))?;
        let path = staging.path().join(rel);
        fs::create_dir_all(path.parent().expect("staged file has a parent"))?;
        Ok(path)
    };
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(host(&config.account.password_file)?)?
        .write_all(format!("{}\n", CANARIES[0]).as_bytes())?;
    if let Some(key) = &config.account.ssh_authorized_key_file {
        let private = staging.path().join("e2e-key");
        let status = Command::new("ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-C", "borea-e2e", "-f"])
            .arg(&private)
            .status()
            .map_err(|_| {
                BoreaError::unsupported("ssh-keygen is required; install openssh-client")
            })?;
        if !status.success() {
            return Err(BoreaError::new("E_VM", "ssh-keygen failed"));
        }
        fs::copy(private.with_extension("pub"), host(key)?)?;
    }
    Ok(staging)
}

/// Record that the plan's storage identity matches the image's.
fn identity_report(plan: &Plan, image: &borea_run::image::ImagePlan) -> serde_json::Value {
    let planned = |role: &str| {
        plan.steps
            .iter()
            .find(|s| s.id == format!("storage.format.{role}"))
            .and_then(|s| match &s.action {
                borea_core::Action::Command { argv, .. } => argv
                    .iter()
                    .position(|a| a == "-U")
                    .and_then(|i| argv.get(i + 1).cloned()),
                _ => None,
            })
    };
    let mut out = serde_json::Map::new();
    for role in ["root", "boot"] {
        let (planned, actual) = (planned(role), image.uuid(role));
        out.insert(
            role.into(),
            serde_json::json!({
                "plan_uuid": planned,
                "image_uuid": actual,
                "match": planned.as_deref() == Some(actual.as_str()),
            }),
        );
    }
    serde_json::Value::Object(out)
}

fn collect(run: &mut Run, keep: bool) -> Result<()> {
    if keep {
        run.artifact("disk", "disk.img");
    }
    for entry in fs::read_dir(&run.dir)? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "img") {
            continue;
        }
        let text = String::from_utf8_lossy(&fs::read(&path)?).into_owned();
        if let Some(canary) = CANARIES.iter().find(|c| text.contains(*c)) {
            return Err(BoreaError::new(
                "E_SECRET",
                format!("canary {canary:?} leaked into {}", path.display()),
            ));
        }
    }
    Ok(())
}

fn order_stages(stages: &mut [serde_json::Value]) {
    stages.sort_by_key(|s| {
        STAGES
            .iter()
            .position(|n| s["stage"] == *n)
            .unwrap_or(STAGES.len())
    });
}

fn commit() -> String {
    if let Ok(sha) = std::env::var("GITHUB_SHA")
        && !sha.is_empty()
    {
        return sha;
    }
    let workspace = crate::workspace();
    Command::new("git")
        .arg("-c")
        .arg(format!("safe.directory={}", workspace.display()))
        .arg("-C")
        .arg(&workspace)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}

/// UTC `YYYYMMDDTHHMMSSZ`, sortable and free of path separators.
fn timestamp(now: SystemTime) -> String {
    let secs = now.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (days, rem) = (secs / 86_400, secs % 86_400);
    // Civil-from-days (Howard Hinnant), valid for the proleptic Gregorian calendar.
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}{month:02}{day:02}T{:02}{:02}{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn parse(a: &[&str]) -> Result<Args> {
        Args::parse(a)
    }

    #[test]
    fn parses_cell_and_overrides() {
        let a = parse(&["--family", "arch", "--profile", "minimal"]).unwrap();
        assert_eq!(
            (a.root_fs, a.loader, a.keep),
            (Filesystem::Btrfs, Loader::Auto, false)
        );
        assert!(a.gate().is_ok());
        let a = parse(&["--family", "arch", "--profile", "minimal-ext4", "--keep"]).unwrap();
        assert_eq!((a.root_fs, a.keep), (Filesystem::Ext4, true));
        let a = parse(&[
            "--family",
            "arch",
            "--profile",
            "minimal",
            "--root-fs",
            "ext4",
            "--out",
            "/tmp/e",
        ])
        .unwrap();
        assert_eq!(a.root_fs, Filesystem::Ext4);
        assert!(parse(&["--family", "arch"]).is_err());
        assert!(parse(&["--family", "arch", "--profile", "minimal", "--loader"]).is_err());
        assert!(parse(&["--family", "arch", "--profile", "minimal", "--out", "rel"]).is_err());
        assert!(
            parse(&[
                "--family",
                "arch",
                "--family",
                "arch",
                "--profile",
                "minimal"
            ])
            .is_err()
        );
    }

    #[test]
    fn refuses_cells_without_an_image_lane() {
        for a in [
            ["--family", "debian", "--profile", "minimal"],
            ["--family", "arch", "--profile", "kde"],
            ["--family", "arch", "--profile", "minimal-grub"],
        ] {
            let e = parse(&a).unwrap().gate().unwrap_err();
            assert_eq!(e.code, "E_UNSUPPORTED");
        }
    }

    #[test]
    fn e2e_plan_is_made_for_the_image() {
        let args = parse(&["--family", "arch", "--profile", "minimal"]).unwrap();
        let image = image_fixture();
        let (config, plan) = plan_for(&args, &image).unwrap();
        assert_eq!(config.target.hostname, VM_HOSTNAME);
        assert_eq!(
            plan.disk.serial,
            format!("borea-image-{}", &image.seed[..16])
        );
        let r = identity_report(&plan, &image);
        for role in ["root", "boot"] {
            assert_eq!(r[role]["plan_uuid"], image.uuid(role));
            assert_eq!(r[role]["match"], true);
        }
        let json = plan.json().unwrap();
        // fstab, the ESP mount and the kernel command line all use image ids.
        assert!(json.contains(&format!("UUID={} / btrfs", image.uuid("root"))));
        assert!(json.contains(&format!("PARTUUID={} /boot/efi", image.uuid("esp"))));
        assert!(json.contains(&format!(
            "root=UUID={} rootflags=subvol=@ console=tty0 console=ttyS0,115200",
            image.uuid("root")
        )));
        assert!(!json.contains(CANARIES[0]));
    }

    #[test]
    fn collect_rejects_canary_in_artifacts() {
        let d = tempfile::tempdir().unwrap();
        let mut run = Run {
            dir: d.path().into(),
            stages: Vec::new(),
            artifacts: BTreeMap::new(),
            identity: serde_json::Value::Null,
        };
        fs::write(d.path().join("plan.json"), "{}").unwrap();
        assert!(collect(&mut run, false).is_ok());
        fs::write(d.path().join("serial.log"), "x canary-not-a-real-password").unwrap();
        assert_eq!(collect(&mut run, false).unwrap_err().code, "E_SECRET");
    }

    #[test]
    fn stages_are_reported_in_pipeline_order() {
        let mut s = vec![
            serde_json::json!({"stage": "collect"}),
            serde_json::json!({"stage": "create"}),
            serde_json::json!({"stage": "boot"}),
        ];
        order_stages(&mut s);
        let names: Vec<_> = s.iter().map(|v| v["stage"].as_str().unwrap()).collect();
        assert_eq!(names, ["create", "boot", "collect"]);
    }

    #[test]
    fn timestamp_is_utc_compact() {
        assert_eq!(timestamp(UNIX_EPOCH), "19700101T000000Z");
        let t = UNIX_EPOCH + Duration::from_secs(1_794_630_660);
        assert_eq!(timestamp(t), "20261114T043100Z");
    }

    fn image_fixture() -> borea_run::image::ImagePlan {
        borea_run::image::ImagePlan {
            schema: 1,
            image: "/tmp/disk.img".into(),
            device: 1,
            inode: 2,
            bytes: IMAGE_BYTES,
            filesystem: "btrfs".into(),
            runtime_sha256: "0".repeat(64),
            seed: "ab".repeat(32),
            digest: "cd".repeat(32),
        }
    }
}
