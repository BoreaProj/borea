use borea_core::{
    BoreaError, Check, DeviceIdentity, FamilyId, Filesystem, Loader, Plan, Result, digest,
};
use borea_run::image::ImagePlan;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    net::TcpListener,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const BOOT_TIMEOUT: Duration = Duration::from_secs(240);
const SSH_TIMEOUT: Duration = Duration::from_secs(180);
const SECTOR: u64 = 512;
/// Serial output that proves boot stopped before a usable system.
const FAILURE_MARKERS: &[&str] = &[
    "kernel panic",
    "emergency mode",
    "emergency shell",
    "recovery shell",
    "cannot open access to console",
    "failed to mount /sysroot",
    "dependency failed for initrd root",
];

struct RunningVm(Child);

impl RunningVm {
    fn stop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

impl Drop for RunningVm {
    fn drop(&mut self) {
        self.stop();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expect {
    Pass,
    Fail,
}

pub struct BootArgs {
    pub image: PathBuf,
    pub filesystem: String,
    pub serial_log: PathBuf,
    /// Plan and report path for `borea verify --ssh` once the VM is up.
    pub verify: Option<(PathBuf, PathBuf)>,
    pub expect: Expect,
}

impl BootArgs {
    pub fn parse(args: &[&str]) -> Result<Self> {
        let usage = || {
            BoreaError::invalid(
                "usage: cargo xtask vm boot --image ABSOLUTE_IMAGE --fs btrfs|ext4 --serial-log ABSOLUTE_LOG [--plan PLAN --report ABSOLUTE_REPORT] [--expect pass|fail]",
            )
        };
        let mut values = std::collections::BTreeMap::new();
        let mut rest = args;
        while let [key, value, tail @ ..] = rest {
            let key = key.strip_prefix("--").ok_or_else(usage)?;
            if !["image", "fs", "serial-log", "plan", "report", "expect"].contains(&key)
                || values.insert(key, *value).is_some()
            {
                return Err(usage());
            }
            rest = tail;
        }
        if !rest.is_empty() {
            return Err(usage());
        }
        let get = |k: &str| values.get(k).copied();
        let expect = match get("expect").unwrap_or("pass") {
            "pass" => Expect::Pass,
            "fail" => Expect::Fail,
            _ => return Err(usage()),
        };
        let verify = match (get("plan"), get("report")) {
            (Some(plan), Some(report)) => Some((PathBuf::from(plan), PathBuf::from(report))),
            (None, None) => None,
            _ => return Err(usage()),
        };
        if expect == Expect::Fail && verify.is_some() {
            return Err(BoreaError::invalid(
                "--expect fail boots a broken image; it cannot also run SSH verification",
            ));
        }
        Ok(Self {
            image: get("image").ok_or_else(usage)?.into(),
            filesystem: get("fs").ok_or_else(usage)?.into(),
            serial_log: get("serial-log").ok_or_else(usage)?.into(),
            verify,
            expect,
        })
    }
}

/// SSH access to a VM: a fresh test key injected through a systemd credential,
/// never written into the image, and a forwarded loopback port.
struct SshAccess {
    _dir: tempfile::TempDir,
    identity: PathBuf,
    known_hosts: PathBuf,
    public_key: String,
    port: u16,
}

impl SshAccess {
    fn new() -> Result<Self> {
        let dir = tempfile::Builder::new().prefix("borea-vm-ssh-").tempdir()?;
        let identity = dir.path().join("id_ed25519");
        let status = Command::new("ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-C", "borea-vm-test", "-f"])
            .arg(&identity)
            .stdin(Stdio::null())
            .status()
            .map_err(|_| {
                BoreaError::unsupported("ssh-keygen is required; install openssh-client")
            })?;
        if !status.success() {
            return Err(BoreaError::new("E_VM", "ssh-keygen failed"));
        }
        let public_key = fs::read_to_string(identity.with_extension("pub"))?;
        let known_hosts = dir.path().join("known_hosts");
        File::create(&known_hosts)?;
        let port = TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();
        Ok(Self {
            _dir: dir,
            identity,
            known_hosts,
            public_key,
            port,
        })
    }

    fn smbios_credential(&self) -> String {
        format!(
            "type=11,value=io.systemd.credential.binary:ssh.authorized_keys.root={}",
            base64(self.public_key.trim_end().as_bytes())
        )
    }

    fn command(&self) -> Command {
        let mut cmd = Command::new("ssh");
        cmd.args(["-o", "BatchMode=yes", "-o", "ConnectTimeout=5"])
            .args(["-o", "IdentitiesOnly=yes", "-i"])
            .arg(&self.identity)
            .arg("-o")
            .arg(format!("UserKnownHostsFile={}", self.known_hosts.display()))
            .args(["-o", "StrictHostKeyChecking=accept-new", "-p"])
            .arg(self.port.to_string());
        cmd
    }
}

fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | (u32::from(*b) << (16 - 8 * i)));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(TABLE[(n >> (18 - 6 * i)) as usize & 63] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn regular_file(path: &Path, what: &str) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() || metadata.len() == 0 {
        return Err(BoreaError::invalid(format!(
            "{what} must be a nonempty regular file, not a symlink"
        )));
    }
    Ok(())
}

pub fn boot(args: &BootArgs) -> Result<()> {
    let filesystem = args.filesystem.as_str();
    let (image, serial_log) = (args.image.as_path(), args.serial_log.as_path());
    if !matches!(filesystem, "btrfs" | "ext4") {
        return Err(BoreaError::invalid("VM filesystem must be btrfs or ext4"));
    }
    if !image.is_absolute()
        || !serial_log.is_absolute()
        || args.verify.as_ref().is_some_and(|(_, r)| !r.is_absolute())
    {
        return Err(BoreaError::invalid(
            "VM image, serial log and report paths must be absolute",
        ));
    }
    regular_file(image, "VM image")?;
    let borea = match &args.verify {
        Some((plan, _)) => {
            regular_file(plan, "verify plan")?;
            let borea = std::env::current_exe()?.with_file_name("borea");
            if !borea.is_file() {
                return Err(BoreaError::unsupported(
                    "borea binary not found next to xtask; build -p borea-cli in the same profile",
                ));
            }
            Some(borea)
        }
        None => None,
    };
    let version = Command::new("qemu-system-x86_64")
        .arg("--version")
        .output()
        .map_err(|_| {
            BoreaError::unsupported(
                "qemu-system-x86_64 is required; install qemu-system-x86 and ovmf",
            )
        })?;
    if !version.status.success() {
        return Err(BoreaError::unsupported(
            "qemu-system-x86_64 could not run; install a working qemu-system-x86 package",
        ));
    }
    let (ovmf_code, ovmf_vars) = find_ovmf()?;
    let ssh = borea.as_ref().map(|_| SshAccess::new()).transpose()?;

    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(serial_log)?;
    let qemu_log = serial_log.with_extension("qemu.log");
    let qemu_stderr = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&qemu_log)?;
    let temporary = tempfile::Builder::new().prefix("borea-ovmf-").tempdir()?;
    let vars_copy = temporary.path().join("OVMF_VARS.fd");
    fs::copy(ovmf_vars, &vars_copy)?;

