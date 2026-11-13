use crate::{BoreaError, FamilyId, Loader, Result, identifier};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FamilyRecipe {
    pub schema: u16,
    pub id: String,
    pub family: FamilyId,
    pub release: String,
    pub bootstrap: String,
    pub base_packages: Vec<String>,
    pub default_loader: Loader,
    pub grub_package: String,
    pub grub_install: String,
    pub grub_mkconfig: String,
    pub grub_cfg: String,
    pub initramfs: Vec<String>,
    pub network_unit: String,
    pub sudo_group: String,
    pub evidence: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    pub schema: u16,
    pub id: String,
    #[serde(default)]
    pub requires: Vec<String>,
    pub evidence: String,
    #[serde(default)]
    pub packages: BTreeMap<FamilyId, Vec<String>>,
    pub exec: Option<String>,
    pub session_file: Option<String>,
    pub greeter: Option<String>,
    pub autostart: Option<String>,
    #[serde(default)]
    pub compatible_compositors: Vec<String>,
}
pub fn families() -> Result<Vec<FamilyRecipe>> {
    [
        include_str!("../../../recipes/families/arch.toml"),
        include_str!("../../../recipes/families/debian.toml"),
        include_str!("../../../recipes/families/fedora.toml"),
        include_str!("../../../recipes/families/suse.toml"),
    ]
    .into_iter()
    .map(|s| toml::from_str(s).map_err(|e| BoreaError::new("E_RECIPE", e.to_string())))
    .collect()
}
pub fn family(f: FamilyId) -> Result<FamilyRecipe> {
    families()?
        .into_iter()
        .find(|r| r.family == f)
        .ok_or_else(|| BoreaError::new("E_RECIPE", "missing family"))
}
pub fn recipes() -> Result<BTreeMap<String, Recipe>> {
    let mut out = BTreeMap::new();
    for s in [
        include_str!("../../../recipes/components/app.firefox.toml"),
        include_str!("../../../recipes/components/apps.flatpak.toml"),
        include_str!("../../../recipes/components/base.minimal.toml"),
        include_str!("../../../recipes/components/boot.grub.toml"),
        include_str!("../../../recipes/components/boot.limine.toml"),
        include_str!("../../../recipes/components/compositor.hyprland.toml"),
        include_str!("../../../recipes/components/compositor.mangowm.toml"),
        include_str!("../../../recipes/components/compositor.niri.toml"),
        include_str!("../../../recipes/components/desktop.cosmic.toml"),
        include_str!("../../../recipes/components/desktop.gnome.toml"),
        include_str!("../../../recipes/components/desktop.kde.toml"),
        include_str!("../../../recipes/components/fs.btrfs.toml"),
        include_str!("../../../recipes/components/fs.ext4.toml"),
        include_str!("../../../recipes/components/greeter.greetd.toml"),
        include_str!("../../../recipes/components/shell.dms.toml"),
        include_str!("../../../recipes/components/shell.end4.toml"),
        include_str!("../../../recipes/components/shell.noctalia.toml"),
        include_str!("../../../recipes/components/system.codecs.toml"),
    ] {
        let r: Recipe =
            toml::from_str(s).map_err(|e| BoreaError::new("E_RECIPE", e.to_string()))?;
        if out.insert(r.id.clone(), r).is_some() {
            return Err(BoreaError::new("E_RECIPE", "duplicate recipe"));
        }
    }
    Ok(out)
}
pub fn resolve(ids: &[String], catalog: &BTreeMap<String, Recipe>) -> Result<Vec<String>> {
    fn visit(
        id: &str,
        c: &BTreeMap<String, Recipe>,
        active: &mut BTreeSet<String>,
        done: &mut BTreeSet<String>,
        out: &mut Vec<String>,
    ) -> Result<()> {
        if done.contains(id) {
            return Ok(());
        }
        if !active.insert(id.into()) {
            return Err(BoreaError::new(
                "E_RECIPE",
                format!("dependency cycle at {id}"),
            ));
        }
        let r = c.get(id).ok_or_else(|| {
            BoreaError::unsupported(format!(
                "recipe {id} is unavailable; select a catalogue item"
            ))
        })?;
        let mut deps = r.requires.clone();
        deps.sort();
        for dep in deps {
            visit(&dep, c, active, done, out)?;
        }
        active.remove(id);
        done.insert(id.into());
        out.push(id.into());
        Ok(())
    }
    let mut out = Vec::new();
    let ids = ids.to_vec();
    let mut done = BTreeSet::new();
    for id in ids_sorted(ids) {
        visit(&id, catalog, &mut BTreeSet::new(), &mut done, &mut out)?;
    }
    Ok(out)
}
fn ids_sorted(mut ids: Vec<String>) -> Vec<String> {
    ids.sort();
    ids.dedup();
    ids
}
pub fn lint() -> Result<()> {
    let catalog = recipes()?;
    resolve(&catalog.keys().cloned().collect::<Vec<_>>(), &catalog)?;
    for r in catalog.values() {
        if r.schema != 1
            || !identifier(&r.id)
            || !r.id.contains('.')
            || !r.evidence.starts_with("https://")
        {
            return Err(BoreaError::new(
                "E_RECIPE",
                format!("invalid recipe {}", r.id),
            ));
        }
        for c in &r.compatible_compositors {
            if !catalog.contains_key(c) {
                return Err(BoreaError::new(
                    "E_RECIPE",
                    format!("unknown compatibility id {c}"),
                ));
            }
        }
        for packages in r.packages.values() {
            if packages.is_empty()
                || packages.iter().any(|s| {
                    s.is_empty()
                        || s.starts_with('-')
                        || s.bytes().any(|b| b.is_ascii_whitespace() || b == 0)
                })
            {
                return Err(BoreaError::new("E_RECIPE", "invalid package name"));
            }
        }
    }
    let matrix = crate::matrix::Matrix::embedded()?;
    for f in families()? {
        if f.schema != 1
            || f.default_loader == Loader::Auto
            || !f.base_packages.iter().any(|s| s.starts_with("btrfs"))
        {
            return Err(BoreaError::new("E_RECIPE", "invalid family recipe"));
        }
        matrix.gate(
            f.family,
            &[format!("boot.{}", f.default_loader.as_str())],
            false,
            false,
        )?;
    }
    for c in &matrix.cells {
        if c.tier == crate::matrix::Tier::Native
            && catalog
                .get(&c.component)
                .is_none_or(|r| !r.packages.contains_key(&c.family))
        {
            return Err(BoreaError::new(
                "E_RECIPE",
                format!(
                    "native cell lacks packages: {} {}",
                    c.family.as_str(),
                    c.component
                ),
            ));
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recipe_lint() {
        lint().unwrap();
    }
    #[test]
    fn recipe_cycle() {
        let mut c = recipes().unwrap();
        c.get_mut("desktop.kde").unwrap().requires = vec!["desktop.kde".into()];
        assert!(resolve(&["desktop.kde".into()], &c).is_err());
    }
}
