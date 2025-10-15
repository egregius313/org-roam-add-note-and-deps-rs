use std::env::home_dir;

pub mod git;
mod roam_file;
pub use roam_file::RoamFile;

pub fn resolve_org_roam_db_path() -> anyhow::Result<std::path::PathBuf> {
    let emacs_cache = home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        .join(".emacs.d")
        .join(".local")
        .join("cache");
    let db_path = emacs_cache.join("org-roam.db");
    if !db_path.exists() {
        return Err(anyhow::anyhow!(
            "Org-roam database not found at expected location: {}",
            db_path.display()
        ));
    }
    Ok(db_path)
}