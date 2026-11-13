use borea_core::*;
use std::{fs, process::Command};
fn borea() -> Command {
    Command::new(env!("CARGO_BIN_EXE_borea"))
}
fn fixture_plan() -> Plan {
    let mut p = Plan {
        schema: 2,
        id: String::new(),
        created: "1970-01-01T00:00:00Z".into(),
        borea_version: version().into(),
        digest: String::new(),
        config_digest: digest(&"fixture").unwrap(),
        matrix_digest: digest(&matrix::Matrix::embedded().unwrap()).unwrap(),
        family: FamilyId::Arch,
        disk: DeviceIdentity {
            path: "/dev/vda".into(),
            serial: "fixture".into(),
            size: 20 * 1024 * 1024 * 1024,
            sector_size: 512,
        },
        root_fs: Filesystem::Btrfs,
        loader: Loader::Limine,
        root: "/mnt".into(),
        steps: vec![Step {
            id: "fixture.hostname".into(),
            title: "Write fixture hostname".into(),
            action: Action::WriteFile {
                path: "/etc/hostname".into(),
                contents: "fixture\n".into(),
                mode: 0o644,
            },
            needs: vec![],
            probe: Probe::FileContains {
                path: "/etc/hostname".into(),
                needle: "fixture\n".into(),
            },
            danger: Danger::Mutating,
            timeout_s: 10,
            phase: Phase::Base,
        }],
        checks: vec![Check::FileContains {
            path: "/etc/hostname".into(),
            needle: "fixture\n".into(),
        }],
        secrets: vec![],
        notes: vec![],
    };
    p.seal().unwrap();
    p
}
#[test]
fn fixture_apply_resume_verify_and_stale_journal() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("root");
    fs::create_dir(&root).unwrap();
    let path = d.path().join("plan.json");
    let journal = d.path().join("journal.jsonl");
    let mut p = fixture_plan();
    fs::write(&path, p.json().unwrap()).unwrap();
    let run = |resume: bool| {
        let mut cmd = borea();
        cmd.arg("apply")
            .arg(&path)
            .arg("--fixture-root")
            .arg(&root)
            .arg("--journal")
            .arg(&journal);
        if resume {
            cmd.arg("--resume");
        }
        cmd.output().unwrap()
    };
    assert!(run(false).status.success());
    assert_eq!(
        fs::read_to_string(root.join("etc/hostname")).unwrap(),
        "fixture\n"
    );
    assert!(run(true).status.success());
    assert!(fs::read_to_string(&journal).unwrap().contains("skipped"));
    assert!(
        borea()
            .arg("verify")
            .arg(&path)
            .arg("--root")
            .arg(&root)
            .output()
            .unwrap()
            .status
            .success()
    );
    fs::write(root.join("etc/hostname"), "mutated").unwrap();
    assert_eq!(
        borea()
            .arg("verify")
            .arg(&path)
            .arg("--root")
            .arg(&root)
            .output()
            .unwrap()
            .status
            .code(),
        Some(5)
    );
    p.config_digest = digest(&"different").unwrap();
    p.seal().unwrap();
    fs::write(&path, p.json().unwrap()).unwrap();
    assert!(!run(true).status.success());
}
#[test]
fn dry_run_never_writes() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("plan.json");
    fs::write(&p, fixture_plan().json().unwrap()).unwrap();
    let before = fs::read_dir(d.path()).unwrap().count();
    let o = borea()
        .arg("apply")
        .arg(&p)
        .arg("--dry-run")
        .arg("--json")
        .output()
        .unwrap();
    assert!(o.status.success());
    assert!(
        String::from_utf8(o.stdout)
            .unwrap()
            .contains("\"executed\": false")
    );
    assert_eq!(before, fs::read_dir(d.path()).unwrap().count());
}
#[test]
fn live_apply_is_an_explicit_gate() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("plan.json");
    fs::write(&p, fixture_plan().json().unwrap()).unwrap();
    let o = borea()
        .arg("apply")
        .arg(&p)
        .args(["--yes", "--device", "/dev/vda"])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(3));
    assert!(
        String::from_utf8(o.stderr)
            .unwrap()
            .contains("live apply is not enabled")
    );
}
#[test]
fn cli_invalid_arguments_and_unsupported_family() {
    assert_eq!(
        borea()
            .args(["catalogue", "--family", "unknown"])
            .output()
            .unwrap()
            .status
            .code(),
        Some(3)
    );
    assert_eq!(
        borea()
            .args(["catalogue", "--family", "arch", "--typo"])
            .output()
            .unwrap()
            .status
            .code(),
        Some(2)
    );
}
#[test]
fn omarchy_preview_and_browser_gate() {
    let o = borea()
        .args(["omarchy", "trim", "docker", "--dry-run", "--json"])
        .output()
        .unwrap();
    assert!(o.status.success());
    assert!(
        String::from_utf8(o.stdout)
            .unwrap()
            .contains("omarchy-pkg-drop")
    );
    assert_eq!(
        borea()
            .args(["omarchy", "trim", "chromium", "--dry-run"])
            .output()
            .unwrap()
            .status
            .code(),
        Some(3)
    );
}
#[test]
fn fixture_symlink_refused_before_journal() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("root");
    fs::create_dir(&root).unwrap();
    std::os::unix::fs::symlink("/etc", root.join("etc")).unwrap();
    let p = d.path().join("plan.json");
    let journal = d.path().join("journal.jsonl");
    fs::write(&p, fixture_plan().json().unwrap()).unwrap();
    let o = borea()
        .arg("apply")
        .arg(&p)
        .arg("--fixture-root")
        .arg(root)
        .arg("--journal")
        .arg(&journal)
        .output()
        .unwrap();
    assert!(!o.status.success());
    assert!(!journal.exists());
}
#[test]
fn fixture_plan_command_and_no_overwrite() {
    let d = tempfile::tempdir().unwrap();
    let config = d.path().join("borea.toml");
    let identity = d.path().join("identity.json");
    let output = d.path().join("plan.json");
    fs::write(
        &config,
        include_str!("../../../recipes/profiles/minimal.toml"),
    )
    .unwrap();
    fs::write(&identity, serde_json::to_vec(&fixture_plan().disk).unwrap()).unwrap();
    let run = || {
        borea()
            .arg("plan")
            .arg("--config")
            .arg(&config)
            .arg("--fixture")
            .arg("--identity")
            .arg(&identity)
            .arg("--out")
            .arg(&output)
            .output()
            .unwrap()
    };
    let result = run();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let p: Plan = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
    p.validate().unwrap();
    assert!(!run().status.success());
}

