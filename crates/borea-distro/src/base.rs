//! C06 base target configuration: the hardening steps and the verify checks
//! for every base item. The planner owns step order; this module owns what
//! each base item means on the target and how to prove it.

use borea_core::{recipe::FamilyRecipe, *};

/// A base step for the planner to append to the base phase.
pub(crate) struct BaseStep {
    pub id: &'static str,
    pub action: Action,
    pub probe: Probe,
}

fn sh(script: &str) -> Vec<String> {
    vec!["sh".into(), "-c".into(), script.into()]
}

/// Root has a locked password: the shadow field starts with `!`.
const ROOT_LOCKED: &str = "getent shadow root | cut -d: -f2 | grep -q '^!'";
/// A valid, initialised machine id.
const MACHINE_ID: &str = "grep -Eqx '[0-9a-f]{32}' /etc/machine-id";
/// Persistent journal with a bounded size.
pub(crate) const JOURNALD_PATH: &str = "/etc/systemd/journald.conf.d/10-borea.conf";
pub(crate) const JOURNALD: &str = "[Journal]\nStorage=persistent\nSystemMaxUse=500M\n";

/// Steps beyond identity, account and network that every base install gets.
pub(crate) fn hardening() -> Vec<BaseStep> {
    vec![
        // The account's sudo rule replaces root logins. `usermod --lock` is
        // in shadow on every family and keeps key-based access (sshd uses PAM).
        BaseStep {
            id: "base.root.lock",
            action: Action::Command {
                argv: vec!["usermod".into(), "--lock".into(), "root".into()],
                chroot: true,
                stdin: None,
            },
            probe: Probe::CommandOk {
                argv: sh(ROOT_LOCKED),
                chroot: true,
            },
        },
        // Each install gets its own id; a missing or empty file is created.
        BaseStep {
            id: "base.machine-id",
            action: Action::Command {
                argv: vec!["systemd-machine-id-setup".into()],
                chroot: true,
                stdin: None,
            },
            probe: Probe::CommandOk {
                argv: sh(MACHINE_ID),
                chroot: true,
            },
        },
        BaseStep {
            id: "base.journald",
            action: Action::WriteFile {
                path: JOURNALD_PATH.into(),
                contents: JOURNALD.into(),
                mode: 0o644,
            },
            probe: Probe::FileContains {
                path: JOURNALD_PATH.into(),
                needle: JOURNALD.into(),
            },
        },
    ]
}

/// One verify check per C06 base item. Hostname, account and network checks
/// already exist in the planner; these cover the rest.
pub(crate) fn checks(c: &Config, family: &FamilyRecipe) -> Vec<Check> {
    let t = &c.target;
    vec![
        Check::FileContains {
            path: "/etc/locale.conf".into(),
            needle: format!("LANG={}", t.locale),
        },
        Check::CommandOk {
            argv: vec![
                "test".into(),
                format!("/usr/share/zoneinfo/{}", t.timezone),
                "-ef".into(),
                "/etc/localtime".into(),
            ],
        },
        Check::FileContains {
            path: "/etc/vconsole.conf".into(),
            needle: format!("KEYMAP={}", t.keymap),
        },
        Check::CommandOk {
            argv: sh(&format!(
                "id -nG {} | tr ' ' '\\n' | grep -qx {}",
                c.account.username, family.sudo_group
            )),
        },
        Check::CommandOk {
            argv: vec![
                "visudo".into(),
                "-c".into(),
                "-f".into(),
                "/etc/sudoers.d/10-borea".into(),
            ],
        },
        Check::CommandOk {
            argv: sh(ROOT_LOCKED),
        },
        Check::CommandOk {
            argv: sh(MACHINE_ID),
        },
        Check::FileContains {
            path: JOURNALD_PATH.into(),
            needle: JOURNALD.into(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Config {
        let mut c = Config::parse(include_str!("../../../recipes/profiles/minimal.toml")).unwrap();
        c.apps.codecs = false;
        c
    }

    #[test]
    fn base_hardening_steps_are_probed_and_idempotent() {
        let steps = hardening();
        let ids: Vec<_> = steps.iter().map(|s| s.id).collect();
        assert_eq!(ids, ["base.root.lock", "base.machine-id", "base.journald"]);
        for s in &steps {
            assert!(!matches!(s.probe, Probe::None), "{}", s.id);
        }
        // The lock probe matches what `usermod --lock` writes, so a second run skips.
        assert!(matches!(
            &steps[0].probe,
            Probe::CommandOk { argv, chroot: true } if argv[2].contains("grep -q '^!'")
        ));
    }

    #[test]
    fn base_checks_cover_every_c06_item() {
        for f in FamilyId::ALL {
            let mut c = config();
            c.target.family = f;
            let family = borea_core::recipe::family(f).unwrap();
            let checks = checks(&c, &family);
            let text = serde_json::to_string(&checks).unwrap();
            for needle in [
                "LANG=en_US.UTF-8",
                "/usr/share/zoneinfo/UTC",
                "KEYMAP=us",
                &format!("grep -qx {}", family.sudo_group),
                "visudo",
                "getent shadow root",
                "/etc/machine-id",
                "Storage=persistent",
            ] {
                assert!(text.contains(needle), "{f:?}: {needle}");
            }
        }
    }
}
