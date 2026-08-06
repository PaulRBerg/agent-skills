use std::{
    ffi::OsString,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

use crate::{
    domain::Scope,
    error::{AppError, Result},
};

pub(crate) const MAX_SCOPE_CHARS: usize = 120;
pub(crate) const UNHASHABLE_BLOB_HASH: &str = "<deleted-or-unhashable>";

const GIT_ROOT_TIMEOUT: Duration = Duration::from_secs(5);
const GIT_INSPECTION_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub(super) struct CommandOutput {
    pub(super) status: ExitStatus,
    pub(super) stdout: Vec<u8>,
    pub(super) stderr: Vec<u8>,
}

/// Run a child with bounded wall time while draining both output pipes.
pub(super) fn run_output_timeout(command: &mut Command, timeout: Duration) -> std::io::Result<CommandOutput> {
    let mut child = command.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    let mut stdout = child.stdout.take().expect("piped stdout");
    let mut stderr = child.stderr.take().expect("piped stderr");
    let stdout_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).map(|_| bytes)
    });
    let stderr_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).map(|_| bytes)
    });
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("command timed out after {} seconds", timeout.as_secs()),
            ));
        }
        thread::sleep(Duration::from_millis(10));
    };
    let stdout = stdout_reader.join().map_err(|_| std::io::Error::other("stdout reader panicked"))??;
    let stderr = stderr_reader.join().map_err(|_| std::io::Error::other("stderr reader panicked"))??;
    Ok(CommandOutput { status, stdout, stderr })
}

pub(crate) fn git_root(cwd: &Path) -> Option<PathBuf> {
    let output = run_output_timeout(
        Command::new("git").args(["-C"]).arg(cwd).args(["rev-parse", "--show-toplevel"]),
        GIT_ROOT_TIMEOUT,
    )
    .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = std::str::from_utf8(&output.stdout).ok()?.trim();
    if value.is_empty() {
        return None;
    }
    weakly_canonical(Path::new(value)).ok()
}

pub(crate) fn normalize_scopes(raw_scopes: &[PathBuf], cwd: &Path, root: &Path) -> Result<Vec<String>> {
    let root = weakly_canonical(root)
        .map_err(|error| AppError::usage(format!("could not resolve repository root: {error}")))?;
    let mut normalized = Vec::new();
    for raw_scope in raw_scopes {
        let display = raw_scope.to_str().ok_or_else(|| AppError::usage("scope is not valid UTF-8"))?;
        if display.is_empty() || display.chars().any(|value| matches!(value, '*' | '?' | '[' | ']')) {
            return Err(AppError::usage(format!("invalid literal scope: {display:?}")));
        }
        let expanded = expand_user(raw_scope);
        let candidate = if expanded.is_absolute() { expanded } else { cwd.join(expanded) };
        let preserve_final_symlink =
            std::fs::symlink_metadata(&candidate).is_ok_and(|metadata| metadata.file_type().is_symlink());
        let resolved = if preserve_final_symlink {
            let parent =
                candidate.parent().ok_or_else(|| AppError::usage(format!("scope is outside repository: {display}")))?;
            let name = candidate
                .file_name()
                .ok_or_else(|| AppError::usage(format!("scope is outside repository: {display}")))?;
            weakly_canonical(parent)
                .map(|parent| parent.join(name))
                .map_err(|_| AppError::usage(format!("scope is outside repository: {display}")))?
        } else {
            weakly_canonical(&candidate)
                .map_err(|_| AppError::usage(format!("scope is outside repository: {display}")))?
        };
        let relative = resolved
            .strip_prefix(&root)
            .map_err(|_| AppError::usage(format!("scope is outside repository: {display}")))?;
        let value = if relative.as_os_str().is_empty() {
            ".".to_owned()
        } else {
            relative
                .to_str()
                .ok_or_else(|| AppError::usage("normalized scope is not valid UTF-8"))?
                .replace(std::path::MAIN_SEPARATOR, "/")
                .trim_start_matches("./")
                .trim_end_matches('/')
                .to_owned()
        };
        if value.chars().any(char::is_control) {
            return Err(AppError::usage(format!("scope contains non-printable characters: {display:?}")));
        }
        if value.chars().count() > MAX_SCOPE_CHARS {
            return Err(AppError::usage(format!("scope exceeds {MAX_SCOPE_CHARS} characters: {display:?}")));
        }
        if !normalized.contains(&value) {
            normalized.push(value);
        }
    }
    Ok(normalized)
}

