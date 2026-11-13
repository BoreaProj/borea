//! Verified tool bundles and read-only, unprivileged namespace execution.
use borea_core::{runtime::RuntimeSpec, *};
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::Read,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
};
fn error(code: &'static str, message: impl Into<String>) -> BoreaError {
    BoreaError::new(code, message)
}
fn no_symlinks(path: &Path) -> Result<()> {
    if !safe_path(path) || path.parent().is_none() {
        return Err(error(
            "E_RUNTIME_PATH",
            "cache path must be absolute without traversal",
        ));
    }
    let mut p = PathBuf::new();
    for part in path.components() {
        p.push(part);
        match fs::symlink_metadata(&p) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(error("E_RUNTIME_PATH", "cache path contains a symlink"));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}
pub fn default_cache() -> Result<PathBuf> {
    let base = if let Some(p) = std::env::var_os("XDG_CACHE_HOME") {
        PathBuf::from(p)
    } else {
        PathBuf::from(
            std::env::var_os("HOME")
                .ok_or_else(|| error("E_RUNTIME_PATH", "set --cache or HOME"))?,
        )
        .join(".cache")
    };
    let p = base.join("borea/runtimes");
    no_symlinks(&p)?;
    Ok(p)
}
fn location(spec: &RuntimeSpec, cache: &Path) -> PathBuf {
    cache
        .join(spec.family.as_str())
        .join(&spec.version)
        .join(&spec.sha256)
}
fn sha256(reader: &mut impl Read) -> Result<String> {
    let mut h = Sha256::new();
    let mut b = [0u8; 65536];
    loop {
        let n = reader.read(&mut b)?;
        if n == 0 {
            break;
        }
        h.update(&b[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}
fn verify_archive(spec: &RuntimeSpec, path: &Path) -> Result<()> {
    let m = fs::symlink_metadata(path)?;
    if !m.is_file() || m.file_type().is_symlink() || m.len() != spec.size {
        return Err(error(
            "E_RUNTIME_INTEGRITY",
            "runtime archive type or size mismatch",
        ));
    }
    if sha256(&mut File::open(path)?)? != spec.sha256 {
        return Err(error(
            "E_RUNTIME_INTEGRITY",
            "runtime archive SHA-256 mismatch; remove the corrupt cache and fetch again",
        ));
    }
    Ok(())
}
fn relative(path: &Path) -> Result<PathBuf> {
    let mut clean = PathBuf::new();
    for part in path.components() {
        match part {
            Component::Normal(p) => clean.push(p),
            Component::CurDir => {}
            _ => {
                return Err(error(
                    "E_RUNTIME_ARCHIVE",
                    "archive path is absolute or traverses its root",
                ));
            }
        }
    }
    Ok(clean)
}
fn parent_set(paths: &mut BTreeSet<PathBuf>, path: &Path) {
    let mut p = Some(path);
    while let Some(path) = p {
        if path.as_os_str().is_empty() {
            break;
        }
        paths.insert(path.into());
        p = path.parent();
    }
}
fn archive_tree(spec: &RuntimeSpec, archive_path: &Path, root: &Path, extract: bool) -> Result<()> {
    let mut archive = tar::Archive::new(GzDecoder::new(File::open(archive_path)?));
    let mut total = 0u64;
    let mut count = 0usize;
    let mut paths = BTreeSet::new();
    for entry in archive.entries()? {
        let mut entry = entry?;
        count += 1;
        if count > 250_000 {
            return Err(error("E_RUNTIME_ARCHIVE", "archive entry limit exceeded"));
        }
        total = total
            .checked_add(entry.size())
            .ok_or_else(|| error("E_RUNTIME_ARCHIVE", "archive size overflow"))?;
        if total > spec.max_unpacked_bytes {
            return Err(error(
                "E_RUNTIME_ARCHIVE",
                "archive unpacked size limit exceeded",
            ));
        }
        let rel = relative(&entry.path()?)?;
        let kind = entry.header().entry_type();
        if !(kind.is_file() || kind.is_dir() || kind.is_symlink() || kind.is_hard_link()) {
            return Err(error(
                "E_RUNTIME_ARCHIVE",
                "device nodes, FIFOs and unsupported archive entries are forbidden",
            ));
        }
        if kind.is_hard_link() {
            relative(
                entry
                    .link_name()?
                    .as_deref()
                    .ok_or_else(|| error("E_RUNTIME_ARCHIVE", "missing hardlink target"))?,
            )?;
        }
        parent_set(&mut paths, &rel);
        if extract {
            entry.set_preserve_permissions(false);
            entry.set_unpack_xattrs(false);
            if !entry.unpack_in(root)? {
                return Err(error("E_RUNTIME_ARCHIVE", "entry escaped the runtime root"));
            }
            // The cache is owned by the unprivileged caller, not host root. Some
            // family images ship mode-000 files. Keep them readable/verifiable
            // by this owner and strip privileged mode bits; execution is ro-bound.
            if !kind.is_symlink() {
                let dest = root.join(&rel);
                let mode =
                    (entry.header().mode()? & 0o777) | if kind.is_dir() { 0o700 } else { 0o600 };
                fs::set_permissions(dest, fs::Permissions::from_mode(mode))?;
            }
        } else {
            let dest = root.join(&rel);
            let m = fs::symlink_metadata(&dest)?;
            if kind.is_symlink() {
                if !m.file_type().is_symlink()
                    || Some(fs::read_link(&dest)?) != entry.link_name()?.map(|p| p.into_owned())
                {
                    return Err(error(
                        "E_RUNTIME_INTEGRITY",
                        format!("runtime symlink changed: {}", rel.display()),
                    ));
                }
            } else if kind.is_dir() {
                if !m.is_dir() || m.file_type().is_symlink() {
                    return Err(error("E_RUNTIME_INTEGRITY", "runtime directory changed"));
                }
            } else {
                if !m.is_file() || m.file_type().is_symlink() {
                    return Err(error("E_RUNTIME_INTEGRITY", "runtime file type changed"));
                }
                // tar hardlinks carry no payload; compare with their verified target instead.
                let expected = if kind.is_hard_link() {
                    let target =
                        relative(entry.link_name()?.as_deref().ok_or_else(|| {
                            error("E_RUNTIME_ARCHIVE", "missing hardlink target")
                        })?)?;
                    sha256(&mut File::open(root.join(target))?)?
                } else {
                    sha256(&mut entry)?
                };
                if sha256(&mut File::open(&dest)?)? != expected {
                    return Err(error(
                        "E_RUNTIME_INTEGRITY",
                        format!("runtime file changed: {}", rel.display()),
                    ));
                }
            }
        }
    }
    if !extract {
        fn walk(root: &Path, current: &Path, out: &mut BTreeSet<PathBuf>) -> Result<()> {
            for entry in fs::read_dir(current)? {
                let e = entry?;
                let p = e.path();
                out.insert(
                    p.strip_prefix(root)
                        .map_err(|_| error("E_RUNTIME_PATH", "invalid runtime tree"))?
                        .into(),
                );
                if e.file_type()?.is_dir() {
                    walk(root, &p, out)?;
                }
                if out.len() > 250_000 {
                    return Err(error(
                        "E_RUNTIME_INTEGRITY",
                        "runtime tree entry limit exceeded",
                    ));
                }
            }
            Ok(())
        }
        let mut actual = BTreeSet::new();
        walk(root, root, &mut actual)?;
        if actual != paths {
            return Err(error(
                "E_RUNTIME_INTEGRITY",
                "runtime tree contains missing or extra paths",
            ));
        }
    }
    Ok(())
}
/// Resolve runtime symlinks as the isolated root will, never against host /usr.
fn in_root(root: &Path, path: &Path) -> Result<PathBuf> {
    let mut todo = path
        .components()
        .filter_map(|c| {
            if let Component::Normal(s) = c {
                Some(s.to_os_string())
            } else {
                None
            }
        })
        .collect::<std::collections::VecDeque<_>>();
    let mut rel = PathBuf::new();
    let mut links = 0;
    while let Some(part) = todo.pop_front() {
        if part == ".." {
            if !rel.pop() {
                return Err(error("E_RUNTIME_PATH", "symlink escaped runtime root"));
            }
            continue;
        }
        if part == "." {
            continue;
        }
        rel.push(&part);
        let dest = root.join(&rel);
        if fs::symlink_metadata(&dest)?.file_type().is_symlink() {
            links += 1;
            if links > 40 {
                return Err(error("E_RUNTIME_PATH", "runtime symlink loop"));
            }
            let link = fs::read_link(&dest)?;
            rel.pop();
            if link.is_absolute() {
                rel.clear();
            }
            let mut parts = link
                .components()
                .filter_map(|c| match c {
                    Component::Normal(s) => Some(s.to_os_string()),
                    Component::ParentDir => Some("..".into()),
                    _ => None,
                })
                .collect::<std::collections::VecDeque<_>>();
            parts.append(&mut todo);
            todo = parts;
        }
    }
    Ok(root.join(rel))
}
fn tool_paths(spec: &RuntimeSpec, root: &Path) -> Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    for tool in &spec.tools {
        let mut found = None;
        for dir in ["/usr/sbin", "/usr/bin", "/sbin", "/bin"] {
            let path = Path::new(dir).join(tool);
            if let Ok(p) = in_root(root, &path) {
                let m = fs::metadata(&p)?;
                if m.is_file() && m.permissions().mode() & 0o111 != 0 {
                    found = Some(path.to_string_lossy().into_owned());
                    break;
                }
            }
        }
        out.push((
            tool.clone(),
            found.ok_or_else(|| {
                error(
                    "E_RUNTIME_TOOLS",
                    format!("runtime lacks executable {tool}"),
                )
            })?,
        ));
    }
    Ok(out)
}
#[derive(Debug, Clone, serde::Serialize)]
pub struct RuntimeStatus {
    pub schema: u16,
    pub family: FamilyId,
    pub version: String,
    pub sha256: String,
    pub root: PathBuf,
    pub archive: PathBuf,
    pub verified: bool,
    pub isolation_verified: bool,
    pub tools: Vec<(String, String)>,
}
pub fn verify(spec: &RuntimeSpec, cache: &Path) -> Result<RuntimeStatus> {
    spec.validate()?;
    no_symlinks(cache)?;
    let dir = location(spec, cache);
    no_symlinks(&dir)?;
    let root = dir.join("rootfs");
    let archive = dir.join("runtime.tar.gz");
    no_symlinks(&root)?;
    verify_archive(spec, &archive)?;
    archive_tree(spec, &archive, &root, false)?;
    let tools = tool_paths(spec, &root)?;
    Ok(RuntimeStatus {
        schema: 1,
        family: spec.family,
        version: spec.version.clone(),
        sha256: spec.sha256.clone(),
        root,
        archive,
        verified: true,
        isolation_verified: false,
        tools,
    })
}
fn cleanup_stage(path: &Path) -> std::io::Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
        for entry in fs::read_dir(path)? {
            cleanup_stage(&entry?.path())?;
        }
        fs::remove_dir(path)
    } else {
        fs::remove_file(path)
    }
}
struct Staging(PathBuf);
impl Drop for Staging {
    fn drop(&mut self) {
        let _ = cleanup_stage(&self.0);
    }
}
pub fn fetch(
    spec: &RuntimeSpec,
    cache: &Path,
    local_archive: Option<&Path>,
    offline: bool,
) -> Result<RuntimeStatus> {
    spec.validate()?;
    no_symlinks(cache)?;
    let existed = cache.exists();
    fs::create_dir_all(cache)?;
    if !existed {
        fs::set_permissions(cache, fs::Permissions::from_mode(0o700))?;
    }
    if fs::metadata(cache)?.permissions().mode() & 0o022 != 0 {
        return Err(error(
            "E_RUNTIME_PATH",
            "cache must not be writable by other users; choose a private cache directory",
        ));
    }
    let lock_path = cache.join(".lock");
    no_symlinks(&lock_path)?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(&lock_path)?;
    lock.try_lock().map_err(|_| {
        error(
            "E_RUNTIME_LOCK",
            "another runtime provisioning operation holds the cache lock",
        )
    })?;
    let dest = location(spec, cache);
    if dest.exists() {
        return verify(spec, cache);
    }
    if offline && local_archive.is_none() {
        return Err(error(
            "E_RUNTIME_OFFLINE",
            "runtime is not cached; provide --archive FILE or fetch once with network access",
        ));
    }
    let stage = cache.join(format!(
        ".stage-{}-{}",
        spec.family.as_str(),
        std::process::id()
    ));
    fs::create_dir(&stage)?;
    fs::set_permissions(&stage, fs::Permissions::from_mode(0o700))?;
    let _guard = Staging(stage.clone());
    let archive = stage.join("runtime.tar.gz");
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&archive)?;
    if let Some(path) = local_archive {
        let m = fs::symlink_metadata(path)?;
        if !m.is_file() || m.file_type().is_symlink() {
            return Err(error(
                "E_RUNTIME_PATH",
                "local archive must be a regular file",
            ));
        }
        let mut source = File::open(path)?;
        let mut output = file;
        let n = std::io::copy(
            &mut Read::by_ref(&mut source).take(spec.size + 1),
            &mut output,
        )?;
        output.sync_all()?;
        if n != spec.size {
            return Err(error("E_RUNTIME_INTEGRITY", "local archive size mismatch"));
        }
    } else {
        let status = Command::new("curl")
            .args([
                "--fail",
                "--silent",
                "--show-error",
                "--location",
                "--proto",
                "=https",
                "--proto-redir",
                "=https",
                "--connect-timeout",
                "30",
                "--max-time",
                "1800",
                "--retry",
                "2",
                "--max-filesize",
                &spec.size.to_string(),
                &spec.url,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::from(file))
            .status()
            .map_err(|e| {
                error(
                    "E_RUNTIME_DOWNLOAD",
                    format!("HTTPS transport requires curl: {e}"),
                )
            })?;
        if !status.success() {
            return Err(error(
                "E_RUNTIME_DOWNLOAD",
                "runtime download failed; the incomplete cache was discarded",
            ));
        }
        File::open(&archive)?.sync_all()?;
    }
    verify_archive(spec, &archive)?;
    let root = stage.join("rootfs");
    fs::create_dir(&root)?;
    archive_tree(spec, &archive, &root, true)?;
    tool_paths(spec, &root)?;
    archive_tree(spec, &archive, &root, false)?;
    let parent = dest
        .parent()
        .ok_or_else(|| error("E_RUNTIME_PATH", "invalid cache destination"))?;
    no_symlinks(parent)?;
    fs::create_dir_all(parent)?;
    fs::rename(&stage, &dest)?;
    File::open(parent)?.sync_all()?;
    verify(spec, cache)
}
/// No host home, disk, package database or engine socket is visible to this process.
fn command(status: &RuntimeStatus, argv: &[String], network: bool) -> Result<Command> {
    valid_argv(argv)?;
    let root = status
        .root
        .to_str()
        .ok_or_else(|| error("E_RUNTIME_PATH", "runtime path must be UTF-8"))?;
    let mut cmd = Command::new("bwrap");
    cmd.env_clear();
    cmd.args([
        "--die-with-parent",
        "--new-session",
        "--unshare-all",
        "--uid",
        "0",
        "--gid",
        "0",
        "--cap-drop",
        "ALL",
        "--ro-bind",
        root,
        "/",
        "--proc",
        "/proc",
        "--dev",
        "/dev",
        "--tmpfs",
        "/tmp",
        "--tmpfs",
        "/run",
        "--tmpfs",
        "/var/tmp",
        "--tmpfs",
        "/var/log",
        "--tmpfs",
        "/var/cache",
        "--chdir",
        "/",
        "--clearenv",
        "--setenv",
        "PATH",
        "/usr/sbin:/usr/bin:/sbin:/bin",
        "--setenv",
        "HOME",
        "/tmp",
        "--setenv",
        "LANG",
        "C.UTF-8",
    ]);
    if network {
        cmd.arg("--share-net");
        if let Ok(resolv) = fs::canonicalize("/etc/resolv.conf") {
            cmd.arg("--ro-bind").arg(resolv).arg("/etc/resolv.conf");
        }
    }
    cmd.arg("--").args(argv).stdin(Stdio::null());
    Ok(cmd)
}
pub fn run(spec: &RuntimeSpec, cache: &Path, argv: &[String], network: bool) -> Result<i32> {
    valid_argv(argv)?;
    let status = verify(spec, cache)?;
    let code = command(&status, argv, network)?.status().map_err(|e| {
        error(
            "E_RUNTIME_ISOLATION",
            format!("Bubblewrap is required; no host fallback: {e}"),
        )
    })?;
    if !code.success() {
        return Err(error(
            "E_RUNTIME_EXEC",
            format!(
                "isolated command exited {} (no host fallback)",
                code.code().unwrap_or(-1)
            ),
        ));
    }
    Ok(0)
}
pub fn probe(spec: &RuntimeSpec, cache: &Path) -> Result<RuntimeStatus> {
    let mut status = verify(spec, cache)?;
    let args = std::iter::once("/bin/sh".into())
        .chain([
            "-ec".into(),
            "for tool do command -v \"$tool\" >/dev/null; done".into(),
            "--".into(),
        ])
        .chain(spec.tools.iter().cloned())
        .collect::<Vec<_>>();
    let result = command(&status, &args, false)?.output().map_err(|e| {
        error(
            "E_RUNTIME_ISOLATION",
            format!("Bubblewrap is required: {e}"),
        )
    })?;
    if !result.status.success() {
        return Err(error(
            "E_RUNTIME_ISOLATION",
            format!(
                "runtime probe failed: {}",
                String::from_utf8_lossy(&result.stderr)
            ),
        ));
    }
    status.isolation_verified = true;
    Ok(status)
}

/// Copy and authenticate bytes before extracting into root-private storage.
/// Never elevate the mutable extracted tree in an unprivileged cache.
pub(crate) fn private_snapshot(
    spec: &RuntimeSpec,
    cache: &Path,
    private: &Path,
) -> Result<PathBuf> {
    spec.validate()?;
    let source = location(spec, cache).join("runtime.tar.gz");
    let mut source = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(source)?;
    if !source.metadata()?.is_file() || source.metadata()?.len() != spec.size {
        return Err(error(
            "E_RUNTIME_INTEGRITY",
            "runtime source must be a regular archive of the pinned size",
        ));
    }
    let archive = private.join("runtime.tar.gz");
    let mut dest = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&archive)?;
    let n = std::io::copy(&mut (&mut source).take(spec.size + 1), &mut dest)?;
    if n != spec.size {
        return Err(error("E_RUNTIME_INTEGRITY", "runtime copy size mismatch"));
    }
    dest.sync_all()?;
    verify_archive(spec, &archive)?;
    let root = private.join("rootfs");
    fs::create_dir(&root)?;
    archive_tree(spec, &archive, &root, true)?;
    tool_paths(spec, &root)?;
    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{Compression, write::GzEncoder};
    fn fixture(dir: &Path) -> (RuntimeSpec, PathBuf) {
        let file = dir.join("fixture.tar.gz");
        let gz = GzEncoder::new(File::create(&file).unwrap(), Compression::fast());
        let mut builder = tar::Builder::new(gz);
        let mut header = tar::Header::new_gnu();
        header.set_mode(0o755);
        header.set_size(7);
        header.set_cksum();
        builder
            .append_data(&mut header, "usr/bin/tool", &b"fixture"[..])
            .unwrap();
        builder.into_inner().unwrap().finish().unwrap();
        let spec=RuntimeSpec{schema:1,family:FamilyId::Arch,version:"runtime-test.1".into(),arch:"x86_64".into(),image:format!("example@sha256:{}","a".repeat(64)),url:"https://github.com/BoreaProj/borea/releases/download/runtime-test.1/borea-runtime-arch-x86_64.tar.gz".into(),sha256:sha256(&mut File::open(&file).unwrap()).unwrap(),size:fs::metadata(&file).unwrap().len(),max_unpacked_bytes:1024,tools:vec!["tool".into()]};
        (spec, file)
    }
    #[test]
    fn runtime_offline_import_verify_and_reuse() {
        let d = tempfile::tempdir().unwrap();
        let (s, a) = fixture(d.path());
        let cache = d.path().join("cache");
        let state = fetch(&s, &cache, Some(&a), true).unwrap();
        assert!(state.verified);
        assert_eq!(verify(&s, &cache).unwrap().tools[0].0, "tool");
        assert!(fetch(&s, &cache, None, true).unwrap().verified);
    }
    #[test]
    fn runtime_rejects_corrupt_archive_before_extract() {
        let d = tempfile::tempdir().unwrap();
        let (mut s, a) = fixture(d.path());
        s.sha256 = "b".repeat(64);
        let cache = d.path().join("cache");
        assert_eq!(
            fetch(&s, &cache, Some(&a), true).unwrap_err().code,
            "E_RUNTIME_INTEGRITY"
        );
        assert!(!location(&s, &cache).exists());
        assert!(!fs::read_dir(cache).unwrap().any(|e| {
            e.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".stage")
        }));
    }
    #[test]
    fn runtime_detects_changed_and_extra_files() {
        let d = tempfile::tempdir().unwrap();
        let (s, a) = fixture(d.path());
        let cache = d.path().join("cache");
        let state = fetch(&s, &cache, Some(&a), true).unwrap();
        fs::write(state.root.join("usr/bin/tool"), "tampered").unwrap();
        assert_eq!(verify(&s, &cache).unwrap_err().code, "E_RUNTIME_INTEGRITY");
        fs::write(state.root.join("usr/bin/tool"), "fixture").unwrap();
        fs::write(state.root.join("extra"), "extra").unwrap();
        assert_eq!(verify(&s, &cache).unwrap_err().code, "E_RUNTIME_INTEGRITY");
    }
    #[test]
    fn runtime_miss_is_offline_and_symlink_cache_refused() {
        let d = tempfile::tempdir().unwrap();
        let (s, _) = fixture(d.path());
        let cache = d.path().join("cache");
        assert_eq!(
            fetch(&s, &cache, None, true).unwrap_err().code,
            "E_RUNTIME_OFFLINE"
        );
        let link = d.path().join("link");
        std::os::unix::fs::symlink(&cache, &link).unwrap();
        assert_eq!(
            fetch(&s, &link, None, true).unwrap_err().code,
            "E_RUNTIME_PATH"
        );
    }
    #[test]
    fn runtime_archive_limits_and_unsafe_paths() {
        assert!(relative(Path::new("../escape")).is_err());
        assert!(relative(Path::new("/etc/passwd")).is_err());
        let d = tempfile::tempdir().unwrap();
        let (mut s, a) = fixture(d.path());
        s.max_unpacked_bytes = 1;
        assert_eq!(
            fetch(&s, &d.path().join("cache"), Some(&a), true)
                .unwrap_err()
                .code,
            "E_RUNTIME_ARCHIVE"
        );
    }
    #[test]
    fn runtime_refuses_symlink_archive_escape() {
        let d = tempfile::tempdir().unwrap();
        let (s, original) = fixture(d.path());
        let gz = GzEncoder::new(File::create(&original).unwrap(), Compression::fast());
        let mut tar = tar::Builder::new(gz);
        let mut h = tar::Header::new_gnu();
        h.set_entry_type(tar::EntryType::Symlink);
        h.set_size(0);
        h.set_mode(0o777);
        h.set_link_name(d.path()).unwrap();
        h.set_cksum();
        tar.append_data(&mut h, "escape", std::io::empty()).unwrap();
        let mut h = tar::Header::new_gnu();
        h.set_mode(0o644);
        h.set_size(1);
        h.set_cksum();
        tar.append_data(&mut h, "escape/outside", &b"x"[..])
            .unwrap();
        tar.into_inner().unwrap().finish().unwrap();
        let mut s = s;
        s.sha256 = sha256(&mut File::open(&original).unwrap()).unwrap();
        s.size = fs::metadata(&original).unwrap().len();
        assert!(fetch(&s, &d.path().join("cache"), Some(&original), true).is_err());
        assert!(!d.path().join("outside").exists());
    }
    #[test]
    fn runtime_never_changes_existing_cache_permissions() {
        let d = tempfile::tempdir().unwrap();
        let (s, a) = fixture(d.path());
        let cache = d.path().join("public");
        fs::create_dir(&cache).unwrap();
        fs::set_permissions(&cache, fs::Permissions::from_mode(0o777)).unwrap();
        assert_eq!(
            fetch(&s, &cache, Some(&a), true).unwrap_err().code,
            "E_RUNTIME_PATH"
        );
        assert_eq!(
            fs::metadata(&cache).unwrap().permissions().mode() & 0o777,
            0o777
        );
        assert!(no_symlinks(Path::new("/")).is_err());
    }
    #[test]
    fn runtime_normalizes_root_only_cache_files() {
        let d = tempfile::tempdir().unwrap();
        let (mut spec, path) = fixture(d.path());
        let gz = GzEncoder::new(File::create(&path).unwrap(), Compression::fast());
        let mut tar = tar::Builder::new(gz);
        for (name, mode) in [("usr/bin/tool", 0o755), ("etc/locked", 0)] {
            let mut h = tar::Header::new_gnu();
            h.set_mode(mode);
            h.set_size(1);
            h.set_cksum();
            tar.append_data(&mut h, name, &b"x"[..]).unwrap();
        }
        tar.into_inner().unwrap().finish().unwrap();
        spec.size = fs::metadata(&path).unwrap().len();
        spec.sha256 = sha256(&mut File::open(&path).unwrap()).unwrap();
        let state = fetch(&spec, &d.path().join("cache"), Some(&path), true).unwrap();
        assert_eq!(
            fs::metadata(state.root.join("etc/locked"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    #[test]
    fn runtime_cleans_locked_staging_without_following_symlinks() {
        let d = tempfile::tempdir().unwrap();
        let stage = d.path().join("stage");
        let locked = stage.join("locked");
        fs::create_dir_all(&locked).unwrap();
        let outside = d.path().join("outside");
        fs::write(&outside, "untouched").unwrap();
        std::os::unix::fs::symlink(&outside, locked.join("link")).unwrap();
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o0)).unwrap();
        drop(Staging(stage.clone()));
        assert!(!stage.exists());
        assert_eq!(fs::read_to_string(outside).unwrap(), "untouched");
    }
    #[test]
    fn runtime_command_exposes_only_bundle_and_private_mounts() {
        let d = tempfile::tempdir().unwrap();
        let (s, a) = fixture(d.path());
        let state = fetch(&s, &d.path().join("cache"), Some(&a), true).unwrap();
        let cmd = command(&state, &["tool".into()], false).unwrap();
        let args = cmd
            .get_args()
            .map(|s| s.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(args.iter().any(|s| s == "--unshare-all"));
        assert!(args.iter().any(|s| s == "--cap-drop"));
        assert!(
            !args.iter().any(|s| s == "--share-net"
                || s == "--dev-bind"
                || s == "/home"
                || s == "/dev/sda")
        );
    }
}
