use crate::{BoreaError, FamilyId, Filesystem, Loader, Result, safe_path};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, path::PathBuf};
pub fn digest<T: Serialize>(v: &T) -> Result<String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&serde_json::to_value(v)?)?)
    ))
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeviceIdentity {
    pub path: PathBuf,
    pub serial: String,
    pub size: u64,
    pub sector_size: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema: u16,
    pub id: String,
    pub created: String,
    pub borea_version: String,
    pub digest: String,
    pub config_digest: String,
    pub matrix_digest: String,
    pub family: FamilyId,
    pub disk: DeviceIdentity,
    pub root_fs: Filesystem,
    pub loader: Loader,
    pub root: PathBuf,
    pub steps: Vec<Step>,
    pub checks: Vec<Check>,
    pub secrets: Vec<SecretSlot>,
    pub notes: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecretSlot {
    pub name: String,
    pub file: PathBuf,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Danger {
    Safe,
    Mutating,
    Destructive,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Storage,
    Bootstrap,
    Base,
    Session,
    Boot,
    Finalise,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub id: String,
    pub title: String,
    pub action: Action,
    pub needs: Vec<String>,
    pub probe: Probe,
    pub danger: Danger,
    pub timeout_s: u64,
    pub phase: Phase,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Command {
        argv: Vec<String>,
        chroot: bool,
        stdin: Option<String>,
    },
    WriteFile {
        path: PathBuf,
        contents: String,
        mode: u32,
    },
    CopyIn {
        source: PathBuf,
        dest: PathBuf,
        mode: u32,
    },
    Mkdir {
        path: PathBuf,
        mode: u32,
    },
    Password {
        user: String,
        secret: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Probe {
    None,
    FileExists { path: PathBuf },
    FileContains { path: PathBuf, needle: String },
    PathMissing { path: PathBuf },
    CommandOk { argv: Vec<String>, chroot: bool },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Check {
    FileExists { path: PathBuf },
    FileContains { path: PathBuf, needle: String },
    PathMissing { path: PathBuf },
    CommandOk { argv: Vec<String> },
}
impl Plan {
    pub fn body_digest(&self) -> Result<String> {
        let mut v = serde_json::to_value(self)?;
        for k in ["id", "created", "digest"] {
            v.as_object_mut().expect("plan object").remove(k);
        }
        digest(&v)
    }
    pub fn seal(&mut self) -> Result<()> {
        self.digest = self.body_digest()?;
        self.id = self.digest[..32].into();
        self.validate()
    }
    pub fn validate(&self) -> Result<()> {
        let bad = |s| BoreaError::new("E_PLAN", s);
        if self.schema != 1 || self.digest != self.body_digest()? {
            return Err(bad("plan schema or digest mismatch"));
        }
        if self.borea_version != crate::version() {
            return Err(bad("plan version differs from running binary; re-plan"));
        }
        if !safe_path(&self.root)
            || self.root == std::path::Path::new("/")
            || !safe_path(&self.disk.path)
            || !self.disk.path.starts_with("/dev")
            || self.disk.serial.is_empty()
            || self.disk.size == 0
            || ![512, 4096].contains(&self.disk.sector_size)
        {
            return Err(bad("invalid target root or device identity"));
        }
        if self.loader == Loader::Auto {
            return Err(bad("unresolved bootloader"));
        }
        let mut slots = BTreeSet::new();
        for slot in &self.secrets {
            if !crate::identifier(&slot.name)
                || !safe_path(&slot.file)
                || !slots.insert(slot.name.as_str())
            {
                return Err(bad("invalid secret slot"));
            }
        }
        let mut ids = BTreeSet::new();
        let mut phase = Phase::Storage;
        for step in &self.steps {
            if step.id.is_empty()
                || !ids.insert(step.id.as_str())
                || step
                    .needs
                    .iter()
                    .any(|n| n == &step.id || !ids.contains(n.as_str()))
                || step.phase < phase
                || step.timeout_s == 0
            {
                return Err(bad(
                    "duplicate step, missing dependency, phase order or timeout",
                ));
            }
            phase = step.phase;
            if (step.danger == Danger::Destructive || matches!(step.probe, Probe::None))
                && step.phase != Phase::Storage
            {
                return Err(bad("destructive/unprobed action outside storage prefix"));
            }
            if matches!(step.probe, Probe::None) && step.danger != Danger::Destructive {
                return Err(bad("non-destructive step needs a probe"));
            }
            match &step.action {
                Action::Command { argv, stdin, .. } => {
                    valid_argv(argv)?;
                    if stdin.as_ref().is_some_and(|s| !slots.contains(s.as_str())) {
                        return Err(bad("unknown secret slot"));
                    }
                }
                Action::WriteFile { path, mode, .. } | Action::Mkdir { path, mode } => {
                    if !safe_path(path) || *mode > 0o777 {
                        return Err(bad("invalid path/mode"));
                    }
                }
                Action::CopyIn { source, dest, mode } => {
                    if !safe_path(source) || !safe_path(dest) || *mode > 0o777 {
                        return Err(bad("invalid copy path/mode"));
                    }
                }
                Action::Password { user, secret } => {
                    if !crate::identifier(user) || !slots.contains(secret.as_str()) {
                        return Err(bad("invalid password slot/user"));
                    }
                }
            }
            match &step.probe {
                Probe::CommandOk { argv, .. } => valid_argv(argv)?,
                Probe::FileExists { path }
                | Probe::FileContains { path, .. }
                | Probe::PathMissing { path } => {
                    if !safe_path(path) {
                        return Err(bad("invalid probe path"));
                    }
                }
                Probe::None => {}
            }
        }
        for check in &self.checks {
            match check {
                Check::CommandOk { argv } => valid_argv(argv)?,
                Check::FileExists { path }
                | Check::FileContains { path, .. }
                | Check::PathMissing { path } => {
                    if !safe_path(path) {
                        return Err(bad("invalid check path"));
                    }
                }
            }
        }
        Ok(())
    }
    pub fn json(&self) -> Result<String> {
        Ok(format!("{}\n", serde_json::to_string_pretty(self)?))
    }
}
pub fn valid_argv(argv: &[String]) -> Result<()> {
    if argv.is_empty() || argv[0].is_empty() || argv.iter().any(|s| s.contains('\0')) {
        Err(BoreaError::new("E_PLAN", "invalid argv"))
    } else {
        Ok(())
    }
}
