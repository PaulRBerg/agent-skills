use std::{
    collections::{BTreeSet, HashMap, HashSet},
    ffi::OsString,
    fs::{self, OpenOptions},
    io::Write,
    path::{Component, Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    thread,
    time::Duration,
};

use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        Client, FindingState, FindingSummary, Identity, OutcomeKind, ProcessFingerprint, ProcessLiveness, ProcessProbe,
        SessionState,
    },
    error::{AppError, Result},
    host::{DetachedProcessRunner, DetachedProcessSpec, NativeDetachedProcessRunner, git_head_oid},
    state::{FindingResolution, SessionUpdate, Store, TriageRun},
};

use super::{
    Coordinator, path_text, resolved,
    triage_command::codex_args,
    triage_config::{TriageSchedule, auto_triage_enabled, main_branch},
    triage_paths::{deterministic_handoff, safe_document_path},
    triage_prompt::triage_prompt,
    triage_schema::result_schema,
};

#[cfg(test)]
const CONFIG_PATH: &str = ".ai-coord.toml";
const RUN_DIRECTORY: &str = "triage-runs";
const RUN_DEADLINE_SECONDS: f64 = 30.0 * 60.0;
const HEARTBEAT_SECONDS: f64 = 2.0;
const HEARTBEAT_GRACE_SECONDS: f64 = 15.0;
const LOG_RETENTION_SECONDS: f64 = 30.0 * 24.0 * 60.0 * 60.0;
const RESULT_FILE: &str = "result.json";
const SCHEMA_FILE: &str = "result-schema.json";
const STDOUT_FILE: &str = "stdout.log";
const STDERR_FILE: &str = "stderr.log";

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RunMetadata {
    run_id: String,
    repo_root: String,
    state_dir: String,
    start_head: String,
    finding_ids: Vec<String>,
    #[serde(default)]
    authorized_paths: Vec<String>,
    started_at: f64,
    heartbeat_at: f64,
    finished_at: Option<f64>,
    worker: Option<ProcessFingerprint>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TriageOutput {
    results: Vec<FindingResult>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FindingResult {
    finding_id: String,
    status: ResultStatus,
    evidence: String,
    changed_paths: Vec<String>,
    validation: Vec<String>,
    commit_oid: Option<String>,
    canonical_id: Option<String>,
    handoff_path: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum ResultStatus {
    Fixed,
    Stale,
    Rejected,
    Duplicate,
    HandedOff,
    Deferred,
}

struct TriageRequest<'a> {
    repo_root: &'a Path,
    state_dir: &'a Path,
    run_dir: &'a Path,
    prompt: &'a str,
}

trait TriageRunner {
    fn run(&self, request: &TriageRequest<'_>, heartbeat: &mut dyn FnMut() -> Result<()>) -> Result<ExitStatus>;
}

struct CodexTriageRunner;

/// True while running as (or nested under) a detached triager worker.
/// Checked ahead of identity resolution: it is a cheap, self-contained
/// recursion guard that must suppress rescheduling even when the ambient
/// process environment doesn't yield a resolvable origin identity.
fn triager_lifecycle_active() -> bool {
    std::env::var_os("AI_COORD_TRIAGE_RUN_ID").is_some() ||
        std::env::var_os("AI_COORD_TRIAGE_ROLE").as_deref() == Some(std::ffi::OsStr::new("triager"))
}

impl Coordinator {
    /// Fail-open scheduling; transactional gates keep concurrent lifecycle hooks idempotent.
    pub(crate) fn schedule_findings_triage(&self, cwd: &Path) -> Result<TriageSchedule> {
        if triager_lifecycle_active() {
            return Ok(TriageSchedule::Skipped("triager-lifecycle"));
        }
        let Some(origin) = self.identity(false)? else {
            return Ok(TriageSchedule::Skipped("missing-origin"));
        };
        self.schedule_findings_triage_for_identity(cwd, &origin)
    }

    /// Schedule from an already-known normal lifecycle identity. This avoids
    /// relying on process discovery after a SessionEnd hook has removed it.
    pub(crate) fn schedule_findings_triage_for_identity(
        &self,
        cwd: &Path,
        origin: &Identity,
    ) -> Result<TriageSchedule> {
        if triager_lifecycle_active() {
            return Ok(TriageSchedule::Skipped("triager-lifecycle"));
        }
        self.schedule_findings_triage_for(cwd, origin, &NativeDetachedProcessRunner)
    }

    pub(crate) fn run_findings_triage(&self, run_id: &str, cwd: &Path) -> Result<()> {
        self.run_findings_triage_with(run_id, cwd, &CodexTriageRunner)
    }

    fn schedule_findings_triage_for(
        &self,
        cwd: &Path,
        origin: &Identity,
        launcher: &dyn DetachedProcessRunner,
    ) -> Result<TriageSchedule> {
        let root = crate::host::git_root(&resolved(cwd))
            .ok_or_else(|| AppError::operational("finding triage requires a Git worktree"))?;
        if !auto_triage_enabled(&root)? {
            return Ok(TriageSchedule::Skipped("disabled"));
        }
        if !main_branch(&root) {
            return Ok(TriageSchedule::Skipped("branch"));
        }
        let now = self.clock.wall();
        let state_dir = fs::canonicalize(
            self.store_path.parent().ok_or_else(|| AppError::operational("state database has no parent directory"))?,
        )?;
        let run_root = state_dir.join(RUN_DIRECTORY);
        fs::create_dir_all(&run_root)?;
        prune_run_logs(&run_root, now)?;
        self.reconcile_inactive_runs(&root, &run_root, now)?;

        let mut store = self.store()?;
        store.release_orphaned_claims(now)?;
        if !self.refresh_inventory(&mut store, false)?.complete {
            return Ok(TriageSchedule::Skipped("coverage"));
        }
        let Some(start) = store.begin_triage_run(&path_text(&root)?, origin, now)? else {
            return Ok(TriageSchedule::Skipped("ineligible"));
        };
        let run_dir = run_root.join(&start.run.id);
        if let Err(error) = fs::create_dir(&run_dir) {
            let _ = store.finish_triage_run(&start.run.id, "launch-failed", now);
            return Err(error.into());
        }
        let Some(start_head) = git_head_oid(&root) else {
            let _ = store.finish_triage_run(&start.run.id, "launch-failed", now);
            return Err(AppError::operational("finding triage requires a current Git HEAD"));
        };
        let mut metadata = RunMetadata {
            run_id: start.run.id.clone(),
            repo_root: path_text(&root)?,
            state_dir: path_text(&state_dir)?,
            start_head,
            finding_ids: start.claims.iter().map(|claim| claim.finding_id.clone()).collect(),
            authorized_paths: Vec::new(),
            started_at: now,
            heartbeat_at: now,
            finished_at: None,
            worker: None,
        };
        write_metadata(&run_dir, &metadata)?;
        let executable = std::env::current_exe()
            .map_err(|error| AppError::operational(format!("could not locate ai-coord executable: {error}")))?;
        let spec = DetachedProcessSpec {
            program: executable,
            args: vec![
                OsString::from("triage-worker"),
                OsString::from("--run-id"),
                OsString::from(&start.run.id),
                OsString::from("--repo"),
                root.as_os_str().to_owned(),
            ],
            current_dir: root.clone(),
            environment: vec![
                (OsString::from("AI_COORD_STATE_DIR"), state_dir.as_os_str().to_owned()),
                (OsString::from("AI_COORD_TRIAGE_RUN_ID"), OsString::from(&start.run.id)),
                (OsString::from("AI_COORD_TRIAGE_ROLE"), OsString::from("triager")),
                (OsString::from("AI_COORD_CLIENT"), OsString::from("codex")),
                (OsString::from("AI_COORD_SESSION_ID"), OsString::from(format!("triage:{}", start.run.id))),
            ],
            stdout_path: run_dir.join("worker.stdout.log"),
            stderr_path: run_dir.join("worker.stderr.log"),
        };
        match launcher.spawn(&spec) {
            Ok(worker) => {
                metadata.worker = Some(worker);
                if let Err(error) = write_metadata(&run_dir, &metadata) {
                    let _ = store.finish_triage_run(&start.run.id, "launch-metadata-failed", now);
                    return Err(error);
                }
                Ok(TriageSchedule::Launched { run_id: start.run.id, finding_count: start.claims.len() })
            }
            Err(error) => {
                let _ = store.finish_triage_run(&start.run.id, "launch-failed", now);
                Err(error)
            }
        }
    }

    fn reconcile_inactive_runs(&self, root: &Path, run_root: &Path, current: f64) -> Result<()> {
        let repo_root = path_text(root)?;
        for run in self.store()?.active_triage_runs(&repo_root)? {
            let run_dir = run_root.join(&run.id);
            let metadata = read_metadata(&run_dir).ok();
            if run_is_live(&run, metadata.as_ref(), self.probe.as_ref(), current) {
                continue;
            }
            if let Some(metadata) = metadata.as_ref() {
                let _ = reconcile_artifacts(self, &run, metadata, root);
            }
            let mut store = self.store()?;
            let _ = store.end_session(&triager_identity(&run.id));
            let _ = store.finish_triage_run(&run.id, "worker-lost", current);
            if let Some(mut metadata) = metadata {
                metadata.finished_at = Some(current);
                metadata.heartbeat_at = current;
                let _ = write_metadata(&run_dir, &metadata);
            }
        }
        Ok(())
    }

    fn run_findings_triage_with(&self, run_id: &str, cwd: &Path, runner: &dyn TriageRunner) -> Result<()> {
        let root = crate::host::git_root(&resolved(cwd))
            .ok_or_else(|| AppError::operational("triage worker requires a Git worktree"))?;
        let mut store = self.store()?;
        let run = store
            .triage_run(run_id)?
            .filter(|run| run.finished_at.is_none())
            .ok_or_else(|| AppError::operational(format!("triage run is not active: {run_id}")))?;
        if run.repo_root != path_text(&root)? {
            return Err(AppError::operational("triage run repository does not match worker repository"));
        }
        let state_dir = fs::canonicalize(self.store_path.parent().expect("store path has parent"))?;
        let run_dir = state_dir.join(RUN_DIRECTORY).join(run_id);
        let mut metadata = read_worker_metadata(&run_dir)?;
        if metadata.run_id != run_id ||
            metadata.repo_root != run.repo_root ||
            metadata.state_dir != path_text(&state_dir)?
        {
            return Err(AppError::operational("triage run metadata does not match the ledger"));
        }

        let _ = reconcile_artifacts(self, &run, &metadata, &root);
        let pending_ids = store.pending_claimed_finding_ids(run_id)?;
        if pending_ids.is_empty() {
            finish_worker(&mut store, &run_dir, &mut metadata, "reconciled", self.clock.wall())?;
            return Ok(());
        }
        if !main_branch(&root) {
            finish_worker(&mut store, &run_dir, &mut metadata, "branch-changed", self.clock.wall())?;
            return Ok(());
        }
        let findings = pending_ids
            .iter()
            .map(|id| {
                store
                    .finding(&run.repo_root, id, self.clock.wall())?
                    .ok_or_else(|| AppError::operational(format!("claimed finding disappeared: {id}")))
            })
            .collect::<Result<Vec<_>>>()?;
        let actor = triager_identity(run_id);
        register_triager_session(&mut store, &actor, &metadata, &root, self.clock.wall())?;
        let authorized_paths = safe_document_paths(&root, &findings)?;
        if !authorized_paths.is_empty() {
            let paths = authorized_paths.iter().map(PathBuf::from).collect::<Vec<_>>();
            let outcome = match self.start_for(actor, &format!("triage findings {run_id}"), &paths, &[], &root) {
                Ok(outcome) => outcome,
                Err(error) => {
                    finish_worker(&mut store, &run_dir, &mut metadata, "scope-failed", self.clock.wall())?;
                    return Err(error);
                }
            };
            if outcome.kind != OutcomeKind::Ready {
                finish_worker(&mut store, &run_dir, &mut metadata, "scope-unavailable", self.clock.wall())?;
                return Ok(());
            }
        }
        metadata.authorized_paths = authorized_paths;
        write_metadata(&run_dir, &metadata)?;
        let prompt = triage_prompt(&metadata.start_head, &findings, &metadata.authorized_paths)?;
        write_private(&run_dir.join(SCHEMA_FILE), serde_json::to_vec_pretty(&result_schema())?.as_slice())?;
        write_private(&run_dir.join("prompt.txt"), prompt.as_bytes())?;

        let request = TriageRequest { repo_root: &root, state_dir: &state_dir, run_dir: &run_dir, prompt: &prompt };
        let mut heartbeat = || {
            let current = self.clock.wall();
            metadata.heartbeat_at = current;
            write_metadata(&run_dir, &metadata)?;
            if !self.store()?.renew_triage_claims(run_id, current)? {
                return Err(AppError::operational("triage run closed while worker was active"));
            }
            Ok(())
        };
        let execution = runner.run(&request, &mut heartbeat);
        let current = self.clock.wall();
        let reconciled = reconcile_artifacts(self, &run, &metadata, &root).unwrap_or_default();
        let outcome = match execution {
            Err(_) => "runner-failed",
            Ok(status) if !status.success() => "runner-failed",
            Ok(_) => match apply_result_file(self, &run, &metadata, &root, &run_dir, &reconciled) {
                Ok(true) => "completed",
                Ok(false) => "partial",
                Err(_) => "invalid-result",
            },
        };
        store = self.store()?;
        finish_worker(&mut store, &run_dir, &mut metadata, outcome, current)?;
        Ok(())
    }
}

impl TriageRunner for CodexTriageRunner {
    fn run(&self, request: &TriageRequest<'_>, heartbeat: &mut dyn FnMut() -> Result<()>) -> Result<ExitStatus> {
        let stdout = private_output(&request.run_dir.join(STDOUT_FILE))?;
        let stderr = private_output(&request.run_dir.join(STDERR_FILE))?;
        let args = codex_args(request.repo_root, request.state_dir, request.run_dir);
        let mut child = Command::new("codex")
            .args(args)
            .current_dir(request.repo_root)
            .env("AI_COORD_TRIAGE_ROLE", "triager")
            .stdin(Stdio::piped())
            .stdout(stdout)
            .stderr(stderr)
            .spawn()
            .map_err(|error| AppError::operational(format!("could not launch Codex triager: {error}")))?;
        let mut stdin = child.stdin.take().ok_or_else(|| AppError::operational("Codex triager stdin unavailable"))?;
        stdin.write_all(request.prompt.as_bytes())?;
        drop(stdin);
        let started = std::time::Instant::now();
        loop {
            if let Some(status) = child.try_wait()? {
                return Ok(status);
            }
            if started.elapsed().as_secs_f64() >= RUN_DEADLINE_SECONDS {
                let _ = child.kill();
                let _ = child.wait();
                return Err(AppError::operational("Codex triage run exceeded the 30-minute deadline"));
            }
            if let Err(error) = heartbeat() {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
            thread::sleep(Duration::from_secs_f64(HEARTBEAT_SECONDS));
        }
    }
}

fn register_triager_session(
    store: &mut Store,
    actor: &Identity,
    metadata: &RunMetadata,
    root: &Path,
    current: f64,
) -> Result<()> {
    let fingerprint =
        metadata.worker.clone().ok_or_else(|| AppError::operational("triage worker process evidence is missing"))?;
    store.upsert_session(&SessionUpdate {
        identity: actor.clone(),
        cwd: path_text(root)?,
        repo_root: Some(metadata.repo_root.clone()),
        state: SessionState::Working,
        source: "triage-worker".to_owned(),
        name: None,
        waiting_for: None,
        permission_mode: None,
        update_permission_mode: false,
        fingerprint: Some(fingerprint),
        started_at: Some(metadata.started_at),
        current,
    })?;
    Ok(())
}

fn safe_document_paths(root: &Path, findings: &[FindingSummary]) -> Result<Vec<String>> {
    let mut paths = BTreeSet::new();
    for path in findings.iter().flat_map(|finding| &finding.paths) {
        if !safe_document_path(path) || !root.join(path).is_file() {
            continue;
        }
        if git_text(root, &["ls-tree", "--name-only", "HEAD", "--", path])?.lines().any(|tracked| tracked == path) {
            paths.insert(path.clone());
        }
    }
    Ok(paths.into_iter().collect())
}

fn run_is_live(run: &TriageRun, metadata: Option<&RunMetadata>, probe: &dyn ProcessProbe, current: f64) -> bool {
    if current - run.started_at >= RUN_DEADLINE_SECONDS {
        return false;
    }
    let Some(metadata) = metadata else {
        return false;
    };
    if metadata.run_id != run.id || current - metadata.heartbeat_at > HEARTBEAT_GRACE_SECONDS {
        return false;
    }
    metadata.worker.as_ref().is_some_and(|fingerprint| probe.liveness(fingerprint) != ProcessLiveness::Dead)
}

fn apply_result_file(
    coordinator: &Coordinator,
    run: &TriageRun,
    metadata: &RunMetadata,
    root: &Path,
    run_dir: &Path,
    reconciled: &HashSet<String>,
) -> Result<bool> {
    if !main_branch(root) {
        return Err(AppError::operational("triage results can be applied only while main is checked out"));
    }
    let bytes = fs::read(run_dir.join(RESULT_FILE))?;
    let output: TriageOutput = serde_json::from_slice(&bytes)?;
    let claimed = metadata.finding_ids.iter().cloned().collect::<HashSet<_>>();
    let statuses =
        output.results.iter().map(|result| (result.finding_id.clone(), result.status)).collect::<HashMap<_, _>>();
    let mut seen = HashSet::new();
    let mut complete = true;
    for result in output.results {
        if !claimed.contains(&result.finding_id) || !seen.insert(result.finding_id.clone()) {
            complete = false;
            continue;
        }
        if reconciled.contains(&result.finding_id) {
            continue;
        }
        if result.status == ResultStatus::Duplicate &&
            result.canonical_id.as_ref().and_then(|id| statuses.get(id)).copied() == Some(ResultStatus::Duplicate)
        {
            complete = false;
            continue;
        }
        if apply_finding_result(coordinator, run, metadata, root, &result).is_err() {
            complete = false;
        }
    }
    if seen.len() != claimed.len() {
        complete = false;
    }
    Ok(complete)
}

fn apply_finding_result(
    coordinator: &Coordinator,
    run: &TriageRun,
    metadata: &RunMetadata,
    root: &Path,
    result: &FindingResult,
) -> Result<()> {
    validate_result_shape(result)?;
    let actor = triager_identity(&run.id);
    let current = coordinator.clock.wall();
    match result.status {
        ResultStatus::Deferred => Ok(()),
        ResultStatus::HandedOff => {
            let path = result.handoff_path.as_deref().expect("shape requires handoff path");
            validate_handoff(root, &result.finding_id, path)?;
            coordinator.store()?.handoff_finding(&run.repo_root, &result.finding_id, path, &actor, current)?;
            Ok(())
        }
        ResultStatus::Fixed => {
            let oid = result.commit_oid.as_deref().expect("shape requires commit");
            let changed = validate_commit(root, &metadata.start_head, &result.finding_id, oid)?;
            if changed != result.changed_paths.iter().cloned().collect::<HashSet<_>>() {
                return Err(AppError::operational("fixed result changed_paths do not exactly match the commit"));
            }
            if result.changed_paths.iter().any(|path| !safe_document_path(path)) {
                return Err(AppError::operational("fixed result changes a path outside the safe documentation tier"));
            }
            let authorized = metadata.authorized_paths.iter().map(String::as_str).collect::<HashSet<_>>();
            if result.changed_paths.iter().any(|path| !authorized.contains(path.as_str())) {
                return Err(AppError::operational("fixed result changes a path outside the pre-authorized scope"));
            }
            coordinator.store()?.resolve_finding(
                &run.repo_root,
                &result.finding_id,
                &FindingResolution {
                    state: FindingState::Fixed,
                    commit_oid: Some(oid.to_owned()),
                    canonical_id: None,
                    actor,
                    current,
                },
            )?;
            Ok(())
        }
        ResultStatus::Stale | ResultStatus::Rejected | ResultStatus::Duplicate => {
            if result.status == ResultStatus::Duplicate {
                let canonical = result.canonical_id.as_deref().expect("shape requires canonical");
                if !metadata.finding_ids.iter().any(|id| id == canonical) &&
                    coordinator.store()?.finding(&run.repo_root, canonical, current)?.is_none()
                {
                    return Err(AppError::operational("duplicate canonical finding does not exist"));
                }
            }
            let state = match result.status {
                ResultStatus::Stale => FindingState::Stale,
                ResultStatus::Rejected => FindingState::Rejected,
                ResultStatus::Duplicate => FindingState::Duplicate,
                _ => unreachable!(),
            };
            coordinator.store()?.resolve_finding(
                &run.repo_root,
                &result.finding_id,
                &FindingResolution {
                    state,
                    commit_oid: None,
                    canonical_id: result.canonical_id.clone(),
                    actor,
                    current,
                },
            )?;
            Ok(())
        }
    }
}

fn validate_result_shape(result: &FindingResult) -> Result<()> {
    if result.evidence.trim().is_empty() {
        return Err(AppError::operational("triage result evidence is required"));
    }
    for path in &result.changed_paths {
        validate_relative_path(path)?;
    }
    let no_commit = result.commit_oid.is_none();
    let no_canonical = result.canonical_id.is_none();
    let no_handoff = result.handoff_path.is_none();
    let valid = match result.status {
        ResultStatus::Fixed => {
            result.commit_oid.is_some() &&
                !result.changed_paths.is_empty() &&
                !result.validation.is_empty() &&
                no_canonical &&
                no_handoff
        }
        ResultStatus::Stale | ResultStatus::Rejected => {
            no_commit && no_canonical && no_handoff && result.changed_paths.is_empty()
        }
        ResultStatus::Duplicate => {
            no_commit &&
                result.canonical_id.as_deref().is_some_and(|id| id != result.finding_id) &&
                no_handoff &&
                result.changed_paths.is_empty()
        }
        ResultStatus::HandedOff => {
            no_commit &&
                no_canonical &&
                !result.validation.is_empty() &&
                result.handoff_path.as_ref().is_some_and(|path| result.changed_paths == [path.clone()])
        }
        ResultStatus::Deferred => no_commit && no_canonical && no_handoff && result.changed_paths.is_empty(),
    };
    if !valid {
        return Err(AppError::operational("triage result fields do not match its status"));
    }
    Ok(())
}

fn reconcile_artifacts(
    coordinator: &Coordinator,
    run: &TriageRun,
    metadata: &RunMetadata,
    root: &Path,
) -> Result<HashSet<String>> {
    let actor = triager_identity(&run.id);
    let current = coordinator.clock.wall();
    let mut reconciled = HashSet::new();
    let authorized = metadata.authorized_paths.iter().map(String::as_str).collect::<HashSet<_>>();
    for finding_id in &metadata.finding_ids {
        let Some(finding) = coordinator.store()?.finding(&run.repo_root, finding_id, current)? else {
            continue;
        };
        if finding.state != FindingState::Pending {
            reconciled.insert(finding_id.clone());
            continue;
        }
        if let Ok(Some(oid)) = commit_for_finding(root, &metadata.start_head, finding_id) &&
            validate_commit(root, &metadata.start_head, finding_id, &oid).is_ok_and(|changed| {
                changed.iter().all(|path| safe_document_path(path) && authorized.contains(path.as_str()))
            }) &&
            coordinator
                .store()?
                .resolve_finding(
                    &run.repo_root,
                    finding_id,
                    &FindingResolution {
                        state: FindingState::Fixed,
                        commit_oid: Some(oid),
                        canonical_id: None,
                        actor: actor.clone(),
                        current,
                    },
                )
                .is_ok()
        {
            reconciled.insert(finding_id.clone());
            continue;
        }
        let handoff = deterministic_handoff(finding_id);
        if validate_handoff(root, finding_id, &handoff).is_ok() &&
            coordinator.store()?.handoff_finding(&run.repo_root, finding_id, &handoff, &actor, current).is_ok()
        {
            reconciled.insert(finding_id.clone());
        }
    }
    Ok(reconciled)
}

fn commit_for_finding(root: &Path, start: &str, finding_id: &str) -> Result<Option<String>> {
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

fn validate_commit(root: &Path, start: &str, finding_id: &str, oid: &str) -> Result<HashSet<String>> {
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
    let changed = git_text(root, &["diff-tree", "--no-commit-id", "--name-only", "-r", oid])?
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect::<HashSet<_>>();
    if changed.is_empty() {
        return Err(AppError::operational("triage commit changes no paths"));
    }
    Ok(changed)
}

fn validate_handoff(root: &Path, finding_id: &str, path: &str) -> Result<()> {
    if path != deterministic_handoff(finding_id) {
        return Err(AppError::operational("handoff path is not deterministic for the finding"));
    }
    validate_relative_path(path)?;
    let candidate = root.join(path);
    let metadata = fs::symlink_metadata(&candidate)?;
    if !metadata.file_type().is_file() {
        return Err(AppError::operational("handoff artifact is not a regular file"));
    }
    let canonical = fs::canonicalize(&candidate)?;
    canonical.strip_prefix(root).map_err(|_| AppError::operational("handoff artifact escapes the repository"))?;
    let text = fs::read_to_string(candidate)?;
    let marker = format!("Source finding: {finding_id}");
    if !text.lines().any(|line| line.trim() == marker) {
        return Err(AppError::operational("handoff artifact has no matching source marker"));
    }
    Ok(())
}

fn validate_relative_path(path: &str) -> Result<()> {
    let candidate = Path::new(path);
    if path.is_empty() ||
        candidate.is_absolute() ||
        path.contains('\\') ||
        candidate.components().any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(AppError::operational("triage artifact path must be a normalized repository-relative path"));
    }
    Ok(())
}

fn git_success(root: &Path, args: &[&str]) -> Result<bool> {
    Ok(Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?
        .success())
}

fn git_text(root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git").args(["-C"]).arg(root).args(args).output()?;
    if !output.status.success() {
        return Err(AppError::operational(format!(
            "Git artifact validation failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn finish_worker(
    store: &mut Store,
    run_dir: &Path,
    metadata: &mut RunMetadata,
    outcome: &str,
    current: f64,
) -> Result<()> {
    metadata.finished_at = Some(current);
    metadata.heartbeat_at = current;
    write_metadata(run_dir, metadata)?;
    store.end_session(&triager_identity(&metadata.run_id))?;
    store.finish_triage_run(&metadata.run_id, outcome, current)?;
    Ok(())
}

pub(super) fn triager_identity(run_id: &str) -> Identity {
    Identity { client: Client::Codex, session_id: format!("triage:{run_id}") }
}

fn metadata_path(run_dir: &Path) -> PathBuf {
    run_dir.join("run.json")
}

fn read_metadata(run_dir: &Path) -> Result<RunMetadata> {
    Ok(serde_json::from_slice(&fs::read(metadata_path(run_dir))?)?)
}

fn read_worker_metadata(run_dir: &Path) -> Result<RunMetadata> {
    for _ in 0..40 {
        let metadata = read_metadata(run_dir)?;
        if metadata.worker.is_some() {
            return Ok(metadata);
        }
        thread::sleep(Duration::from_millis(50));
    }
    Err(AppError::operational("triage worker process evidence was not recorded"))
}

fn write_metadata(run_dir: &Path, metadata: &RunMetadata) -> Result<()> {
    let temporary = run_dir.join("run.json.tmp");
    write_private(&temporary, serde_json::to_vec_pretty(metadata)?.as_slice())?;
    fs::rename(temporary, metadata_path(run_dir))?;
    Ok(())
}

fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut options = OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn private_output(path: &Path) -> Result<Stdio> {
    let mut options = OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    Ok(Stdio::from(options.open(path)?))
}

fn prune_run_logs(run_root: &Path, current: f64) -> Result<()> {
    for entry in fs::read_dir(run_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let age_basis = read_metadata(&entry.path()).ok().and_then(|metadata| metadata.finished_at).or_else(|| {
            entry
                .metadata()
                .ok()?
                .modified()
                .ok()?
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map(|age| age.as_secs_f64())
        });
        if age_basis.is_some_and(|timestamp| current - timestamp > LOG_RETENTION_SECONDS) {
            fs::remove_dir_all(entry.path())?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "triage_tests.rs"]
mod tests;