pub(crate) fn normalize_claim_scopes(
    files: &[PathBuf],
    recursive: &[PathBuf],
    cwd: &Path,
    root: &Path,
) -> Result<Vec<Scope>> {
    let files = normalize_scopes(files, cwd, root)?;
    let recursive = normalize_scopes(recursive, cwd, root)?;
    if let Some(ambiguous) = files.iter().find(|value| recursive.contains(value)) {
        return Err(AppError::usage(format!("scope cannot be both exact and recursive: {ambiguous}")));
    }
    for value in &files {
        let candidate = root.join(value);
        let symlink = std::fs::symlink_metadata(&candidate).is_ok_and(|metadata| metadata.file_type().is_symlink());
        if !symlink && candidate.is_dir() {
            return Err(AppError::usage(format!("directory scope requires --recursive: {value}")));
        }
    }
    for value in &recursive {
        let candidate = root.join(value);
        if std::fs::symlink_metadata(&candidate).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
            return Err(AppError::usage(format!("recursive scope cannot be a symlink: {value}")));
        }
        if candidate.exists() && !candidate.is_dir() {
            return Err(AppError::usage(format!("recursive scope is not a directory: {value}")));
        }
    }
    Ok(files
        .into_iter()
        .map(|path| Scope { path, recursive: false })
        .chain(recursive.into_iter().map(|path| Scope { path, recursive: true }))
        .collect())
}

pub(crate) fn scope_covers(covering: &Scope, covered: &Scope) -> bool {
    covering.path == covered.path ||
        (covering.recursive && (covering.path == "." || covered.path.starts_with(&format!("{}/", covering.path))))
}

pub(crate) fn scopes_cover(covering: &[Scope], covered: &[Scope]) -> bool {
    covered.iter().all(|value| covering.iter().any(|parent| scope_covers(parent, value)))
}

pub(crate) fn scopes_overlap(left: &Scope, right: &Scope) -> bool {
    left.path == right.path ||
        (left.recursive && (left.path == "." || right.path.starts_with(&format!("{}/", left.path)))) ||
        (right.recursive && (right.path == "." || left.path.starts_with(&format!("{}/", right.path))))
}

pub(crate) fn any_overlap(left: &[Scope], right: &[Scope]) -> bool {
    left.iter().any(|a| right.iter().any(|b| scopes_overlap(a, b)))
}

pub(crate) fn overlapping_paths(left: &[Scope], right: &[Scope]) -> Vec<String> {
    let mut values = Vec::new();
    for a in left {
        for b in right {
            if !scopes_overlap(a, b) {
                continue;
            }
            let value = if a.path.len() >= b.path.len() { &a.path } else { &b.path };
            if !values.contains(value) {
                values.push(value.clone());
            }
        }
    }
    values.sort();
    values
}

pub(crate) fn overlaps_outside_coverage(requested: &[Scope], contender: &[Scope], existing: &[Scope]) -> Vec<String> {
    overlapping_paths(requested, contender)
        .into_iter()
        .filter(|path| {
            let intersection = Scope { path: path.clone(), recursive: false };
            !scopes_cover(existing, &[intersection])
        })
        .collect()
}

