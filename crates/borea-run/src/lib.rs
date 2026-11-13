//! Execution boundaries, append-only journals and offline verification.
pub mod journal;
pub mod omarchy;
pub mod runtime;
pub mod verify;
use borea_core::*;
use std::{
    collections::BTreeSet,
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;
pub fn read_secret(path: &Path) -> Result<Zeroizing<String>> {
    let text = Zeroizing::new(fs::read_to_string(path)?);
    let value = text.trim_end_matches(['\n', '\r']);
    if value.is_empty() || value.contains(['\0', '\n', '\r']) {
        return Err(BoreaError::invalid(
            "secret must be one nonempty line without NUL",
        ));
    }
    Ok(Zeroizing::new(value.to_string()))
}
pub fn redact(text: &str, secrets: &[Zeroizing<String>]) -> String {
    let mut out = text.to_string();
    let mut secrets = secrets.iter().filter(|s| !s.is_empty()).collect::<Vec<_>>();
    secrets.sort_by_key(|s| std::cmp::Reverse(s.len()));
    for s in secrets {
        out = out.replace(s.as_str(), "<redacted>");
    }
    out
}
/// Resolve a path inside a fixture root, refusing every symlink component.
pub fn rooted(root: &Path, path: &Path) -> Result<PathBuf> {
    if !safe_path(path) || !root.is_absolute() || root == Path::new("/") {
        return Err(BoreaError::new(
            "E_PATH",
            "absolute non-host root and safe target path required",
        ));
    }
    let root = fs::canonicalize(root)?;
    if root == Path::new("/") {
        return Err(BoreaError::new("E_PATH", "host root forbidden"));
    }
    let mut dest = root;
    for c in path
        .strip_prefix("/")
        .map_err(|_| BoreaError::new("E_PATH", "absolute target path required"))?
        .components()
    {
        dest.push(c);
        match fs::symlink_metadata(&dest) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(BoreaError::new(
                    "E_PATH",
                    "symlink target refused; use a real fixture directory",
                ));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(dest)
}
pub fn atomic_write(path: &Path, bytes: &[u8], mode: u32) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| BoreaError::new("E_PATH", "missing parent"))?;
    fs::create_dir_all(parent)?;
    let temp = parent.join(format!(".borea-{}.tmp", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(&temp)?;
    let result = (|| -> Result<()> {
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::set_permissions(&temp, fs::Permissions::from_mode(mode))?;
        fs::rename(&temp, path)?;
        std::fs::File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
pub fn dry_run(plan: &Plan) -> Result<Vec<serde_json::Value>> {
    plan.validate()?;
    Ok(plan.steps.iter().map(|s|serde_json::json!({"step":s.id,"phase":s.phase,"danger":s.danger,"action":s.action,"probe":s.probe,"executed":false})).collect())
}
/// A real installation remains a hard gate until the image/VM lanes validate it.
/// The fixture executor below only accepts file operations in a separate root.
pub fn apply_live(plan: &Plan, yes: bool, device: &Path) -> Result<()> {
    plan.validate()?;
    if !yes || device != plan.disk.path {
        return Err(BoreaError::new(
            "E_CONFIRM",
            "apply requires --yes and --device matching the plan exactly",
        ));
    }
    Err(BoreaError::unsupported(
        "live apply is not enabled in the first implementation: storage identity rechecks, UUID/layout probes, chroot lifecycle and boot adapters require image and VM validation; use --dry-run, never run these draft commands by hand on a real disk",
    ))
}
pub fn apply_fixture(plan: &Plan, root: &Path, journal_path: &Path, resume: bool) -> Result<()> {
    plan.validate()?;
    if !root.is_dir() {
        return Err(BoreaError::new("E_PATH", "fixture root must already exist"));
    }
    if plan.steps.iter().any(|s| {
        s.danger == Danger::Destructive
            || !matches!(
                s.action,
                Action::WriteFile { .. } | Action::Mkdir { .. } | Action::CopyIn { .. }
            )
            || matches!(s.probe, Probe::CommandOk { .. } | Probe::None)
    }) {
        return Err(BoreaError::unsupported(
            "fixture apply accepts file-only plans; installer plans require the image/VM lane",
        ));
    }
    // Validate confinement for the complete plan before opening the journal or writing.
    for step in &plan.steps {
        match &step.action {
            Action::WriteFile { path, .. } | Action::Mkdir { path, .. } => {
                rooted(root, path)?;
            }
            Action::CopyIn { dest, .. } => {
                rooted(root, dest)?;
            }
            _ => unreachable!(),
        };
    }
    let mut journal = journal::Journal::open(journal_path, plan, resume)?;
    let mut done = BTreeSet::new();
    for step in &plan.steps {
        if step.needs.iter().any(|id| !done.contains(id)) {
            return Err(BoreaError::new("E_PLAN_ORDER", "dependency incomplete"));
        }
        let skip = probe(root, &step.probe)?;
        if skip {
            journal.record(&step.id, "skipped", None)?;
            done.insert(step.id.clone());
            continue;
        }
        journal.record(&step.id, "start", None)?;
        let result = match &step.action {
            Action::WriteFile {
                path,
                contents,
                mode,
            } => atomic_write(&rooted(root, path)?, contents.as_bytes(), *mode),
            Action::CopyIn { source, dest, mode } => fs::read(source)
                .map_err(BoreaError::from)
                .and_then(|bytes| atomic_write(&rooted(root, dest)?, &bytes, *mode)),
            Action::Mkdir { path, mode } => {
                let p = rooted(root, path)?;
                fs::create_dir_all(&p)?;
                fs::set_permissions(&p, fs::Permissions::from_mode(*mode))?;
                Ok(())
            }
            _ => unreachable!(),
        };
        match result {
            Ok(()) => {
                if !probe(root, &step.probe)? {
                    journal.record(&step.id, "failed", Some("postcondition failed"))?;
                    return Err(BoreaError::new(
                        "E_STEP",
                        format!("{} postcondition failed", step.id),
                    ));
                }
                journal.record(&step.id, "ok", None)?;
                done.insert(step.id.clone());
            }
            Err(e) => {
                journal.record(&step.id, "failed", Some(&e.to_string()))?;
                return Err(e);
            }
        }
    }
    Ok(())
}
fn probe(root: &Path, p: &Probe) -> Result<bool> {
    match p {
        Probe::FileExists { path } => Ok(rooted(root, path)?.exists()),
        Probe::PathMissing { path } => Ok(!rooted(root, path)?.exists()),
        Probe::FileContains { path, needle } => match fs::read_to_string(rooted(root, path)?) {
            Ok(s) => Ok(s.contains(needle)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e.into()),
        },
        _ => Err(BoreaError::unsupported(
            "probe unavailable in fixture mode; use file probes",
        )),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn secret_canary() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("pass");
        fs::write(&p, "canary-not-a-real-password\n").unwrap();
        let s = read_secret(&p).unwrap();
        assert_eq!(
            redact("error: canary-not-a-real-password", &[s]),
            "error: <redacted>"
        );
    }
    #[test]
    fn secret_invalid() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("pass");
        for text in ["", "a\nb", "a\0b"] {
            fs::write(&p, text).unwrap();
            assert!(read_secret(&p).is_err());
        }
    }
    #[test]
    fn fixture_symlink_escape() {
        let d = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink("/etc", d.path().join("etc")).unwrap();
        assert!(rooted(d.path(), Path::new("/etc/passwd")).is_err());
        assert!(rooted(d.path(), Path::new("/../escape")).is_err());
    }
}
