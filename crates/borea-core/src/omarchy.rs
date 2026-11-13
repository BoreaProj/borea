//! Upstream defaults plus explicit user choices. Planning never runs upstream scripts.
use crate::{
    defaults::{CATALOG, DefaultsPolicy, default_item, trim_item},
    *,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePin {
    pub repository: String,
    pub commit: String,
    pub entrypoint: String,
    pub entrypoint_sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Baseline {
    pub packages: Vec<String>,
    pub webapps: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Upstream {
    pub schema: u16,
    pub checked: String,
    pub omarchy: SourcePin,
    pub linutil: SourcePin,
    pub baseline: Baseline,
}
pub fn upstream() -> Result<Upstream> {
    let u: Upstream = toml::from_str(include_str!("../../../recipes/omarchy/upstream.toml"))
        .map_err(|e| BoreaError::new("E_RECIPE", e.to_string()))?;
    if u.schema != 1 || u.checked.is_empty() {
        return Err(BoreaError::new(
            "E_RECIPE",
            "invalid Omarchy evidence schema/date",
        ));
    }
    for pin in [&u.omarchy, &u.linutil] {
        if !pin.repository.starts_with("https://github.com/")
            || pin.commit.len() != 40
            || !pin.commit.bytes().all(|b| b.is_ascii_hexdigit())
            || pin.entrypoint_sha256.len() != 64
            || !pin.entrypoint_sha256.bytes().all(|b| b.is_ascii_hexdigit())
            || pin.entrypoint.starts_with('/')
            || pin.entrypoint.split('/').any(|p| p == "..")
        {
            return Err(BoreaError::new(
                "E_RECIPE",
                "invalid Omarchy/Linutil source pin",
            ));
        }
    }
    for item in CATALOG {
        if item
            .packages
            .iter()
            .any(|p| !u.baseline.packages.iter().any(|x| x == p))
            || item
                .webapp
                .is_some_and(|w| !u.baseline.webapps.iter().any(|x| x == w))
        {
            return Err(BoreaError::new(
                "E_RECIPE",
                format!("{} is absent from pinned Omarchy baseline", item.id),
            ));
        }
    }
    Ok(u)
}
fn preset() -> String {
    "linutil".into()
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OmarchyConfig {
    #[serde(default = "preset")]
    pub preset: String,
    #[serde(default)]
    pub keep: Vec<String>,
    #[serde(default)]
    pub remove: Vec<String>,
    #[serde(default)]
    pub replace: BTreeMap<String, String>,
    /// Legacy spelling retained only as an explicit removal list, never a preset.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trim: Vec<String>,
    #[serde(default)]
    pub keep_removed_after_updates: bool,
    pub default_browser: Option<String>,
}
impl Default for OmarchyConfig {
    fn default() -> Self {
        Self {
            preset: preset(),
            keep: Vec::new(),
            remove: Vec::new(),
            replace: BTreeMap::new(),
            trim: Vec::new(),
            keep_removed_after_updates: false,
            default_browser: None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionKind {
    Keep,
    Remove,
    Replace,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppDecision {
    pub id: String,
    pub decision: DecisionKind,
    pub explicit: bool,
    pub packages: Vec<String>,
    pub webapp: Option<String>,
    pub replacement: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Selection {
    pub schema: u16,
    pub preset: String,
    pub upstream: Upstream,
    pub apps: Vec<AppDecision>,
    pub install_components: Vec<String>,
    pub default_browser: Option<String>,
    pub keep_removed_after_updates: bool,
    pub preserve_unlisted: bool,
    pub chromium_preserved: bool,
}
impl OmarchyConfig {
    pub fn selection(&self) -> Result<Selection> {
        if self.preset != "linutil" {
            return Err(BoreaError::invalid(
                "Omarchy preset must be linutil; upstream defaults are retained unless explicitly changed",
            ));
        }
        let mut choices = BTreeMap::new();
        for (list, kind) in [
            (&self.keep, DecisionKind::Keep),
            (&self.remove, DecisionKind::Remove),
            (&self.trim, DecisionKind::Remove),
        ] {
            for id in list {
                if id != "chromium" {
                    trim_item(id)?;
                }
                if id == "chromium" && !matches!(kind, DecisionKind::Keep) {
                    return Err(BoreaError::unsupported(
                        "Chromium is required by Omarchy web apps; choose default_browser instead of removing or replacing it",
                    ));
                }
                if choices.insert(id.clone(), (kind.clone(), None)).is_some() {
                    return Err(BoreaError::invalid(format!(
                        "conflicting or duplicate Omarchy app choice: {id}"
                    )));
                }
            }
        }
        let mut install_components = BTreeSet::new();
        let catalog = crate::recipe::recipes()?;
        for (id, replacement) in &self.replace {
            trim_item(id)?;
            let component = format!("app.{replacement}");
            let r = catalog.get(&component).ok_or_else(|| {
                BoreaError::unsupported(format!(
                    "unknown replacement {replacement}; choose an app from the catalogue"
                ))
            })?;
            let source = trim_item(id)?;
            if r.packages
                .values()
                .flatten()
                .any(|p| source.packages.contains(&p.as_str()))
            {
                return Err(BoreaError::invalid(format!(
                    "{id} replacement would reinstall the removed app"
                )));
            }
            if choices
                .insert(id.clone(), (DecisionKind::Replace, Some(component.clone())))
                .is_some()
            {
                return Err(BoreaError::invalid(format!(
                    "conflicting Omarchy app choice: {id}"
                )));
            }
            install_components.insert(component);
        }
        if let Some(browser) = &self.default_browser {
            if !["chromium", "firefox"].contains(&browser.as_str()) {
                return Err(BoreaError::unsupported(
                    "default_browser currently supports chromium or firefox; Chromium will be retained",
                ));
            }
            if browser == "firefox" {
                install_components.insert("app.firefox".into());
            }
        }
        let mut apps = CATALOG
            .iter()
            .map(|item| {
                let (decision, replacement) = choices
                    .get(item.id)
                    .cloned()
                    .unwrap_or((DecisionKind::Keep, None));
                AppDecision {
                    id: item.id.into(),
                    decision,
                    explicit: choices.contains_key(item.id),
                    packages: item.packages.iter().map(|p| (*p).into()).collect(),
                    webapp: item.webapp.map(String::from),
                    replacement,
                }
            })
            .collect::<Vec<_>>();
        apps.push(AppDecision {
            id: "chromium".into(),
            decision: DecisionKind::Keep,
            explicit: choices.contains_key("chromium"),
            packages: vec!["chromium".into()],
            webapp: None,
            replacement: None,
        });
        apps.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(Selection {
            schema: 1,
            preset: self.preset.clone(),
            upstream: upstream()?,
            apps,
            install_components: install_components.into_iter().collect(),
            default_browser: self.default_browser.clone(),
            keep_removed_after_updates: self.keep_removed_after_updates,
            preserve_unlisted: true,
            chromium_preserved: true,
        })
    }
}
impl Selection {
    pub fn removals(&self) -> impl Iterator<Item = &AppDecision> {
        self.apps
            .iter()
            .filter(|a| matches!(a.decision, DecisionKind::Remove | DecisionKind::Replace))
    }
    pub fn policy(&self, user: &str) -> Result<DefaultsPolicy> {
        let mut p = DefaultsPolicy::new(user.into());
        p.upstream = self.upstream.omarchy.commit.clone();
        if self.keep_removed_after_updates {
            for a in self.removals() {
                p.items.push(default_item(&a.id)?);
            }
        }
        Ok(p)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plain_schema_one_configs_remain_readable() {
        let c = include_str!("../../../recipes/profiles/minimal.toml")
            .replace("schema = 2", "schema = 1");
        assert!(Config::parse(&c).is_ok());
    }
    #[test]
    fn omarchy_requires_schema_two() {
        let c = include_str!("../../../examples/omarchy.toml").replace("schema = 2", "schema = 1");
        assert!(Config::parse(&c).unwrap_err().message.contains("schema 2"));
    }
    #[test]
    fn omarchy_upstream_mapping_is_pinned() {
        upstream().unwrap();
    }
    #[test]
    fn omarchy_linutil_keeps_everything_by_default() {
        let s = OmarchyConfig::default().selection().unwrap();
        assert_eq!(s.removals().count(), 0);
        assert!(s.install_components.is_empty());
        assert!(s.preserve_unlisted && s.chromium_preserved);
        assert!(!s.keep_removed_after_updates);
    }
    #[test]
    fn omarchy_explicit_changes_only() {
        let c = OmarchyConfig {
            keep: vec!["spotify".into()],
            remove: vec!["docker".into()],
            replace: BTreeMap::from([("obsidian".into(), "neovim".into())]),
            default_browser: Some("firefox".into()),
            ..Default::default()
        };
        let s = c.selection().unwrap();
        assert_eq!(
            s.removals().map(|a| a.id.as_str()).collect::<Vec<_>>(),
            ["docker", "obsidian"]
        );
        assert_eq!(s.install_components, ["app.firefox", "app.neovim"]);
        assert!(s.policy("tester").unwrap().items.is_empty());
    }
    #[test]
    fn omarchy_conflicts_and_unknowns_refused() {
        for c in [
            OmarchyConfig {
                keep: vec!["docker".into()],
                remove: vec!["docker".into()],
                ..Default::default()
            },
            OmarchyConfig {
                remove: vec!["unknown".into()],
                ..Default::default()
            },
            OmarchyConfig {
                remove: vec!["chromium".into()],
                ..Default::default()
            },
            OmarchyConfig {
                remove: vec!["docker".into(), "docker".into()],
                ..Default::default()
            },
            OmarchyConfig {
                replace: BTreeMap::from([("obsidian".into(), "unknown".into())]),
                ..Default::default()
            },
        ] {
            assert!(c.selection().is_err());
        }
    }
    #[test]
    fn omarchy_policy_only_enforces_explicit_removals() {
        let c = OmarchyConfig {
            remove: vec!["docker".into()],
            keep_removed_after_updates: true,
            ..Default::default()
        };
        let s = c.selection().unwrap();
        let p = s.policy("tester").unwrap();
        assert_eq!(p.items.len(), 1);
        assert_eq!(p.items[0].id, "docker");
    }
}
