use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::error::{Error, Result};

pub(crate) fn canonical_root(candidate: &Path) -> Result<PathBuf> {
    let physical = candidate
        .canonicalize()
        .map_err(|error| Error::usage(format!("cannot resolve repository {}: {error}", candidate.display())))?;
    if !physical.is_dir() {
        return Err(Error::usage(format!("not a Git worktree: {}", candidate.display())));
    }

    let output = Command::new("git")
        .arg("-C")
        .arg(&physical)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|error| Error::operational(format!("cannot execute git: {error}")))?;
    if !output.status.success() {
        return Err(Error::usage(format!("not a Git worktree: {}", candidate.display())));
    }
    let root =
        String::from_utf8(output.stdout).map_err(|_| Error::operational("git returned a non-UTF-8 repository path"))?;
    let root = root.trim_end_matches(['\r', '\n']);
    if root.contains(['\r', '\n']) {
        return Err(Error::usage(format!("repository path contains a line break: {}", candidate.display())));
    }
    PathBuf::from(root)
        .canonicalize()
        .map_err(|error| Error::operational(format!("cannot resolve Git root {root}: {error}")))
}

pub(crate) fn is_ignored(repository: &Path, relative: &Path) -> Result<bool> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(["check-ignore", "-q", "--"])
        .arg(relative)
        .output()
        .map_err(|error| Error::operational(format!("cannot execute git: {error}")))?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => {
            let message = String::from_utf8_lossy(&output.stderr);
            Err(Error::operational(format!("git check-ignore failed: {}", message.trim())))
        }
    }
}
