//! Read-only device inventory. No device mutation takes place here.
use borea_core::*;
use serde_json::Value;
use std::{os::unix::fs::FileTypeExt, path::Path, process::Command};
pub fn identity(path: &Path) -> Result<DeviceIdentity> {
    if !safe_path(path) || !path.starts_with("/dev") {
        return Err(BoreaError::invalid("expected absolute /dev path"));
    }
    let resolved = std::fs::canonicalize(path)?;
    if !std::fs::metadata(&resolved)?.file_type().is_block_device() {
        return Err(BoreaError::invalid("target is not a block device"));
    }
    let out = Command::new("lsblk")
        .args([
            "--json",
            "--bytes",
            "--paths",
            "--output",
            "PATH,SERIAL,SIZE,LOG-SEC,TYPE,MOUNTPOINTS",
        ])
        .arg(&resolved)
        .output()?;
    if !out.status.success() {
        return Err(BoreaError::new("E_DISK", "lsblk failed"));
    }
    parse(&out.stdout, &resolved)
}
pub fn parse(bytes: &[u8], path: &Path) -> Result<DeviceIdentity> {
    let v: Value = serde_json::from_slice(bytes)?;
    let disks = v["blockdevices"]
        .as_array()
        .ok_or_else(|| BoreaError::new("E_DISK", "missing lsblk blockdevices"))?;
    if disks.len() != 1 {
        return Err(BoreaError::new("E_DISK", "ambiguous lsblk identity"));
    }
    let d = &disks[0];
    if d["path"].as_str() != path.to_str()
        || ![Some("disk"), Some("loop")].contains(&d["type"].as_str())
    {
        return Err(BoreaError::new(
            "E_DISK",
            "expected whole disk or test loop device",
        ));
    }
    fn mounted(v: &Value) -> bool {
        v["mountpoints"]
            .as_array()
            .is_some_and(|m| m.iter().any(|x| x.as_str().is_some_and(|s| !s.is_empty())))
            || v["children"]
                .as_array()
                .is_some_and(|a| a.iter().any(mounted))
    }
    if mounted(d) {
        return Err(BoreaError::new(
            "E_DISK_MOUNTED",
            "target or descendant is mounted; select an unused, unmounted disk from an independent execution host",
        ));
    }
    fn number(v: &Value) -> Option<u64> {
        v.as_u64()
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
    }
    let serial=d["serial"].as_str().filter(|s|!s.trim().is_empty()).ok_or_else(||BoreaError::new("E_DISK","disk lacks a stable serial; use an identifiable disk (loop tests supply identity separately)"))?;
    let size = number(&d["size"])
        .filter(|n| *n > 0)
        .ok_or_else(|| BoreaError::new("E_DISK", "invalid disk size"))?;
    let sector_size = number(&d["log-sec"])
        .filter(|n| [512, 4096].contains(n))
        .ok_or_else(|| BoreaError::new("E_DISK", "unsupported logical sector size"))?;
    Ok(DeviceIdentity {
        path: path.into(),
        serial: serial.into(),
        size,
        sector_size,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn device_mounted_child_refused() {
        let v = serde_json::json!({"blockdevices":[{"path":"/dev/vda","type":"disk","serial":"test","size":20000000000u64,"log-sec":512,"mountpoints":[null],"children":[{"mountpoints":["/"]}]}]});
        assert_eq!(
            parse(&serde_json::to_vec(&v).unwrap(), Path::new("/dev/vda"))
                .unwrap_err()
                .code,
            "E_DISK_MOUNTED"
        );
    }
    #[test]
    fn device_requires_serial() {
        let v=br#"{"blockdevices":[{"path":"/dev/vda","type":"disk","serial":null,"size":2048,"log-sec":512}]}"#;
        assert!(parse(v, Path::new("/dev/vda")).is_err());
    }
}
