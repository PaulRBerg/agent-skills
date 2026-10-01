use std::{
    collections::{BTreeSet, HashMap, HashSet},
    ffi::OsString,
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::{Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

use serde::Deserialize;

use crate::{
    domain::{FindingState, FindingSummary, Identity, OutcomeKind, Scope, SessionState, WorkState},
    error::{AppError, Result},
    host::{DetachedProcessRunner, DetachedProcessSpec, NativeDetachedProcessRunner, git_head_oid},
    state::{FindingResolution, SessionUpdate, Store, TriageRun},
};

use super::{
    Clock, Coordinator, path_text, resolved,
    triage_command::codex_args,
    triage_config::{TriageSchedule, auto_triage_enabled, main_branch},
    triage_paths::{deterministic_handoff, safe_document_path},
    triage_prompt::triage_prompt,
    triage_run::{
        HEARTBEAT_SECONDS, RUN_DEADLINE_SECONDS, RunMetadata, finish_failed_schedule_run, finish_worker, lock_metadata,
        private_output, prune_run_logs, read_metadata, read_worker_metadata, record_reconcile_detail, run_is_live,
        triager_identity, with_heartbeat, write_metadata, write_private,
    },
    triage_schema::result_schema,
    triage_worktree::{TriageWorktree, admit_commits, commit_for_finding, copy_handoff, git_text, validate_commit},
};

const RUN_DIRECTORY: &str = "triage-runs";
const RESULT_FILE: &str = "result.json";
const SCHEMA_FILE: &str = "result-schema.json";
const STDOUT_FILE: &str = "stdout.log";
const STDERR_FILE: &str = "stderr.log";

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
    worktree: &'a Path,
    state_dir: &'a Path,
    run_dir: &'a Path,
    prompt: &'a str,
    deadline: Duration,
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
        let now = self.clock.wall();
        let state_dir = fs::canonicalize(
            self.store_path.parent().ok_or_else(|| AppError::operational("state database has no parent directory"))?,
        )?;
        let run_root = state_dir.join(RUN_DIRECTORY);
        // Retention applies to every repository's runs, independent of this one's opt-in or branch.
        prune_run_logs(self, &run_root, now)?;
        let root = crate::host::git_root(&resolved(cwd))
            .ok_or_else(|| AppError::operational("finding triage requires a Git worktree"))?;
        if !auto_triage_enabled(&root)? {
            return Ok(TriageSchedule::Skipped("disabled"));
        }
        if !main_branch(&root) {
            return Ok(TriageSchedule::Skipped("branch"));
        }
        fs::create_dir_all(&run_root)?;
        self.reconcile_inactive_runs(&root, &run_root, now)?;

        let repo_root = path_text(&root)?;
        let mut store = self.store()?;
        store.release_orphaned_claims(now)?;
        // Cheap SQL precheck before the potentially multi-second provider probe
        // (which can spawn `codex app-server`): skip early when active/queued
        // work, an open run, the cooldown, or no unclaimed pending findings
        // already rule out a run. `begin_triage_run` re-checks atomically below.
        if !store.triage_precheck_eligible(&repo_root, now)? {
            return Ok(TriageSchedule::Skipped("ineligible"));
        }
        if !self.refresh_inventory(&mut store, false)?.complete {
            return Ok(TriageSchedule::Skipped("coverage"));
        }
        // Start the run's liveness clock after the probe, right before launch.
        let now = self.clock.wall();
        let Some(start) = store.begin_triage_run(&repo_root, origin, now)? else {
            return Ok(TriageSchedule::Skipped("ineligible"));
        };
        let run_dir = run_root.join(&start.run.id);
        let finding_ids = start.claims.iter().map(|claim| claim.finding_id.clone()).collect();
        let launch = LaunchRequest { root: &root, state_dir: &state_dir, run_dir: &run_dir, run: &start.run };
        match launch_worker(&launch, finding_ids, self.clock.as_ref(), launcher) {
            Ok(()) => Ok(TriageSchedule::Launched { run_id: start.run.id, finding_count: start.claims.len() }),
            Err((outcome, error)) => {
                finish_failed_schedule_run(&mut store, &start.run.id, &run_dir, outcome, self.clock.wall(), &error);
                Err(error)
            }
        }
    }

    fn reconcile_inactive_runs(&self, root: &Path, run_root: &Path, current: f64) -> Result<()> {
        let repo_root = path_text(root)?;
        for run in self.store()?.active_triage_runs(&repo_root)? {
            let run_dir = run_root.join(&run.id);
            let metadata = read_metadata(&run_dir);
            if run_is_live(&run, metadata.as_ref().ok(), self.probe.as_ref(), current) {
                continue;
            }
            let metadata = match metadata {
                Ok(metadata) => Some(metadata),
                Err(error) => {
                    record_reconcile_detail(&run_dir, current, "worker-lost", &error);
                    None
                }
            };
            let _worktree = TriageWorktree::new(root, &run_dir, &run.id, current);
            record_reconcile_detail(&run_dir, current, "worker-lost", &"triage worker is no longer live");
            if let Some(metadata) = metadata.as_ref() &&
                let Err(error) = reconcile_artifacts(self, &run, metadata, root)
            {
                record_reconcile_detail(&run_dir, current, "worker-lost", &error);
            }
            let mut store = self.store()?;
            if let Err(error) = store.end_session(&triager_identity(&run.id)) {
                record_reconcile_detail(&run_dir, current, "worker-lost", &error);
            }
            if let Err(error) = store.finish_triage_run(&run.id, "worker-lost", current) {
                record_reconcile_detail(&run_dir, current, "worker-lost", &error);
            }
            if let Some(mut metadata) = metadata {
                metadata.finished_at = Some(current);
                metadata.heartbeat_at = current;
                if let Err(error) = write_metadata(&run_dir, &metadata) {
                    record_reconcile_detail(&run_dir, current, "worker-lost", &error);
                }
            }
        }
        Ok(())
    }

    fn run_findings_triage_with(&self, run_id: &str, cwd: &Path, runner: &dyn TriageRunner) -> Result<()> {
        let root = crate::host::git_root(&resolved(cwd))
            .ok_or_else(|| AppError::operational("triage worker requires a Git worktree"))?;
        let run = self
            .store()?
            .triage_run(run_id)?
            .filter(|run| run.finished_at.is_none())
            .ok_or_else(|| AppError::operational(format!("triage run is not active: {run_id}")))?;
        if run.repo_root != path_text(&root)? {
            return Err(AppError::operational("triage run repository does not match worker repository"));
        }
        let state_dir = fs::canonicalize(self.store_path.parent().expect("store path has parent"))?;
        let run_dir = state_dir.join(RUN_DIRECTORY).join(run_id);
        let metadata = read_worker_metadata(&run_dir)?;
        if metadata.run_id != run_id ||
            metadata.repo_root != run.repo_root ||
            metadata.state_dir != path_text(&state_dir)?
        {
            return Err(AppError::operational("triage run metadata does not match the ledger"));
        }
        let metadata = Mutex::new(metadata);
        let worker =
            WorkerContext { run: &run, root: &root, state_dir: &state_dir, run_dir: &run_dir, metadata: &metadata };
        let result =
            with_heartbeat(self.clock.as_ref(), &run_dir, &metadata, || self.run_triage_worker(&worker, runner));
        if let Err(error) = &result {
            // Any unfinished exit still releases the run's claims and triager session now.
            let mut metadata = lock_metadata(&metadata);
            if metadata.finished_at.is_none() {
                let current = self.clock.wall();
                record_reconcile_detail(&run_dir, current, "worker-failed", error);
                if let Ok(mut store) = self.store() {
                    let _ = finish_worker(&mut store, &run_dir, &mut metadata, "worker-failed", current);
                }
            }
        }
        result
    }

    fn run_triage_worker(&self, worker: &WorkerContext<'_>, runner: &dyn TriageRunner) -> Result<()> {
        let WorkerContext { run, root, state_dir, run_dir, metadata } = *worker;
        let run_id = run.id.as_str();
        let finish = |store: &mut Store, outcome: &str| {
            finish_worker(store, run_dir, &mut lock_metadata(metadata), outcome, self.clock.wall())
        };
        let mut store = self.store()?;
        let worktree = TriageWorktree::new(root, run_dir, run_id, self.clock.wall());

        let snapshot = lock_metadata(metadata).clone();
        if let Err(error) = reconcile_artifacts(self, run, &snapshot, root) {
            record_reconcile_detail(run_dir, self.clock.wall(), "reconcile-failed", &error);
        }
        let pending_ids = store.pending_claimed_finding_ids(run_id)?;
        if pending_ids.is_empty() {
            return finish(&mut store, "reconciled");
        }
        if !main_branch(root) {
            return finish(&mut store, "branch-changed");
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
        register_triager_session(&mut store, &actor, &snapshot, root, self.clock.wall())?;
        let authorized_paths = safe_document_paths(root, &findings)?;
        if !authorized_paths.is_empty() {
            let paths = authorized_paths.iter().map(PathBuf::from).collect::<Vec<_>>();
            let outcome = match self.start_for(actor, &format!("triage findings {run_id}"), &paths, &[], root) {
                Ok(outcome) => outcome,
                Err(error) => {
                    record_reconcile_detail(run_dir, self.clock.wall(), "scope-failed", &error);
                    finish(&mut store, "scope-failed")?;
                    return Err(error);
                }
            };
            if outcome.kind != OutcomeKind::Ready {
                record_reconcile_detail(run_dir, self.clock.wall(), "scope-unavailable", &outcome.detail);
                return finish(&mut store, "scope-unavailable");
            }
        }
        let prompt = triage_prompt(run_id, &snapshot.start_head, &findings, &authorized_paths)?;
        {
            let mut metadata = lock_metadata(metadata);
            metadata.authorized_paths = authorized_paths;
            write_metadata(run_dir, &metadata)?;
        }
        write_private(&run_dir.join(SCHEMA_FILE), serde_json::to_vec_pretty(&result_schema())?.as_slice())?;
        write_private(&run_dir.join("prompt.txt"), prompt.as_bytes())?;

        {
            let mut metadata = lock_metadata(metadata);
            metadata.worktree_path = Some(worktree.path.clone());
            metadata.worktree_branch = Some(worktree.branch.clone());
            write_metadata(run_dir, &metadata)?;
        }
        if let Err(error) = worktree.create(&snapshot.start_head) {
            record_reconcile_detail(run_dir, self.clock.wall(), "runner-failed", &error);
            finish(&mut store, "runner-failed")?;
            return Err(error);
        }
        // The Codex deadline shares the ledger start with `run_is_live`, so setup time counts against it.
        let remaining = run.started_at + RUN_DEADLINE_SECONDS - self.clock.wall();
        let request = TriageRequest {
            worktree: &worktree.path,
            state_dir,
            run_dir,
            prompt: &prompt,
            deadline: Duration::from_secs_f64(remaining.max(0.0)),
        };
        let mut heartbeat = || {
            if !self.store()?.renew_triage_claims(run_id, self.clock.wall())? {
                return Err(AppError::operational("triage run closed while worker was active"));
            }
            Ok(())
        };
        let execution = runner.run(&request, &mut heartbeat);
        let current = self.clock.wall();
        let snapshot = lock_metadata(metadata).clone();
        let reconciled = match reconcile_artifacts(self, run, &snapshot, root) {
            Ok(reconciled) => reconciled,
            Err(error) => {
                record_reconcile_detail(run_dir, current, "reconcile-failed", &error);
                Reconciliation {
                    resolved: HashSet::new(),
                    admission_failed: snapshot.finding_ids.iter().cloned().collect(),
                }
            }
        };
        let (outcome, failure_detail) = match execution {
            Err(error) => ("runner-failed", Some(error.to_string())),
            Ok(status) if !status.success() => {
                ("runner-failed", Some(format!("triage runner exited unsuccessfully: {status}")))
            }
            Ok(_) => match apply_result_file(self, run, &snapshot, &pending_ids, root, run_dir, &reconciled) {
                Ok(true) => ("completed", None),
                Ok(false) => ("partial", Some("triage result did not resolve every prompted finding".to_owned())),
                Err(error) => ("invalid-result", Some(error.to_string())),
            },
        };
        if let Some(detail) = failure_detail {
            record_reconcile_detail(run_dir, current, outcome, &detail);
        }
        store = self.store()?;
        finish_worker(&mut store, run_dir, &mut lock_metadata(metadata), outcome, current)
    }
}

struct WorkerContext<'a> {
    run: &'a TriageRun,
    root: &'a Path,
    state_dir: &'a Path,
    run_dir: &'a Path,
    metadata: &'a Mutex<RunMetadata>,
}

