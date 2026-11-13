//! Native family command adapters and deterministic storage planning.
pub mod device;
pub mod planner;
use borea_core::{recipe::FamilyRecipe, *};
pub fn loader(c: &Config, f: &FamilyRecipe, m: &borea_core::matrix::Matrix) -> Result<Loader> {
    let l = if c.boot.loader == Loader::Auto {
        f.default_loader
    } else {
        c.boot.loader
    };
    m.gate(
        c.target.family,
        &[format!("boot.{}", l.as_str())],
        false,
        false,
    )?;
    if c.session.kind == "omarchy" && (c.disk.root_fs != Filesystem::Btrfs || l != Loader::Limine) {
        return Err(BoreaError::unsupported(
            "pinned Omarchy requires Btrfs and Limine; choose root_fs = 'btrfs' and loader = 'auto' or 'limine'",
        ));
    }
    Ok(l)
}
pub fn install(f: FamilyId, packages: &[String]) -> Vec<String> {
    let prefix = match f {
        FamilyId::Arch => vec!["pacman", "-S", "--needed", "--noconfirm"],
        FamilyId::Debian => vec!["apt-get", "-y", "install"],
        FamilyId::Fedora => vec!["dnf5", "-y", "install"],
        FamilyId::Suse => vec!["zypper", "--non-interactive", "install", "--no-recommends"],
    };
    prefix
        .into_iter()
        .map(String::from)
        .chain(packages.iter().cloned())
        .collect()
}
pub fn package_query(f: FamilyId, p: &str) -> Vec<String> {
    match f {
        FamilyId::Arch => vec!["pacman", "-Q", p],
        FamilyId::Debian => vec!["dpkg-query", "-W", "-f=${db:Status-Status}", p],
        FamilyId::Fedora | FamilyId::Suse => vec!["rpm", "-q", p],
    }
    .into_iter()
    .map(String::from)
    .collect()
}
