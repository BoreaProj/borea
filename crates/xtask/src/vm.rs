use borea_core::{BoreaError, Result};
use std::{
    fs::{self, OpenOptions},
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const BOOT_TIMEOUT: Duration = Duration::from_secs(240);

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

pub fn boot(image: &Path, filesystem: &str, serial_log: &Path) -> Result<()> {
    if !matches!(filesystem, "btrfs" | "ext4") {
        return Err(BoreaError::invalid("VM filesystem must be btrfs or ext4"));
    }
    if !image.is_absolute() || !serial_log.is_absolute() {
        return Err(BoreaError::invalid(
            "VM image and serial log paths must be absolute",
        ));
    }
    let image_metadata = fs::symlink_metadata(image)?;
    if !image_metadata.file_type().is_file() || image_metadata.len() == 0 {
        return Err(BoreaError::invalid(
            "VM image must be a nonempty regular file, not a symlink",
        ));
    }
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

    let mut vm = RunningVm(
        Command::new("qemu-system-x86_64")
            .args(["-machine", "q35,accel=tcg", "-m", "2048", "-smp", "2"])
            .arg("-drive")
            .arg(format!("if=pflash,format=raw,readonly=on,file={ovmf_code}"))
            .arg("-drive")
            .arg(format!("if=pflash,format=raw,file={}", vars_copy.display()))
            .arg("-drive")
            .arg(format!("file={},format=raw,if=virtio", image.display()))
            .args([
                "-display",
                "none",
                "-monitor",
                "none",
                "-no-reboot",
                "-nic",
                "none",
                "-snapshot",
            ])
            .arg("-serial")
            .arg(format!("file:{}", serial_log.display()))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::from(qemu_stderr))
            .spawn()?,
    );

    let started = Instant::now();
    loop {
        let serial = fs::read_to_string(serial_log)?;
        if serial.contains("borea login:") {
            verify_serial_log(&serial, filesystem)?;
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
                    "serial_log": serial_log.display().to_string(),
                    "qemu_log": qemu_log.display().to_string()
                })
            );
            return Ok(());
        }
        if let Some(status) = vm.0.try_wait()? {
            return Err(BoreaError::new(
                "E_VM",
                format!(
                    "QEMU exited with {status} before the login prompt; inspect {} and {}",
                    serial_log.display(),
                    qemu_log.display()
                ),
            ));
        }
        if started.elapsed() >= BOOT_TIMEOUT {
            return Err(BoreaError::new(
                "E_VM",
                format!(
                    "VM did not reach the login prompt within {} seconds; inspect {} and {}",
                    BOOT_TIMEOUT.as_secs(),
                    serial_log.display(),
                    qemu_log.display()
                ),
            ));
        }
        thread::sleep(Duration::from_secs(1));
    }
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

fn verify_serial_log(serial: &str, filesystem: &str) -> Result<()> {
    let cmdline = serial
        .lines()
        .find_map(|line| {
            line.split_once("Kernel command line:")
                .map(|(_, rest)| rest)
        })
        .ok_or_else(|| BoreaError::new("E_VM", "kernel command line is absent from serial log"))?;
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
    if !serial.contains(&format!("VFS: Mounted root ({filesystem} filesystem)")) {
        return Err(BoreaError::new(
            "E_VM",
            format!("VM did not report mounting the {filesystem} root filesystem"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::verify_serial_log;

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
}