pub(crate) fn git_dirty_paths(root: &Path) -> Result<Vec<String>> {
    let output = run_output_timeout(
        Command::new("git").args(["-C"]).arg(root).args(["status", "--porcelain=v1", "-z", "--untracked-files=all"]),
        GIT_INSPECTION_TIMEOUT,
    )
    .map_err(|error| AppError::operational(format!("could not inspect Git dirt: {error}")))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        let detail = if detail.is_empty() { format!("exit {}", output.status.code().unwrap_or(-1)) } else { detail };
        return Err(AppError::operational(format!("could not inspect Git dirt: {detail}")));
    }

    let parts: Vec<&[u8]> = output.stdout.split(|byte| *byte == 0).collect();
    let mut dirty = Vec::new();
    let mut index = 0;
    while index < parts.len() {
        let entry = parts[index];
        index += 1;
        if entry.is_empty() || entry.len() < 4 {
            continue;
        }
        let status = &entry[..2];
        push_git_path(&mut dirty, &entry[3..]);
        if status.contains(&b'R') || status.contains(&b'C') {
            if let Some(other) = parts.get(index) {
                push_git_path(&mut dirty, other);
                index += 1;
            }
        }
    }
    Ok(dirty)
}

pub(crate) fn git_blob_hash(root: &Path, path: &str, write: bool) -> String {
    let mut command = Command::new("git");
    command.args(["-C"]).arg(root).arg("hash-object");
    if write {
        command.arg("-w");
    }
    let output = run_output_timeout(command.args(["--", path]), GIT_INSPECTION_TIMEOUT);
    let Ok(output) = output else {
        return UNHASHABLE_BLOB_HASH.to_owned();
    };
    if !output.status.success() {
        return UNHASHABLE_BLOB_HASH.to_owned();
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if value.is_empty() { UNHASHABLE_BLOB_HASH.to_owned() } else { value }
}

pub(crate) fn relevant_dirty(scopes: &[Scope], dirty_paths: &[String]) -> Vec<String> {
    dirty_paths
        .iter()
        .filter(|path| {
            let dirty = Scope { path: (*path).clone(), recursive: false };
            scopes.iter().any(|scope| scopes_overlap(scope, &dirty))
        })
        .cloned()
        .collect()
}

fn push_git_path(paths: &mut Vec<String>, bytes: &[u8]) {
    if bytes.is_empty() {
        return;
    }
    let path = String::from_utf8_lossy(bytes).into_owned();
    if !paths.contains(&path) {
        paths.push(path);
    }
}

fn expand_user(path: &Path) -> PathBuf {
    let Some(value) = path.to_str() else {
        return path.to_owned();
    };
    if value == "~" {
        return home_dir().unwrap_or_else(|| path.to_owned());
    }
    if let Some(suffix) = value.strip_prefix("~/") {
        if let Some(home) = home_dir() {
            return home.join(suffix);
        }
    }
    path.to_owned()
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").filter(|value| !value.is_empty()).map(PathBuf::from)
}

fn weakly_canonical(path: &Path) -> std::io::Result<PathBuf> {
    let mut cursor = path.to_owned();
    let mut missing: Vec<OsString> = Vec::new();
    loop {
        match std::fs::canonicalize(&cursor) {
            Ok(mut resolved) => {
                for component in missing.iter().rev() {
                    resolved.push(component);
                }
                return Ok(resolved);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let Some(name) = cursor.file_name() else {
                    return Err(error);
                };
                missing.push(name.to_os_string());
                let Some(parent) = cursor.parent() else {
                    return Err(error);
                };
                cursor = parent.to_owned();
            }
            Err(error) => return Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn scope(path: &str, recursive: bool) -> Scope {
        Scope { path: path.to_owned(), recursive }
    }

    #[test]
    fn normalizes_exact_and_recursive_scopes_and_rejects_ambiguity() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        fs::create_dir(root.join("src")).unwrap();
        fs::write(root.join("src/lib.rs"), "fn main() {}\n").unwrap();

        let scopes =
            normalize_claim_scopes(&[PathBuf::from("src/lib.rs")], &[PathBuf::from("src")], root, root).unwrap();
        assert_eq!(scopes, vec![scope("src/lib.rs", false), scope("src", true)]);
        assert!(
            normalize_claim_scopes(&[PathBuf::from("src/lib.rs")], &[PathBuf::from("src/lib.rs")], root, root,)
                .is_err()
        );
        assert!(normalize_claim_scopes(&[PathBuf::from("src")], &[], root, root).is_err());
    }

    #[test]
    fn rejects_globs_and_paths_outside_repository() {
        let temp = TempDir::new().unwrap();
        assert!(normalize_scopes(&[PathBuf::from("*.rs")], temp.path(), temp.path()).is_err());
        assert!(normalize_scopes(&[PathBuf::from("../outside")], temp.path(), temp.path()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn exact_symlink_is_literal_but_recursive_symlink_is_rejected() {
        use std::os::unix::fs::symlink;

        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join("real")).unwrap();
        symlink(temp.path().join("real"), temp.path().join("link")).unwrap();
        assert_eq!(
            normalize_claim_scopes(&[PathBuf::from("link")], &[], temp.path(), temp.path()).unwrap(),
            vec![scope("link", false)]
        );
        assert!(normalize_claim_scopes(&[], &[PathBuf::from("link")], temp.path(), temp.path()).is_err());
    }

    #[test]
    fn exact_and_recursive_overlap_semantics_are_distinct() {
        let exact_dir = scope("src", false);
        let child = scope("src/lib.rs", false);
        let recursive_dir = scope("src", true);
        assert!(!scopes_overlap(&exact_dir, &child));
        assert!(scopes_overlap(&recursive_dir, &child));
        assert!(scopes_cover(&[recursive_dir], &[child]));
        assert!(!scopes_cover(&[exact_dir], &[scope("src/lib.rs", false)]));
    }

    #[test]
    fn inspects_git_root_dirt_and_blob_hashes() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        assert!(Command::new("git").args(["init", "-q"]).current_dir(root).status().unwrap().success());
        fs::create_dir(root.join("nested")).unwrap();
        fs::write(root.join("nested/file.txt"), "content\n").unwrap();

        assert_eq!(git_root(&root.join("nested")), Some(root.canonicalize().unwrap()));
        assert_eq!(git_dirty_paths(root).unwrap(), vec!["nested/file.txt"]);
        assert_ne!(git_blob_hash(root, "nested/file.txt", false), UNHASHABLE_BLOB_HASH);
        assert_eq!(git_blob_hash(root, "missing", false), UNHASHABLE_BLOB_HASH);
        assert_eq!(relevant_dirty(&[scope("nested", true)], &["nested/file.txt".to_owned()]), vec!["nested/file.txt"]);
    }

    #[test]
    fn git_dirt_includes_both_sides_of_a_rename() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        assert!(Command::new("git").args(["init", "-q"]).current_dir(root).status().unwrap().success());
        fs::write(root.join("before.txt"), "content\n").unwrap();
        assert!(Command::new("git").args(["add", "before.txt"]).current_dir(root).status().unwrap().success());
        assert!(
            Command::new("git")
                .args(["-c", "user.name=ai-coord test", "-c", "user.email=test@invalid", "commit", "-qm", "fixture",])
                .current_dir(root)
                .status()
                .unwrap()
                .success()
        );
        fs::rename(root.join("before.txt"), root.join("after.txt")).unwrap();
        assert!(
            Command::new("git").args(["add", "before.txt", "after.txt"]).current_dir(root).status().unwrap().success()
        );

        let dirty = git_dirty_paths(root).unwrap();
        assert!(dirty.contains(&"before.txt".to_owned()));
        assert!(dirty.contains(&"after.txt".to_owned()));
    }
}
