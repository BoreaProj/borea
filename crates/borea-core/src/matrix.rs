use crate::{BoreaError, FamilyId, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tier {
    Native,
    ThirdParty,
    Source,
    Unsupported,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cell {
    pub family: FamilyId,
    pub component: String,
    pub tier: Tier,
    pub reason: Option<String>,
    pub remedy: Option<String>,
    pub evidence: String,
    pub checked: Option<toml::value::Datetime>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Matrix {
    pub schema: u16,
    pub revision: String,
    #[serde(rename = "cell")]
    pub cells: Vec<Cell>,
}
impl Matrix {
    pub fn embedded() -> Result<Self> {
        Self::parse(include_str!("../../../recipes/matrix.toml"))
    }
    pub fn parse(s: &str) -> Result<Self> {
        let m: Self = toml::from_str(s).map_err(|e| BoreaError::new("E_MATRIX", e.to_string()))?;
        m.validate()?;
        Ok(m)
    }
    pub fn validate(&self) -> Result<()> {
        if self.schema != 1 || self.revision.is_empty() {
            return Err(BoreaError::new("E_MATRIX", "schema/revision invalid"));
        }
        let mut seen = BTreeSet::new();
        let mut components = BTreeSet::new();
        for c in &self.cells {
            components.insert(&c.component);
            if !seen.insert((c.family, &c.component))
                || !c.evidence.starts_with("https://")
                || c.component.is_empty()
                || (c.tier == Tier::Unsupported
                    && (c.reason.as_ref().is_none_or(|s| s.is_empty())
                        || c.remedy.as_ref().is_none_or(|s| s.is_empty())))
                || (c.tier != Tier::Unsupported && c.checked.is_none())
            {
                return Err(BoreaError::new(
                    "E_MATRIX",
                    format!(
                        "invalid or duplicate cell: {} {}",
                        c.family.as_str(),
                        c.component
                    ),
                ));
            }
        }
        for component in components {
            for family in FamilyId::ALL {
                if !seen.contains(&(family, component)) {
                    return Err(BoreaError::new(
                        "E_MATRIX",
                        format!("missing {} {component}", family.as_str()),
                    ));
                }
            }
        }
        Ok(())
    }
    pub fn cell(&self, f: FamilyId, id: &str) -> Result<&Cell> {
        self.cells
            .iter()
            .find(|c| c.family == f && c.component == id)
            .ok_or_else(|| {
                BoreaError::unsupported(format!(
                    "{}: no cell for {id}; choose a listed component",
                    f.as_str()
                ))
            })
    }
    pub fn gate(
        &self,
        f: FamilyId,
        components: &[String],
        allow_source: bool,
        deny_third_party: bool,
    ) -> Result<()> {
        let mut failures = Vec::new();
        for id in components {
            match self.cell(f, id) {
                Err(e) => failures.push(e.message),
                Ok(c) => match c.tier {
                    Tier::Unsupported => failures.push(format!(
                        "{id}: {}; remedy: {}",
                        c.reason.as_deref().unwrap_or("unsupported"),
                        c.remedy.as_deref().unwrap_or("choose another component")
                    )),
                    Tier::Source if !allow_source => {
                        failures.push(format!("{id}: source build requires --allow-source"))
                    }
                    Tier::ThirdParty if deny_third_party => failures.push(format!(
                        "{id}: third-party repository refused by --deny-third-party"
                    )),
                    _ => {}
                },
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(BoreaError::unsupported(failures.join("\n")))
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matrix_complete() {
        Matrix::embedded().unwrap();
    }
    #[test]
    fn matrix_collects_gates() {
        let m = Matrix::embedded().unwrap();
        let e = m
            .gate(
                FamilyId::Debian,
                &[
                    "compositor.niri".into(),
                    "desktop.cosmic".into(),
                    "boot.limine".into(),
                ],
                false,
                false,
            )
            .unwrap_err();
        assert!(
            e.message.contains("niri")
                && e.message.contains("cosmic")
                && e.message.contains("limine")
        );
    }
    #[test]
    fn matrix_rejects_duplicates() {
        let mut m = Matrix::embedded().unwrap();
        m.cells.push(m.cells[0].clone());
        assert!(m.validate().is_err());
    }
}
