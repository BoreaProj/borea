//! Resolve session intent without distro command syntax.
use borea_core::{
    matrix::Matrix,
    recipe::{self, Recipe},
    *,
};
use std::collections::BTreeMap;
#[derive(Debug)]
pub struct Selection {
    pub components: Vec<String>,
    pub greeter: String,
    pub session: Option<Recipe>,
    pub autostart: Option<String>,
}
pub fn select(
    c: &Config,
    m: &Matrix,
    loader: Loader,
    allow_source: bool,
    deny_third_party: bool,
) -> Result<Selection> {
    c.validate()?;
    let catalog = recipe::recipes()?;
    let mut ids = vec![
        "base.minimal".into(),
        format!("fs.{}", c.disk.root_fs.as_str()),
        format!("boot.{}", loader.as_str()),
    ];
    let session_id = match c.session.kind.as_str() {
        "desktop" => c.session.desktop.as_ref().map(|s| format!("desktop.{s}")),
        "shell" => c
            .session
            .compositor
            .as_ref()
            .map(|s| format!("compositor.{s}")),
        _ => None,
    };
    let session = session_id.as_ref().and_then(|s| catalog.get(s)).cloned();
    if let Some(id) = session_id {
        ids.push(id);
    }
    let greeter = if c.session.greeter == "auto" {
        session
            .as_ref()
            .and_then(|s| s.greeter.clone())
            .unwrap_or("none".into())
    } else {
        c.session.greeter.clone()
    };
    if c.session.kind == "minimal" && greeter != "none" {
        return Err(BoreaError::invalid(
            "minimal session requires greeter none or auto",
        ));
    }
    if let Some(s) = &session
        && greeter != "none"
        && s.greeter.as_deref() != Some(&greeter)
    {
        return Err(BoreaError::unsupported(
            "custom greeter/session pair is unverified; use greeter = 'auto'",
        ));
    }
    if greeter == "greetd" {
        ids.push("greeter.greetd".into());
    }
    let mut autostart = None;
    if let Some(shell) = c.session.shell.as_ref().filter(|s| s.as_str() != "none") {
        let id = format!("shell.{shell}");
        let r = catalog
            .get(&id)
            .ok_or_else(|| BoreaError::unsupported("missing shell recipe; choose another shell"))?;
        if let Some(s) = &session
            && !r.compatible_compositors.contains(&s.id)
        {
            return Err(BoreaError::unsupported(
                "shell/compositor pair unavailable; choose a compatible shell",
            ));
        }
        autostart = r.autostart.clone();
        ids.push(id);
    }
    if c.apps.flatpak {
        ids.push("apps.flatpak".into());
    }
    if c.apps.codecs {
        ids.push("system.codecs".into());
    }
    ids.extend(c.apps.enable.iter().map(|s| format!("app.{s}")));
    let components = recipe::resolve(&ids, &catalog)?;
    m.gate(c.target.family, &components, allow_source, deny_third_party)?;
    for id in &components {
        let r = &catalog[id];
        if !r.packages.contains_key(&c.target.family) {
            return Err(BoreaError::unsupported(format!(
                "{id}: repository fingerprint/source commit is not pinned for {}; remedy: omit this component until its recipe is reviewed",
                c.target.family.as_str()
            )));
        }
    }
    Ok(Selection {
        components,
        greeter,
        session,
        autostart,
    })
}
pub fn packages(
    selection: &Selection,
    family: FamilyId,
    catalog: &BTreeMap<String, Recipe>,
) -> Vec<String> {
    let mut p = selection
        .components
        .iter()
        .flat_map(|id| {
            catalog[id]
                .packages
                .get(&family)
                .into_iter()
                .flatten()
                .cloned()
        })
        .collect::<Vec<_>>();
    p.sort();
    p.dedup();
    p
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn minimal_selection() {
        let c = Config::parse(include_str!("../../../recipes/profiles/minimal.toml")).unwrap();
        let s = select(
            &c,
            &Matrix::embedded().unwrap(),
            Loader::Limine,
            false,
            false,
        )
        .unwrap();
        assert_eq!(s.greeter, "none");
    }
    #[test]
    fn unpinned_third_party_refused() {
        let mut c = Config::parse(include_str!(
            "../../../recipes/profiles/hyprland-noctalia.toml"
        ))
        .unwrap();
        c.target.family = FamilyId::Debian;
        let e = select(&c, &Matrix::embedded().unwrap(), Loader::Grub, false, false).unwrap_err();
        assert!(e.message.contains("pinned"));
    }
}