struct LaunchRequest<'a> {
    root: &'a Path,
    state_dir: &'a Path,
    run_dir: &'a Path,
    run: &'a TriageRun,
}

/// Create the run directory and metadata, then spawn the detached worker.
/// Errors carry the outcome under which the scheduler finishes the run.
fn launch_worker(
    launch: &LaunchRequest<'_>,
    finding_ids: Vec<String>,
    clock: &dyn Clock,
    launcher: &dyn DetachedProcessRunner,
) -> std::result::Result<(), (&'static str, AppError)> {
    let failed = |error: AppError| ("launch-failed", error);
    let LaunchRequest { root, state_dir, run_dir, run } = *launch;
    fs::create_dir(run_dir).map_err(|error| failed(error.into()))?;
    let start_head = git_head_oid(root)
        .ok_or_else(|| failed(AppError::operational("finding triage requires a current Git HEAD")))?;
    let mut metadata = RunMetadata {
        run_id: run.id.clone(),
        repo_root: run.repo_root.clone(),
        state_dir: path_text(state_dir).map_err(failed)?,
        start_head,
        worktree_path: None,
        worktree_branch: None,
        finding_ids,
        authorized_paths: Vec::new(),
        started_at: run.started_at,
        heartbeat_at: clock.wall(),
        finished_at: None,
        worker: None,
    };
    write_metadata(run_dir, &metadata).map_err(failed)?;
    let executable = std::env::current_exe()
        .map_err(|error| failed(AppError::operational(format!("could not locate ai-coord executable: {error}"))))?;
    let spec = DetachedProcessSpec {
        program: executable,
        args: vec![
            OsString::from("triage-worker"),
            OsString::from("--run-id"),
            OsString::from(&run.id),
            OsString::from("--repo"),
            root.as_os_str().to_owned(),
        ],
        current_dir: root.to_owned(),
        environment: vec![
            (OsString::from("AI_COORD_STATE_DIR"), state_dir.as_os_str().to_owned()),
            (OsString::from("AI_COORD_TRIAGE_RUN_ID"), OsString::from(&run.id)),
            (OsString::from("AI_COORD_TRIAGE_ROLE"), OsString::from("triager")),
            (OsString::from("AI_COORD_CLIENT"), OsString::from("codex")),
            (OsString::from("AI_COORD_SESSION_ID"), OsString::from(format!("triage:{}", run.id))),
        ],
        stdout_path: run_dir.join("worker.stdout.log"),
        stderr_path: run_dir.join("worker.stderr.log"),
    };
    metadata.worker = Some(launcher.spawn(&spec).map_err(failed)?);
    metadata.heartbeat_at = clock.wall();
    write_metadata(run_dir, &metadata).map_err(|error| ("launch-metadata-failed", error))
}

