use borea_core::{
    matrix::{Matrix, Tier},
    recipe, *,
};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

mod e2e;
mod vm;

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(e.exit_code());
    }
}
fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn run() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let words = args.iter().map(String::as_str).collect::<Vec<_>>();
    match words.as_slice() {
        ["matrix", "check"] => {
            Matrix::embedded()?.validate()?;
            recipe::lint()?;
            println!(
                "matrix: complete cells, unique keys, evidence and native package mappings checked; 0 VM-supported cells"
            );
        }
        ["runtime", "check"] => {
            borea_core::runtime::RuntimeLock::embedded()?.validate()?;
            println!("runtime manifest: four content-pinned x86_64 family tool bundles");
        }
        ["recipe", "lint"] => {
            recipe::lint()?;
            for entry in fs::read_dir(workspace().join("recipes/profiles"))? {
                let p = entry?.path();
                if p.extension().is_some_and(|s| s == "toml") {
                    Config::parse(&fs::read_to_string(p)?)?;
                }
            }
            println!(
                "recipes and profile configs valid; unpinned third-party/source paths remain gated"
            );
        }
        ["matrix", "render"] => render(false)?,
        ["matrix", "render", "--check"] => render(true)?,
        ["matrix", "verify"] | ["matrix", "verify", "--strict"] => verify_matrix()?,
        ["plan", "golden"] => golden(false)?,
        ["plan", "golden", "--update"] => golden(true)?,
        ["secret", "canary"] => canary()?,
        ["disk", "selftest", "--stage", "create"] => disk_create()?,
        ["disk", "selftest", "--fs", fs] => disk_storage(fs, FamilyId::Arch)?,
        ["disk", "selftest", "--fs", fs, "--family", family] => {
            disk_storage(fs, FamilyId::parse(family)?)?
        }
        ["disk", "bootstrap", "--fs", fs] => disk_bootstrap(fs, None)?,
        ["disk", "bootstrap", "--fs", fs, "--out", image] => {
            disk_bootstrap(fs, Some((Path::new(image), None)))?
        }
        [
            "disk",
            "bootstrap",
            "--fs",
            fs,
            "--out",
            image,
            "--plan-out",
            plan,
        ] => disk_bootstrap(fs, Some((Path::new(image), Some(Path::new(plan)))))?,
        ["disk", "bootstrap", "--fs", fs, "--pacman-cache", cache] => {
            disk_bootstrap_cached(fs, Some(Path::new(cache)), None)?
        }
        ["vm", "boot", rest @ ..] => vm::boot(&vm::BootArgs::parse(rest)?)?,
        ["vm", "corrupt-root", "--image", image, "--out", out] => {
            vm::corrupt_root(Path::new(image), Path::new(out))?
        }
        ["disk", ..] => {
            return Err(BoreaError::unsupported(
                "unknown disk task or option; see cargo xtask --help",
            ));
        }
        ["e2e", "--profile", "omarchy-trim"] => {
            let ids = vec!["docker".into(), "whatsapp".into()];
            let preview = borea_run::omarchy::trim_preview(&ids)?;
            if preview[0]["argv"][0] != "omarchy-pkg-drop"
                || preview[1]["argv"][0] != "omarchy-webapp-remove"
            {
                return Err(BoreaError::new("E_TEST", "trim preview mapping mismatch"));
            }
            println!(
                "Omarchy trim preview fixture passed; no package removal or update-window integration tested"
            );
        }
        ["e2e", rest @ ..] => e2e::run(&e2e::Args::parse(rest)?)?,
        ["env", ..] => envrun(&args[1..])?,
        ["vm", ..] => {
            return Err(BoreaError::invalid(
                "usage: cargo xtask vm boot --image ABSOLUTE_IMAGE --fs btrfs|ext4 --serial-log ABSOLUTE_LOG [--plan PLAN --report ABSOLUTE_REPORT] [--expect pass|fail] | vm corrupt-root --image ABSOLUTE_IMAGE --out ABSOLUTE_COPY",
            ));
        }
        [] | ["--help"] => println!(
            "cargo xtask runtime check\ncargo xtask matrix check|render [--check]|verify --strict\ncargo xtask recipe lint\ncargo xtask plan golden [--update]\ncargo xtask secret canary\ncargo xtask disk selftest --stage create\ncargo xtask disk bootstrap --fs btrfs|ext4 [--out ABSOLUTE_IMAGE [--plan-out ABSOLUTE_PLAN]]\ncargo xtask vm boot --image ABSOLUTE_IMAGE --fs btrfs|ext4 --serial-log ABSOLUTE_LOG [--plan PLAN --report ABSOLUTE_REPORT] [--expect pass|fail]\ncargo xtask vm corrupt-root --image ABSOLUTE_IMAGE --out ABSOLUTE_COPY\ncargo xtask e2e --family FAMILY --profile PROFILE [--loader auto|limine|grub] [--root-fs btrfs|ext4] [--keep] [--out ABSOLUTE_DIR]\ncargo xtask env --family FAMILY -- COMMAND...\ne2e apply is gated (D-038); runs end at the apply stage with result.json pass=false."
        ),
        _ => {
            return Err(BoreaError::invalid(
                "unknown xtask or options; run cargo xtask --help",
            ));
        }
    }
    Ok(())
}
fn render(check: bool) -> Result<()> {
    let m = Matrix::embedded()?;
    let mut groups = BTreeMap::<String, BTreeMap<FamilyId, String>>::new();
    for c in &m.cells {
        let tier = match c.tier {
            Tier::Native => "native",
            Tier::ThirdParty => "third-party",
            Tier::Source => "source",
            Tier::Unsupported => "unsupported",
        };
        groups
            .entry(c.component.clone())
            .or_default()
            .insert(c.family, tier.into());
    }
    let mut text = format!(
        "<!-- Generated by cargo xtask matrix render. -->\n# Matrix data ({})\n\nPackage tiers describe draft packaging paths. All cells are VM-untested.\n\n| Component | Arch | Debian | Fedora | openSUSE |\n| --- | --- | --- | --- | --- |\n",
        m.revision
    );
    for (id, families) in groups {
        text += &format!(
            "| {id} | {} | {} | {} | {} |\n",
            families[&FamilyId::Arch],
            families[&FamilyId::Debian],
            families[&FamilyId::Fedora],
            families[&FamilyId::Suse]
        );
    }
    let path = workspace().join("docs/MATRIX.generated.md");
    if check {
        if fs::read_to_string(path)? != text {
            return Err(BoreaError::new(
                "E_MATRIX",
                "generated matrix drift; run cargo xtask matrix render",
            ));
        }
    } else {
        fs::write(path, text)?;
    }
    Ok(())
}
fn cases() -> Vec<(FamilyId, String, Filesystem, Loader)> {
    let mut out = Vec::new();
    for f in FamilyId::ALL {
        for profile in [
            "minimal",
            "kde",
            "gnome",
            "hyprland",
            "hyprland-noctalia",
            "hyprland-dms",
        ] {
            out.push((f, profile.into(), Filesystem::Btrfs, Loader::Auto));
        }
        out.push((f, "minimal-ext4".into(), Filesystem::Ext4, Loader::Auto));
    }
    out.push((
        FamilyId::Arch,
        "minimal-grub".into(),
        Filesystem::Btrfs,
        Loader::Grub,
    ));
    out
}
fn case_config(f: FamilyId, profile: &str, fs: Filesystem, l: Loader) -> Result<Config> {
    let mut c = Config::parse(include_str!("../../../recipes/profiles/minimal.toml"))?;
    c.target.family = f;
    c.disk.root_fs = fs;
    c.boot.loader = l;
    // Codecs are independently tested recipe requests; required session golden cases isolate the session.
    c.apps.codecs = false;
    match profile {
        "omarchy" | "omarchy-custom" => {
            c.session.kind = "omarchy".into();
            c.session.greeter = "auto".into();
            let mut o = borea_core::omarchy::OmarchyConfig::default();
            if profile == "omarchy-custom" {
                o.keep = vec!["spotify".into()];
                o.remove = vec!["docker".into(), "whatsapp".into()];
                o.replace.insert("obsidian".into(), "neovim".into());
                o.default_browser = Some("firefox".into());
            }
            c.omarchy = Some(o);
        }
        "kde" | "gnome" => {
            c.session.kind = "desktop".into();
            c.session.desktop = Some(profile.into());
            c.session.greeter = "auto".into();
        }
        s if s.starts_with("hyprland") => {
            c.session.kind = "shell".into();
            c.session.compositor = Some("hyprland".into());
            c.session.shell = match profile {
                "hyprland-noctalia" => Some("noctalia".into()),
                "hyprland-dms" => Some("dms".into()),
                _ => None,
            };
            c.session.greeter = "auto".into();
        }
        _ => {}
    }
    Ok(c)
}
fn build_case(c: &Config) -> Result<Plan> {
    build_case_for(
        c,
        DeviceIdentity {
            path: c.disk.device.clone(),
            serial: "fixture-disk".into(),
            size: 20 * 1024 * 1024 * 1024,
            sector_size: 512,
        },
    )
}
/// Plan a golden case against a specific disk identity (e2e plans for the image).
fn build_case_for(c: &Config, d: DeviceIdentity) -> Result<Plan> {
    let m = Matrix::embedded()?;
    let l = borea_distro::loader(c, &recipe::family(c.target.family)?, &m)?;
    let s = borea_session::select(c, &m, l, c.session.kind == "omarchy", false)?;
    borea_distro::planner::build(
        c,
        d,
        borea_distro::planner::Intent {
            components: s.components,
            greeter: s.greeter,
            session: s.session,
            autostart: s.autostart,
        },
    )
}
fn golden(update: bool) -> Result<()> {
    let dir = workspace().join("fixtures/plans");
    if update {
        fs::create_dir_all(&dir)?;
    }
    let mut plans = 0;
    let mut refusals = 0;
    for (f, profile, fs, l) in
        cases()
            .into_iter()
            .chain(["omarchy", "omarchy-custom"].into_iter().map(|profile| {
                (
                    FamilyId::Arch,
                    profile.into(),
                    Filesystem::Btrfs,
                    Loader::Auto,
                )
            }))
    {
        let c = case_config(f, &profile, fs, l)?;
        let result = match build_case(&c) {
            Ok(p) => {
                plans += 1;
                let second = build_case(&c)?;
                if p.json()? != second.json()? {
                    return Err(BoreaError::new("E_GOLDEN", "nondeterministic plan"));
                }
                p.json()?
            }
            Err(e) if e.code == "E_UNSUPPORTED" => {
                refusals += 1;
                format!(
                    "{}\n",
                    serde_json::to_string_pretty(
                        &serde_json::json!({"schema":1,"status":"refused","code":e.code,"reason":e.message})
                    )?
                )
            }
            Err(e) => return Err(e),
        };
        let path = dir.join(format!("{}-{profile}.json", f.as_str()));
        if update {
            fs::write(path, result)?;
        } else if fs::read_to_string(&path)? != result {
            return Err(BoreaError::new(
                "E_GOLDEN",
                format!("{} changed; review diff then --update", path.display()),
            ));
        }
    }
    println!(
        "golden: {plans} deterministic draft plans, {refusals} explicit refusal fixtures; 29 required cases plus 2 optional Omarchy cases represented, none VM-validated"
    );
    Ok(())
}
fn verify_matrix() -> Result<()> {
    // No VM orchestrator exists yet, so no user-supplied pass marker may promote a cell.
    let required = cases()
        .into_iter()
        .map(|(f, p, _, _)| format!("{}/{p}", f.as_str()))
        .collect::<Vec<_>>();
    Err(BoreaError::new(
        "E_MATRIX_UNTESTED",
        format!(
            "release gate failed: 0/{} required VM cells validated; no validated artifact producer exists\n{}",
            required.len(),
            required.join("\n")
        ),
    ))
}
fn canary() -> Result<()> {
    let secret = zeroizing_canary();
    let redacted = borea_run::redact(&format!("stderr {secret}"), &[secret.into()]);
    if redacted.contains("canary-not-a-real-password") {
        return Err(BoreaError::new("E_SECRET", "redaction canary leaked"));
    }
    let c = case_config(FamilyId::Arch, "minimal", Filesystem::Btrfs, Loader::Auto)?;
    let p = build_case(&c)?;
    let output = format!(
        "{}{}",
        p.json()?,
        serde_json::to_string(&borea_run::dry_run(&p)?)?
    );
    if output.contains("canary-not-a-real-password") {
        return Err(BoreaError::new("E_SECRET", "plan/dry-run leaked canary"));
    }
    println!(
        "secret redaction and plan/dry-run boundary canary passed; live command capture is not implemented"
    );
    Ok(())
}
fn zeroizing_canary() -> String {
    "canary-not-a-real-password".into()
}
fn disk_create() -> Result<()> {
    let dir = workspace().join("target/disk-selftest");
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("image-{}.img", std::process::id()));
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    let result = (|| -> Result<()> {
        file.set_len(20 * 1024 * 1024 * 1024)?;
        if !file.metadata()?.is_file() {
            return Err(BoreaError::new("E_IMAGE", "image must be regular file"));
        }
        file.sync_all()?;
        Ok(())
    })();
    drop(file);
    fs::remove_file(&path)?;
    result?;
    println!(
        "20 GiB sparse image create/remove passed; no loop attachment, formatting or mount attempted"
    );
    Ok(())
}
fn envrun(args: &[String]) -> Result<()> {
    if args.len() < 4 || args[0] != "--family" || args[2] != "--" {
        return Err(BoreaError::invalid("env --family FAMILY -- COMMAND..."));
    }
    let f = FamilyId::parse(&args[1])?;
    let image = match f {
        FamilyId::Arch => "archlinux:latest",
        FamilyId::Debian => "debian:trixie",
        FamilyId::Fedora => "fedora:43",
        FamilyId::Suse => "opensuse/tumbleweed:latest",
    };
    let root = fs::canonicalize(workspace())?;
    let status = std::process::Command::new("podman")
        .args([
            "run",
            "--rm",
            "--pull=never",
            "--read-only",
            "--network=none",
            "--cap-drop=ALL",
            "--security-opt=no-new-privileges",
            "-v",
        ])
        .arg(format!("{}:/work:ro", root.display()))
        .args(["-w", "/work", image])
        .args(&args[3..])
        .status()?;
    if !status.success() {
        return Err(BoreaError::new(
            "E_STEP",
            "family container failed; prepare the documented tool image first",
        ));
    }
    Ok(())
}

