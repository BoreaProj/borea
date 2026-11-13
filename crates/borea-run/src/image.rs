//! Closed storage execution for Borea-created disposable images, never raw devices.
use borea_core::{runtime::RuntimeSpec, *};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    os::{
        fd::AsRawFd,
        unix::{
            fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
            process::CommandExt,
        },
    },
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const GIB: u64 = 1024 * 1024 * 1024;
const MIB: u64 = 1024 * 1024;
fn fail(message: impl Into<String>) -> BoreaError {
    BoreaError::new("E_IMAGE", message)
}
fn root() -> Result<()> {
    // SAFETY: geteuid takes no pointers and has no preconditions.
    if unsafe { libc::geteuid() } != 0 {
        return Err(fail(
            "image execution requires root; use a disposable test runner",
        ));
    }
    Ok(())
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn uuid(seed: &str, name: &str) -> String {
    let h = hash(format!("{seed}:{name}").as_bytes());
    format!(
        "{}-{}-8{}-a{}-{}",
        &h[..8],
        &h[8..12],
        &h[13..16],
        &h[17..20],
        &h[20..32]
    )
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImagePlan {
    pub schema: u16,
    pub image: PathBuf,
    pub device: u64,
    pub inode: u64,
    pub bytes: u64,
    pub filesystem: String,
    pub runtime_sha256: String,
    pub seed: String,
    pub digest: String,
}
impl ImagePlan {
    fn digest(&self) -> Result<String> {
        let mut p = self.clone();
        p.digest.clear();
        Ok(hash(&serde_json::to_vec(&p)?))
    }
    fn validate(&self, spec: &RuntimeSpec) -> Result<()> {
        spec.validate()?;
        if self.schema != 1
            || !safe_path(&self.image)
            || !self.image.is_absolute()
            || self.bytes < 8 * GIB
            || self.bytes > 1024 * GIB
            || !self.bytes.is_multiple_of(MIB)
            || !["btrfs", "ext4"].contains(&self.filesystem.as_str())
            || self.runtime_sha256 != spec.sha256
            || self.seed.len() != 64
            || !self.seed.bytes().all(|b| b.is_ascii_hexdigit())
            || self.digest != self.digest()?
        {
            return Err(fail("invalid/stale image manifest or runtime pin"));
        }
        Ok(())
    }
}
fn locked(path: &Path, create: bool) -> Result<File> {
    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(create)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    let m = f.metadata()?;
    if !m.is_file() || m.uid() != 0 || m.nlink() != 1 || m.mode() & 0o7777 != 0o600 {
        return Err(fail(
            "requires a root-owned mode-0600 regular file with one link",
        ));
    }
    // SAFETY: flock uses a live descriptor; the File retains the lock until drop.
    if unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err(fail("image or journal is already locked"));
    }
    Ok(f)
}
/// Create-new only. The initial marker proves this was an empty Borea image.
pub fn create(
    path: &Path,
    manifest: &Path,
    fs: &str,
    bytes: u64,
    spec: &RuntimeSpec,
) -> Result<ImagePlan> {
    root()?;
    let mut p = ImagePlan {
        schema: 1,
        image: path.to_owned(),
        device: 0,
        inode: 0,
        bytes,
        filesystem: fs.into(),
        runtime_sha256: spec.sha256.clone(),
        seed: "0".repeat(64),
        digest: String::new(),
    };
    p.digest = p.digest()?;
    p.validate(spec)?;
    let mut f = locked(path, true)?;
    f.set_len(bytes)?;
    let m = f.metadata()?;
    p.device = m.dev();
    p.inode = m.ino();
    let mut random = [0u8; 32];
    File::open("/dev/urandom")?.read_exact(&mut random)?;
    p.seed = hash(&random);
    p.digest = p.digest()?;
    f.write_all(format!("BOREA-IMAGE-1:{}\n", p.seed).as_bytes())?;
    f.sync_all()?;
    let mut out = locked(manifest, true)?;
    out.write_all(&serde_json::to_vec_pretty(&p)?)?;
    out.sync_all()?;
    File::open(path.parent().ok_or_else(|| fail("image needs a parent"))?)?.sync_all()?;
    File::open(
        manifest
            .parent()
            .ok_or_else(|| fail("manifest needs a parent"))?,
    )?
    .sync_all()?;
    Ok(p)
}

#[repr(C)]
struct LoopInfo {
    device: u64,
    inode: u64,
    rdevice: u64,
    offset: u64,
    size: u64,
    number: u32,
    encryption: u32,
    key_size: u32,
    flags: u32,
    name: [u8; 64],
    crypt: [u8; 64],
    key: [u8; 32],
    init: [u64; 2],
}
impl Default for LoopInfo {
    fn default() -> Self {
        Self {
            device: 0,
            inode: 0,
            rdevice: 0,
            offset: 0,
            size: 0,
            number: 0,
            encryption: 0,
            key_size: 0,
            flags: 0,
            name: [0; 64],
            crypt: [0; 64],
            key: [0; 32],
            init: [0; 2],
        }
    }
}
#[repr(C)]
struct LoopConfig {
    fd: u32,
    block_size: u32,
    info: LoopInfo,
    reserved: [u64; 8],
}
struct Loop {
    file: File,
    path: PathBuf,
}
impl Loop {
    fn attach(image: &File, offset: u64, size: u64) -> Result<Self> {
        let ctl = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/loop-control")?;
        for _ in 0..32 {
            // SAFETY: LOOP_CTL_GET_FREE has no pointer argument.
            let n = unsafe { libc::ioctl(ctl.as_raw_fd(), 0x4c82) };
            if n < 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            let path = PathBuf::from(format!("/dev/loop{n}"));
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(libc::O_NOFOLLOW)
                .open(&path)?;
            let m = file.metadata()?;
            if m.mode() & libc::S_IFMT != libc::S_IFBLK
                || libc::major(m.rdev()) != 7
                || libc::minor(m.rdev()) != n as u32
            {
                return Err(fail("loop device identity mismatch"));
            }
            let cfg = LoopConfig {
                fd: image.as_raw_fd() as u32,
                block_size: 512,
                info: LoopInfo {
                    offset,
                    size,
                    flags: 4,
                    ..Default::default()
                },
                reserved: [0; 8],
            };
            // SAFETY: Linux LOOP_CONFIGURE reads a correctly laid-out initialized loop_config.
            if unsafe { libc::ioctl(file.as_raw_fd(), 0x4c0a, &cfg) } == 0 {
                return Ok(Self { file, path });
            }
            let e = std::io::Error::last_os_error();
            if e.raw_os_error() != Some(libc::EBUSY) {
                return Err(e.into());
            }
        }
        Err(fail(
            "no free loop device after concurrent allocation retries",
        ))
    }
}
impl Drop for Loop {
    fn drop(&mut self) {
        // SAFETY: LOOP_CLR_FD has no pointer argument. AUTOCLEAR also protects crash cleanup.
        unsafe {
            libc::ioctl(self.file.as_raw_fd(), 0x4c01);
        }
    }
}

struct RunningChild {
    child: std::process::Child,
    running: bool,
}
impl Drop for RunningChild {
    fn drop(&mut self) {
        if self.running {
            // SAFETY: the live child owns the process group established at spawn.
            unsafe {
                libc::kill(-(self.child.id() as i32), libc::SIGKILL);
            }
            let _ = self.child.wait();
        }
    }
}

/// Apply only the closed image storage program. No plan-supplied command executes.
pub fn apply(
    plan: &ImagePlan,
    spec: &RuntimeSpec,
    cache: &Path,
    journal: &Path,
    resume: bool,
    yes: bool,
) -> Result<()> {
    root()?;
    plan.validate(spec)?;
    if !yes {
        return Err(fail("image apply requires --yes"));
    }
    let mut image = locked(&plan.image, false)?;
    let m = image.metadata()?;
    if (m.dev(), m.ino(), m.len()) != (plan.device, plan.inode, plan.bytes) {
        return Err(fail("image identity changed; recreate/replan"));
    }
    let mut receipt = locked(journal, !resume)?;
    if !resume {
        File::open(
            journal
                .parent()
                .ok_or_else(|| fail("journal needs a parent"))?,
        )?
        .sync_all()?;
    }
    if resume {
        let mut text = String::new();
        (&mut receipt).take(65537).read_to_string(&mut text)?;
        validate_journal(&text, &plan.digest)?;
    } else {
        let marker = format!("BOREA-IMAGE-1:{}\n", plan.seed);
        let mut bytes = vec![0; marker.len()];
        image.read_exact(&mut bytes)?;
        if bytes != marker.as_bytes() {
            return Err(fail(
                "image lacks its creation marker; refuse destructive replay",
            ));
        }
        writeln!(receipt, "{}", plan.digest)?;
        receipt.sync_all()?;
    }
    let private = tempfile::Builder::new()
        .prefix("borea-image-")
        .tempdir_in("/tmp")?;
    fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700))?;
    let runtime = crate::runtime::private_snapshot(spec, cache, private.path())?;
    fs::create_dir(runtime.join("target"))?;
    fs::write(runtime.join("receipt"), [])?;
    fs::write(runtime.join("storage.sh"), [])?;
    let script = private.path().join("storage.sh");
    fs::write(&script, include_bytes!("image-storage.sh"))?;
    let whole = Loop::attach(&image, 0, 0)?;
    let esp = Loop::attach(&image, MIB, GIB)?;
    let boot = Loop::attach(&image, MIB + GIB, 2 * GIB)?;
    let rootdev = Loop::attach(&image, MIB + 3 * GIB, plan.bytes - 2 * MIB - 3 * GIB)?;
    let mut cmd = Command::new("/usr/bin/bwrap");
    cmd.env_clear()
        .args([
            "--die-with-parent",
            "--unshare-pid",
            "--unshare-ipc",
            "--unshare-uts",
            "--unshare-net",
            "--cap-drop",
            "ALL",
            "--cap-add",
            "CAP_SYS_ADMIN",
            "--cap-add",
            "CAP_SYS_CHROOT",
            "--ro-bind",
        ])
        .arg(runtime)
        .arg("/")
        .args([
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
            "--dir",
            "/target",
        ]);
    for (l, name) in [
        (&whole, "disk"),
        (&esp, "esp"),
        (&boot, "boot"),
        (&rootdev, "root"),
    ] {
        cmd.arg("--dev-bind")
            .arg(&l.path)
            .arg(format!("/dev/borea-{name}"));
    }
    // Bind held descriptors, not the mutable journal pathname.
    cmd.arg("--bind")
        .arg(format!(
            "/proc/{}/fd/{}",
            std::process::id(),
            receipt.as_raw_fd()
        ))
        .arg("/receipt")
        .arg("--ro-bind")
        .arg(script)
        .arg("/storage.sh")
        .args([
            "--chdir",
            "/",
            "--clearenv",
            "--setenv",
            "PATH",
            "/usr/sbin:/usr/bin:/sbin:/bin",
            "--setenv",
            "LC_ALL",
            "C",
            "--",
            "/bin/sh",
            "/storage.sh",
        ])
        .arg(&plan.filesystem)
        .arg(uuid(&plan.seed, "esp"))
        .arg(uuid(&plan.seed, "boot"))
        .arg(uuid(&plan.seed, "root"))
        .arg(uuid(&plan.seed, "disk"))
        .arg(((plan.bytes - MIB) / 512 - 1).to_string())
        .arg(if resume { "resume" } else { "new" })
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    cmd.process_group(0);
    let mut child = RunningChild {
        child: cmd.spawn()?,
        running: true,
    };
    let stderr = child
        .child
        .stderr
        .take()
        .ok_or_else(|| fail("missing diagnostic pipe"))?;
    let reader = std::thread::spawn(move || {
        let mut tail = Vec::new();
        let mut buf = [0; 4096];
        let mut stream = stderr;
        while let Ok(n) = stream.read(&mut buf) {
            if n == 0 {
                break;
            }
            tail.extend_from_slice(&buf[..n]);
            if tail.len() > 8192 {
                tail.drain(..tail.len() - 8192);
            }
        }
        tail
    });
    let start = Instant::now();
    let status = loop {
        if let Some(s) = child.child.try_wait()? {
            child.running = false;
            break s;
        }
        if start.elapsed() > Duration::from_secs(600) {
            // SAFETY: child owns a new process group; negative pid targets only that group.
            unsafe {
                libc::kill(-(child.child.id() as i32), libc::SIGKILL);
            }
            child.child.wait()?;
            child.running = false;
            return Err(fail(
                "image step timeout; inspect receipt and resume only if probes pass",
            ));
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let diagnostics = reader
        .join()
        .map_err(|_| fail("diagnostic reader failed"))?;
    image.sync_all()?;
    receipt.seek(SeekFrom::Start(0))?;
    let mut text = String::new();
    (&mut receipt).take(65537).read_to_string(&mut text)?;
    validate_journal(&text, &plan.digest)?;
    if !status.success() || !text.ends_with("verified\n") {
        return Err(fail(format!(
            "image storage execution failed; inspect receipt before resume: {}",
            String::from_utf8_lossy(&diagnostics)
        )));
    }
    Ok(())
}
fn validate_journal(text: &str, digest: &str) -> Result<()> {
    let steps = ["partition", "esp", "boot", "root"];
    let mut lines = text.lines();
    if lines.next() != Some(digest) || !text.ends_with('\n') || text.len() > 65536 {
        return Err(fail("stale, truncated or invalid image journal"));
    }
    let mut completed = 0;
    let mut active = false;
    for line in lines {
        if line == "verified" {
            if active || completed != steps.len() {
                return Err(fail("premature verification receipt"));
            }
            continue;
        }
        let step = steps
            .get(completed)
            .ok_or_else(|| fail("receipt after completed storage"))?;
        if line == format!("start:{step}") && !active {
            active = true;
        } else if line == format!("ok:{step}") && active {
            active = false;
            completed += 1;
        } else {
            return Err(fail("unknown or out-of-order image receipt"));
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn journals_reject_stale_truncated_unknown() {
        assert!(validate_journal("abc\nstart:partition\nok:partition\nstart:esp\nok:esp\nstart:boot\nok:boot\nstart:root\nok:root\nverified\n", "abc").is_ok());
        assert!(validate_journal("abc\nstart:partition\n", "abc").is_ok());
        for j in [
            "abc",
            "other\n",
            "abc\nstart:anything\n",
            "abc\nok:root\n",
            "abc\nstart:root\nstart:boot\n",
        ] {
            assert!(validate_journal(j, "abc").is_err());
        }
    }
    #[test]
    fn linux_loop_abi() {
        assert_eq!(std::mem::size_of::<LoopInfo>(), 232);
        assert_eq!(std::mem::size_of::<LoopConfig>(), 304);
    }
    #[test]
    fn uuid_stable_distinct() {
        assert_eq!(uuid("a", "root"), uuid("a", "root"));
        assert_ne!(uuid("a", "root"), uuid("a", "boot"));
    }
}
