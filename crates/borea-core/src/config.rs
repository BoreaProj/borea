use crate::{BoreaError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FamilyId {
    Arch,
    Debian,
    Fedora,
    Suse,
}
impl FamilyId {
    pub const ALL: [Self; 4] = [Self::Arch, Self::Debian, Self::Fedora, Self::Suse];
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Arch => "arch",
            Self::Debian => "debian",
            Self::Fedora => "fedora",
            Self::Suse => "suse",
        }
    }
    pub fn parse(s: &str) -> Result<Self> {
        Self::ALL
            .into_iter()
            .find(|f| f.as_str() == s)
            .ok_or_else(|| {
                BoreaError::unsupported(format!(
                    "unknown family {s}; choose arch, debian, fedora or suse"
                ))
            })
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Filesystem {
    #[default]
    Btrfs,
    Ext4,
}
impl Filesystem {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Btrfs => "btrfs",
            Self::Ext4 => "ext4",
        }
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Loader {
    #[default]
    Auto,
    Limine,
    Grub,
}
impl Loader {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Limine => "limine",
            Self::Grub => "grub",
        }
    }
}
fn schema() -> u16 {
    1
}
fn hostname() -> String {
    "borea".into()
}
fn timezone() -> String {
    "UTC".into()
}
fn locale() -> String {
    "en_US.UTF-8".into()
}
fn keymap() -> String {
    "us".into()
}
fn arch() -> String {
    "x86_64".into()
}
fn firmware() -> String {
    "uefi".into()
}
fn replace() -> String {
    "replace".into()
}
fn esp() -> String {
    "1GiB".into()
}
fn boot() -> String {
    "2GiB".into()
}
fn zram() -> String {
    "zram".into()
}
fn same() -> String {
    "same".into()
}
fn auto() -> String {
    "auto".into()
}
fn minimal() -> String {
    "minimal".into()
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "schema")]
    pub schema: u16,
    pub target: TargetConfig,
    pub disk: DiskConfig,
    #[serde(default)]
    pub boot: BootConfig,
    pub account: AccountConfig,
    #[serde(default)]
    pub session: SessionConfig,
    #[serde(default)]
    pub apps: AppsConfig,
    pub omarchy: Option<OmarchyConfig>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetConfig {
    pub family: FamilyId,
    pub release: Option<String>,
    #[serde(default = "hostname")]
    pub hostname: String,
    #[serde(default = "timezone")]
    pub timezone: String,
    #[serde(default = "locale")]
    pub locale: String,
    #[serde(default = "keymap")]
    pub keymap: String,
    #[serde(default = "arch")]
    pub arch: String,
    #[serde(default = "firmware")]
    pub firmware: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiskConfig {
    pub device: PathBuf,
    #[serde(default = "replace")]
    pub mode: String,
    #[serde(default)]
    pub root_fs: Filesystem,
    #[serde(default)]
    pub encrypt: bool,
    pub luks_password_file: Option<PathBuf>,
    #[serde(default = "esp")]
    pub esp_size: String,
    #[serde(default = "boot")]
    pub boot_size: String,
    #[serde(default = "zram")]
    pub swap: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootConfig {
    #[serde(default)]
    pub loader: Loader,
    #[serde(default)]
    pub secure_boot: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccountConfig {
    pub username: String,
    #[serde(default)]
    pub full_name: String,
    pub password_file: PathBuf,
    #[serde(default = "same")]
    pub sudo: String,
    pub sudo_password_file: Option<PathBuf>,
    pub ssh_authorized_key_file: Option<PathBuf>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionConfig {
    #[serde(rename = "type", default = "minimal")]
    pub kind: String,
    pub desktop: Option<String>,
    pub compositor: Option<String>,
    pub shell: Option<String>,
    #[serde(default = "auto")]
    pub greeter: String,
}
impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            kind: minimal(),
            desktop: None,
            compositor: None,
            shell: None,
            greeter: auto(),
        }
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppsConfig {
    #[serde(default)]
    pub enable: Vec<String>,
    #[serde(default)]
    pub codecs: bool,
    #[serde(default)]
    pub flatpak: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OmarchyConfig {
    #[serde(default)]
    pub trim: Vec<String>,
    #[serde(default)]
    pub keep_removed_after_updates: bool,
    pub default_browser: Option<String>,
}
pub fn safe_path(p: &Path) -> bool {
    p.is_absolute()
        && !p.components().any(|c| matches!(c, Component::ParentDir))
        && !p.as_os_str().as_encoded_bytes().contains(&0)
}
pub fn identifier(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with('-')
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
pub fn size_mib(s: &str) -> Result<u64> {
    let (n, m) = if let Some(v) = s.strip_suffix("GiB") {
        (v, 1024)
    } else if let Some(v) = s.strip_suffix("MiB") {
        (v, 1)
    } else {
        return Err(BoreaError::invalid("size must use GiB or MiB"));
    };
    n.parse::<u64>()
        .ok()
        .and_then(|v| v.checked_mul(m))
        .filter(|v| *v > 0 && *v <= 1_048_576)
        .ok_or_else(|| BoreaError::invalid("partition size out of range"))
}
impl Config {
    pub fn parse(s: &str) -> Result<Self> {
        let c: Self = toml::from_str(s).map_err(|e| BoreaError::invalid(e.to_string()))?;
        c.validate()?;
        Ok(c)
    }
    pub fn validate(&self) -> Result<()> {
        if self.schema != 1 {
            return Err(BoreaError::invalid("config schema must be 1"));
        }
        if self.target.arch != "x86_64"
            || self.target.firmware != "uefi"
            || self.disk.mode != "replace"
            || self.boot.secure_boot
        {
            return Err(BoreaError::unsupported(
                "v1 requires x86_64, UEFI, replace mode and Secure Boot off; change the config",
            ));
        }
        if !safe_path(&self.disk.device) || !self.disk.device.starts_with("/dev") {
            return Err(BoreaError::invalid(
                "disk.device must be an absolute /dev path without traversal",
            ));
        }
        let u = &self.account.username;
        if !identifier(u)
            || u.len() > 32
            || u.bytes().next().is_none_or(|b| !b.is_ascii_lowercase())
            || ["root", "daemon", "nobody", "bin", "systemd", "builder"].contains(&u.as_str())
        {
            return Err(BoreaError::invalid("invalid or reserved username"));
        }
        for p in [
            Some(&self.account.password_file),
            self.account.sudo_password_file.as_ref(),
            self.account.ssh_authorized_key_file.as_ref(),
            self.disk.luks_password_file.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            if !safe_path(p) {
                return Err(BoreaError::invalid(
                    "secret/key paths must be absolute without traversal",
                ));
            }
        }
        if !["same", "separate"].contains(&self.account.sudo.as_str())
            || (self.account.sudo == "separate" && self.account.sudo_password_file.is_none())
        {
            return Err(BoreaError::invalid(
                "separate sudo requires sudo_password_file",
            ));
        }
        if self.account.sudo == "separate" {
            return Err(BoreaError::unsupported(
                "separate sudo password is not implemented; use sudo = 'same'",
            ));
        }
        if !identifier(&self.target.hostname) || self.target.hostname.len() > 63 {
            return Err(BoreaError::invalid("invalid hostname"));
        }
        for s in [&self.target.locale, &self.target.keymap] {
            if !identifier(s) {
                return Err(BoreaError::invalid("invalid locale/keymap"));
            }
        }
        if !safe_path(&Path::new("/").join(&self.target.timezone))
            || self.target.timezone.starts_with('/')
            || !self
                .target
                .timezone
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"/_+-".contains(&b))
        {
            return Err(BoreaError::invalid("invalid timezone"));
        }
        if self.account.full_name.contains(['\n', '\r', ':', '\0']) {
            return Err(BoreaError::invalid("invalid full name"));
        }
        size_mib(&self.disk.esp_size)?;
        size_mib(&self.disk.boot_size)?;
        if !["none", "zram", "file"].contains(&self.disk.swap.as_str()) {
            return Err(BoreaError::invalid("swap must be none, zram or file"));
        }
        if self.disk.swap == "file" {
            return Err(BoreaError::unsupported(
                "swapfile creation is pending B05; use zram or none",
            ));
        }
        if !["auto", "none", "sddm", "gdm", "greetd"].contains(&self.session.greeter.as_str()) {
            return Err(BoreaError::invalid("invalid greeter"));
        }
        match self.session.kind.as_str() {
            "minimal"
                if self.session.desktop.is_none()
                    && self.session.compositor.is_none()
                    && self.session.shell.is_none() => {}
            "desktop"
                if self
                    .session
                    .desktop
                    .as_deref()
                    .is_some_and(|s| ["kde", "gnome", "cosmic"].contains(&s))
                    && self.session.compositor.is_none()
                    && self.session.shell.is_none() => {}
            "shell"
                if self
                    .session
                    .compositor
                    .as_deref()
                    .is_some_and(|s| ["hyprland", "niri", "mangowm"].contains(&s))
                    && self.session.desktop.is_none()
                    && self
                        .session
                        .shell
                        .as_deref()
                        .is_none_or(|s| ["none", "noctalia", "dms", "end4"].contains(&s)) => {}
            _ => {
                return Err(BoreaError::invalid(
                    "session fields do not match type minimal, desktop or shell",
                ));
            }
        }
        for app in &self.apps.enable {
            if !identifier(app) {
                return Err(BoreaError::invalid("invalid app id"));
            }
        }
        if self.omarchy.as_ref().is_some_and(|o| {
            !o.trim.is_empty() || o.default_browser.is_some() || o.keep_removed_after_updates
        }) {
            return Err(BoreaError::unsupported(
                "Omarchy configuration is for existing installations; use borea omarchy separately and omit [omarchy] from install configs",
            ));
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const CONFIG: &str = include_str!("../../../recipes/profiles/minimal.toml");
    #[test]
    fn config_valid() {
        assert!(Config::parse(CONFIG).is_ok());
    }
    #[test]
    fn config_unknown_and_unsafe() {
        for (a, b) in [
            ("schema = 1", "unknown = 1"),
            ("username = \"tester\"", "username = \"root\""),
            ("/run/borea/user.pass", "relative"),
            ("encrypt = false", "encrypt = false\nextra = true"),
            ("hostname = \"borea-test\"", "hostname = \"bad\\nname\""),
        ] {
            assert!(Config::parse(&CONFIG.replace(a, b)).is_err(), "{a}");
        }
    }
    #[test]
    fn config_non_goals() {
        assert_eq!(
            Config::parse(&CONFIG.replace("secure_boot = false", "secure_boot = true"))
                .unwrap_err()
                .code,
            "E_UNSUPPORTED"
        );
    }
}
