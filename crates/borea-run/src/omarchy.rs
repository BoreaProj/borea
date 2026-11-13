use borea_core::{defaults::*, *};
use std::path::Path;
pub fn detect(root: &Path) -> Result<()> {
    let release = std::fs::read_to_string(crate::rooted(root, Path::new("/etc/os-release"))?)?;
    if !release
        .lines()
        .any(|l| l == "ID=arch" || l == "ID=\"arch\"")
        || !crate::rooted(root, Path::new("/etc/omarchy"))?.exists()
    {
        return Err(BoreaError::unsupported(
            "existing Omarchy installation required; use its own installer first",
        ));
    }
    Ok(())
}
pub fn trim_preview(ids: &[String]) -> Result<Vec<serde_json::Value>> {
    ids.iter().map(|id|{let item=trim_item(id)?;let args=if let Some(app)=item.webapp{vec!["omarchy-webapp-remove",app]}else if !item.packages.is_empty(){std::iter::once("omarchy-pkg-drop").chain(item.packages.iter().copied()).collect()}else{Vec::new()};Ok(serde_json::json!({"id":item.id,"argv":args,"already_absent":args.is_empty(),"executed":false,"upstream_pin":"pending"}))}).collect()
}
pub fn enforce_gate() -> Result<()> {
    Err(BoreaError::unsupported(
        "Omarchy mutations await a verified upstream commit and update-window lock integration; use trim --dry-run and defaults status/restore; no removals were performed",
    ))
}
