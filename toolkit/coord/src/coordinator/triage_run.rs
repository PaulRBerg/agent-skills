//! Triage run metadata, liveness, lifecycle bookkeeping, and log retention.

use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Mutex, MutexGuard, mpsc},
    thread,
    time::Duration,
};

use serde::{Deserialize, Serialize};

use crate::{
    domain::{Client, Identity, ProcessFingerprint, ProcessLiveness, ProcessProbe},
    error::{AppError, Result},
    state::{Store, TriageRun},
};

use super::{Clock, Coordinator};

pub(super) const RUN_DEADLINE_SECONDS: f64 = 30.0 * 60.0;
/// A run stops counting as live one minute after its deadline, leaving the
/// worker time to reconcile and finish after the deadline stops Codex.
pub(super) const RUN_EXPIRY_SECONDS: f64 = RUN_DEADLINE_SECONDS + 60.0;
pub(super) const HEARTBEAT_SECONDS: f64 = 2.0;
pub(super) const HEARTBEAT_GRACE_SECONDS: f64 = 15.0;
const LOG_RETENTION_SECONDS: f64 = 30.0 * 24.0 * 60.0 * 60.0;
pub(super) const RECONCILE_LOG_FILE: &str = "reconcile.log";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct RunMetadata {
    pub(super) run_id: String,
    pub(super) repo_root: String,
    pub(super) state_dir: String,
    pub(super) start_head: String,
    #[serde(default)]
    pub(super) worktree_path: Option<PathBuf>,
    #[serde(default)]
    pub(super) worktree_branch: Option<String>,
    pub(super) finding_ids: Vec<String>,
    #[serde(default)]
    pub(super) authorized_paths: Vec<String>,
    pub(super) started_at: f64,
    pub(super) heartbeat_at: f64,
    pub(super) finished_at: Option<f64>,
    pub(super) worker: Option<ProcessFingerprint>,
}

/// Liveness shared by every reconciler. The limit is measured from the ledger
/// start, the same origin the worker uses for its Codex deadline.
pub(super) fn run_is_live(
    run: &TriageRun,
    metadata: Option<&RunMetadata>,
    probe: &dyn ProcessProbe,
    current: f64,
) -> bool {
    if current - run.started_at >= RUN_EXPIRY_SECONDS {
        return false;
    }
    let Some(metadata) = metadata else {
        // The scheduler records the ledger row before it writes run metadata.
        return current - run.started_at <= HEARTBEAT_GRACE_SECONDS;
    };
    if metadata.run_id != run.id || current - metadata.heartbeat_at > HEARTBEAT_GRACE_SECONDS {
        return false;
    }
    // No fingerprint yet means the scheduler is still spawning the worker.
    metadata.worker.as_ref().is_none_or(|fingerprint| probe.liveness(fingerprint) != ProcessLiveness::Dead)
}

pub(super) fn lock_metadata(metadata: &Mutex<RunMetadata>) -> MutexGuard<'_, RunMetadata> {
    metadata.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Run `body` while a background thread keeps the metadata heartbeat fresh,
/// starting immediately so slow setup (provider probes, worktree hooks) never
/// looks like a lost worker.
pub(super) fn with_heartbeat<T>(
    clock: &dyn Clock,
    run_dir: &Path,
    metadata: &Mutex<RunMetadata>,
    body: impl FnOnce() -> T,
) -> T {
    thread::scope(|scope| {
        // Dropping the sender, including during unwinding, stops the thread.
        let (_stop, stopped) = mpsc::channel::<()>();
        scope.spawn(move || {
            loop {
                let mut metadata = lock_metadata(metadata);
                if metadata.finished_at.is_none() {
                    metadata.heartbeat_at = clock.wall();
                    let _ = write_metadata(run_dir, &metadata);
                }
                drop(metadata);
                if !matches!(
                    stopped.recv_timeout(Duration::from_secs_f64(HEARTBEAT_SECONDS)),
                    Err(mpsc::RecvTimeoutError::Timeout)
                ) {
                    break;
                }
            }
        });
        body()
    })
}

