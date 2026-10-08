use crate::{
    error::{AppError, Result},
    git::{Repository, git_error},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PushOutcome {
    Pushed { branch: String, rebased: Option<u64> },
    PushedNew { branch: String },
    Behind { branch: String, count: u64, refusal: Option<String> },
}

impl PushOutcome {
    pub fn print(&self) {
        match self {
            Self::Pushed { branch, rebased } => {
                if let Some(count) = rebased {
                    println!("REBASED {branch} {count}");
                }
                println!("PUSHED {branch}");
            }
            Self::PushedNew { branch } => println!("PUSHED_NEW {branch}"),
            Self::Behind { branch, count, .. } => println!("BEHIND {branch} {count}"),
        }
    }

    /// The exit-3 error for a `Behind` outcome. Its message names why `--rebase` did not
    /// integrate the upstream, or is empty when no rebase was requested.
    pub fn retry_error(&self) -> AppError {
        match self {
            Self::Behind { refusal, .. } => AppError::retry(refusal.clone().unwrap_or_default()),
            Self::Pushed { .. } | Self::PushedNew { .. } => AppError::retry(""),
        }
    }
}

#[derive(Clone, Debug)]
struct Destination {
    branch: String,
    remote: String,
    remote_branch: String,
    compare_ref: Option<String>,
    new_branch: bool,
    set_upstream: bool,
}

pub fn execute(repository: &Repository, rebase: bool) -> Result<PushOutcome> {
    let branch = repository.branch()?;
    repository.head()?;
    let mut destination = destination(repository, &branch)?;
    fetch(repository, &destination.remote)?;
    destination.compare_ref = refreshed_compare_ref(repository, &destination)?;
    if destination.set_upstream {
        destination.new_branch = destination.compare_ref.is_none();
    }
    let mut rebased = None;
    if let Some(count) = behind_count(repository, destination.compare_ref.as_deref())? {
        if !rebase {
            return Ok(PushOutcome::Behind { branch, count, refusal: None });
        }
        let compare_ref = destination.compare_ref.as_deref().expect("behind implies a compare ref");
        if let Err(refusal) = rebase_onto_upstream(repository, compare_ref)? {
            return Ok(PushOutcome::Behind { branch, count, refusal: Some(refusal) });
        }
        rebased = Some(count);
    }

    let first = attempt(repository, &destination)?;
    if first.status.success() {
        return Ok(success_outcome(destination, rebased));
    }
    if !is_retryable_rejection(&first.stdout, &destination.remote_branch) {
        return Err(git_error(first));
    }

    fetch(repository, &destination.remote)?;
    destination.compare_ref = refreshed_compare_ref(repository, &destination)?;
    if destination.set_upstream {
        destination.new_branch = destination.compare_ref.is_none();
    }
    if let Some(count) = behind_count(repository, destination.compare_ref.as_deref())? {
        if !rebase {
            return Ok(PushOutcome::Behind { branch, count, refusal: None });
        }
        // The remote moved between the rebase and the push: integrate once more before the
        // final attempt, so the caller sees either a push or an explained refusal.
        let compare_ref = destination.compare_ref.as_deref().expect("behind implies a compare ref");
        if let Err(refusal) = rebase_onto_upstream(repository, compare_ref)? {
            return Ok(PushOutcome::Behind { branch, count, refusal: Some(refusal) });
        }
        rebased = Some(rebased.unwrap_or(0) + count);
    }
    let second = attempt(repository, &destination)?;
    if !second.status.success() {
        return Err(git_error(second));
    }
    Ok(success_outcome(destination, rebased))
}

fn destination(repository: &Repository, branch: &str) -> Result<Destination> {
    let branch_ref = format!("refs/heads/{branch}");
    let compare_ref = repository.text(["for-each-ref", "--format=%(upstream)", &branch_ref], None)?;
    if !compare_ref.is_empty() {
        let remote_key = format!("branch.{branch}.remote");
        let merge_key = format!("branch.{branch}.merge");
        let remote = repository.text(["config", "--get", &remote_key], None)?;
        let merge = repository.text(["config", "--get", &merge_key], None)?;
        let remote_branch = merge
            .strip_prefix("refs/heads/")
            .ok_or_else(|| AppError::usage(format!("unsupported upstream merge ref: {merge}")))?
            .to_owned();
        if remote_branch != branch {
            return Err(AppError::usage(format!(
                "upstream {remote}/{remote_branch} does not match current branch {branch}; ai-commit push refuses to \
                 move a differently named ref; push explicitly with `git push {remote} HEAD:refs/heads/{remote_branch}`"
            )));
        }
        return Ok(Destination {
            branch: branch.to_owned(),
            remote,
            remote_branch,
            compare_ref: Some(compare_ref),
            new_branch: false,
            set_upstream: false,
        });
    }

    let remote_exists = repository.raw(["remote", "get-url", "origin"], None)?;
    if !remote_exists.status.success() {
        return Err(AppError::usage(format!("branch {branch} has no upstream and remote 'origin' does not exist")));
    }
    Ok(Destination {
        branch: branch.to_owned(),
        remote: "origin".to_owned(),
        remote_branch: branch.to_owned(),
        compare_ref: None,
        new_branch: true,
        set_upstream: true,
    })
}

fn fetch(repository: &Repository, remote: &str) -> Result<()> {
    let output = repository.raw(["fetch", "--quiet", remote], None)?;
    if output.status.success() { Ok(()) } else { Err(git_error(output)) }
}

fn refreshed_compare_ref(repository: &Repository, destination: &Destination) -> Result<Option<String>> {
    let reference = if destination.set_upstream {
        format!("refs/remotes/{}/{}", destination.remote, destination.remote_branch)
    } else if let Some(compare_ref) = &destination.compare_ref {
        compare_ref.clone()
    } else {
        return Ok(None);
    };
    let output = repository.raw(["show-ref", "--verify", "--quiet", &reference], None)?;
    match output.status.code() {
        Some(0) => Ok(Some(reference)),
        Some(1) => Ok(None),
        _ => Err(git_error(output)),
    }
}

fn behind_count(repository: &Repository, compare_ref: Option<&str>) -> Result<Option<u64>> {
    let Some(compare_ref) = compare_ref else {
        return Ok(None);
    };
    let range = format!("HEAD...{compare_ref}");
    let counts = repository.text(["rev-list", "--left-right", "--count", &range], None)?;
    let mut fields = counts.split_ascii_whitespace();
    let _ahead =
        fields.next().ok_or_else(|| AppError::operational(format!("cannot parse upstream comparison: {counts}")))?;
    let behind = fields
        .next()
        .ok_or_else(|| AppError::operational(format!("cannot parse upstream comparison: {counts}")))?
        .parse::<u64>()
        .map_err(|_| AppError::operational(format!("cannot parse upstream comparison: {counts}")))?;
    Ok((behind > 0).then_some(behind))
}

/// Rebases the current branch onto the fetched upstream under the same conditions an operator
/// must check by hand: no Git operation in progress, and a clean working tree and index, where
/// untracked files count as dirt and ignored files do not. A rebase that stops (conflicts) is
/// aborted so the branch returns to its pre-rebase state. The inner `Err` names the reason the
/// upstream was not integrated.
fn rebase_onto_upstream(repository: &Repository, compare_ref: &str) -> Result<std::result::Result<(), String>> {
    if let Err(error) = repository.ensure_idle() {
        return Ok(Err(format!("rebase skipped: {}", error.message)));
    }
    let dirt = repository.bytes(["status", "--porcelain=v1", "-z", "--untracked-files=all"], None)?;
    if !dirt.is_empty() {
        return Ok(Err("rebase skipped: the working tree or index is not clean".to_owned()));
    }
    let output = repository.raw(["rebase", "--no-autostash", "--quiet", compare_ref], None)?;
    if output.status.success() {
        return Ok(Ok(()));
    }
    let detail = git_error(output).message;
    if repository.ensure_idle().is_err() {
        let abort = repository.raw(["rebase", "--abort"], None)?;
        if !abort.status.success() {
            return Err(AppError::operational(format!(
                "rebase onto {compare_ref} failed ({detail}) and `git rebase --abort` also failed: {}",
                git_error(abort).message
            )));
        }
    }
    Ok(Err(format!("rebase onto {compare_ref} was aborted: {detail}")))
}

fn attempt(repository: &Repository, destination: &Destination) -> Result<std::process::Output> {
    let refspec = format!("HEAD:refs/heads/{}", destination.remote_branch);
    if destination.set_upstream {
        repository.raw(["push", "--porcelain", "--set-upstream", &destination.remote, &refspec], None)
    } else {
        repository.raw(["push", "--porcelain", &destination.remote, &refspec], None)
    }
}

fn is_retryable_rejection(stdout: &[u8], remote_branch: &str) -> bool {
    let refspec = format!("HEAD:refs/heads/{remote_branch}");
    String::from_utf8_lossy(stdout).lines().any(|line| {
        let mut fields = line.splitn(3, '\t');
        matches!(
            (fields.next(), fields.next(), fields.next()),
            (Some("!"), Some(pushed_refspec), Some(summary))
                if pushed_refspec == refspec
                    && (summary.contains("non-fast-forward") || summary.contains("fetch first"))
        )
    })
}

fn success_outcome(destination: Destination, rebased: Option<u64>) -> PushOutcome {
    if destination.new_branch {
        PushOutcome::PushedNew { branch: destination.branch }
    } else {
        PushOutcome::Pushed { branch: destination.branch, rebased }
    }
}
