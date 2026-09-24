use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use super::{
    triage_config::main_branch,
    triage_paths::{deterministic_handoff, safe_document_path},
    triage_run::record_reconcile_detail,
};
use crate::{
    domain::{Scope, ScopeKind},
    error::{AppError, Result},
    host::{git_head_oid, scope_covers},
};

pub(super) struct TriageWorktree {
    root: PathBuf,
    run_dir: PathBuf,
    pub(super) path: PathBuf,
    pub(super) branch: String,
    current: f64,
}

impl TriageWorktree {
    pub(super) fn new(root: &Path, run_dir: &Path, run_id: &str, current: f64) -> Self {
        Self {
            root: root.to_owned(),
            run_dir: run_dir.to_owned(),
            path: run_dir.join("worktree"),
            branch: format!("triage/{run_id}"),
            current,
        }
    }

    pub(super) fn create(&self, start: &str) -> Result<()> {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(["worktree", "add", "-b", &self.branch])
            .arg(&self.path)
            .arg(start)
            .output()?;
        if !output.status.success() {
            return Err(AppError::operational(format!(
                "could not create triage worktree: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(())
    }

    fn cleanup(&self) -> Result<()> {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(["worktree", "remove", "--force"])
            .arg(&self.path)
            .output()?;
        if !output.status.success() {
            if self.path.exists() {
                return Err(AppError::operational(format!(
                    "could not remove triage worktree: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                )));
            }
            // A registration whose directory is gone would otherwise pin the branch.
            git_text(&self.root, &["worktree", "prune"])?;
        }
        if git_success(&self.root, &["show-ref", "--verify", "--quiet", &format!("refs/heads/{}", self.branch)])? {
            git_text(&self.root, &["branch", "-D", &self.branch])?;
        }
        Ok(())
    }
}

impl Drop for TriageWorktree {
    fn drop(&mut self) {
        if let Err(error) = self.cleanup() {
            record_reconcile_detail(&self.run_dir, self.current, "cleanup-failed", &error);
        }
    }
}

pub(super) fn admit_commits(
    root: &Path,
    worktree: &Path,
    start: &str,
    finding_ids: &[String],
    authorized_paths: &[String],
    peer_claims: &[Scope],
    current: f64,
) -> Result<HashSet<String>> {
    let mut failed = HashSet::new();
    if !worktree.exists() {
        return Ok(failed);
    }
    let authorized = authorized_paths.iter().map(String::as_str).collect::<HashSet<_>>();
    let candidates = finding_ids
        .iter()
        .filter_map(|id| commit_for_finding(worktree, start, id).ok().flatten().map(|oid| (oid, id)))
        .collect::<HashMap<_, _>>();
    // Admit in ancestry order so every incoming commit is validated before it can reach main.
    for oid in git_text(worktree, &["rev-list", "--reverse", &format!("{start}..HEAD")])?.lines() {
        let Some(id) = candidates.get(oid) else {
            continue;
        };
        let admission = (|| {
            let changed = validate_commit(worktree, start, id, oid)?;
            if !changed.iter().all(|path| safe_document_path(path) && authorized.contains(path.as_str())) {
                return Err(AppError::operational(
                    "triage commit changes a path outside the authorized documentation tier",
                ));
            }
            if !main_branch(root) {
                return Err(AppError::operational("triage commits can be admitted only while main is checked out"));
            }
            if git_success(root, &["merge-base", "--is-ancestor", oid, "HEAD"])? {
                return Ok(());
            }
            if changed.iter().any(|path| {
                let changed = Scope { path: path.clone(), kind: ScopeKind::Exact };
                peer_claims.iter().any(|claim| scope_covers(claim, &changed))
            }) {
                return Err(AppError::operational("triage commit changes a path claimed by another session's work"));
            }
            let head = git_head_oid(root).ok_or_else(|| AppError::operational("main has no HEAD"))?;
            if git_text(worktree, &["rev-list", "--parents", "-n", "1", oid])?.split_whitespace().collect::<Vec<_>>() !=
                [oid, &head]
            {
                return Err(AppError::operational("main moved or triage commit has unadmitted ancestors"));
            }
            git_text(root, &["merge", "--ff-only", oid])?;
            Ok(())
        })();
        if let Err(error) = admission {
            record_reconcile_detail(
                worktree.parent().expect("worktree is under run directory"),
                current,
                "admission-failed",
                &format!("{id}: {error}"),
            );
            failed.insert((*id).clone());
        }
    }
    Ok(failed)
}

pub(super) fn copy_handoff(worktree: Option<&Path>, root: &Path, finding_id: &str) -> Result<()> {
    let Some(worktree) = worktree else {
        return Ok(());
    };
    let relative = deterministic_handoff(finding_id);
    let source = worktree.join(&relative);
    match fs::symlink_metadata(&source) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
        Ok(metadata) if metadata.is_file() => {}
        Ok(_) => return Err(AppError::operational("handoff source must be a regular file")),
    }
    for base in [worktree, root] {
        for directory in [base.join(".ai"), base.join(".ai/task-handoffs")] {
            if base == root {
                match fs::create_dir(&directory) {
                    Ok(()) => {}
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                    Err(error) => return Err(error.into()),
                }
            }
            if !fs::symlink_metadata(&directory)?.is_dir() {
                return Err(AppError::operational("handoff parent must be a physical directory"));
            }
        }
    }
    let destination = root.join(relative);
    match fs::symlink_metadata(&destination) {
        Ok(_) => return Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let bytes = fs::read(source)?;
    let mut temporary = tempfile::NamedTempFile::new_in(destination.parent().expect("handoff has parent"))?;
    temporary.write_all(&bytes)?;
    temporary.as_file().sync_all()?;
    match temporary.persist_noclobber(destination) {
        Ok(_) => Ok(()),
        Err(error) if error.error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error.error.into()),
    }
}

pub(super) fn commit_for_finding(root: &Path, start: &str, finding_id: &str) -> Result<Option<String>> {
    let pattern = format!("Finding-ID: {finding_id}");
    let output = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["log", "--format=%H", "--fixed-strings", "--grep"])
        .arg(&pattern)
        .arg(format!("{start}..HEAD"))
        .output()?;
    if !output.status.success() {
        return Ok(None);
    }
    let oids = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    Ok((oids.len() == 1).then(|| oids[0].clone()))
}

pub(super) fn validate_commit(root: &Path, start: &str, finding_id: &str, oid: &str) -> Result<HashSet<String>> {
    if oid.len() < 7 || oid.len() > 64 || !oid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AppError::operational("invalid triage commit OID"));
    }
    if !git_success(root, &["merge-base", "--is-ancestor", start, oid])? ||
        !git_success(root, &["merge-base", "--is-ancestor", oid, "HEAD"])?
    {
        return Err(AppError::operational("triage commit is not between the run start and current HEAD"));
    }
    let message = git_text(root, &["show", "-s", "--format=%B", oid])?;
    let trailer = format!("Finding-ID: {finding_id}");
    if !message.lines().any(|line| line == trailer) {
        return Err(AppError::operational("triage commit is missing the exact Finding-ID trailer"));
    }
    if message.lines().filter(|line| line.starts_with("Finding-ID:")).count() != 1 {
        return Err(AppError::operational("triage commit must contain exactly one Finding-ID trailer"));
    }
    let matching = commit_for_finding(root, start, finding_id)?;
    if matching.as_deref() != Some(oid) {
        return Err(AppError::operational("finding must map to exactly one triage commit"));
    }
    let changed = changed_regular_files(root, oid)?;
    if changed.is_empty() {
        return Err(AppError::operational("triage commit changes no paths"));
    }
    Ok(changed)
}

/// Paths a commit adds or modifies, rejecting deletions, renames, type
/// changes, symlinks, gitlinks, and mode changes: only regular-file content
/// edits (or new non-executable files) qualify as documentation fixes.
fn changed_regular_files(root: &Path, oid: &str) -> Result<HashSet<String>> {
    let raw = git_text(root, &["diff-tree", "--no-commit-id", "-r", "--raw", "-z", oid])?;
    let mut fields = raw.split('\0').filter(|field| !field.is_empty());
    let mut changed = HashSet::new();
    while let Some(header) = fields.next() {
        let path = fields.next().ok_or_else(|| AppError::operational("malformed triage commit diff"))?;
        let entry = header.strip_prefix(':').unwrap_or_default().split(' ').collect::<Vec<_>>();
        let regular = match entry[..] {
            [_, new_mode, _, _, "A"] => new_mode == "100644",
            [old_mode, new_mode, _, _, "M"] => old_mode == new_mode && matches!(new_mode, "100644" | "100755"),
            _ => false,
        };
        if !regular {
            return Err(AppError::operational(format!("triage commit may only edit regular files: {path}")));
        }
        changed.insert(path.to_owned());
    }
    Ok(changed)
}

pub(super) fn git_success(root: &Path, args: &[&str]) -> Result<bool> {
    Ok(Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?
        .success())
}

pub(super) fn git_text(root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git").args(["-C"]).arg(root).args(args).output()?;
    if !output.status.success() {
        return Err(AppError::operational(format!(
            "Git artifact validation failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