fn disk_storage(filesystem: &str, family: FamilyId) -> Result<()> {
    let cache = std::env::var_os("BOREA_RUNTIME_CACHE")
        .map(PathBuf::from)
        .ok_or_else(|| {
            BoreaError::invalid("set BOREA_RUNTIME_CACHE to the fetched runtime cache")
        })?;
    let temp = tempfile::Builder::new()
        .prefix("borea-disk-selftest-")
        .tempdir_in("/tmp")?;
    let image = temp.path().join("disk.img");
    let manifest = temp.path().join("manifest.json");
    let journal = temp.path().join("journal");
    let lock = borea_core::runtime::RuntimeLock::embedded()?;
    let spec = lock.get(family)?;
    let plan =
        borea_run::image::create(&image, &manifest, filesystem, 20 * 1024 * 1024 * 1024, spec)?;
    let result = (|| -> Result<()> {
        borea_run::image::apply(&plan, spec, &cache, &journal, false, true)?;
        borea_run::image::apply(&plan, spec, &cache, &journal, true, true)?;
        // Emulate a crash after format completed but before its durable `ok`.
        let receipts = fs::read_to_string(&journal)?;
        let pending = format!(
            "{}start:root\n",
            receipts.split("start:root\n").next().unwrap()
        );
        fs::write(&journal, &pending)?;
        borea_run::image::apply(&plan, spec, &cache, &journal, true, true)?;
        // An unknown destructive outcome with a failed UUID probe must not retry.
        let pending_esp = format!(
            "{}start:esp\n",
            receipts.split("start:esp\n").next().unwrap()
        );
        fs::write(&journal, pending_esp)?;
        {
            use std::io::{Read, Seek, SeekFrom, Write};
            let mut file = fs::OpenOptions::new().read(true).write(true).open(&image)?;
            file.seek(SeekFrom::Start(1024 * 1024))?;
            let mut sector = [0; 512];
            file.read_exact(&mut sector)?;
            file.seek(SeekFrom::Start(1024 * 1024))?;
            file.write_all(&[0; 512])?;
            file.sync_all()?;
            if borea_run::image::apply(&plan, spec, &cache, &journal, true, true).is_ok() {
                return Err(BoreaError::new(
                    "E_TEST",
                    "unknown destructive result replayed",
                ));
            }
            let mut unchanged = [1; 512];
            file.seek(SeekFrom::Start(1024 * 1024))?;
            file.read_exact(&mut unchanged)?;
            if unchanged != [0; 512] {
                return Err(BoreaError::new(
                    "E_TEST",
                    "failed probe reformatted the ESP",
                ));
            }
            file.seek(SeekFrom::Start(1024 * 1024))?;
            file.write_all(&sector)?;
            file.sync_all()?;
        }
        fs::write(&journal, &receipts)?;
        let held = fs::File::open(&image)?;
        held.lock()?;
        if borea_run::image::apply(&plan, spec, &cache, &journal, true, true).is_ok() {
            return Err(BoreaError::new("E_TEST", "concurrent image lock accepted"));
        }
        drop(held);
        // Execute two independent images concurrently in separate foreground CLI runs.
        let second = temp.path().join("parallel.img");
        let second_manifest = temp.path().join("parallel.json");
        let second_journal = temp.path().join("parallel.journal");
        borea_run::image::create(&second, &second_manifest, filesystem, plan.bytes, spec)?;
        let exe = std::env::current_exe()?.with_file_name("borea");
        let spawn =
            |manifest: &Path, journal: &Path, resume: bool| -> Result<std::process::Child> {
                let mut cmd = std::process::Command::new(&exe);
                cmd.args(["image", "apply", "--family", family.as_str(), "--manifest"])
                    .arg(manifest)
                    .arg("--cache")
                    .arg(&cache)
                    .arg("--journal")
                    .arg(journal)
                    .arg("--yes");
                if resume {
                    cmd.arg("--resume");
                }
                Ok(cmd.spawn()?)
            };
        let mut first = spawn(&manifest, &journal, true)?;
        let second = spawn(&second_manifest, &second_journal, false);
        let first_status = first.wait()?;
        let second_status = second?.wait()?;
        if !first_status.success() || !second_status.success() {
            return Err(BoreaError::new("E_TEST", "parallel images failed"));
        }
        // SIGKILL the real CLI after it has allocated an image loop mapping.
        let mut interrupted = spawn(&second_manifest, &second_journal, true)?;
        let interrupted_pid = interrupted.id();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
        loop {
            if !image_loops(temp.path())?.is_empty() {
                interrupted.kill()?;
                interrupted.wait()?;
                break;
            }
            if interrupted.try_wait()?.is_some() {
                return Err(BoreaError::new("E_TEST", "missed image crash window"));
            }
            if std::time::Instant::now() > deadline {
                interrupted.kill()?;
                interrupted.wait()?;
                return Err(BoreaError::new(
                    "E_TEST",
                    "timed out waiting for image crash window",
                ));
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        // SIGKILL cannot run user-space temporary-directory destructors. Reclaim
        // only this test child's root-private runtime staging on the ephemeral runner.
        let stage_prefix = format!("borea-image-{interrupted_pid}-");
        for entry in fs::read_dir("/tmp")? {
            let entry = entry?;
            if entry
                .file_name()
                .to_string_lossy()
                .starts_with(&stage_prefix)
            {
                fs::remove_dir_all(entry.path())?;
            }
        }
        for attempt in 0..20 {
            let leaked = !image_loops(temp.path())?.is_empty();
            if !leaked {
                break;
            }
            if attempt == 19 {
                return Err(BoreaError::new(
                    "E_TEST",
                    "loop mapping leaked after image execution",
                ));
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let mut stale = plan.clone();
        stale.inode += 1;
        if borea_run::image::apply(&stale, spec, &cache, &journal, true, true).is_ok() {
            return Err(BoreaError::new("E_TEST", "stale identity accepted"));
        }
        fs::rename(&image, temp.path().join("original.img"))?;
        fs::File::create(&image)?.set_len(plan.bytes)?;
        if borea_run::image::apply(&plan, spec, &cache, &journal, true, true).is_ok() {
            return Err(BoreaError::new("E_TEST", "replacement image accepted"));
        }
        Ok(())
    })();
    if let Ok(receipts) = fs::read_to_string(&journal) {
        println!("image receipts:\n{receipts}");
    }
    result?;
    println!(
        "{}",
        serde_json::json!({"schema":1,"family":family,"filesystem":filesystem,"size_gib":20,"storage":"passed","repeat_probes":"passed","pending_format_resume":"passed","unknown_format":"refused_without_rewrite","exclusive_lock":"passed","parallel_images":"passed","loop_cleanup":"passed","sigkill_loop_cleanup":"passed","stale_identity":"rejected","replacement_image":"rejected","bootable":false})
    );
    Ok(())
}

fn disk_bootstrap(filesystem: &str, image_out: Option<(&Path, Option<&Path>)>) -> Result<()> {
    disk_bootstrap_cached(filesystem, None, image_out)
}

fn disk_bootstrap_cached(
    filesystem: &str,
    pacman_cache: Option<&Path>,
    image_out: Option<(&Path, Option<&Path>)>,
) -> Result<()> {
    let (image_out, plan_out) = match image_out {
        Some((image, plan)) => (Some(image), plan),
        None => (None, None),
    };
    if plan_out.is_some_and(|p| !p.is_absolute() || p.exists()) {
        return Err(BoreaError::invalid(
            "--plan-out requires an absolute path that does not already exist",
        ));
    }
    let cache = std::env::var_os("BOREA_RUNTIME_CACHE")
        .map(PathBuf::from)
        .ok_or_else(|| {
            BoreaError::invalid("set BOREA_RUNTIME_CACHE to the fetched runtime cache")
        })?;
    let temp = tempfile::Builder::new()
        .prefix("borea-bootstrap-selftest-")
        .tempdir_in("/tmp")?;
    let image = image_out
        .map(Path::to_path_buf)
        .unwrap_or_else(|| temp.path().join("disk.img"));
    if image_out.is_some() && (!image.is_absolute() || image.exists()) {
        return Err(BoreaError::invalid(
            "--out requires an absolute path that does not already exist",
        ));
    }
    let manifest = temp.path().join("manifest.json");
    let storage_journal = temp.path().join("storage.journal");
    let bootstrap_journal = temp.path().join("bootstrap.journal");
    let password_path = temp.path().join("bootstrap.password");
    let mut password_file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&password_path)?;
    let canary = "borea-bootstrap-ci-canary-not-a-real-password";
    password_file.write_all(canary.as_bytes())?;
    password_file.write_all(b"\n")?;
    password_file.sync_all()?;
    let lock = borea_core::runtime::RuntimeLock::embedded()?;
    let spec = lock.get(FamilyId::Arch)?;
    let plan =
        borea_run::image::create(&image, &manifest, filesystem, 20 * 1024 * 1024 * 1024, spec)?;
    borea_run::image::apply(&plan, spec, &cache, &storage_journal, false, true)?;
    borea_run::image::bootstrap(
        &plan,
        spec,
        &cache,
        &bootstrap_journal,
        borea_run::image::BootstrapOptions {
            password_file: &password_path,
            resume: false,
            yes: true,
            pacman_cache,
        },
    )?;
    borea_run::image::bootstrap(
        &plan,
        spec,
        &cache,
        &bootstrap_journal,
        borea_run::image::BootstrapOptions {
            password_file: &password_path,
            resume: true,
            yes: true,
            pacman_cache,
        },
    )?;
    let receipts = fs::read_to_string(&bootstrap_journal)?;
    if receipts.contains(canary) {
        return Err(BoreaError::new(
            "E_SECRET",
            "bootstrap password appeared in the image receipt",
        ));
    }
    let report = serde_json::to_string(&serde_json::json!({
        "schema": 1,
        "family": "arch",
        "filesystem": filesystem,
        "size_gib": 20,
        "storage": "passed",
        "bootstrap": "passed",
        "system_config": "passed",
        "initramfs": "passed",
        "limine_bootloader": "passed",
        "repeat_probes": "passed",
        "bootloader_configured": true,
        "bootable": false,
        "vm_boot_verified": false,
        "release": borea_core::RELEASE
    }))?;
    if report.contains(canary) {
        return Err(BoreaError::new(
            "E_SECRET",
            "bootstrap password appeared in the selftest report",
        ));
    }
    if let Some(plan_out) = plan_out {
        let verify_plan = vm::verify_plan(&plan)?;
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(plan_out)?
            .write_all(verify_plan.json()?.as_bytes())?;
    }
    println!("bootstrap receipts:\n{receipts}");
    println!("{report}");
    Ok(())
}

fn image_loops(root: &Path) -> Result<Vec<PathBuf>> {
    let prefix = root.to_string_lossy();
    let mut loops = Vec::new();
    for entry in fs::read_dir("/sys/block")? {
        let entry = entry?;
        if !entry.file_name().to_string_lossy().starts_with("loop") {
            continue;
        }
        match fs::read_to_string(entry.path().join("loop/backing_file")) {
            Ok(backing) if backing.contains(prefix.as_ref()) => loops.push(entry.path()),
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(loops)
}