    let mut qemu = Command::new("qemu-system-x86_64");
    qemu.args(["-machine", "q35,accel=tcg", "-m", "2048", "-smp", "2"])
        .arg("-drive")
        .arg(format!("if=pflash,format=raw,readonly=on,file={ovmf_code}"))
        .arg("-drive")
        .arg(format!("if=pflash,format=raw,file={}", vars_copy.display()))
        .arg("-drive")
        .arg(format!(
            "if=none,id=disk,format=raw,file={}",
            image.display()
        ))
        .arg("-device")
        .arg("ich9-ahci,id=sata")
        .arg("-device")
        .arg("ide-hd,drive=disk,bus=sata.0,bootindex=1")
        .arg("-boot")
        .arg("order=c,menu=off,strict=on")
        .args([
            "-display",
            "none",
            "-monitor",
            "none",
            "-no-reboot",
            "-snapshot",
        ]);
    match &ssh {
        Some(ssh) => {
            qemu.arg("-netdev")
                .arg(format!(
                    "user,id=net0,restrict=off,hostfwd=tcp:127.0.0.1:{}-:22",
                    ssh.port
                ))
                .args(["-device", "virtio-net-pci,netdev=net0,romfile="])
                .arg("-smbios")
                .arg(ssh.smbios_credential());
        }
        None => {
            qemu.args(["-nic", "none"]);
        }
    }
    let mut vm = RunningVm(
        qemu.arg("-serial")
            .arg(format!("file:{}", serial_log.display()))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::from(qemu_stderr))
            .spawn()?,
    );
    let logs = format!(
        "inspect {} and {}",
        serial_log.display(),
        qemu_log.display()
    );

    let started = Instant::now();
    let outcome = loop {
        let serial = fs::read_to_string(serial_log)?;
        if serial.contains("borea login:") {
            break verify_serial_log(&serial, filesystem).map(|()| "login");
        }
        let lower = strip_ansi(&serial).to_lowercase();
        if let Some(marker) = FAILURE_MARKERS.iter().find(|m| lower.contains(*m)) {
            break Err(BoreaError::new(
                "E_VM",
                format!("boot failed: serial log reports \"{marker}\"; {logs}"),
            ));
        }
        if let Some(status) = vm.0.try_wait()? {
            break Err(BoreaError::new(
                "E_VM",
                format!("QEMU exited with {status} before the login prompt; {logs}"),
            ));
        }
        if started.elapsed() >= BOOT_TIMEOUT {
            break Err(BoreaError::new(
                "E_VM",
                format!(
                    "VM did not reach the login prompt within {} seconds; {logs}",
                    BOOT_TIMEOUT.as_secs()
                ),
            ));
        }
        thread::sleep(Duration::from_secs(1));
    };

    if args.expect == Expect::Fail {
        vm.stop();
        let serial = strip_ansi(&fs::read_to_string(serial_log)?);
        return match outcome {
            Ok(_) => Err(BoreaError::new(
                "E_VM",
                format!("image was expected to fail but reached the login prompt; {logs}"),
            )),
            Err(_) if !serial.contains("Linux version") => Err(BoreaError::new(
                "E_VM",
                format!(
                    "expected failure did not reach the kernel, so the serial log proves nothing; {logs}"
                ),
            )),
            Err(e) => {
                println!(
                    "{}",
                    serde_json::json!({
                        "schema": 1,
                        "filesystem": filesystem,
                        "expect": "fail",
                        "boot_to_login": "failed_as_expected",
                        "failure": e.to_string(),
                        "serial_log": serial_log.display().to_string(),
                        "qemu_log": qemu_log.display().to_string()
                    })
                );
                Ok(())
            }
        };
    }
    outcome?;

    let mut ssh_verify = serde_json::Value::Null;
    if let (Some(ssh), Some(borea), Some((plan, report))) = (&ssh, &borea, &args.verify) {
        wait_for_ssh(ssh, &mut vm, &logs)?;
        let output = Command::new(borea)
            .arg("verify")
            .arg(plan)
            .args(["--ssh", "root@127.0.0.1", "--ssh-port"])
            .arg(ssh.port.to_string())
            .arg("--identity")
            .arg(&ssh.identity)
            .arg("--known-hosts")
            .arg(&ssh.known_hosts)
            .arg("--json")
            .stdin(Stdio::null())
            .output()?;
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(report)?
            .write_all(&output.stdout)?;
        let code = output.status.code().unwrap_or(-1);
        if !output.status.success() {
            return Err(BoreaError::new(
                "E_VERIFY",
                format!(
                    "borea verify --ssh exited {code}: {}; report {}; {logs}",
                    String::from_utf8_lossy(&output.stderr).trim(),
                    report.display()
                ),
            ));
        }
        ssh_verify = serde_json::json!({
            "result": "passed",
            "exit_code": code,
            "report": report.display().to_string()
        });
    }
    vm.stop();
    println!(
        "{}",
        serde_json::json!({
            "schema": 1,
            "filesystem": filesystem,
            "boot_to_login": "passed",
            "root_mount": filesystem,
            "bootloader": "limine",
            "vm_boot_verified": true,
            "ssh_verify": ssh_verify,
            "serial_log": serial_log.display().to_string(),
            "qemu_log": qemu_log.display().to_string()
        })
    );
    Ok(())
}

