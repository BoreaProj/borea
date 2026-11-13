//! Reviewed, immutable runtime payload descriptors. No I/O or transport here.
use crate::{BoreaError, FamilyId, Result, identifier};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeSpec {
    pub schema: u16,
    pub family: FamilyId,
    pub version: String,
    pub arch: String,
    pub image: String,
    pub url: String,
    pub sha256: String,
    pub size: u64,
    pub max_unpacked_bytes: u64,
    pub tools: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeLock {
    pub schema: u16,
    pub runtimes: Vec<RuntimeSpec>,
}
impl RuntimeSpec {
    pub fn validate(&self) -> Result<()> {
        let valid_hex = |s: &str| {
            s.len() == 64
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        };
        let expected = format!(
            "https://github.com/BoreaProj/borea/releases/download/{}/borea-runtime-{}-x86_64.tar.gz",
            self.version,
            self.family.as_str()
        );
        let image_digest = self.image.rsplit_once("@sha256:").map(|(_, s)| s);
        if self.schema != 1
            || self.arch != "x86_64"
            || !identifier(&self.version)
            || self.version.len() > 64
            || self
                .version
                .bytes()
                .next()
                .is_none_or(|b| !b.is_ascii_alphanumeric())
            || self.url != expected
            || !valid_hex(&self.sha256)
            || image_digest.is_none_or(|s| !valid_hex(s))
            || self.size == 0
            || self.size > 2 * 1024 * 1024 * 1024
            || self.max_unpacked_bytes == 0
            || self.max_unpacked_bytes > 8 * 1024 * 1024 * 1024
            || self.tools.is_empty()
            || self.tools.iter().any(|s| !identifier(s))
            || self.tools.iter().collect::<BTreeSet<_>>().len() != self.tools.len()
        {
            return Err(BoreaError::new(
                "E_RUNTIME_MANIFEST",
                "invalid runtime identity, origin, limits or tools",
            ));
        }
        Ok(())
    }
}
impl RuntimeLock {
    pub fn embedded() -> Result<Self> {
        let lock: Self =
            serde_json::from_str(include_str!("../../../recipes/families/runtimes.lock.json"))?;
        lock.validate()?;
        Ok(lock)
    }
    pub fn validate(&self) -> Result<()> {
        if self.schema != 1 || self.runtimes.len() != FamilyId::ALL.len() {
            return Err(BoreaError::new(
                "E_RUNTIME_MANIFEST",
                "runtime lock must contain exactly four families",
            ));
        }
        let tool_sets: std::collections::BTreeMap<FamilyId, Vec<String>> =
            serde_json::from_str(include_str!("../../../runtime/tools.json"))?;
        let images: std::collections::BTreeMap<FamilyId, serde_json::Value> = serde_json::from_str(
            include_str!("../../../recipes/families/runtime-images.json"),
        )?;
        let mut seen = BTreeSet::new();
        for spec in &self.runtimes {
            if tool_sets.get(&spec.family) != Some(&spec.tools)
                || images.get(&spec.family).and_then(|v| v["image"].as_str())
                    != Some(spec.image.as_str())
            {
                return Err(BoreaError::new(
                    "E_RUNTIME_MANIFEST",
                    "runtime manifest differs from reviewed family source image or tool inventory",
                ));
            }
            spec.validate()?;
            if !seen.insert(spec.family) {
                return Err(BoreaError::new(
                    "E_RUNTIME_MANIFEST",
                    "duplicate runtime family",
                ));
            }
        }
        Ok(())
    }
    pub fn get(&self, family: FamilyId) -> Result<&RuntimeSpec> {
        self.runtimes
            .iter()
            .find(|s| s.family == family)
            .ok_or_else(|| BoreaError::new("E_RUNTIME_MANIFEST", "missing runtime family"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runtime_lock_covers_four_families() {
        let lock = RuntimeLock::embedded().unwrap();
        for f in FamilyId::ALL {
            assert_eq!(lock.get(f).unwrap().family, f);
        }
    }
    #[test]
    fn runtime_manifest_rejects_untrusted_origins_and_missing_pins() {
        let lock = RuntimeLock::embedded().unwrap();
        let spec = lock.runtimes[0].clone();
        for change in 0..5 {
            let mut s = spec.clone();
            match change {
                0 => s.url = "http://example.com/runtime.tar.gz".into(),
                1 => s.sha256 = "latest".into(),
                2 => s.arch = "aarch64".into(),
                3 => s.image = "archlinux:latest".into(),
                _ => s.version = "..".into(),
            };
            assert!(s.validate().is_err());
        }
    }
    #[test]
    fn runtime_lock_rejects_duplicates() {
        let mut lock = RuntimeLock::embedded().unwrap();
        lock.runtimes[1] = lock.runtimes[0].clone();
        assert!(lock.validate().is_err());
    }
}