impl TriageRunner for CodexTriageRunner {
    fn run(&self, request: &TriageRequest<'_>, heartbeat: &mut dyn FnMut() -> Result<()>) -> Result<ExitStatus> {
        let stdout = private_output(&request.run_dir.join(STDOUT_FILE))?;
        let stderr = private_output(&request.run_dir.join(STDERR_FILE))?;
        let args = codex_args(request.worktree, request.state_dir, request.run_dir);
        let mut command = Command::new("codex");
        command
            .args(args)
            .current_dir(request.worktree)
            .env("AI_COORD_TRIAGE_ROLE", "triager")
            .stdin(Stdio::piped())
            .stdout(stdout)
            .stderr(stderr);
        configure_triage_process_group(&mut command);
        let child = command
            .spawn()
            .map_err(|error| AppError::operational(format!("could not launch Codex triager: {error}")))?;
        run_triage_child(child, request.prompt, request.deadline, heartbeat)
    }
}

#[cfg(unix)]
fn configure_triage_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;

    command.process_group(0);
}

#[cfg(not(unix))]
fn configure_triage_process_group(_: &mut Command) {}

fn run_triage_child(
    child: Child,
    prompt: &str,
    deadline: Duration,
    heartbeat: &mut dyn FnMut() -> Result<()>,
) -> Result<ExitStatus> {
    run_triage_child_with_limits(child, prompt, heartbeat, deadline, Duration::from_secs_f64(HEARTBEAT_SECONDS))
}