fn wait_for_ssh(ssh: &SshAccess, vm: &mut RunningVm, logs: &str) -> Result<()> {
    let started = Instant::now();
    loop {
        let status = ssh
            .command()
            .args(["--", "root@127.0.0.1", "true"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if status.success() {
            return Ok(());
        }
        if let Some(status) = vm.0.try_wait()? {
            return Err(BoreaError::new(
                "E_VM",
                format!("QEMU exited with {status} while waiting for SSH; {logs}"),
            ));
        }
        if started.elapsed() >= SSH_TIMEOUT {
            return Err(BoreaError::new(
                "E_VM",
                format!(
                    "SSH with the injected test key did not succeed within {} seconds; {logs}",
                    SSH_TIMEOUT.as_secs()
                ),
            ));
        }
        thread::sleep(Duration::from_secs(3));
    }
}

/// Verification-only plan for a disposable Arch bootstrap image. It carries
/// checks derived from the image manifest and no installation steps.
pub fn verify_plan(image: &ImagePlan) -> Result<Plan> {
    let fs = image.filesystem.as_str();
    let root_fs = match fs {
        "btrfs" => Filesystem::Btrfs,
        "ext4" => Filesystem::Ext4,
        _ => {
            return Err(BoreaError::invalid(
                "image filesystem must be btrfs or ext4",
            ));
        }
    };
    let (root_uuid, boot_uuid) = (image.uuid("root"), image.uuid("boot"));
    let cmd = |argv: &[&str]| Check::CommandOk {
        argv: argv.iter().map(|s| (*s).into()).collect(),
    };
    let contains = |path: &str, needle: String| Check::FileContains {
        path: path.into(),
        needle,
    };
    let mut checks = vec![
        contains("/etc/fstab", format!("UUID={root_uuid} / {fs} ")),
        contains("/etc/fstab", format!("UUID={boot_uuid} /boot ext4 ")),
        contains("/etc/hostname", "borea".into()),
        contains(
            "/boot/efi/EFI/BOOT/limine.conf",
            format!("root=UUID={root_uuid}"),
        ),
        cmd(&["findmnt", "--mountpoint", "/", "--types", fs]),
        cmd(&["findmnt", "--mountpoint", "/boot", "--types", "ext4"]),
        cmd(&["findmnt", "--mountpoint", "/boot/efi", "--types", "vfat"]),
        cmd(&["id", "-u", "borea"]),
        cmd(&["systemctl", "is-enabled", "NetworkManager.service"]),
        cmd(&["systemctl", "is-active", "sshd.service"]),
        Check::FileExists {
            path: "/boot/efi/EFI/BOOT/BOOTX64.EFI".into(),
        },
        Check::FileExists {
            path: "/boot/efi/vmlinuz-linux".into(),
        },
        Check::FileExists {
            path: "/boot/efi/initramfs-linux.img".into(),
        },
        Check::PathMissing {
            path: "/boot/grub".into(),
        },
        Check::PackagesAbsent {
            packages: vec!["grub".into()],
        },
    ];
    if root_fs == Filesystem::Btrfs {
        for mountpoint in ["/home", "/.snapshots", "/var/log", "/var/cache"] {
            checks.push(cmd(&[
                "findmnt",
                "--mountpoint",
                mountpoint,
                "--types",
                "btrfs",
            ]));
        }
    }
    let mut plan = Plan {
        schema: 2,
        id: String::new(),
        created: "1970-01-01T00:00:00Z".into(),
        borea_version: borea_core::version().into(),
        digest: String::new(),
        config_digest: image.digest.clone(),
        matrix_digest: digest(&borea_core::matrix::Matrix::embedded()?)?,
        family: FamilyId::Arch,
        disk: DeviceIdentity {
            path: "/dev/sda".into(),
            serial: format!("borea-image-{}", &image.seed[..16]),
            size: image.bytes,
            sector_size: SECTOR,
        },
        root_fs,
        loader: Loader::Limine,
        root: "/mnt".into(),
        steps: Vec::new(),
        checks,
        secrets: Vec::new(),
        notes: vec![
            "Verification-only plan for a disposable Borea Arch bootstrap image; it has no installation steps.".into(),
        ],
    };
    plan.seal()?;
    Ok(plan)
}

/// Copy a bootstrapped image and zero the start of its root partition, which
/// destroys the root filesystem superblock while leaving firmware, Limine and
/// the kernel intact. Booting the copy must fail after the kernel starts.
pub fn corrupt_root(source: &Path, out: &Path) -> Result<()> {
    if !source.is_absolute() || !out.is_absolute() {
        return Err(BoreaError::invalid("corrupt-root paths must be absolute"));
    }
    regular_file(source, "source image")?;
    let root_start = gpt_partition_start(&mut File::open(source)?, 3)?;
    // Claim the destination exclusively before copying into it.
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(out)?;
    let status = Command::new("cp")
        .args(["--sparse=always", "--"])
        .arg(source)
        .arg(out)
        .status()?;
    if !status.success() {
        return Err(BoreaError::new("E_VM", "sparse image copy failed"));
    }
    let mut output = OpenOptions::new().write(true).open(out)?;
    output.seek(SeekFrom::Start(root_start * SECTOR))?;
    output.write_all(&vec![0u8; 1024 * 1024])?;
    output.sync_all()?;
    println!(
        "{}",
        serde_json::json!({
            "schema": 1,
            "corrupted": out.display().to_string(),
            "root_partition_start_lba": root_start,
            "zeroed_bytes": 1024 * 1024
        })
    );
    Ok(())
}

fn gpt_partition_start(disk: &mut File, number: u32) -> Result<u64> {
    let bad = || {
        BoreaError::new(
            "E_VM",
            "image has no valid GPT with the expected root partition",
        )
    };
    let mut header = [0u8; 92];
    disk.seek(SeekFrom::Start(SECTOR))?;
    disk.read_exact(&mut header)?;
    if &header[..8] != b"EFI PART" {
        return Err(bad());
    }
    let u64_at = |b: &[u8], o: usize| u64::from_le_bytes(b[o..o + 8].try_into().expect("8 bytes"));
    let u32_at = |b: &[u8], o: usize| u32::from_le_bytes(b[o..o + 4].try_into().expect("4 bytes"));
    let (entries, count, size) = (
        u64_at(&header, 72),
        u32_at(&header, 80),
        u32_at(&header, 84),
    );
    if number == 0 || number > count || !(128..=4096).contains(&size) {
        return Err(bad());
    }
    let mut entry = vec![0u8; size as usize];
    disk.seek(SeekFrom::Start(
        entries * SECTOR + u64::from(number - 1) * u64::from(size),
    ))?;
    disk.read_exact(&mut entry)?;
    let first = u64_at(&entry, 32);
    if entry[..16].iter().all(|b| *b == 0) || first == 0 {
        return Err(bad());
    }
    Ok(first)
}

fn find_ovmf() -> Result<(&'static str, &'static str)> {
    const PAIRS: &[(&str, &str)] = &[
        (
            "/usr/share/OVMF/OVMF_CODE_4M.fd",
            "/usr/share/OVMF/OVMF_VARS_4M.fd",
        ),
        (
            "/usr/share/OVMF/OVMF_CODE.fd",
            "/usr/share/OVMF/OVMF_VARS.fd",
        ),
        (
            "/usr/share/edk2/x64/OVMF_CODE.4m.fd",
            "/usr/share/edk2/x64/OVMF_VARS.4m.fd",
        ),
        (
            "/usr/share/edk2-ovmf/x64/OVMF_CODE.fd",
            "/usr/share/edk2-ovmf/x64/OVMF_VARS.fd",
        ),
    ];
    PAIRS
        .iter()
        .copied()
        .find(|(code, vars)| Path::new(code).is_file() && Path::new(vars).is_file())
        .ok_or_else(|| {
            BoreaError::unsupported("OVMF firmware files not found; install the ovmf package")
        })
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            match chars.peek() {
                Some(']') => {
                    chars.next();
                    while let Some(sc) = chars.next() {
                        if sc == '\x07' {
                            break;
                        }
                        if sc == '\x1b' && chars.peek() == Some(&'\\') {
                            chars.next();
                            break;
                        }
                    }
                }
                Some('[') => {
                    chars.next();
                    while let Some(&sc) = chars.peek() {
                        chars.next();
                        if sc.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
                _ => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn verify_serial_log(serial: &str, filesystem: &str) -> Result<()> {
    let clean = strip_ansi(serial);
    if let Some(cmdline) = clean.lines().find_map(|line| {
        line.split_once("Kernel command line:")
            .map(|(_, rest)| rest)
    }) {
        if !cmdline.contains("root=UUID=") {
            return Err(BoreaError::new(
                "E_VM",
                "kernel command line is missing the root UUID",
            ));
        }
        if filesystem == "btrfs" && !cmdline.contains("rootflags=subvol=@") {
            return Err(BoreaError::new(
                "E_VM",
                "Btrfs kernel command line is missing rootflags=subvol=@",
            ));
        }
        if filesystem == "ext4" && cmdline.contains("rootflags=subvol=") {
            return Err(BoreaError::new(
                "E_VM",
                "ext4 kernel command line unexpectedly contains a Btrfs subvolume flag",
            ));
        }
    } else if !clean.contains("Expecting device /dev/disk/by-uuid") && !clean.contains("root=UUID=")
    {
        return Err(BoreaError::new(
            "E_VM",
            "kernel command line or root UUID is absent from serial log",
        ));
    }

    let root_mounted = clean.contains(&format!("VFS: Mounted root ({filesystem} filesystem)"))
        || (clean.contains("Mounted /sysroot.")
            && match filesystem {
                "btrfs" => clean.contains("Mounted /home.") && clean.contains("Mounted /var/log."),
                "ext4" => !clean.contains("Mounted /home.") && !clean.contains("Mounted /var/log."),
                _ => false,
            });
    if !root_mounted {
        return Err(BoreaError::new(
            "E_VM",
            format!("VM did not report mounting the {filesystem} root filesystem"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_rfc4648_vectors() {
        for (input, encoded) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(input.as_bytes()), encoded);
        }
    }

    #[test]
    fn boot_args_parse_verify_and_expect() {
        let args = BootArgs::parse(&[
            "--image",
            "/i.img",
            "--fs",
            "ext4",
            "--serial-log",
            "/s.log",
            "--plan",
            "/p.json",
            "--report",
            "/r.json",
        ])
        .unwrap();
        assert_eq!(args.expect, Expect::Pass);
        assert!(args.verify.is_some());
        assert!(
            BootArgs::parse(&[
                "--image",
                "/i",
                "--fs",
                "ext4",
                "--serial-log",
                "/s",
                "--plan",
                "/p"
            ])
            .is_err()
        );
        assert!(
            BootArgs::parse(&[
                "--image",
                "/i",
                "--fs",
                "ext4",
                "--serial-log",
                "/s",
                "--expect",
                "fail",
                "--plan",
                "/p",
                "--report",
                "/r"
            ])
            .is_err()
        );
        assert!(
            BootArgs::parse(&[
                "--image",
                "/i",
                "--image",
                "/j",
                "--fs",
                "ext4",
                "--serial-log",
                "/s"
            ])
            .is_err()
        );
    }

    #[test]
    fn verify_plan_is_sealed_with_image_uuids() {
        for fs in ["btrfs", "ext4"] {
            let image = ImagePlan {
                schema: 1,
                image: "/tmp/disk.img".into(),
                device: 1,
                inode: 2,
                bytes: 20 * 1024 * 1024 * 1024,
                filesystem: fs.into(),
                runtime_sha256: "0".repeat(64),
                seed: "ab".repeat(32),
                digest: "cd".repeat(32),
            };
            let plan = verify_plan(&image).unwrap();
            plan.validate().unwrap();
            assert!(plan.steps.is_empty());
            let json = plan.json().unwrap();
            assert!(json.contains(&format!("UUID={} / {fs} ", image.uuid("root"))));
            assert_eq!(json.contains("/var/log"), fs == "btrfs");
        }
    }

    #[test]
    fn finds_gpt_partition_start() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("disk.img");
        let mut disk = vec![0u8; 4 * SECTOR as usize + 3 * 128];
        let header = SECTOR as usize;
        disk[header..header + 8].copy_from_slice(b"EFI PART");
        disk[header + 72..header + 80].copy_from_slice(&2u64.to_le_bytes());
        disk[header + 80..header + 84].copy_from_slice(&128u32.to_le_bytes());
        disk[header + 84..header + 88].copy_from_slice(&128u32.to_le_bytes());
        let entry = 2 * SECTOR as usize + 2 * 128;
        disk[entry] = 1;
        disk[entry + 32..entry + 40].copy_from_slice(&6293504u64.to_le_bytes());
        fs::write(&path, disk).unwrap();
        let mut file = File::open(&path).unwrap();
        assert_eq!(gpt_partition_start(&mut file, 3).unwrap(), 6293504);
        assert!(gpt_partition_start(&mut file, 2).is_err());
    }

    #[test]
    fn verifies_btrfs_root_mount_and_subvolume_cmdline() {
        let log = "Kernel command line: root=UUID=abc rootflags=subvol=@ rw\nVFS: Mounted root (btrfs filesystem) readonly on device 0:44\nborea login:";
        assert!(verify_serial_log(log, "btrfs").is_ok());
    }

    #[test]
    fn verifies_ext4_root_mount_without_subvolume_flag() {
        let log = "Kernel command line: root=UUID=abc rw\nVFS: Mounted root (ext4 filesystem) readonly on device 0:44\nborea login:";
        assert!(verify_serial_log(log, "ext4").is_ok());
    }

    #[test]
    fn rejects_wrong_root_filesystem() {
        let log = "Kernel command line: root=UUID=abc rw\nVFS: Mounted root (btrfs filesystem) readonly on device 0:44\nborea login:";
        assert!(verify_serial_log(log, "ext4").is_err());
    }

    #[test]
    fn verifies_systemd_initrd_btrfs_mount() {
        let log = "Expecting device /dev/disk/by-uuid/1234...\nMounted /sysroot.\nMounted /home.\nMounted /var/log.\nborea login:";
        assert!(verify_serial_log(log, "btrfs").is_ok());
    }

    #[test]
    fn verifies_systemd_initrd_ext4_mount() {
        let log = "Expecting device /dev/disk/by-uuid/1234...\nMounted /sysroot.\nMounted /boot.\nborea login:";
        assert!(verify_serial_log(log, "ext4").is_ok());
    }
}
