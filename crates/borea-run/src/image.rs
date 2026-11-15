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
use zeroize::Zeroizing;

const GIB: u64 = 1024 * 1024 * 1024;
const MIB: u64 = 1024 * 1024;
fn fail(message: impl Into<String>) -> BoreaError {
    BoreaError::new("E_IMAGE", message)
}
pub fn read_bootstrap_password(path: &Path) -> Result<Zeroizing<String>> {
    const MAX_PASSWORD_BYTES: u64 = 4096;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file()
        || metadata.nlink() != 1
        || metadata.mode() & 0o077 != 0
        || metadata.len() == 0
        || metadata.len() > MAX_PASSWORD_BYTES + 1
    {
        return Err(fail(
            "bootstrap password must be a private, single-link regular file of at most 4096 bytes",
        ));
    }
    let mut bytes = Zeroizing::new(Vec::new());
    file.take(MAX_PASSWORD_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_PASSWORD_BYTES + 1 {
        return Err(fail("bootstrap password file exceeds 4096 bytes"));
    }
    if bytes.last() == Some(&b'\n') {
        bytes.pop();
        if bytes.last() == Some(&b'\r') {
            bytes.pop();
        }
    }
    if bytes.len() as u64 > MAX_PASSWORD_BYTES
        || bytes.is_empty()
        || bytes.iter().any(|b| matches!(b, b'\0' | b'\n' | b'\r'))
    {
        return Err(fail(
            "bootstrap password must be one nonempty line without NUL",
        ));
    }
    let password = match String::from_utf8(std::mem::take(&mut *bytes)) {
        Ok(password) => password,
        Err(error) => {
            drop(Zeroizing::new(error.into_bytes()));
            return Err(fail("bootstrap password file must contain UTF-8"));
        }
    };
    Ok(Zeroizing::new(password))
}
fn redacted_bootstrap_diagnostics(diagnostics: &[u8], password: &Zeroizing<String>) -> String {
    let text = Zeroizing::new(String::from_utf8_lossy(diagnostics).into_owned());
    crate::redact(&text, std::slice::from_ref(password))
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
    /// Deterministic GPT/filesystem UUID for `esp`, `boot`, `root` or `disk`.
    pub fn uuid(&self, name: &str) -> String {
        uuid(&self.seed, name)
    }
    fn root_fs(&self) -> Result<Filesystem> {
        match self.filesystem.as_str() {
            "btrfs" => Ok(Filesystem::Btrfs),
            "ext4" => Ok(Filesystem::Ext4),
            _ => Err(fail("unknown image filesystem")),
        }
    }
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
/// Open a second, unlocked description of the same file for the loop driver.
///
/// `flock` belongs to the open file description. If the locked descriptor were
/// handed to `LOOP_CONFIGURE`, the kernel's reference would keep the image
/// lock held until a lazy detach finished (for example while udev still probes
/// the device), and the next stage would fail with "already locked" (G06).
fn unlocked_backing(image: &File) -> Result<File> {
    let backing = OpenOptions::new()
        .read(true)
        .write(true)
        .open(format!("/proc/self/fd/{}", image.as_raw_fd()))?;
    let (a, b) = (image.metadata()?, backing.metadata()?);
    if a.dev() != b.dev() || a.ino() != b.ino() {
        return Err(fail("loop backing file identity mismatch"));
    }
    Ok(backing)
}
/// `LO_FLAGS_AUTOCLEAR`: detach when the last opener closes.
const LO_AUTOCLEAR: u32 = 4;
/// `LO_FLAGS_PARTSCAN`: the kernel creates partition devices for the GPT.
const LO_PARTSCAN: u32 = 8;
impl Loop {
    fn attach(image: &File, offset: u64, size: u64) -> Result<Self> {
        Self::attach_with(image, offset, size, LO_AUTOCLEAR)
    }
    /// Attach the whole image with partition scanning, so its partitions are
    /// real kernel partitions of one disk (G07). GRUB probes `/boot` through
    /// sysfs and rejects bare offset loops.
    fn attach_disk(image: &File) -> Result<Self> {
        Self::attach_with(image, 0, 0, LO_AUTOCLEAR | LO_PARTSCAN)
    }
    fn attach_with(image: &File, offset: u64, size: u64, flags: u32) -> Result<Self> {
        let backing = unlocked_backing(image)?;
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
                fd: backing.as_raw_fd() as u32,
                block_size: 512,
                info: LoopInfo {
                    offset,
                    size,
                    flags,
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
/// The kernel partition `n` of a partition-scanned loop disk, checked against
/// the expected start and length (bytes) through sysfs.
fn loop_partition(disk: &Loop, n: u8, start: u64, len: u64) -> Result<PathBuf> {
    let name = disk
        .path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| fail("loop path"))?;
    let part = format!("{name}p{n}");
    let sys = Path::new("/sys/class/block").join(&part);
    let dev = Path::new("/dev").join(&part);
    // The kernel creates partition nodes asynchronously after LOOP_CONFIGURE.
    for _ in 0..100 {
        if sys.join("start").is_file() && dev.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let sectors = |f: &str| -> Result<u64> {
        fs::read_to_string(sys.join(f))
            .map_err(|e| fail(format!("{part} has no {f} in sysfs: {e}")))?
            .trim()
            .parse::<u64>()
            .map_err(|_| fail(format!("{part} {f} is not a number")))
    };
    if (sectors("start")? * 512, sectors("size")? * 512) != (start, len) {
        return Err(fail(format!(
            "{part} does not match the image layout; recreate the image"
        )));
    }
    let m = fs::symlink_metadata(&dev)?;
    let dev_no = fs::read_to_string(sys.join("dev"))?;
    if m.mode() & libc::S_IFMT != libc::S_IFBLK
        || format!("{}:{}", libc::major(m.rdev()), libc::minor(m.rdev())) != dev_no.trim()
    {
        return Err(fail(format!("{part} device node identity mismatch")));
    }
    Ok(dev)
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
        .prefix(&format!("borea-image-{}-", std::process::id()))
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

pub struct BootstrapOptions<'a> {
    pub password_file: &'a Path,
    pub resume: bool,
    pub yes: bool,
    pub pacman_cache: Option<&'a Path>,
}

/// Execute target Arch bootstrap, base configuration and Limine bootloader on the prepared image layout.
pub fn bootstrap(
    plan: &ImagePlan,
    spec: &RuntimeSpec,
    cache: &Path,
    journal: &Path,
    options: BootstrapOptions<'_>,
) -> Result<()> {
    root()?;
    plan.validate(spec)?;
    if spec.family != FamilyId::Arch {
        return Err(fail(
            "bootstrap executor is currently supported for Arch only; other families remain gated",
        ));
    }
    if !options.yes {
        return Err(fail("image bootstrap requires --yes"));
    }
    let image = locked(&plan.image, false)?;
    let m = image.metadata()?;
    if (m.dev(), m.ino(), m.len()) != (plan.device, plan.inode, plan.bytes) {
        return Err(fail("image identity changed; recreate/replan"));
    }
    let password = read_bootstrap_password(options.password_file)?;
    let mut receipt = locked(journal, !options.resume)?;
    if !options.resume {
        File::open(
            journal
                .parent()
                .ok_or_else(|| fail("journal needs a parent"))?,
        )?
        .sync_all()?;
    }
    if options.resume {
        let mut text = String::new();
        (&mut receipt).take(65537).read_to_string(&mut text)?;
        validate_bootstrap_journal(&text, &plan.digest)?;
    } else {
        writeln!(receipt, "{}", plan.digest)?;
        receipt.sync_all()?;
    }
    let private = tempfile::Builder::new()
        .prefix(&format!("borea-bootstrap-{}-", std::process::id()))
        .tempdir_in("/tmp")?;
    fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700))?;
    let runtime = crate::runtime::private_snapshot(spec, cache, private.path())?;
    fs::create_dir(runtime.join("target"))?;
    fs::write(runtime.join("receipt"), [])?;
    fs::write(runtime.join("bootstrap.sh"), [])?;
    let script = private.path().join("bootstrap.sh");
    fs::write(&script, include_bytes!("image-bootstrap.sh"))?;
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
            "--share-net",
            "--cap-drop",
            "ALL",
            "--cap-add",
            "CAP_SYS_ADMIN",
            "--cap-add",
            "CAP_SYS_CHROOT",
            "--cap-add",
            "CAP_MKNOD",
            "--cap-add",
            "CAP_CHOWN",
            "--cap-add",
            "CAP_FOWNER",
            "--cap-add",
            "CAP_DAC_OVERRIDE",
            "--cap-add",
            "CAP_SETUID",
            "--cap-add",
            "CAP_SETGID",
            "--cap-add",
            "CAP_FSETID",
            "--cap-add",
            "CAP_SETFCAP",
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
    if let Ok(resolv) = fs::canonicalize("/etc/resolv.conf") {
        cmd.arg("--ro-bind").arg(resolv).arg("/etc/resolv.conf");
    }
    if let Some(pkg_cache) = options.pacman_cache.filter(|p| p.is_dir()) {
        cmd.arg("--bind")
            .arg(pkg_cache)
            .arg("/var/cache/pacman/pkg");
    }
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
    cmd.arg("--bind")
        .arg(format!(
            "/proc/{}/fd/{}",
            std::process::id(),
            receipt.as_raw_fd()
        ))
        .arg("/receipt")
        .arg("--ro-bind")
        .arg(script)
        .arg("/bootstrap.sh")
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
            "/bootstrap.sh",
        ])
        .arg(&plan.filesystem)
        .arg(uuid(&plan.seed, "esp"))
        .arg(uuid(&plan.seed, "boot"))
        .arg(uuid(&plan.seed, "root"))
        .arg(uuid(&plan.seed, "disk"))
        .arg(((plan.bytes - MIB) / 512 - 1).to_string())
        .arg(if options.resume { "resume" } else { "new" })
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    cmd.process_group(0);
    let mut process = cmd.spawn()?;
    let mut stdin = process
        .stdin
        .take()
        .ok_or_else(|| fail("missing bootstrap password pipe"))?;
    let write_result = stdin
        .write_all(password.as_bytes())
        .and_then(|()| stdin.write_all(b"\n"));
    drop(stdin);
    if let Err(error) = write_result {
        let _ = process.kill();
        let _ = process.wait();
        return Err(error.into());
    }
    let mut child = RunningChild {
        child: process,
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
        if start.elapsed() > Duration::from_secs(1200) {
            unsafe {
                libc::kill(-(child.child.id() as i32), libc::SIGKILL);
            }
            child.child.wait()?;
            child.running = false;
            return Err(fail(
                "image bootstrap timeout; inspect receipt and resume only if probes pass",
            ));
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let diagnostics = Zeroizing::new(
        reader
            .join()
            .map_err(|_| fail("diagnostic reader failed"))?,
    );
    image.sync_all()?;
    receipt.seek(SeekFrom::Start(0))?;
    let mut text = String::new();
    (&mut receipt).take(65537).read_to_string(&mut text)?;
    validate_bootstrap_journal(&text, &plan.digest)?;
    if !status.success() || !text.ends_with("bootstrap-complete\n") {
        return Err(fail(format!(
            "image bootstrap execution failed; inspect receipt before resume: {}",
            redacted_bootstrap_diagnostics(&diagnostics, &password)
        )));
    }
    Ok(())
}

fn validate_step_journal(
    text: &str,
    digest: &str,
    steps: &[&str],
    final_record: &str,
) -> Result<()> {
    let mut lines = text.lines();
    if lines.next() != Some(digest) || !text.ends_with('\n') || text.len() > 65536 {
        return Err(fail("stale, truncated or invalid image journal"));
    }
    let mut completed = 0;
    let mut active = false;
    for line in lines {
        if line == final_record {
            if active || completed != steps.len() {
                return Err(fail("premature verification receipt"));
            }
            continue;
        }
        let step = steps
            .get(completed)
            .ok_or_else(|| fail("receipt after completed steps"))?;
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

pub fn validate_journal(text: &str, digest: &str) -> Result<()> {
    validate_step_journal(
        text,
        digest,
        &["partition", "esp", "boot", "root"],
        "verified",
    )
}

pub fn validate_bootstrap_journal(text: &str, digest: &str) -> Result<()> {
    validate_step_journal(
        text,
        digest,
        &[
            "storage_probe",
            "pacstrap",
            "system_config",
            "initramfs",
            "bootloader",
        ],
        "bootstrap-complete",
    )
}
/// Inputs for [`apply_plan`].
pub struct PlanOptions<'a> {
    /// Root-private directory holding the plan's host files. A plan path such
    /// as `/run/borea/user.pass` is read from `<staging>/run/borea/user.pass`.
    pub staging: &'a Path,
    pub resume: bool,
    pub yes: bool,
}

/// One generated sandbox program set for a plan step.
#[derive(Debug, PartialEq, Eq)]
enum StepProgram {
    /// Destructive storage step: never executed, only checked against the image.
    Accept { check: String },
    Run {
        probe: String,
        action: String,
        /// Secret slot delivered over the control pipe for this action.
        secret: Option<String>,
        /// File actions must satisfy their own probe afterwards.
        postcondition: bool,
    },
}

fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
fn sh_argv(argv: &[String]) -> String {
    argv.iter()
        .map(|a| sh_quote(a))
        .collect::<Vec<_>>()
        .join(" ")
}
fn sh_path(path: &Path) -> Result<String> {
    path.to_str()
        .map(sh_quote)
        .ok_or_else(|| fail("plan path is not UTF-8"))
}
/// Kernel partition naming: `/dev/vda1`, `/dev/nvme0n1p1`.
fn partition_path(disk: &str, n: u8) -> String {
    let p = if disk.ends_with(|c: char| c.is_ascii_digit()) {
        "p"
    } else {
        ""
    };
    format!("{disk}{p}{n}")
}
fn flag_value<'a>(argv: &'a [String], flag: &str) -> Result<&'a str> {
    argv.iter()
        .position(|a| a == flag)
        .and_then(|i| argv.get(i + 1))
        .map(String::as_str)
        .ok_or_else(|| BoreaError::unsupported(format!("storage step lacks {flag}")))
}
fn hex_id(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

/// Translate a destructive storage command into a read-only check that the
/// image already holds exactly that result. Unknown commands are refused.
fn storage_check(argv: &[String], disk: &str) -> Result<String> {
    let unsupported = |what: &str| {
        BoreaError::unsupported(format!(
            "image plan executor cannot accept destructive step `{what}`; only GPT partitioning and FAT/ext4/Btrfs formatting of a Borea image are supported (no LUKS)"
        ))
    };
    let device = argv.last().map(String::as_str).unwrap_or_default();
    let known = (1..=3).any(|n| partition_path(disk, n) == device);
    let mut out = String::from("set -eu\n");
    match argv[0].as_str() {
        "sgdisk" if device == disk => {
            for arg in &argv[1..argv.len() - 1] {
                let Some((key, value)) = arg.split_once('=') else {
                    if ["--zap-all", "--clear"].contains(&arg.as_str()) {
                        continue;
                    }
                    return Err(unsupported(arg));
                };
                let (n, v) = value.split_once(':').ok_or_else(|| unsupported(arg))?;
                if !["1", "2", "3"].contains(&n) {
                    return Err(unsupported(arg));
                }
                let info = format!("sgdisk --info={n} {}", sh_quote(disk));
                match key {
                    "--partition-guid" if hex_id(v) => {
                        out += &format!(
                            "{info} | grep -qi {}\n",
                            sh_quote(&format!("Partition unique GUID: {v}"))
                        )
                    }
                    "--typecode" => {
                        let code = match v {
                            "ef00" => "C12A7328-F81F-11D2-BA4B-00A0C93EC93B",
                            "8300" => "0FC63DAF-8483-4772-8E79-3D69D8477DE4",
                            _ => return Err(unsupported(arg)),
                        };
                        out += &format!(
                            "{info} | grep -qi {}\n",
                            sh_quote(&format!("Partition GUID code: {code}"))
                        );
                    }
                    "--new" => match v.strip_prefix("0:") {
                        Some("0") => {}
                        Some(size) => {
                            let mib: u64 = size
                                .strip_prefix('+')
                                .and_then(|s| s.strip_suffix('M'))
                                .and_then(|s| s.parse().ok())
                                .ok_or_else(|| unsupported(arg))?;
                            out += &format!(
                                "{info} | grep -q {}\n",
                                sh_quote(&format!("Partition size: {} sectors", mib * 2048))
                            );
                        }
                        None => return Err(unsupported(arg)),
                    },
                    _ => return Err(unsupported(arg)),
                }
            }
        }
        tool @ ("mkfs.fat" | "mkfs.ext4" | "mkfs.btrfs") if known => {
            let (fs, uuid) = match tool {
                "mkfs.fat" => {
                    let id = flag_value(argv, "-i")?;
                    if id.len() != 8 || !hex_id(id) || id.contains('-') {
                        return Err(unsupported("mkfs.fat -i"));
                    }
                    let id = id.to_ascii_uppercase();
                    ("vfat", format!("{}-{}", &id[..4], &id[4..]))
                }
                _ => {
                    let uuid = flag_value(argv, "-U")?;
                    if !hex_id(uuid) {
                        return Err(unsupported("mkfs -U"));
                    }
                    (&tool[5..], uuid.to_string())
                }
            };
            let dev = sh_quote(device);
            out += &format!(
                "test \"$(blkid -p -s TYPE -o value {dev})\" = {fs}\ntest \"$(blkid -p -s UUID -o value {dev})\" = {}\n",
                sh_quote(&uuid)
            );
        }
        other => return Err(unsupported(other)),
    }
    Ok(out)
}

/// Generate the sandbox programs for every step. `files` receives payloads
/// (`c<i>` contents, `n<i>` needles) to place beside the programs.
fn plan_programs(
    plan: &Plan,
    staging: &Path,
    files: &mut Vec<(String, Vec<u8>)>,
) -> Result<Vec<StepProgram>> {
    let disk = plan
        .disk
        .path
        .to_str()
        .ok_or_else(|| fail("plan disk path is not UTF-8"))?;
    let mut out = Vec::new();
    for (i, step) in plan.steps.iter().enumerate() {
        if step.danger == Danger::Destructive {
            let Action::Command {
                argv,
                chroot: false,
                stdin: None,
            } = &step.action
            else {
                return Err(BoreaError::unsupported(format!(
                    "image plan executor cannot accept destructive step {}",
                    step.id
                )));
            };
            out.push(StepProgram::Accept {
                check: storage_check(argv, disk)?,
            });
            continue;
        }
        let probe = match &step.probe {
            Probe::FileExists { path } => {
                format!("p=$(target {})\ntest -e \"$p\"\n", sh_path(path)?)
            }
            Probe::PathMissing { path } => format!(
                "p=$(target {})\ntest ! -e \"$p\"\ntest ! -L \"$p\"\n",
                sh_path(path)?
            ),
            Probe::FileContains { path, needle } if needle.is_empty() => {
                format!("p=$(target {})\ntest -f \"$p\"\n", sh_path(path)?)
            }
            Probe::FileContains { path, needle } => {
                files.push((format!("n{i}"), needle.as_bytes().to_vec()));
                format!(
                    "p=$(target {})\ntest -f \"$p\"\ncontains \"$p\" /plan/n{i} {}\n",
                    sh_path(path)?,
                    u8::from(!needle.ends_with('\n'))
                )
            }
            Probe::CommandOk { argv, chroot: true } => format!("in_target {}\n", sh_argv(argv)),
            Probe::CommandOk {
                argv,
                chroot: false,
            } => format!("{}\n", sh_argv(argv)),
            Probe::None | Probe::OmarchyInstalled { .. } | Probe::PackagesAbsent { .. } => {
                return Err(BoreaError::unsupported(format!(
                    "image plan executor has no probe for step {}",
                    step.id
                )));
            }
        };
        let write = |files: &mut Vec<(String, Vec<u8>)>, dest: &Path, bytes: Vec<u8>, mode: u32| {
            files.push((format!("c{i}"), bytes));
            Ok::<_, BoreaError>(format!(
                "p=$(target {})\nmkdir -p \"$(dirname \"$p\")\"\ninstall -m {mode:o} /plan/c{i} \"$p.borea-tmp\"\nmv -f \"$p.borea-tmp\" \"$p\"\n",
                sh_path(dest)?
            ))
        };
        let mut secret = None;
        let mut postcondition = false;
        let action = match &step.action {
            Action::Command {
                argv,
                chroot,
                stdin,
            } => {
                let run = if *chroot { "in_target " } else { "" };
                match stdin {
                    Some(slot) => {
                        secret = Some(slot.clone());
                        format!("printf '%s' \"$secret\" | {run}{}\n", sh_argv(argv))
                    }
                    None => format!("{run}{}\n", sh_argv(argv)),
                }
            }
            Action::Password { user, secret: slot } => {
                secret = Some(slot.clone());
                format!(
                    "printf '%s:%s\\n' {} \"$secret\" | in_target chpasswd\n",
                    sh_quote(user)
                )
            }
            Action::WriteFile {
                path,
                contents,
                mode,
            } => {
                postcondition = true;
                write(files, path, contents.as_bytes().to_vec(), *mode)?
            }
            Action::CopyIn { source, dest, mode } => {
                postcondition = true;
                let host = staging.join(
                    source
                        .strip_prefix("/")
                        .map_err(|_| fail("copy_in source must be absolute"))?,
                );
                let bytes = fs::read(&host).map_err(|e| {
                    fail(format!(
                        "step {} needs {} in the staging directory: {e}",
                        step.id,
                        source.display()
                    ))
                })?;
                write(files, dest, bytes, *mode)?
            }
            Action::Mkdir { path, mode } => {
                postcondition = true;
                format!(
                    "p=$(target {})\nmkdir -p \"$p\"\nchmod {mode:o} \"$p\"\n",
                    sh_path(path)?
                )
            }
            Action::InstallOmarchy { .. } => {
                return Err(BoreaError::unsupported(
                    "image plan executor does not run the Omarchy installer (E06)",
                ));
            }
        };
        out.push(StepProgram::Run {
            probe: format!("set -eu\n{probe}"),
            action: format!("set -eu\n{action}"),
            secret,
            postcondition,
        });
    }
    Ok(out)
}

/// A running plan sandbox: one operation in, one status out.
struct Driver {
    child: RunningChild,
    stdin: Option<std::process::ChildStdin>,
    status: std::sync::mpsc::Receiver<String>,
    tail: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
    /// Set when the protocol is out of step (timeout, dead sandbox).
    broken: bool,
}
impl Driver {
    fn run(&mut self, op: &str, secret: Option<&str>, timeout: Duration) -> Result<i32> {
        let result = self.exchange(op, secret, timeout);
        self.broken |= result.is_err();
        result
    }
    fn exchange(&mut self, op: &str, secret: Option<&str>, timeout: Duration) -> Result<i32> {
        if self.broken {
            return Err(fail("plan sandbox is no longer usable"));
        }
        let stdin = self.stdin.as_mut().ok_or_else(|| fail("driver closed"))?;
        self.tail
            .lock()
            .map_err(|_| fail("diagnostics lock"))?
            .clear();
        match secret {
            Some(s) => {
                let line = Zeroizing::new(format!("s:{op}\n{s}\n"));
                stdin.write_all(line.as_bytes())?;
            }
            None => stdin.write_all(format!("{op}\n").as_bytes())?,
        }
        stdin.flush()?;
        match self.status.recv_timeout(timeout) {
            Ok(line) => line
                .strip_prefix("rc ")
                .and_then(|n| n.parse().ok())
                .ok_or_else(|| fail("plan sandbox returned an invalid status")),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                Err(fail(format!("timed out after {}s", timeout.as_secs())))
            }
            Err(_) => Err(fail("plan sandbox exited unexpectedly")),
        }
    }
    fn diagnostics(&self, secrets: &[Zeroizing<String>]) -> String {
        let tail = self.tail.lock().map(|t| t.clone()).unwrap_or_default();
        let text = Zeroizing::new(String::from_utf8_lossy(&tail).into_owned());
        crate::redact(&text, secrets)
    }
}

/// Execute a sealed Plan against a Borea disposable image whose storage was
/// prepared by [`apply`] (D-040).
///
/// Destructive storage steps are never run: each is accepted only if the image
/// already holds exactly that partition or filesystem identity. Every other
/// step runs in order inside the runtime sandbox, with its probe evaluated
/// first, and is recorded in the plan-digest-bound journal.
pub fn apply_plan(
    image: &ImagePlan,
    plan: &Plan,
    spec: &RuntimeSpec,
    cache: &Path,
    journal: &Path,
    options: PlanOptions<'_>,
) -> Result<()> {
    root()?;
    image.validate(spec)?;
    plan.validate()?;
    if !options.yes {
        return Err(fail("image plan apply requires --yes"));
    }
    let disk = plan.disk.path.to_str().unwrap_or_default();
    if spec.family != plan.family
        || plan.disk.serial != format!("borea-image-{}", &image.seed[..16])
        || plan.disk.size != image.bytes
        || plan.disk.sector_size != 512
        || plan.root_fs != image.root_fs()?
        || Path::new(disk).parent() != Some(Path::new("/dev"))
        || plan.root.parent() != Some(Path::new("/"))
    {
        return Err(fail(
            "plan was not made for this image: family, disk identity, size, filesystem or target root differ; re-plan against the image",
        ));
    }
    let staging = fs::symlink_metadata(options.staging)?;
    if !staging.is_dir() || staging.uid() != 0 || staging.mode() & 0o077 != 0 {
        return Err(fail("staging must be a root-owned mode-0700 directory"));
    }
    let mut files = Vec::new();
    let programs = plan_programs(plan, options.staging, &mut files)?;
    let mut secrets = Vec::new();
    let mut slots = std::collections::BTreeMap::new();
    for slot in &plan.secrets {
        let used = programs
            .iter()
            .any(|p| matches!(p, StepProgram::Run { secret: Some(s), .. } if *s == slot.name));
        if used {
            let host = options.staging.join(
                slot.file
                    .strip_prefix("/")
                    .map_err(|_| fail("secret slot file must be absolute"))?,
            );
            let value = read_bootstrap_password(&host)?;
            slots.insert(slot.name.as_str(), secrets.len());
            secrets.push(value);
        }
    }
    let file = locked(&image.image, false)?;
    let m = file.metadata()?;
    if (m.dev(), m.ino(), m.len()) != (image.device, image.inode, image.bytes) {
        return Err(fail("image identity changed; recreate/replan"));
    }
    let mut journal = crate::journal::Journal::open(journal, plan, options.resume)?;

    let private = tempfile::Builder::new()
        .prefix(&format!("borea-plan-{}-", std::process::id()))
        .tempdir_in("/tmp")?;
    fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700))?;
    let runtime = crate::runtime::private_snapshot(spec, cache, private.path())?;
    let plan_dir = private.path().join("plan");
    fs::create_dir(&plan_dir)?;
    for (name, bytes) in &files {
        fs::write(plan_dir.join(name), bytes)?;
    }
    for (i, program) in programs.iter().enumerate() {
        match program {
            StepProgram::Accept { check } => fs::write(plan_dir.join(format!("a{i}")), check)?,
            StepProgram::Run { probe, action, .. } => {
                fs::write(plan_dir.join(format!("p{i}")), probe)?;
                fs::write(plan_dir.join(format!("a{i}")), action)?;
            }
        }
    }
    let driver_script = private.path().join("driver.sh");
    fs::write(&driver_script, include_bytes!("image-plan.sh"))?;
    // Mount points inside the read-only runtime root.
    let target = runtime.join(plan.root.strip_prefix("/").unwrap_or(&plan.root));
    fs::create_dir_all(&target)?;
    fs::create_dir_all(runtime.join("plan"))?;
    fs::write(runtime.join("driver.sh"), [])?;
    // The runtime is built from a container image whose NoExtract rules strip
    // configuration, documentation and locales. They must not reach the target.
    let pacman_conf = runtime.join("etc/pacman.conf");
    if let Ok(text) = fs::read_to_string(&pacman_conf) {
        let kept: String = text
            .lines()
            .filter(|l| !l.trim_start().starts_with("NoExtract"))
            .map(|l| format!("{l}\n"))
            .collect();
        fs::write(&pacman_conf, kept)?;
    }

    let whole = Loop::attach_disk(&file)?;
    let parts = [
        loop_partition(&whole, 1, MIB, GIB)?,
        loop_partition(&whole, 2, MIB + GIB, 2 * GIB)?,
        loop_partition(&whole, 3, MIB + 3 * GIB, image.bytes - 2 * MIB - 3 * GIB)?,
    ];
    let mut cmd = Command::new("/usr/bin/bwrap");
    cmd.env_clear().args([
        "--die-with-parent",
        "--unshare-pid",
        "--unshare-ipc",
        "--unshare-uts",
        "--share-net",
        "--cap-drop",
        "ALL",
    ]);
    for cap in [
        "CAP_SYS_ADMIN",
        "CAP_SYS_CHROOT",
        "CAP_MKNOD",
        "CAP_CHOWN",
        "CAP_FOWNER",
        "CAP_DAC_OVERRIDE",
        "CAP_SETUID",
        "CAP_SETGID",
        "CAP_FSETID",
        "CAP_SETFCAP",
    ] {
        cmd.args(["--cap-add", cap]);
    }
    cmd.arg("--ro-bind").arg(&runtime).arg("/").args([
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
    ]);
    if let Ok(resolv) = fs::canonicalize("/etc/resolv.conf") {
        cmd.arg("--ro-bind").arg(resolv).arg("/etc/resolv.conf");
    }
    // The plan's own device paths resolve to the held disk and its kernel
    // partitions.
    cmd.arg("--dev-bind").arg(&whole.path).arg(disk);
    for (n, part) in parts.iter().enumerate() {
        cmd.arg("--dev-bind")
            .arg(part)
            .arg(partition_path(disk, n as u8 + 1));
    }
    cmd.arg("--ro-bind")
        .arg(&plan_dir)
        .arg("/plan")
        .arg("--ro-bind")
        .arg(&driver_script)
        .arg("/driver.sh")
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
            "/driver.sh",
        ])
        .arg(&plan.root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd.process_group(0);
    let mut process = cmd.spawn()?;
    let stdin = process.stdin.take();
    let stdout = process
        .stdout
        .take()
        .ok_or_else(|| fail("missing status pipe"))?;
    let stderr = process
        .stderr
        .take()
        .ok_or_else(|| fail("missing diagnostic pipe"))?;
    let (send, status) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        use std::io::BufRead;
        for line in std::io::BufReader::new(stdout)
            .lines()
            .map_while(|l| l.ok())
        {
            if send.send(line).is_err() {
                break;
            }
        }
    });
    let tail = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = tail.clone();
    std::thread::spawn(move || {
        let mut buf = [0; 4096];
        let mut stream = stderr;
        while let Ok(n) = stream.read(&mut buf) {
            if n == 0 {
                break;
            }
            if let Ok(mut tail) = sink.lock() {
                tail.extend_from_slice(&buf[..n]);
                let extra = tail.len().saturating_sub(16384);
                tail.drain(..extra);
            }
        }
    });
    let mut driver = Driver {
        child: RunningChild {
            child: process,
            running: true,
        },
        stdin,
        status,
        tail,
        broken: false,
    };

    let result = (|| -> Result<()> {
        for (i, (step, program)) in plan.steps.iter().zip(&programs).enumerate() {
            let timeout = Duration::from_secs(step.timeout_s);
            let failed = |driver: &Driver, journal: &mut crate::journal::Journal, what: String| {
                journal.record(&step.id, "failed", Some(&what))?;
                Err(BoreaError::new(
                    "E_STEP",
                    format!("step {} {what}: {}", step.id, driver.diagnostics(&secrets)),
                ))
            };
            match program {
                StepProgram::Accept { .. } => match driver.run(&format!("a{i}"), None, timeout) {
                    Ok(0) => journal.record(
                        &step.id,
                        "skipped",
                        Some("accepted: image storage matches this step"),
                    )?,
                    Ok(_) => {
                        return failed(
                            &driver,
                            &mut journal,
                            "is destructive and the image storage does not match it".into(),
                        );
                    }
                    Err(e) => return failed(&driver, &mut journal, e.message),
                },
                StepProgram::Run {
                    secret,
                    postcondition,
                    ..
                } => {
                    let probe = format!("p{i}");
                    match driver.run(&probe, None, timeout) {
                        Ok(0) => {
                            journal.record(&step.id, "skipped", None)?;
                            continue;
                        }
                        Ok(_) => {}
                        Err(e) => return failed(&driver, &mut journal, e.message),
                    }
                    journal.record(&step.id, "start", None)?;
                    let value = secret
                        .as_ref()
                        .and_then(|s| slots.get(s.as_str()))
                        .map(|&n| secrets[n].as_str());
                    match driver.run(&format!("a{i}"), value, timeout) {
                        Ok(0) => {}
                        Ok(code) => {
                            return failed(&driver, &mut journal, format!("exited {code}"));
                        }
                        Err(e) => return failed(&driver, &mut journal, e.message),
                    }
                    if *postcondition && driver.run(&probe, None, timeout)? != 0 {
                        return failed(&driver, &mut journal, "postcondition failed".into());
                    }
                    journal.record(&step.id, "ok", None)?;
                }
            }
        }
        Ok(())
    })();
    // Ask the driver to sync and unmount, even after a failed step. A sandbox
    // that timed out or died is killed by `RunningChild` instead.
    let teardown = if driver.broken {
        Ok(())
    } else {
        driver
            .run("quit", None, Duration::from_secs(300))
            .and_then(|code| match code {
                0 => Ok(()),
                _ => Err(fail(format!(
                    "target teardown failed: {}",
                    driver.diagnostics(&secrets)
                ))),
            })
    };
    drop(driver.stdin.take());
    if teardown.is_ok() && !driver.broken {
        driver.child.child.wait()?;
        driver.child.running = false;
    }
    drop(driver);
    drop(whole);
    result.and(teardown)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn bootstrap_password_requires_private_single_line_file() {
        let dir = tempfile::tempdir().unwrap();
        let password_path = dir.path().join("password");
        fs::write(&password_path, "test-password\n").unwrap();
        fs::set_permissions(&password_path, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(
            read_bootstrap_password(&password_path).unwrap().as_str(),
            "test-password"
        );

        fs::set_permissions(&password_path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(read_bootstrap_password(&password_path).is_err());
        fs::set_permissions(&password_path, fs::Permissions::from_mode(0o600)).unwrap();
        fs::write(&password_path, "first\nsecond\n").unwrap();
        assert!(read_bootstrap_password(&password_path).is_err());
        fs::write(&password_path, vec![b'a'; 4097]).unwrap();
        assert!(read_bootstrap_password(&password_path).is_err());
        fs::remove_file(&password_path).unwrap();
        std::os::unix::fs::symlink(dir.path().join("missing"), &password_path).unwrap();
        assert!(read_bootstrap_password(&password_path).is_err());
    }

    #[test]
    fn bootstrap_diagnostics_redact_the_password() {
        let password = Zeroizing::new("bootstrap-canary".to_string());
        assert_eq!(
            redacted_bootstrap_diagnostics(b"failure: bootstrap-canary", &password),
            "failure: <redacted>"
        );
    }

    #[test]
    fn bootstrap_script_has_no_default_credentials_or_passwordless_sudo() {
        let script = include_str!("image-bootstrap.sh");
        assert!(!script.contains("NOPASSWD"));
        assert!(script.contains("read -r account_password"));
        assert!(script.contains("printf 'borea:%s\\n' \"$account_password\""));
        assert!(script.contains("passwd --lock root"));
        assert!(script.contains("serial: yes"));
        assert!(script.contains("path: boot():/vmlinuz-linux"));
        assert!(script.contains("module_path: boot():/initramfs-linux.img"));
        assert!(script.contains("borea-limine-sync.hook"));
    }

    #[test]
    fn loop_backing_does_not_carry_the_image_lock() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("disk.img");
        let image = File::create_new(&path).unwrap();
        let lock = |f: &File| unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        assert_eq!(lock(&image), 0);
        let backing = unlocked_backing(&image).unwrap();
        let next = File::open(&path).unwrap();
        assert_ne!(lock(&next), 0, "image lock is held while the stage runs");
        drop(image);
        // The loop driver's reference outlives the stage; the lock must not.
        assert_eq!(lock(&next), 0);
        drop(backing);
    }

    fn golden(name: &str) -> Plan {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/plans")
            .join(name);
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn plan_programs_cover_the_arch_minimal_plan() {
        let plan = golden("arch-minimal.json");
        let staging = tempfile::tempdir().unwrap();
        fs::create_dir_all(staging.path().join("run/borea")).unwrap();
        fs::write(
            staging.path().join("run/borea/test.pub"),
            "ssh-ed25519 AAAA test\n",
        )
        .unwrap();
        let mut files = Vec::new();
        let programs = plan_programs(&plan, staging.path(), &mut files).unwrap();
        assert_eq!(programs.len(), plan.steps.len());
        for (step, program) in plan.steps.iter().zip(&programs) {
            assert_eq!(
                step.danger == Danger::Destructive,
                matches!(program, StepProgram::Accept { .. }),
                "{}",
                step.id
            );
        }
        let find = |id: &str| &programs[plan.steps.iter().position(|s| s.id == id).unwrap()];
        // Destructive steps become read-only identity checks.
        let StepProgram::Accept { check } = find("storage.format.root") else {
            panic!()
        };
        assert!(check.contains("blkid -p -s TYPE -o value '/dev/vda3')\" = btrfs"));
        assert!(!check.contains("mkfs"));
        let StepProgram::Accept { check } = find("storage.partition") else {
            panic!()
        };
        assert!(check.contains("Partition size: 2097152 sectors"));
        assert!(check.contains("C12A7328-F81F-11D2-BA4B-00A0C93EC93B"));
        assert!(!check.contains("--zap-all"));
        // The password travels over the control pipe, never in a program file.
        let StepProgram::Run { action, secret, .. } = find("base.password") else {
            panic!()
        };
        assert_eq!(secret.as_deref(), Some("account.password"));
        assert!(action.contains("\"$secret\" | in_target chpasswd"));
        let StepProgram::Run {
            action,
            probe,
            postcondition,
            ..
        } = find("base.hostname")
        else {
            panic!()
        };
        assert!(*postcondition && action.contains("target '/etc/hostname'"));
        assert!(probe.contains("contains \"$p\" /plan/n"));
        let StepProgram::Run { action, .. } = find("base.timezone") else {
            panic!()
        };
        assert!(action.starts_with("set -eu\nin_target 'ln' '-sf'"));
        assert!(files.iter().any(|(_, b)| b == b"ssh-ed25519 AAAA test\n"));
    }

    #[test]
    fn plan_programs_refuse_what_the_image_lane_cannot_prove() {
        let staging = tempfile::tempdir().unwrap();
        let mut files = Vec::new();
        // LUKS formatting is destructive and has no image identity check.
        let mut luks = golden("arch-minimal.json");
        let step = luks
            .steps
            .iter_mut()
            .find(|s| s.id == "storage.format.root")
            .unwrap();
        step.action = Action::Command {
            argv: vec!["cryptsetup".into(), "luksFormat".into(), "/dev/vda3".into()],
            chroot: false,
            stdin: None,
        };
        let e = plan_programs(&luks, staging.path(), &mut files).unwrap_err();
        assert_eq!(e.code, "E_UNSUPPORTED");
        // A missing copy_in source fails before anything runs.
        let e =
            plan_programs(&golden("arch-minimal.json"), staging.path(), &mut files).unwrap_err();
        assert!(e.message.contains("staging"));
        for argv in [
            vec!["sgdisk", "--zap-all", "/dev/vdb"],
            vec!["sgdisk", "--typecode=1:8309", "/dev/vda"],
            vec!["mkfs.ext4", "-U", "x; rm -rf /", "/dev/vda2"],
            vec![
                "mkfs.ext4",
                "-U",
                "5bca32f4-42b8-5018-b454-0a6b5dab2a01",
                "/dev/sda2",
            ],
            vec!["wipefs", "-a", "/dev/vda"],
        ] {
            let argv: Vec<String> = argv.into_iter().map(String::from).collect();
            assert!(storage_check(&argv, "/dev/vda").is_err(), "{argv:?}");
        }
    }

    #[test]
    fn shell_quoting_and_partition_names() {
        assert_eq!(sh_quote("a'b $(x)"), "'a'\\''b $(x)'");
        assert_eq!(partition_path("/dev/vda", 3), "/dev/vda3");
        assert_eq!(partition_path("/dev/nvme0n1", 1), "/dev/nvme0n1p1");
    }

    #[test]
    fn bootstrap_journals_validate() {
        let valid = "abc\nstart:storage_probe\nok:storage_probe\nstart:pacstrap\nok:pacstrap\nstart:system_config\nok:system_config\nstart:initramfs\nok:initramfs\nstart:bootloader\nok:bootloader\nbootstrap-complete\n";
        assert!(validate_bootstrap_journal(valid, "abc").is_ok());
        assert!(
            validate_bootstrap_journal(
                &valid.replace("bootstrap-complete", "verified-bootable"),
                "abc"
            )
            .is_err()
        );
        assert!(validate_bootstrap_journal("abc\nstart:storage_probe\n", "abc").is_ok());
        for bad in [
            "abc",
            "wrong_digest\n",
            "abc\nstart:storage_probe\nok:storage_probe\nbootstrap-complete\n",
            "abc\nstart:unknown\n",
            "abc\nok:bootloader\n",
        ] {
            assert!(validate_bootstrap_journal(bad, "abc").is_err());
        }
    }
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
