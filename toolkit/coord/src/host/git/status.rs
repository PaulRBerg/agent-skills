use std::{collections::HashSet, path::Path, process::Command};

use crate::error::{AppError, Result};

use super::{GIT_INSPECTION_TIMEOUT, run_output_timeout};

pub(crate) fn git_dirty_paths(root: &Path) -> Result<Vec<String>> {
    inspect_git_dirt(root, &[])
}

/// Inspect literal repository-relative paths without traversing unrelated untracked directories.
/// Git may include descendants or the other side of a rename; callers retain scope filtering.
pub(crate) fn git_dirty_paths_in(root: &Path, paths: &[String]) -> Result<Vec<String>> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    inspect_git_dirt(root, paths)
}

fn inspect_git_dirt(root: &Path, paths: &[String]) -> Result<Vec<String>> {
    let mut command = Command::new("git");
    command.args(["--no-optional-locks", "--literal-pathspecs", "-C"]).arg(root).args([
        "status",
        "--porcelain=v1",
        "-z",
        "--untracked-files=all",
    ]);
    if !paths.is_empty() {
        command.arg("--").args(paths);
    }
    let output = run_output_timeout(&mut command, GIT_INSPECTION_TIMEOUT)
        .map_err(|error| AppError::operational(format!("could not inspect Git dirt: {error}")))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        let detail = if detail.is_empty() { format!("exit {}", output.status.code().unwrap_or(-1)) } else { detail };
        return Err(AppError::operational(format!("could not inspect Git dirt: {detail}")));
    }

    parse_git_dirty_paths(&output.stdout)
}

/// Distinct paths in first-seen order. A path may repeat across index and
/// worktree entries or as the other side of a rename or copy.
fn parse_git_dirty_paths(output: &[u8]) -> Result<Vec<String>> {
    let parts: Vec<&[u8]> = output.split(|byte| *byte == 0).collect();
    let mut dirty = Vec::new();
    let mut seen = HashSet::new();
    let mut index = 0;
    while index < parts.len() {
        let entry = parts[index];
        index += 1;
        if entry.is_empty() || entry.len() < 4 {
            continue;
        }
        let status = &entry[..2];
        push_git_path(&mut dirty, &mut seen, &entry[3..])?;
        if (status.contains(&b'R') || status.contains(&b'C')) &&
            let Some(other) = parts.get(index)
        {
            push_git_path(&mut dirty, &mut seen, other)?;
            index += 1;
        }
    }
    Ok(dirty)
}

