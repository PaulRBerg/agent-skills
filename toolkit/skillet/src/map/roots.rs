use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cli::MapArgs;
use crate::error::Error;
use crate::traversal::RootRequest;

use super::model::{PortfolioRecord, UserRootRecord};

pub struct ResolvedRoots {
    pub requests: Vec<RootRequest>,
    pub portfolio: Option<PortfolioRecord>,
}

pub fn resolve(args: &MapArgs) -> Result<ResolvedRoots, Error> {
    if let Some(requested) = args.portfolio_root.as_ref() {
        return resolve_portfolio(requested);
    }

    let requests = if args.root.is_empty() {
        let home = home_directory()?;
        vec![if args.include_catalog_sources {
            RootRequest::broad_including_catalog_sources(home)
        } else {
            RootRequest::broad(home)
        }]
    } else {
        args.root.iter().map(RootRequest::explicit).collect()
    };
    Ok(ResolvedRoots { requests, portfolio: None })
}

fn resolve_portfolio(requested: &Path) -> Result<ResolvedRoots, Error> {
    let requested = absolute_lexical(requested)?;
    let metadata = match fs::metadata(&requested) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(Error::RootMissing(requested));
        }
        Err(error) => return Err(Error::io("inspect", &requested, error)),
    };
    if !metadata.is_dir() {
        return Err(Error::RootNotDirectory(requested));
    }

    let output = Command::new("git")
        .args(["-C"])
        .arg(&requested)
        .args(["rev-parse", "--path-format=absolute", "--show-toplevel"])
        .output()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                Error::GitUnavailable
            } else {
                Error::io("run Git for", &requested, error)
            }
        })?;
    if !output.status.success() {
        return Err(Error::PortfolioNotGit(requested));
    }
    let repository_root = path_from_git_output(output.stdout)?;
    let repository_root = fs::canonicalize(&repository_root)
        .map_err(|error| Error::io("resolve Git repository root", &repository_root, error))?;

    let home = home_directory()?;
    let mut user_roots = Vec::new();
    let mut requests = vec![RootRequest::explicit(&repository_root)];
    for (relative, client) in [(".agents/skills", "codex"), (".claude/skills", "claude-code")] {
        let path = home.join(relative);
        let present = match fs::metadata(&path) {
            Ok(metadata) if metadata.is_dir() => true,
            Ok(_) => return Err(Error::RootNotDirectory(path)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => return Err(Error::io("inspect user skill root", &path, error)),
        };
        if present {
            requests.push(RootRequest::explicit(&path));
        }
        user_roots.push(UserRootRecord { path, client: client.to_owned(), present });
    }

    Ok(ResolvedRoots {
        requests,
        portfolio: Some(PortfolioRecord { requested_path: requested, repository_root, user_roots }),
    })
}

fn home_directory() -> Result<PathBuf, Error> {
    env::var_os("HOME").map(PathBuf::from).ok_or(Error::HomeUnavailable)
}

fn absolute_lexical(path: &Path) -> Result<PathBuf, Error> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        env::current_dir()
            .map(|directory| directory.join(path))
            .map_err(|error| Error::io("resolve current directory for", path, error))
    }
}

fn path_from_git_output(mut output: Vec<u8>) -> Result<PathBuf, Error> {
    if output.last() == Some(&b'\n') {
        output.pop();
        if output.last() == Some(&b'\r') {
            output.pop();
        }
    }
    if output.is_empty() {
        return Err(Error::GitOutput("Git returned an empty repository root".to_owned()));
    }

    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;

        Ok(PathBuf::from(OsString::from_vec(output)))
    }
    #[cfg(not(unix))]
    {
        String::from_utf8(output)
            .map(PathBuf::from)
            .map_err(|_| Error::GitOutput("Git returned a non-UTF-8 repository root".to_owned()))
    }
}
