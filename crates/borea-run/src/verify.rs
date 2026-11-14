use borea_core::*;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
#[derive(Debug, Serialize)]
pub struct CheckResult {
    pub check: Check,
    pub ok: bool,
    pub detail: String,
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub schema: u16,
    pub plan: String,
    pub passed: bool,
    pub checks: Vec<CheckResult>,
}
pub fn offline(plan: &Plan, root: &Path) -> Result<Report> {
    plan.validate()?;
    if !root.is_dir() {
        return Err(BoreaError::invalid(
            "verify root must be an existing directory",
        ));
    }
    let mut checks = Vec::new();
    for check in &plan.checks {
        let result: Result<bool> = match check {
            Check::FileExists { path } => crate::rooted(root, path).map(|p| p.exists()),
            Check::PathMissing { path } => crate::rooted(root, path).map(|p| !p.exists()),
            Check::FileContains { path, needle } => crate::rooted(root, path).and_then(|p| {
                fs::read_to_string(p)
                    .map(|s| s.contains(needle))
                    .map_err(BoreaError::from)
            }),
            Check::PackagesAbsent { .. } => Err(BoreaError::unsupported(
                "package checks require online --ssh verification; offline mode never executes target binaries",
            )),
            Check::CommandOk { .. } => Err(BoreaError::unsupported(
                "command checks require online --ssh verification; offline mode never executes target binaries",
            )),
        };
        let (ok, detail) = match result {
            Ok(true) => (true, "ok".into()),
            Ok(false) => (false, "check not satisfied".into()),
            Err(e) => (false, e.to_string()),
        };
        checks.push(CheckResult {
            check: check.clone(),
            ok,
            detail,
        });
    }
    Ok(Report {
        schema: 1,
        plan: plan.digest.clone(),
        passed: checks.iter().all(|c| c.ok),
        checks,
    })
}
fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
/// Connection settings for online verification. `known_hosts` pins host keys to
/// a caller-owned file and accepts only a first key for a new host there.
#[derive(Debug, Default, Clone)]
pub struct SshOptions {
    pub port: Option<u16>,
    pub identity: Option<PathBuf>,
    pub known_hosts: Option<PathBuf>,
}
impl SshOptions {
    fn validate(&self) -> Result<()> {
        if self.port == Some(0) {
            return Err(BoreaError::invalid("SSH port must be 1-65535"));
        }
        for path in [&self.identity, &self.known_hosts].into_iter().flatten() {
            let text = path.to_string_lossy();
            if !path.is_absolute() || text.contains([',', '"', ' ', '\n']) {
                return Err(BoreaError::invalid(
                    "SSH identity and known-hosts paths must be absolute without spaces, quotes or commas",
                ));
            }
        }
        Ok(())
    }
    fn command(&self, host: &str, remote: &str) -> Command {
        let mut cmd = Command::new("ssh");
        cmd.args(["-o", "BatchMode=yes", "-o", "ConnectTimeout=10"]);
        if let Some(port) = self.port {
            cmd.arg("-p").arg(port.to_string());
        }
        if let Some(identity) = &self.identity {
            cmd.args(["-o", "IdentitiesOnly=yes", "-i"]).arg(identity);
        }
        if let Some(known_hosts) = &self.known_hosts {
            cmd.arg("-o")
                .arg(format!("UserKnownHostsFile={}", known_hosts.display()))
                .args(["-o", "StrictHostKeyChecking=accept-new"]);
        }
        cmd.args(["--", host, remote]);
        cmd
    }
}
pub fn ssh(plan: &Plan, host: &str) -> Result<Report> {
    ssh_with(plan, host, &SshOptions::default())
}
pub fn ssh_with(plan: &Plan, host: &str, options: &SshOptions) -> Result<Report> {
    plan.validate()?;
    options.validate()?;
    if host.is_empty()
        || host.starts_with('-')
        || !host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"@._-:[]".contains(&b))
    {
        return Err(BoreaError::invalid("invalid SSH host"));
    }
    let mut checks = Vec::new();
    for check in &plan.checks {
        if let Check::PackagesAbsent { packages } = check {
            // Query the complete installed set once; query errors must never count as absence.
            let output = options.command(host, "pacman -Qq").output()?;
            let installed = String::from_utf8_lossy(&output.stdout);
            let ok = output.status.success()
                && packages
                    .iter()
                    .all(|p| !installed.lines().any(|line| line == p));
            checks.push(CheckResult {
                check: check.clone(),
                ok,
                detail: if ok {
                    "selected packages absent".into()
                } else {
                    "package query failed or selected packages remain installed".into()
                },
            });
            continue;
        }
        let args = match check {
            Check::FileExists { path } => vec![
                "test".into(),
                "-e".into(),
                path.to_string_lossy().into_owned(),
            ],
            Check::PathMissing { path } => vec![
                "test".into(),
                "!".into(),
                "-e".into(),
                path.to_string_lossy().into_owned(),
            ],
            Check::FileContains { path, needle } => vec![
                "grep".into(),
                "-Fq".into(),
                "--".into(),
                needle.clone(),
                path.to_string_lossy().into_owned(),
            ],
            Check::CommandOk { argv } => argv.clone(),
            Check::PackagesAbsent { .. } => unreachable!(),
        };
        let remote = args.iter().map(|s| quote(s)).collect::<Vec<_>>().join(" ");
        let status = options.command(host, &remote).status()?;
        checks.push(CheckResult {
            check: check.clone(),
            ok: status.success(),
            detail: format!("ssh exit {}", status.code().unwrap_or(-1)),
        });
    }
    Ok(Report {
        schema: 1,
        plan: plan.digest.clone(),
        passed: checks.iter().all(|c| c.ok),
        checks,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shell_arguments_are_quoted() {
        assert_eq!(quote("a'b;$()"), "'a'\\''b;$()'");
    }
    #[test]
    fn ssh_options_build_pinned_command() {
        let options = SshOptions {
            port: Some(2222),
            identity: Some("/tmp/key".into()),
            known_hosts: Some("/tmp/known".into()),
        };
        options.validate().unwrap();
        let cmd = options.command("root@127.0.0.1", "true");
        let args: Vec<_> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            [
                "-o",
                "BatchMode=yes",
                "-o",
                "ConnectTimeout=10",
                "-p",
                "2222",
                "-o",
                "IdentitiesOnly=yes",
                "-i",
                "/tmp/key",
                "-o",
                "UserKnownHostsFile=/tmp/known",
                "-o",
                "StrictHostKeyChecking=accept-new",
                "--",
                "root@127.0.0.1",
                "true"
            ]
        );
    }
    #[test]
    fn ssh_options_reject_unsafe_paths_and_port() {
        for options in [
            SshOptions {
                port: Some(0),
                ..Default::default()
            },
            SshOptions {
                identity: Some("relative".into()),
                ..Default::default()
            },
            SshOptions {
                known_hosts: Some("/tmp/a b".into()),
                ..Default::default()
            },
        ] {
            assert!(options.validate().is_err());
        }
    }
}