fn push_git_path<'a>(paths: &mut Vec<String>, seen: &mut HashSet<&'a [u8]>, bytes: &'a [u8]) -> Result<()> {
    if bytes.is_empty() || !seen.insert(bytes) {
        return Ok(());
    }
    let path = std::str::from_utf8(bytes)
        .map_err(|_| AppError::operational("could not inspect Git dirt: non-UTF-8 path in porcelain output"))?;
    paths.push(path.to_owned());
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{Duration, SystemTime},
    };

    use tempfile::TempDir;

    use super::*;
    use crate::{
        domain::{Scope, ScopeKind},
        host::relevant_dirty,
    };

    fn git(root: &Path, args: &[&str]) {
        let output = Command::new("git").args(args).current_dir(root).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    }

    fn commit(root: &Path) {
        git(root, &["add", "."]);
        git(root, &["-c", "user.name=ai-coord test", "-c", "user.email=test@invalid", "commit", "-qm", "fixture"]);
    }

    #[test]
    fn scoped_dirt_matches_full_status_for_literal_paths_and_scope_kinds() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        git(root, &["init", "-q"]);
        fs::create_dir_all(root.join("app/[slug]")).unwrap();
        fs::create_dir_all(root.join("app/s")).unwrap();
        fs::write(root.join("app/[slug]/deleted"), "delete me").unwrap();
        fs::write(root.join("tracked"), "before").unwrap();
        commit(root);
        fs::remove_file(root.join("app/[slug]/deleted")).unwrap();
        fs::write(root.join("app/[slug]/untracked"), "new").unwrap();
        fs::write(root.join("app/s/unrelated"), "unrelated").unwrap();
        fs::write(root.join("tracked"), "after").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("missing", root.join("app/[slug]/link")).unwrap();
        let full = git_dirty_paths(root).unwrap();
        for (path, kind) in
            [("app/[slug]", ScopeKind::Recursive), ("app/[slug]", ScopeKind::Exact), ("tracked", ScopeKind::Exact)]
        {
            let scopes = [Scope { path: path.to_owned(), kind }];
            let scoped = git_dirty_paths_in(root, &[path.to_owned()]).unwrap();
            assert_eq!(relevant_dirty(&scopes, &scoped), relevant_dirty(&scopes, &full));
            assert!(!scoped.iter().any(|path| path.contains("unrelated")));
        }
        assert!(git_dirty_paths_in(root, &[]).unwrap().is_empty());
    }

    #[test]
    fn scoped_dirt_preserves_renames_across_scope_boundaries() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        git(root, &["init", "-q"]);
        fs::create_dir(root.join("inside")).unwrap();
        fs::create_dir(root.join("outside")).unwrap();
        fs::write(root.join("inside/from"), "moving out").unwrap();
        fs::write(root.join("outside/from"), "moving in").unwrap();
        commit(root);
        git(root, &["mv", "inside/from", "outside/to"]);
        git(root, &["mv", "outside/from", "inside/to"]);
        let full = git_dirty_paths(root).unwrap();
        for path in ["inside", "inside/from", "inside/to", "outside/from", "outside/to"] {
            let kind = if path == "inside" { ScopeKind::Recursive } else { ScopeKind::Exact };
            let scopes = [Scope { path: path.to_owned(), kind }];
            let scoped = git_dirty_paths_in(root, &[path.to_owned()]).unwrap();
            let mut actual = relevant_dirty(&scopes, &scoped);
            let mut expected = relevant_dirty(&scopes, &full);
            actual.sort();
            expected.sort();
            assert_eq!(actual, expected, "scope {path}");
            assert!(!actual.is_empty(), "scope {path} must stay dirty");
        }
    }

    #[test]
    fn dirt_inspection_does_not_refresh_the_index() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        git(root, &["init", "-q"]);
        fs::write(root.join("tracked"), "clean").unwrap();
        commit(root);
        let index = root.join(".git/index");
        let before = fs::read(&index).unwrap();
        let modified = fs::metadata(&index).unwrap().modified().unwrap();
        let changed_time = SystemTime::now() + Duration::from_secs(2);
        fs::File::open(root.join("tracked"))
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(changed_time))
            .unwrap();
        assert!(git_dirty_paths(root).unwrap().is_empty());
        assert!(git_dirty_paths_in(root, &["tracked".to_owned()]).unwrap().is_empty());
        assert_eq!(fs::read(&index).unwrap(), before);
        assert_eq!(fs::metadata(&index).unwrap().modified().unwrap(), modified);
        git(root, &["status", "--porcelain=v1"]);
        assert_ne!(fs::read(&index).unwrap(), before, "fixture must require an index refresh");
    }

    #[test]
    fn porcelain_paths_are_distinct_in_first_seen_order_with_both_rename_sides() {
        let output =
            b"?? a.rs\0 M b.rs\0R  old.rs\0new.rs\0C  src.rs\0copy.rs\0?? a.rs\0MM b.rs\0?? new.rs\0 D copy.rs\0";
        assert_eq!(parse_git_dirty_paths(output).unwrap(), ["a.rs", "b.rs", "old.rs", "new.rs", "src.rs", "copy.rs"]);

        let count = 20_000;
        let mut large = Vec::new();
        for index in 0..count {
            large.extend_from_slice(format!("?? file-{index}.rs\0").as_bytes());
        }
        for index in (0..count).rev() {
            large.extend_from_slice(format!(" M file-{index}.rs\0").as_bytes());
            large.extend_from_slice(format!("R  file-{index}.rs\0moved-{index}.rs\0").as_bytes());
        }
        let parsed = parse_git_dirty_paths(&large).unwrap();
        assert_eq!(parsed.len(), 2 * count);
        assert!(parsed[..count].iter().enumerate().all(|(index, path)| *path == format!("file-{index}.rs")));
        assert!(parsed[count..].iter().zip((0..count).rev()).all(|(path, index)| *path == format!("moved-{index}.rs")));
    }

    #[test]
    fn git_dirt_rejects_non_utf8_porcelain_paths() {
        let error = parse_git_dirty_paths(b"?? foo\xff.txt\0").unwrap_err();
        assert_eq!(error.to_string(), "could not inspect Git dirt: non-UTF-8 path in porcelain output");
    }
}
