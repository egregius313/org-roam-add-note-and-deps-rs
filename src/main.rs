//! A tool to add an org-roam note and all its dependencies to the git repository.
use clap::Parser;
use log::{debug, info};
use rusqlite::{Connection, Statement};
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;

use add_note_and_deps::{
    RoamFile,
    git::{find_git_repo, is_modified},
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

fn find_files_referenced_by(
    stmt: &mut Statement,
    path: &RoamFile,
) -> anyhow::Result<Vec<RoamFile>> {
    debug!("Querying for files referenced by {}", path);
    let rows = stmt.query_map([path], |row| {
        let file: RoamFile = row.get(0)?;
        debug!("Found referenced file: {}", file);
        Ok(file)
    })?;
    let results: Vec<RoamFile> = rows.collect::<Result<_, _>>()?;
    Ok(results)
}

fn transitive_closure_of_files(
    conn: &Connection,
    paths: &[RoamFile],
    exclude: impl Fn(&RoamFile) -> bool,
) -> anyhow::Result<Vec<RoamFile>> {
    let mut stmt = conn.prepare(r#"
        WITH source_file_node AS (SELECT nodes.id from nodes join files on nodes.file = files.file where files.file = ?1),
             referenced_nodes AS (SELECT dest from links, source_file_node where links.source = source_file_node.id and links.type = '"id"')
        SELECT nodes.file from nodes, referenced_nodes where nodes.id = referenced_nodes.dest;
    "#).unwrap();
    info!("Prepared statement for finding referenced files");

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut result = Vec::new();

    for path in paths {
        if visited.insert(path.clone()) {
            info!("Starting from file: {}", path);
            queue.push_back(path.clone());
            result.push(path.clone());
        }
    }

    while let Some(current) = queue.pop_front() {
        debug!("Processing file: {}", current);
        if exclude(&current) {
            debug!("File {} does not pass filter, skipping", current);
            continue;
        }
        for referenced in find_files_referenced_by(&mut stmt, &current)? {
            debug!("Found referenced file: {}", referenced);
            if visited.insert(referenced.clone()) {
                queue.push_back(referenced.clone());
                result.push(referenced);
            }
        }
    }

    Ok(result)
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

    let transitive = if args.exclude_unchanged {
        transitive_closure_of_files(&conn, &args.files, |file| {
            is_modified(&repo, file.as_ref()).unwrap_or(false)
        })?
    } else {
        // No filtering, include all files
        transitive_closure_of_files(&conn, &args.files, |_| false)?
    };

    if args.add {
        let mut index = repo.index()?;
        for file in transitive {
            index.add_path(file.as_ref())?;
        }
        index.write()?;
    } else {
        for file in transitive {
            if args.show_all || is_modified(&repo, file.as_ref())? {
                println!("{}", file);
            }
        }
    }

    Ok(())
}
