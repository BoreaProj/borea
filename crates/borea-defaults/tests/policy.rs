use borea_core::defaults::*;
use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
};
fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_borea-defaults"))
}
#[test]
fn process_policy_tracks_update_and_user_restore() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("defaults.json");
    let mut p = DefaultsPolicy::new("tester".into());
    p.items.push(default_item("docker").unwrap());
    fs::write(&path, serde_json::to_vec(&p).unwrap()).unwrap();
    for (window, state) in [
        ("update", ItemState::Reintroduced),
        ("user", ItemState::Restored),
        ("update", ItemState::Restored),
    ] {
        let mut child = binary()
            .args(["note-install", "--window", window, "--policy"])
            .arg(&path)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(b"docker\n").unwrap();
        assert!(child.wait().unwrap().success());
        let p: DefaultsPolicy = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(p.items[0].state, state);
    }
    assert!(
        binary()
            .args(["status", "--policy"])
            .arg(&path)
            .output()
            .unwrap()
            .status
            .success()
    );
}
#[test]
fn enforce_never_removes_without_pins() {
    assert_eq!(
        binary().arg("enforce").output().unwrap().status.code(),
        Some(3)
    );
}
