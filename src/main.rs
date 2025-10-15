//! A tool to add an org-roam note and all its dependencies to the git repository.
use clap::Parser;
use log::info;
use rusqlite::Connection;
use std::path::PathBuf;

use add_note_and_deps::{
    RoamFile,
    git::{find_git_repo, is_modified},
    references::{ReferencedFiles, transitive_closure_of_files},
};

/// Add an org-roam note and all its dependencies to the git repository.
#[derive(Parser)]
struct Args {
    /// Add files to git index instead of printing them
    #[arg(long, action = clap::ArgAction::SetTrue)]
    add: bool,
    /// Only consider files that are new or modified in git
    #[arg(long, action = clap::ArgAction::SetTrue)]
    exclude_unchanged: bool,
    /// Show all files, not just modified ones
    #[arg(long, action = clap::ArgAction::SetTrue)]
    show_all: bool,
    /// Path to org-roam database (if not specified, will try to find it in ~/.emacs.d/.local/cache/org-roam.db)
    #[arg(long = "roam-db")]
    org_roam_db: Option<PathBuf>,
    /// Files to start from
    #[arg(required = true)]
    files: Vec<RoamFile>,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Args::parse();

    let repo = find_git_repo()?;
    info!("Found git repository at {}", repo.path().display());

    let db_path = add_note_and_deps::resolve_org_roam_db_path()?;
    info!("Using org-roam database at {}", db_path.display());

    let conn = Connection::open(db_path)?;
    info!("Connected to database");

    let ReferencedFiles { notes, assets } = if args.exclude_unchanged {
        transitive_closure_of_files(&conn, &args.files, |file| {
            is_modified(&repo, file.as_ref()).unwrap_or(false)
        })?
    } else {
        // No filtering, include all files
        transitive_closure_of_files(&conn, &args.files, |_| false)?
    };

    if args.add {
        let mut index = repo.index()?;
        for file in notes {
            index.add_path(file.as_ref())?;
        }
        for file in assets {
            index.add_path(&file)?;
        }
        index.write()?;
    } else {
        for file in notes {
            if args.show_all || is_modified(&repo, file.as_ref())? {
                println!("{}", file);
            }
        }
        for file in assets {
            if args.show_all || is_modified(&repo, file.as_ref())? {
                println!("{}", file.display());
            }
        }
    }

    Ok(())
}