pub(super) fn finish_worker(
    store: &mut Store,
    run_dir: &Path,
    metadata: &mut RunMetadata,
    outcome: &str,
    current: f64,
) -> Result<()> {
    metadata.finished_at = Some(current);
    metadata.heartbeat_at = current;
    if let Err(error) = write_metadata(run_dir, metadata) {
        record_reconcile_detail(run_dir, current, outcome, &error);
        return Err(error);
    }
    if let Err(error) = store.end_session(&triager_identity(&metadata.run_id)) {
        record_reconcile_detail(run_dir, current, outcome, &error);
        return Err(error);
    }
    if let Err(error) = store.finish_triage_run(&metadata.run_id, outcome, current) {
        record_reconcile_detail(run_dir, current, outcome, &error);
        return Err(error);
    }
    Ok(())
}

pub(super) fn finish_failed_schedule_run(
    store: &mut Store,
    run_id: &str,
    run_dir: &Path,
    outcome: &str,
    current: f64,
    cause: &dyn std::fmt::Display,
) {
    let finish_error = store.finish_triage_run(run_id, outcome, current).err();
    record_reconcile_detail(run_dir, current, outcome, cause);
    if let Some(error) = finish_error {
        record_reconcile_detail(run_dir, current, outcome, &error);
    }
}

pub(super) fn record_reconcile_detail(run_dir: &Path, current: f64, outcome: &str, error: &dyn std::fmt::Display) {
    let _ = append_reconcile_detail(run_dir, current, outcome, error);
}

fn append_reconcile_detail(
    run_dir: &Path,
    current: f64,
    outcome: &str,
    error: &dyn std::fmt::Display,
) -> io::Result<()> {
    let detail = error.to_string().split_whitespace().collect::<Vec<_>>().join(" ");
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(run_dir.join(RECONCILE_LOG_FILE))?;
    let line = format!("{current:.6}\t{outcome}\t{detail}\n");
    file.write_all(line.as_bytes())?;
    file.sync_all()
}

pub(super) fn triager_identity(run_id: &str) -> Identity {
    Identity { client: Client::Codex, session_id: format!("triage:{run_id}") }
}

fn metadata_path(run_dir: &Path) -> PathBuf {
    run_dir.join("run.json")
}

pub(super) fn read_metadata(run_dir: &Path) -> Result<RunMetadata> {
    Ok(serde_json::from_slice(&fs::read(metadata_path(run_dir))?)?)
}

pub(super) fn read_worker_metadata(run_dir: &Path) -> Result<RunMetadata> {
    for _ in 0..40 {
        let metadata = read_metadata(run_dir)?;
        if metadata.worker.is_some() {
            return Ok(metadata);
        }
        thread::sleep(Duration::from_millis(50));
    }
    Err(AppError::operational("triage worker process evidence was not recorded"))
}

pub(super) fn write_metadata(run_dir: &Path, metadata: &RunMetadata) -> Result<()> {
    let temporary = tempfile::NamedTempFile::new_in(run_dir)?;
    write_private(temporary.path(), serde_json::to_vec_pretty(metadata)?.as_slice())?;
    temporary.persist(metadata_path(run_dir)).map_err(|error| error.error)?;
    Ok(())
}

pub(super) fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
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

pub(super) fn private_output(path: &Path) -> Result<Stdio> {
    let mut options = OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    Ok(Stdio::from(options.open(path)?))
}

/// Remove run directories past retention, except runs the ledger still holds
/// open: their worktree and metadata are needed by a later reconcile.
pub(super) fn prune_run_logs(coordinator: &Coordinator, run_root: &Path, current: f64) -> Result<()> {
    let entries = match fs::read_dir(run_root) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        entries => entries?,
    };
    for entry in entries {
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
        if !age_basis.is_some_and(|timestamp| current - timestamp > LOG_RETENTION_SECONDS) {
            continue;
        }
        if let Some(run_id) = entry.file_name().to_str() &&
            coordinator.store()?.triage_run(run_id)?.is_some_and(|run| run.finished_at.is_none())
        {
            continue;
        }
        fs::remove_dir_all(entry.path())?;
    }
    Ok(())
}
