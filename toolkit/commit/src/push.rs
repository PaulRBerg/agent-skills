use crate::{
    error::{AppError, Result},
    git::{Repository, git_error},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PushOutcome {
    Pushed { branch: String },
    PushedNew { branch: String },
    Behind { branch: String, count: u64 },
}

impl PushOutcome {
    pub fn print(&self) {
        match self {
            Self::Pushed { branch } => println!("PUSHED {branch}"),
            Self::PushedNew { branch } => println!("PUSHED_NEW {branch}"),
            Self::Behind { branch, count } => println!("BEHIND {branch} {count}"),
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

pub fn execute(repository: &Repository) -> Result<PushOutcome> {
    let branch = repository.branch()?;
    repository.head()?;
    let mut destination = destination(repository, &branch)?;
    fetch(repository, &destination.remote)?;
    destination.compare_ref = refreshed_compare_ref(repository, &destination)?;
    if destination.set_upstream {
        destination.new_branch = destination.compare_ref.is_none();
    }
    if let Some(count) = behind_count(repository, destination.compare_ref.as_deref())? {
        return Ok(PushOutcome::Behind { branch, count });
    }

    let first = attempt(repository, &destination)?;
    if first.status.success() {
        return Ok(success_outcome(destination));
    }
    if !is_non_fast_forward(&first.stderr) {
        return Err(git_error(first));
    }

    fetch(repository, &destination.remote)?;
    destination.compare_ref = refreshed_compare_ref(repository, &destination)?;
    if destination.set_upstream {
        destination.new_branch = destination.compare_ref.is_none();
    }
    if let Some(count) = behind_count(repository, destination.compare_ref.as_deref())? {
        return Ok(PushOutcome::Behind { branch, count });
    }
    let second = attempt(repository, &destination)?;
    if !second.status.success() {
        return Err(git_error(second));
    }
    Ok(success_outcome(destination))
}

fn destination(repository: &Repository, branch: &str) -> Result<Destination> {
    let upstream = repository.raw(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{upstream}"], None)?;
    if upstream.status.success() {
        let compare_ref = String::from_utf8(upstream.stdout)
            .map_err(|_| AppError::operational("Git returned a non-UTF-8 upstream"))?
            .trim()
            .to_owned();
        let remote_key = format!("branch.{branch}.remote");
        let merge_key = format!("branch.{branch}.merge");
        let remote = repository.text(["config", "--get", &remote_key], None)?;
        let merge = repository.text(["config", "--get", &merge_key], None)?;
        let remote_branch = merge
            .strip_prefix("refs/heads/")
            .ok_or_else(|| AppError::usage(format!("unsupported upstream merge ref: {merge}")))?
            .to_owned();
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
    if destination.set_upstream {
        let reference = format!("refs/remotes/{}/{}", destination.remote, destination.remote_branch);
        let output = repository.raw(["show-ref", "--verify", "--quiet", &reference], None)?;
        return Ok(output.status.success().then_some(reference));
    }
    Ok(destination.compare_ref.clone())
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

fn attempt(repository: &Repository, destination: &Destination) -> Result<std::process::Output> {
    let refspec = format!("HEAD:refs/heads/{}", destination.remote_branch);
    if destination.set_upstream {
        repository.raw(["push", "--set-upstream", &destination.remote, &refspec], None)
    } else {
        repository.raw(["push", &destination.remote, &refspec], None)
    }
}

fn is_non_fast_forward(stderr: &[u8]) -> bool {
    let stderr = String::from_utf8_lossy(stderr);
    stderr.contains("non-fast-forward") || stderr.contains("(fetch first)")
}

fn success_outcome(destination: Destination) -> PushOutcome {
    if destination.new_branch {
        PushOutcome::PushedNew { branch: destination.branch }
    } else {
        PushOutcome::Pushed { branch: destination.branch }
    }
}
