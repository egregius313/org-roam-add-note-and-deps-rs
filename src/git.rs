//! Git-related helper functions.

use git2::Repository;
use std::env::current_dir;

pub fn find_git_repo() -> anyhow::Result<Repository> {
    let dir = current_dir()?;
    let repo = Repository::discover(dir)?;
    Ok(repo)
}

pub fn is_modified(repo: &Repository, path: &std::path::Path) -> anyhow::Result<bool> {
    let relative_path = path.strip_prefix(
        repo.workdir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine git workdir"))?,
    )?;
    let status = repo.status_file(relative_path)?;
    Ok(status != git2::Status::CURRENT && status != git2::Status::IGNORED)
}