fn run_triage_child_with_limits(
    mut child: Child,
    prompt: &str,
    heartbeat: &mut dyn FnMut() -> Result<()>,
    deadline: Duration,
    heartbeat_interval: Duration,
) -> Result<ExitStatus> {
    let started = Instant::now();
    let result = (|| {
        let mut stdin = child.stdin.take().ok_or_else(|| AppError::operational("Codex triager stdin unavailable"))?;
        let prompt = prompt.as_bytes().to_vec();
        let (writer, writer_result) = mpsc::sync_channel(1);
        thread::spawn(move || {
            let result = stdin.write_all(&prompt);
            drop(stdin);
            let _ = writer.send(result);
        });
        let mut prompt_delivered = false;
        loop {
            if !prompt_delivered {
                match writer_result.try_recv() {
                    Ok(Ok(())) => prompt_delivered = true,
                    Err(mpsc::TryRecvError::Empty) => {}
                    Ok(Err(error)) => return Err(error.into()),
                    Err(mpsc::TryRecvError::Disconnected) => {
                        return Err(AppError::operational("Codex triager prompt writer stopped unexpectedly"));
                    }
                }
            }
            if let Some(status) = child.try_wait()? {
                return Ok(status);
            }
            let elapsed = started.elapsed();
            if elapsed >= deadline {
                return Err(AppError::operational("Codex triage run exceeded the 30-minute deadline"));
            }
            heartbeat()?;
            thread::sleep(heartbeat_interval.min(deadline.saturating_sub(started.elapsed())));
        }
    })();
    if result.is_err() {
        terminate_triage_child(&mut child);
    }
    result
}