fn omarchy_config() -> String {
    include_str!("../../../examples/omarchy.toml").to_string()
}
fn omarchy_plan(text: &str, allow_source: bool) -> (std::process::Output, Option<Plan>) {
    let d = tempfile::tempdir().unwrap();
    let config = d.path().join("config.toml");
    let identity = d.path().join("identity.json");
    let plan = d.path().join("plan.json");
    fs::write(&config, text).unwrap();
    fs::write(&identity, serde_json::to_vec(&fixture_plan().disk).unwrap()).unwrap();
    let mut cmd = borea();
    cmd.arg("plan")
        .arg("--config")
        .arg(config)
        .arg("--fixture")
        .arg("--identity")
        .arg(identity)
        .arg("--out")
        .arg(&plan);
    if allow_source {
        cmd.arg("--allow-source");
    }
    let output = cmd.output().unwrap();
    let parsed = if plan.exists() {
        Some(serde_json::from_slice(&fs::read(plan).unwrap()).unwrap())
    } else {
        None
    };
    (output, parsed)
}
#[test]
fn omarchy_default_profile_keeps_apps_and_requires_source_opt_in() {
    let text = omarchy_config();
    let (out, p) = omarchy_plan(&text, false);
    assert_eq!(out.status.code(), Some(3));
    assert!(p.is_none());
    assert!(String::from_utf8_lossy(&out.stderr).contains("--allow-source"));
    let (out, p) = omarchy_plan(&text, true);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let p = p.unwrap();
    assert_eq!(p.schema, 2);
    assert_eq!(p.family, FamilyId::Arch);
    assert!(
        p.steps
            .iter()
            .any(|s| matches!(s.action, Action::InstallOmarchy { .. }))
    );
    assert!(
        !p.steps
            .iter()
            .any(|s| s.id.starts_with("omarchy.apps.remove.")
                || s.id.starts_with("omarchy.webapps.remove."))
    );
    let policy = p
        .steps
        .iter()
        .find(|s| s.id == "omarchy.defaults.policy")
        .unwrap();
    if let Action::WriteFile { contents, .. } = &policy.action {
        let v: serde_json::Value = serde_json::from_str(contents).unwrap();
        assert_eq!(v["items"], serde_json::json!([]));
    } else {
        panic!("policy must be a declarative file")
    }
}
#[test]
fn omarchy_custom_plan_only_changes_selected_apps() {
    let text = omarchy_config()
        .replace("keep = []", "keep = [\"spotify\", \"chromium\"]")
        .replace("remove = []", "remove = [\"docker\", \"whatsapp\"]")
        .replace(
            "# default_browser = \"firefox\"",
            "default_browser = \"firefox\"",
        )
        + "\n[omarchy.replace]\nobsidian = \"neovim\"\n";
    let (out, p) = omarchy_plan(&text, true);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let p = p.unwrap();
    let ids = p
        .steps
        .iter()
        .filter(|s| {
            s.id.starts_with("omarchy.apps.remove.") || s.id.starts_with("omarchy.webapps.remove.")
        })
        .map(|s| s.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        [
            "omarchy.apps.remove.docker",
            "omarchy.apps.remove.obsidian",
            "omarchy.webapps.remove.whatsapp"
        ]
    );
    let install = p
        .steps
        .iter()
        .position(|s| s.id == "omarchy.apps.install")
        .unwrap();
    let remove = p
        .steps
        .iter()
        .position(|s| s.id == "omarchy.apps.remove.obsidian")
        .unwrap();
    assert!(install < remove);
    let docker = p
        .steps
        .iter()
        .find(|s| s.id == "omarchy.apps.remove.docker")
        .unwrap();
    if let Action::Command { argv, .. } = &docker.action {
        assert_eq!(&argv[..3], ["pacman", "-R", "--noconfirm"]);
        assert!(!argv.iter().any(|a| a.contains("nodeps") || a == "-Rns"));
    }
    assert!(p.checks.iter().any(
        |c| matches!(c,Check::PackagesAbsent{packages} if packages.contains(&"obsidian".into()))
    ));
}
#[test]
fn omarchy_wrong_family_layout_and_conflicts_fail_before_output() {
    for text in [
        omarchy_config().replace("family = \"arch\"", "family = \"debian\""),
        omarchy_config().replace("root_fs = \"btrfs\"", "root_fs = \"ext4\""),
        omarchy_config().replace("loader = \"auto\"", "loader = \"grub\""),
        omarchy_config()
            .replace("keep = []", "keep = [\"docker\"]")
            .replace("remove = []", "remove = [\"docker\"]"),
        omarchy_config().replace("remove = []", "remove = [\"chromium\"]"),
    ] {
        let (o, p) = omarchy_plan(&text, true);
        assert!(!o.status.success());
        assert!(p.is_none());
    }
}
#[test]
fn omarchy_configure_is_read_only_and_shows_preserved_apps() {
    let d = tempfile::tempdir().unwrap();
    let config = d.path().join("config.toml");
    fs::write(&config, omarchy_config()).unwrap();
    let o = borea()
        .args(["omarchy", "configure", "--config"])
        .arg(&config)
        .arg("--json")
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["executed"], false);
    assert_eq!(v["selection"]["preserve_unlisted"], true);
    assert_eq!(v["selection"]["chromium_preserved"], true);
    assert_eq!(fs::read_dir(d.path()).unwrap().count(), 1);
}

#[test]
fn runtime_list_and_offline_miss() {
    let o = borea()
        .args(["runtime", "list", "--json"])
        .output()
        .unwrap();
    assert!(o.status.success());
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["runtimes"].as_array().unwrap().len(), 4);
    let d = tempfile::tempdir().unwrap();
    let o = borea()
        .args([
            "runtime",
            "fetch",
            "--family",
            "debian",
            "--offline",
            "--cache",
        ])
        .arg(d.path())
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&o.stderr).contains("E_RUNTIME_OFFLINE"));
}
#[test]
fn doctor_reports_missing_runtime_without_host_package_manager_fallback() {
    let d = tempfile::tempdir().unwrap();
    let o = borea()
        .args(["doctor", "--family", "arch", "--cache"])
        .arg(d.path())
        .arg("--json")
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(3));
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["schema"], 2);
    assert_eq!(v["runtime"]["verified"], false);
    assert!(
        v["runtime"]["remedy"]
            .as_str()
            .unwrap()
            .contains("runtime fetch")
    );
}
