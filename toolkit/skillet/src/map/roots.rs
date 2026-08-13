use std::{
    env,
    ffi::OsString,
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

use wait_timeout::ChildExt;

use crate::{cli::MapArgs, error::Error, traversal::RootRequest};

use super::model::{PortfolioRecord, UserRootRecord};

const GIT_TIMEOUT: Duration = Duration::from_secs(10);

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

    let output = run_output_timeout(
        Command::new("git").args(["-C"]).arg(&requested).args([
            "rev-parse",
            "--path-format=absolute",
            "--show-toplevel",
        ]),
        GIT_TIMEOUT,
    )
    .map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => Error::GitUnavailable,
        io::ErrorKind::TimedOut => Error::io("resolve portfolio root", &requested, error),
        _ => Error::io("run Git for", &requested, error),
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

fn run_output_timeout(command: &mut Command, timeout: Duration) -> io::Result<Output> {
    let mut child = command.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let stdout_reader = output_reader(stdout).inspect_err(|_| terminate_child(&mut child))?;
    let stderr_reader = output_reader(stderr).inspect_err(|_| terminate_child(&mut child))?;
    let status = match child.wait_timeout(timeout) {
        Ok(Some(status)) => status,
        Ok(None) => {
            terminate_child(&mut child);
            return Err(io::Error::new(io::ErrorKind::TimedOut, format!("Git command timed out after {timeout:?}")));
        }
        Err(error) => return cleanup_on_error(Err(error), &mut child),
    };
    let stdout = cleanup_on_error(read_output(&stdout_reader), &mut child)?;
    let stderr = cleanup_on_error(read_output(&stderr_reader), &mut child)?;
    Ok(Output { status, stdout, stderr })
}

fn output_reader(mut output: impl Read + Send + 'static) -> io::Result<Receiver<io::Result<Vec<u8>>>> {
    let (sender, receiver) = mpsc::sync_channel(1);
    thread::Builder::new().name("ai-skillet-git-output".to_owned()).spawn(move || {
        let mut bytes = Vec::new();
        let result = output.read_to_end(&mut bytes).map(|_| bytes);
        let _ = sender.send(result);
    })?;
    Ok(receiver)
}

fn read_output(reader: &Receiver<io::Result<Vec<u8>>>) -> io::Result<Vec<u8>> {
    reader.recv().map_err(|_| io::Error::other("Git output reader stopped"))?
}

fn cleanup_on_error<T>(result: io::Result<T>, child: &mut Child) -> io::Result<T> {
    if result.is_err() {
        terminate_child(child);
    }
    result
}

fn terminate_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
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

#[cfg(all(test, unix))]
mod tests {
    use std::{
        process::Command,
        time::{Duration, Instant},
    };

    use super::run_output_timeout;

    #[test]
    fn git_output_wait_is_bounded() {
        let started = Instant::now();
        let error =
            run_output_timeout(Command::new("/bin/sh").args(["-c", "sleep 1"]), Duration::from_millis(50)).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
        assert!(error.to_string().contains("Git command timed out"));
        assert!(started.elapsed() < Duration::from_millis(500));
    }
}