fn terminate_triage_child(child: &mut Child) {
    #[cfg(unix)]
    if let Ok(group_id) = i32::try_from(child.id()) {
        let _ = nix::sys::signal::kill(nix::unistd::Pid::from_raw(-group_id), nix::sys::signal::Signal::SIGKILL);
    }
    let _ = child.kill();
    let _ = child.wait();
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
        coordination_waived: None,
        fingerprint: Some(fingerprint),
        transcript_path: None,
        started_at: Some(metadata.started_at),
        current,
    })?;
    Ok(())
}

fn safe_document_paths(root: &Path, findings: &[FindingSummary]) -> Result<Vec<String>> {
    let mut paths = BTreeSet::new();
    for path in findings.iter().flat_map(|finding| &finding.paths) {
        if !safe_document_path(path) || !fs::symlink_metadata(root.join(path)).is_ok_and(|metadata| metadata.is_file())
        {
            continue;
        }
        if git_text(root, &["ls-tree", "--name-only", "HEAD", "--", path])?.lines().any(|tracked| tracked == path) {
            paths.insert(path.clone());
        }
    }
    Ok(paths.into_iter().collect())
}

fn apply_result_file(
    coordinator: &Coordinator,
    run: &TriageRun,
    metadata: &RunMetadata,
    prompted_ids: &[String],
    root: &Path,
    run_dir: &Path,
    reconciled: &Reconciliation,
) -> Result<bool> {
    if !main_branch(root) {
        return Err(AppError::operational("triage results can be applied only while main is checked out"));
    }
    let bytes = fs::read(run_dir.join(RESULT_FILE))?;
    let output: TriageOutput = serde_json::from_slice(&bytes)?;
    // Completeness covers only the findings the worker was asked about, not
    // claims another session resolved before the prompt was written.
    let prompted = prompted_ids.iter().collect::<HashSet<_>>();
    let statuses =
        output.results.iter().map(|result| (result.finding_id.clone(), result.status)).collect::<HashMap<_, _>>();
    let mut seen = HashSet::new();
    let mut complete = true;
    for result in output.results {
        if !prompted.contains(&result.finding_id) || !seen.insert(result.finding_id.clone()) {
            complete = false;
            continue;
        }
        if reconciled.admission_failed.contains(&result.finding_id) {
            complete = false;
            continue;
        }
        if reconciled.resolved.contains(&result.finding_id) {
            continue;
        }
        if result.status == ResultStatus::Deferred {
            complete = false;
        }
        if result.status == ResultStatus::Duplicate &&
            result.canonical_id.as_ref().and_then(|id| statuses.get(id)).copied() == Some(ResultStatus::Duplicate)
        {
            complete = false;
            continue;
        }
        if let Err(error) = apply_finding_result(coordinator, run, metadata, root, &result) {
            record_reconcile_detail(run_dir, coordinator.clock.wall(), "partial", &error);
            complete = false;
        }
    }
    if seen.len() != prompted.len() {
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
            copy_handoff(metadata.worktree_path.as_deref(), root, &result.finding_id)?;
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

#[derive(Default)]
struct Reconciliation {
    resolved: HashSet<String>,
    admission_failed: HashSet<String>,
}

fn reconcile_artifacts(
    coordinator: &Coordinator,
    run: &TriageRun,
    metadata: &RunMetadata,
    root: &Path,
) -> Result<Reconciliation> {
    let mut reconciled = Reconciliation::default();
    if let Some(worktree) = metadata.worktree_path.as_deref() {
        reconciled.admission_failed = admit_commits(
            root,
            worktree,
            &metadata.start_head,
            &metadata.finding_ids,
            &metadata.authorized_paths,
            &peer_claimed_scopes(coordinator, run)?,
            coordinator.clock.wall(),
        )?;
    }
    if !main_branch(root) {
        return Ok(reconciled);
    }
    let actor = triager_identity(&run.id);
    let current = coordinator.clock.wall();
    let authorized = metadata.authorized_paths.iter().map(String::as_str).collect::<HashSet<_>>();
    for finding_id in &metadata.finding_ids {
        if reconciled.admission_failed.contains(finding_id) {
            continue;
        }
        let Some(finding) = coordinator.store()?.finding(&run.repo_root, finding_id, current)? else {
            continue;
        };
        if finding.state != FindingState::Pending {
            reconciled.resolved.insert(finding_id.clone());
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
            reconciled.resolved.insert(finding_id.clone());
            continue;
        }
        let handoff = deterministic_handoff(finding_id);
        if copy_handoff(metadata.worktree_path.as_deref(), root, finding_id).is_ok() &&
            validate_handoff(root, finding_id, &handoff).is_ok() &&
            coordinator.store()?.handoff_finding(&run.repo_root, finding_id, &handoff, &actor, current).is_ok()
        {
            reconciled.resolved.insert(finding_id.clone());
        }
    }
    Ok(reconciled)
}

/// Scopes of every other session's active work in the run's repository. A
/// dead worker's own claim may already be gone, so admission must not
/// fast-forward over a path a peer has since been granted.
fn peer_claimed_scopes(coordinator: &Coordinator, run: &TriageRun) -> Result<Vec<Scope>> {
    let actor = triager_identity(&run.id);
    Ok(coordinator
        .store()?
        .works_in_repo(&run.repo_root)?
        .into_iter()
        .filter(|work| work.state == WorkState::Active && work.identity != actor)
        .filter_map(|work| work.claim(&run.repo_root).map(|claim| claim.scopes.clone()))
        .flatten()
        .collect())
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

#[cfg(test)]
#[path = "triage_tests.rs"]
mod tests;
