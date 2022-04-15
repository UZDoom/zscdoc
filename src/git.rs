use std::path::{Path, PathBuf};

use anyhow::Context;
use directories::ProjectDirs;
use git2::Repository;
use sha2::{Digest, Sha256};

fn change_head_in<P: AsRef<Path>>(path: P, refname: &str) -> anyhow::Result<()> {
    let repo = Repository::open(path)?;
    let (object, reference) = repo.revparse_ext(refname)?;

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
            match change_head_in(&path, refname) {
                Ok(()) => {
                    return Ok(path);
                }
                Err(_) => {
                    std::fs::remove_dir_all(&path)?;
                }
            };
        }
        Repository::clone(repo, &path)?;
        change_head_in(&path, refname)?;
        Ok(path)
    } else {
        anyhow::bail!("Couldn't get a cache directory");
    }
}
