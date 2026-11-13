//! Omarchy policy model. Unknown legacy fields are retained on round-trip.
use crate::{BoreaError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ItemState {
    Enforced,
    Reintroduced,
    Restored,
    Blocked { reason: String },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultItem {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub packages: Vec<String>,
    #[serde(default)]
    pub launchers: Vec<String>,
    pub state: ItemState,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsPolicy {
    pub schema: u16,
    pub distro: String,
    pub upstream: String,
    pub package_manager: String,
    pub update_window: String,
    pub user: String,
    #[serde(default)]
    pub last_update_started: u64,
    #[serde(default)]
    pub items: Vec<DefaultItem>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}
impl DefaultsPolicy {
    pub fn new(user: String) -> Self {
        Self {
            schema: 1,
            distro: "omarchy".into(),
            upstream: "unverified".into(),
            package_manager: "pacman".into(),
            update_window: "omarchy_update_lock".into(),
            user,
            last_update_started: 0,
            items: Vec::new(),
            extra: BTreeMap::new(),
        }
    }
    pub fn validate(&self) -> Result<()> {
        if self.schema != 1 || self.distro != "omarchy" || self.package_manager != "pacman" {
            Err(BoreaError::unsupported(
                "only Omarchy schema 1 policies are implemented; keep the original policy",
            ))
        } else {
            Ok(())
        }
    }
    pub fn note_install(&mut self, packages: &[String], in_update: bool) {
        for item in &mut self.items {
            if item.packages.iter().any(|p| packages.contains(p))
                && !matches!(item.state, ItemState::Restored)
            {
                item.state = if in_update {
                    ItemState::Reintroduced
                } else {
                    ItemState::Restored
                };
            }
        }
    }
    pub fn restore(&mut self, ids: &[String]) -> Result<()> {
        for id in ids {
            let i = self
                .items
                .iter_mut()
                .find(|i| &i.id == id)
                .ok_or_else(|| BoreaError::invalid(format!("unknown policy item {id}")))?;
            i.state = ItemState::Restored;
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct TrimItem {
    pub id: &'static str,
    pub label: &'static str,
    pub group: &'static str,
    pub packages: &'static [&'static str],
    pub webapp: Option<&'static str>,
}
pub const CATALOG: &[TrimItem] = &[
    TrimItem {
        id: "libreoffice",
        label: "LibreOffice",
        group: "apps",
        packages: &["libreoffice-fresh"],
        webapp: None,
    },
    TrimItem {
        id: "obsidian",
        label: "Obsidian",
        group: "apps",
        packages: &["obsidian"],
        webapp: None,
    },
    TrimItem {
        id: "pinta",
        label: "Pinta",
        group: "apps",
        packages: &["pinta"],
        webapp: None,
    },
    TrimItem {
        id: "localsend",
        label: "LocalSend",
        group: "apps",
        packages: &["localsend"],
        webapp: None,
    },
    TrimItem {
        id: "docker",
        label: "Docker",
        group: "developer-tools",
        packages: &["ufw-docker", "docker-compose", "docker-buildx", "docker"],
        webapp: None,
    },
    TrimItem {
        id: "hey",
        label: "HEY",
        group: "web-apps",
        packages: &[],
        webapp: Some("HEY"),
    },
    TrimItem {
        id: "basecamp",
        label: "Basecamp",
        group: "web-apps",
        packages: &[],
        webapp: Some("Basecamp"),
    },
    TrimItem {
        id: "whatsapp",
        label: "WhatsApp",
        group: "web-apps",
        packages: &[],
        webapp: Some("WhatsApp"),
    },
    TrimItem {
        id: "youtube",
        label: "YouTube",
        group: "web-apps",
        packages: &[],
        webapp: Some("YouTube"),
    },
    TrimItem {
        id: "x",
        label: "X",
        group: "web-apps",
        packages: &[],
        webapp: Some("X"),
    },
    TrimItem {
        id: "zoom",
        label: "Zoom",
        group: "web-apps",
        packages: &[],
        webapp: Some("Zoom"),
    },
    TrimItem {
        id: "discord",
        label: "Discord",
        group: "web-apps",
        packages: &[],
        webapp: Some("Discord"),
    },
    TrimItem {
        id: "google-photos",
        label: "Google Photos",
        group: "web-apps",
        packages: &[],
        webapp: Some("Google Photos"),
    },
    TrimItem {
        id: "google-maps",
        label: "Google Maps",
        group: "web-apps",
        packages: &[],
        webapp: Some("Google Maps"),
    },
    TrimItem {
        id: "google-messages",
        label: "Google Messages",
        group: "web-apps",
        packages: &[],
        webapp: Some("Google Messages"),
    },
    TrimItem {
        id: "google-contacts",
        label: "Google Contacts",
        group: "web-apps",
        packages: &[],
        webapp: Some("Google Contacts"),
    },
    TrimItem {
        id: "kdenlive",
        label: "Kdenlive",
        group: "apps",
        packages: &[],
        webapp: None,
    },
    TrimItem {
        id: "obs-studio",
        label: "OBS Studio",
        group: "apps",
        packages: &[],
        webapp: None,
    },
    TrimItem {
        id: "xournal",
        label: "Xournal++",
        group: "apps",
        packages: &[],
        webapp: None,
    },
    TrimItem {
        id: "signal",
        label: "Signal",
        group: "apps",
        packages: &[],
        webapp: None,
    },
    TrimItem {
        id: "spotify",
        label: "Spotify",
        group: "apps",
        packages: &[],
        webapp: None,
    },
    TrimItem {
        id: "typora",
        label: "Typora",
        group: "apps",
        packages: &[],
        webapp: None,
    },
    TrimItem {
        id: "1password",
        label: "1Password",
        group: "apps",
        packages: &[],
        webapp: None,
    },
    TrimItem {
        id: "chatgpt",
        label: "ChatGPT",
        group: "web-apps",
        packages: &[],
        webapp: None,
    },
    TrimItem {
        id: "grok",
        label: "Grok",
        group: "web-apps",
        packages: &[],
        webapp: None,
    },
    TrimItem {
        id: "github",
        label: "GitHub",
        group: "web-apps",
        packages: &[],
        webapp: None,
    },
    TrimItem {
        id: "figma",
        label: "Figma",
        group: "web-apps",
        packages: &[],
        webapp: None,
    },
    TrimItem {
        id: "fizzy",
        label: "Fizzy",
        group: "web-apps",
        packages: &[],
        webapp: None,
    },
    TrimItem {
        id: "lazydocker",
        label: "lazydocker",
        group: "developer-tools",
        packages: &[],
        webapp: None,
    },
    TrimItem {
        id: "claude-code",
        label: "Claude Code",
        group: "developer-tools",
        packages: &[],
        webapp: None,
    },
    TrimItem {
        id: "opencode",
        label: "opencode",
        group: "developer-tools",
        packages: &[],
        webapp: None,
    },
];
pub fn trim_item(id: &str) -> Result<&'static TrimItem> {
    CATALOG.iter().find(|i|i.id==id).ok_or_else(||BoreaError::unsupported(format!("{id} cannot be trimmed; browsers and system components are protected; choose an id from trim --list")))
}
pub fn default_item(id: &str) -> Result<DefaultItem> {
    let i = trim_item(id)?;
    Ok(DefaultItem {
        id: i.id.into(),
        label: i.label.into(),
        packages: i.packages.iter().map(|p| (*p).into()).collect(),
        launchers: i.webapp.map(|w| vec![w.into()]).unwrap_or_default(),
        state: ItemState::Enforced,
        extra: BTreeMap::new(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn omarchy_browser_protected() {
        for id in ["chromium", "firefox", "linux", "systemd"] {
            assert!(trim_item(id).is_err());
        }
    }
    #[test]
    fn omarchy_policy_transitions() {
        let mut p = DefaultsPolicy::new("tester".into());
        p.items.push(default_item("docker").unwrap());
        let packages = vec!["docker".into()];
        p.note_install(&packages, true);
        assert_eq!(p.items[0].state, ItemState::Reintroduced);
        p.note_install(&packages, false);
        assert_eq!(p.items[0].state, ItemState::Restored);
        p.note_install(&packages, true);
        assert_eq!(p.items[0].state, ItemState::Restored);
    }
    #[test]
    fn omarchy_round_trip() {
        let mut p = DefaultsPolicy::new("tester".into());
        p.extra.insert("legacy".into(), true.into());
        let s = serde_json::to_string(&p).unwrap();
        let p: DefaultsPolicy = serde_json::from_str(&s).unwrap();
        assert_eq!(p.extra["legacy"], true);
    }
}
