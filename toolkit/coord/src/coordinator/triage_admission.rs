use std::{collections::HashSet, path::Path};
#[cfg(unix)]
use std::{
    io::{Read, Seek, Write},
    os::unix::process::CommandExt,
    process::{Child, Command, Stdio},
};

use crate::{
    domain::{Client, Identity, ProcessFingerprint, Scope, ScopeKind, SessionState, WorkState},
    error::{AppError, Result},
    host::scope_covers,
    state::{SessionUpdate, Store, TriageRun, WorkClaimUpdate, WorkUpdate},
};

use super::{
    Coordinator,
    triage_run::{record_reconcile_detail, triager_identity},
};

/// Reserve paths atomically, then retain them through Git without holding a ledger transaction across hooks.
pub(super) struct AdmissionReservation {
    store: Store,
    identity: Identity,
    repo_root: String,
    run_dir: std::path::PathBuf,
    current: f64,
}

impl AdmissionReservation {
    pub(super) fn acquire(
        coordinator: &Coordinator,
        run: &TriageRun,
        changed: &HashSet<String>,
        worktree: &Path,
        current: f64,
    ) -> Result<Self> {
        let mut reservation = Self {
            store: coordinator.store()?,
            identity: Identity {
                client: Client::Codex,
                session_id: format!("triage-admission:{}:{}", run.id, hex::encode(rand::random::<[u8; 16]>())),
            },
            repo_root: run.repo_root.clone(),
            run_dir: worktree.parent().expect("worktree is under run directory").to_owned(),
            current,
        };
        reservation.bind_process(coordinator.probe.fingerprint(std::process::id())?)?;
        let actor = triager_identity(&run.id);
        let scopes =
            changed.iter().map(|path| Scope { path: path.clone(), kind: ScopeKind::Exact }).collect::<Vec<_>>();
        reservation.store.with_work_transaction(|transaction| {
            let works = transaction.works()?;
            let worker_scopes = works
                .iter()
                .find(|work| work.identity == actor && work.state == WorkState::Active)
                .and_then(|work| work.claim(&run.repo_root))
                .map(|claim| claim.scopes.as_slice())
                .unwrap_or_default();
            for peer in works.iter().filter(|work| work.identity != actor && work.identity != reservation.identity) {
                let Some(claim) = peer.claim(&run.repo_root) else {
                    continue;
                };
                for scope in &scopes {
                    if !claim.scopes.iter().any(|owned| scope_covers(owned, scope)) {
                        continue;
                    }
                    if peer.state == WorkState::Active {
                        return Err(AppError::operational(
                            "triage commit changes a path claimed by another session's work",
                        ));
                    }
                    if !worker_scopes.iter().any(|owned| scope_covers(owned, scope)) {
                        return Err(AppError::operational(
                            "triage commit changes a path queued by another session's work",
                        ));
                    }
                }
            }
            transaction.save_work(&WorkUpdate {
                identity: reservation.identity.clone(),
                label: format!("triage admission {}", run.id),
                state: WorkState::Active,
                blocked_reason: None,
                claims: vec![WorkClaimUpdate {
                    repo_root: run.repo_root.clone(),
                    blocked_reason: None,
                    scopes,
                    baselines: None,
                }],
                submitted_at: Some(current),
                updated_at: current,
                expected_revision: None,
            })?;
            Ok(())
        })?;
        Ok(reservation)
    }

    fn bind_process(&mut self, fingerprint: ProcessFingerprint) -> Result<()> {
        self.store.upsert_session(&SessionUpdate {
            identity: self.identity.clone(),
            cwd: self.repo_root.clone(),
            repo_root: Some(self.repo_root.clone()),
            state: SessionState::Working,
            source: "triage-admission".to_owned(),
            name: None,
            waiting_for: None,
            permission_mode: None,
            update_permission_mode: false,
            coordination_waived: None,
            fingerprint: Some(fingerprint),
            transcript_path: None,
            started_at: Some(self.current),
            current: self.current,
        })?;
        Ok(())
    }

    #[cfg(unix)]
    pub(super) fn merge(mut self, coordinator: &Coordinator, root: &Path, oid: &str) -> Result<()> {
        // File-backed stderr remains writable if the reconciler dies while Git runs.
        let mut stderr = tempfile::tempfile()?;
        let mut child = AdmissionChild {
            child: Command::new("/bin/sh")
                .args(["-c", "IFS= read -r gate && exec \"$@\"", "--", "git", "-C"])
                .arg(root)
                .args(["merge", "--ff-only", oid])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::from(stderr.try_clone()?))
                .process_group(0)
                .spawn()?,
            reaped: false,
        };
        // The pipe closes on parent death. Git cannot start before its own fingerprint protects the reservation.
        self.bind_process(coordinator.probe.fingerprint(child.child.id())?)?;
        let mut gate = child.child.stdin.take().expect("admission child has piped stdin");
        gate.write_all(b"merge\n")?;
        drop(gate);
        let status = child.child.wait()?;
        child.reaped = true;
        stderr.rewind()?;
        let mut error_output = Vec::new();
        stderr.read_to_end(&mut error_output)?;
        if !status.success() {
            return Err(AppError::operational(format!(
                "Git artifact validation failed: {}",
                String::from_utf8_lossy(&error_output).trim()
            )));
        }
        Ok(())
    }

    #[cfg(not(unix))]
    pub(super) fn merge(self, _: &Coordinator, _: &Path, _: &str) -> Result<()> {
        Err(AppError::operational("triage commit admission requires a Unix process gate"))
    }
}

#[cfg(unix)]
struct AdmissionChild {
    child: Child,
    reaped: bool,
}

#[cfg(unix)]
impl Drop for AdmissionChild {
    fn drop(&mut self) {
        if !self.reaped {
            if let Ok(group) = i32::try_from(self.child.id()) {
                let _ = nix::sys::signal::kill(nix::unistd::Pid::from_raw(-group), nix::sys::signal::Signal::SIGKILL);
            }
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

impl Drop for AdmissionReservation {
    fn drop(&mut self) {
        if let Err(error) = self.store.end_session(&self.identity) {
            record_reconcile_detail(&self.run_dir, self.current, "admission-cleanup-failed", &error);
        }
    }
}
