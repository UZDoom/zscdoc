use std::{
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::Context;
use crossterm::{
    terminal::{Clear, ClearType},
    ExecutableCommand,
};
use directories::ProjectDirs;
use git2::{FetchOptions, RemoteCallbacks, Repository};
use sha2::{Digest, Sha256};

fn change_head_in<P: AsRef<Path>>(url: &str, path: P, refname: &str) -> anyhow::Result<()> {
    use std::io::IsTerminal;

    let repo = Repository::open(path)?;

    repo.remote_set_url("origin", url)?;
    repo.remote_add_fetch("origin", "+refs/heads/*:refs/remotes/origin/*")?;

    let mut remote = repo.find_remote("origin")?;
    let mut cb = RemoteCallbacks::new();

    let mut cleared = false;
    cb.transfer_progress(move |stats| {
        if !std::io::stdout().is_terminal() {
            std::io::stderr().flush().unwrap();
            return true;
        }
        if stats.received_objects() == stats.total_objects() {
            if !cleared {
                std::io::stderr()
                    .execute(Clear(ClearType::CurrentLine))
                    .unwrap();
                cleared = true;
            }
            eprint!(
                "Resolving deltas {}/{}\r",
                stats.indexed_deltas(),
                stats.total_deltas()
            );
        } else if stats.total_objects() > 0 {
            eprint!(
                "Received {}/{} objects ({}) in {} bytes\r",
                stats.received_objects(),
                stats.total_objects(),
                stats.indexed_objects(),
                stats.received_bytes()
            );
        }
        std::io::stderr().flush().unwrap();
        true
    });

    let mut fo = FetchOptions::new();
    fo.remote_callbacks(cb);
    remote.fetch(&[] as &[&str], Some(&mut fo), None)?;
    std::io::stderr()
        .execute(Clear(ClearType::CurrentLine))
        .unwrap();

    let (object, reference) = repo.revparse_ext(refname)?;

    repo.checkout_tree(&object, None)?;
    repo.reset(&object, git2::ResetType::Hard, None)?;

    match reference {
        Some(gref) => repo.set_head(gref.name().unwrap()),
        None => repo.set_head_detached(object.id()),
    }?;

    Ok(())
}

pub fn clone_git(repo: &str, refname: &str) -> anyhow::Result<PathBuf> {
    if let Some(proj_dirs) = ProjectDirs::from("", "zscdoc", "zscdoc") {
        let cache_dir = proj_dirs.cache_dir();
        let r = fs_extra::dir::create_all(cache_dir, false);
        match &r {
            Ok(()) => {}
            Err(e) => match e.kind {
                fs_extra::error::ErrorKind::AlreadyExists => {}
                _ => {
                    r.context("failed to make cache directory")?;
                }
            },
        };
        let mut repo_name_hasher = Sha256::new();
        repo_name_hasher.update(repo.as_bytes());
        let repo_hash = repo_name_hasher.finalize();
        let path = cache_dir.join("checkouts").join(format!("{:x}", repo_hash));
        if path.exists() {
            match change_head_in(repo, &path, refname) {
                Ok(()) => {
                    return Ok(path);
                }
                Err(_) => {
                    std::fs::remove_dir_all(&path)?;
                }
            };
        }
        Repository::init(&path)?;
        change_head_in(repo, &path, refname)?;
        Ok(path)
    } else {
        anyhow::bail!("Couldn't get a cache directory");
    }
}
