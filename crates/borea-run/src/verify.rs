use borea_core::*;
use serde::Serialize;
use std::{fs, path::Path, process::Command};
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
pub fn ssh(plan: &Plan, host: &str) -> Result<Report> {
    plan.validate()?;
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
        };
        let remote = args.iter().map(|s| quote(s)).collect::<Vec<_>>().join(" ");
        let status = Command::new("ssh")
            .args([
                "-o",
                "BatchMode=yes",
                "-o",
                "ConnectTimeout=10",
                host,
                &remote,
            ])
            .status()?;
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
}
