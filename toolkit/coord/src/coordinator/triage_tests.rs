use std::{
    os::unix::{fs::PermissionsExt, process::ExitStatusExt},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{Value, json};
use tempfile::TempDir;

use crate::{
    coordinator::{Clock, inventory::StaticInventory},
    domain::{Client, FindingKind, ProcessFingerprint, ProcessLiveness, ProcessProbe, WorkState},
    host::NativeProcessProbe,
    state::{FindingAdd, FindingPathObservation},
};

use super::{
    super::triage_run::{HEARTBEAT_GRACE_SECONDS, RECONCILE_LOG_FILE, RUN_EXPIRY_SECONDS},
    *,
};

const CONFIG_PATH: &str = ".agents/coord.toml";

#[derive(Default)]
struct FakeLauncher {
    specs: Mutex<Vec<DetachedProcessSpec>>,
}

impl DetachedProcessRunner for FakeLauncher {
    fn spawn(&self, spec: &DetachedProcessSpec) -> Result<ProcessFingerprint> {
        self.specs.lock().unwrap().push(spec.clone());
        NativeProcessProbe::new().fingerprint(std::process::id())
    }
}

struct FailingLauncher;

impl DetachedProcessRunner for FailingLauncher {
    fn spawn(&self, _: &DetachedProcessSpec) -> Result<ProcessFingerprint> {
        Err(AppError::operational("simulated launch failure"))
    }
}

struct FakeClock(Mutex<f64>);
impl FakeClock {
    fn at(now: f64) -> Arc<Self> {
        Arc::new(Self(Mutex::new(now)))
    }
    fn set(&self, now: f64) {
        *self.0.lock().unwrap() = now;
    }
}
impl Clock for FakeClock {
    fn wall(&self) -> f64 {
        *self.0.lock().unwrap()
    }
    fn monotonic(&self) -> f64 {
        self.wall()
    }
    fn sleep(&self, _: Duration) {}
}

struct FakeRunner {
    result: Value,
}
impl TriageRunner for FakeRunner {
    fn run(&self, request: &TriageRequest<'_>, heartbeat: &mut dyn FnMut() -> Result<()>) -> Result<ExitStatus> {
        heartbeat()?;
        assert_isolated_worktree(request);
        let metadata = read_metadata(request.run_dir)?;
        let store = Store::open(request.state_dir.join("state.db"))?;
        let work = store.work(&triager_identity(&metadata.run_id))?;
        if metadata.authorized_paths.is_empty() {
            assert!(work.is_none());
        } else {
            let work = work.expect("safe-document triage owns an exact scope before the model runs");
            assert_eq!(work.state, WorkState::Active);
            assert_eq!(work.claims.len(), 1);
            let claim = work.claims.into_iter().next().expect("triage work has one repository claim");
            assert!(claim.scopes.iter().all(|scope| !scope.is_recursive()));
            assert_eq!(
                claim.scopes.into_iter().map(|scope| scope.path).collect::<BTreeSet<_>>(),
                metadata.authorized_paths.into_iter().collect()
            );
        }
        write_private(&request.run_dir.join(RESULT_FILE), serde_json::to_vec(&self.result)?.as_slice())?;
        Ok(ExitStatus::from_raw(0))
    }
}

struct FailingRunner;
impl TriageRunner for FailingRunner {
    fn run(&self, request: &TriageRequest<'_>, heartbeat: &mut dyn FnMut() -> Result<()>) -> Result<ExitStatus> {
        heartbeat()?;
        assert_isolated_worktree(request);
        Err(AppError::operational("simulated deadline"))
    }
}

fn repository(auto_triage: bool) -> TempDir {
    let temp = tempfile::tempdir().unwrap();
    assert!(
        Command::new("git").args(["init", "-q", "-b", "main"]).current_dir(temp.path()).status().unwrap().success()
    );
    fs::create_dir_all(temp.path().join(CONFIG_PATH).parent().unwrap()).unwrap();
    fs::write(temp.path().join(CONFIG_PATH), format!("[findings]\nauto_triage = {auto_triage}\n")).unwrap();
    fs::write(temp.path().join("README.md"), "old prose\n").unwrap();
    fs::write(temp.path().join("NOTES.md"), "other prose\n").unwrap();
    fs::create_dir(temp.path().join("src")).unwrap();
    fs::write(temp.path().join("src/lib.rs"), "pub fn value() -> u8 { 1 }\n").unwrap();
    assert!(Command::new("git").args(["add", "."]).current_dir(temp.path()).status().unwrap().success());
    assert!(
        Command::new("git")
            .args(["-c", "user.name=test", "-c", "user.email=test@invalid", "commit", "-qm", "base"])
            .current_dir(temp.path())
            .status()
            .unwrap()
            .success()
    );
    temp
}

fn fixture(repo: &Path, now: f64) -> (Coordinator, Identity) {
    fixture_with_coverage(repo, now, true)
}

fn fixture_with_coverage(repo: &Path, now: f64, complete: bool) -> (Coordinator, Identity) {
    let (coordinator, origin, _) = clocked_fixture(repo, now, complete);
    (coordinator, origin)
}

fn clocked_fixture(repo: &Path, now: f64, complete: bool) -> (Coordinator, Identity, Arc<FakeClock>) {
    let state = repo.join("state");
    let store = Store::open(state.join("state.db")).unwrap();
    let clock = FakeClock::at(now);
    let coordinator = Coordinator::with_components(
        store,
        Box::new(StaticInventory { complete, refreshes: Default::default() }),
        std::sync::Arc::new(NativeProcessProbe::new()),
        clock.clone(),
    );
    (coordinator, Identity { client: Client::Codex, session_id: "origin".to_owned() }, clock)
}

fn add_finding(coordinator: &Coordinator, repo: &Path, summary: &str, current: f64) -> String {
    add_finding_at(coordinator, repo, summary, "README.md", FindingKind::Docs, current)
}

fn add_finding_at(
    coordinator: &Coordinator,
    repo: &Path,
    summary: &str,
    finding_path: &str,
    kind: FindingKind,
    current: f64,
) -> String {
    let repo = crate::host::git_root(repo).unwrap();
    coordinator
        .store()
        .unwrap()
        .add_finding(&FindingAdd {
            repo_root: path_text(&repo).unwrap(),
            summary: summary.to_owned(),
            kind: Some(kind),
            paths: vec![finding_path.to_owned()],
            head_oid: git_head_oid(&repo),
            observations: vec![FindingPathObservation { path: finding_path.to_owned(), content_sha256: None }],
            author: Identity { client: Client::Codex, session_id: "source".to_owned() },
            turn_id: None,
            current,
        })
        .unwrap()
        .finding
        .id
}

#[test]
fn exact_opt_in_branch_and_work_guards_control_launch() {
    let repo = repository(true);
    let (coordinator, origin) = fixture(repo.path(), 100.0);
    add_finding(&coordinator, repo.path(), "stale prose", 1.0);
    let launcher = FakeLauncher::default();
    let TriageSchedule::Launched { run_id, finding_count } =
        coordinator.schedule_findings_triage_for(repo.path(), &origin, &launcher).unwrap()
    else {
        panic!("expected launch")
    };
    assert_eq!(finding_count, 1);
    let specs = launcher.specs.lock().unwrap();
    assert_eq!(specs.len(), 1);
    assert!(specs[0].environment.contains(&(OsString::from("AI_COORD_CLIENT"), OsString::from("codex"))));
    assert!(
        specs[0]
            .environment
            .contains(&(OsString::from("AI_COORD_SESSION_ID"), OsString::from(format!("triage:{run_id}"))))
    );
    drop(specs);

    let other = repository(false);
    let (disabled, disabled_origin) = fixture(other.path(), 100.0);
    add_finding(&disabled, other.path(), "pending", 1.0);
    assert_eq!(
        disabled.schedule_findings_triage_for(other.path(), &disabled_origin, &launcher).unwrap(),
        TriageSchedule::Skipped("disabled")
    );
}

#[test]
fn incomplete_coverage_does_not_create_or_launch_a_run() {
    let repo = repository(true);
    let (coordinator, origin) = fixture_with_coverage(repo.path(), 100.0, false);
    add_finding(&coordinator, repo.path(), "stale prose", 1.0);
    let launcher = FakeLauncher::default();
    assert_eq!(
        coordinator.schedule_findings_triage_for(repo.path(), &origin, &launcher).unwrap(),
        TriageSchedule::Skipped("coverage")
    );
    assert!(launcher.specs.lock().unwrap().is_empty());
    let root = path_text(&crate::host::git_root(repo.path()).unwrap()).unwrap();
    assert!(coordinator.store().unwrap().active_triage_runs(&root).unwrap().is_empty());
}

#[test]
fn ineligible_repository_never_reaches_the_inventory_refresh() {
    let repo = repository(true);
    let state = repo.path().join("state");
    let store = Store::open(state.join("state.db")).unwrap();
    let refreshes = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let coordinator = Coordinator::with_components(
        store,
        Box::new(StaticInventory { complete: true, refreshes: refreshes.clone() }),
        std::sync::Arc::new(NativeProcessProbe::new()),
        FakeClock::at(100.0),
    );
    let origin = Identity { client: Client::Codex, session_id: "origin".to_owned() };
    // No finding was ever recorded, so the cheap SQL precheck rules this
    // repository out before the expensive provider probe would otherwise run.
    let launcher = FakeLauncher::default();
    assert_eq!(
        coordinator.schedule_findings_triage_for(repo.path(), &origin, &launcher).unwrap(),
        TriageSchedule::Skipped("ineligible")
    );
    assert_eq!(refreshes.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert!(launcher.specs.lock().unwrap().is_empty());
}

#[test]
fn launch_failure_writes_the_specific_reconcile_detail() {
    let repo = repository(true);
    let (coordinator, origin) = fixture(repo.path(), 100.0);
    add_finding(&coordinator, repo.path(), "stale prose", 1.0);

    let error = coordinator.schedule_findings_triage_for(repo.path(), &origin, &FailingLauncher).unwrap_err();
    assert_eq!(error.to_string(), "simulated launch failure");

    let run_root = repo.path().join("state/triage-runs");
    let run_dirs = fs::read_dir(&run_root).unwrap().collect::<std::io::Result<Vec<_>>>().unwrap();
    assert_eq!(run_dirs.len(), 1);
    let run_id = run_dirs[0].file_name().into_string().unwrap();
    let detail = fs::read_to_string(run_dirs[0].path().join(RECONCILE_LOG_FILE)).unwrap();
    assert_eq!(detail, "100.000000\tlaunch-failed\tsimulated launch failure\n");
    assert_eq!(
        coordinator.store().unwrap().triage_run(&run_id).unwrap().unwrap().outcome.as_deref(),
        Some("launch-failed")
    );
}

#[test]
fn codex_command_is_ephemeral_sandboxed_offline_and_agentless() {
    let worktree = Path::new("/state/triage-runs/a/worktree");
    let state = Path::new("/state");
    let run = Path::new("/state/triage-runs/a");
    let request = TriageRequest {
        worktree,
        state_dir: state,
        run_dir: run,
        prompt: "prompt",
        deadline: Duration::from_secs_f64(RUN_DEADLINE_SECONDS),
    };
    let args = codex_args(request.worktree, request.state_dir, request.run_dir)
        .into_iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    for expected in [
        "gpt-5.6-luna",
        "model_reasoning_effort=\"xhigh\"",
        "/state",
        "sandbox_workspace_write.network_access=false",
        "web_search=\"disabled\"",
        "agents.enabled=false",
        "--approve-for-me",
        "--ephemeral",
        "--ignore-user-config",
        "--output-schema",
        "--output-last-message",
    ] {
        assert!(args.iter().any(|arg| arg == expected), "missing {expected}: {args:?}");
    }
    assert!(args.windows(2).any(|args| args == ["-C", "/state/triage-runs/a/worktree"]));
    assert!(args.windows(2).any(|args| args == ["--add-dir", "/state"]));
    assert!(!args.iter().any(|arg| arg == "--sandbox"), "--approve-for-me selects workspace-write: {args:?}");
}

#[test]
fn structured_handoff_is_validated_and_reconciled() {
    let repo = repository(true);
    let (coordinator, origin) = fixture(repo.path(), 100.0);
    let finding_id = add_finding(&coordinator, repo.path(), "needs broad work", 1.0);
    let launcher = FakeLauncher::default();
    let TriageSchedule::Launched { run_id, .. } =
        coordinator.schedule_findings_triage_for(repo.path(), &origin, &launcher).unwrap()
    else {
        panic!()
    };
    let handoff = deterministic_handoff(&finding_id);
    fs::create_dir_all(repo.path().join(".ai/task-handoffs")).unwrap();
    fs::write(repo.path().join(&handoff), format!("# Handoff\n\nSource finding: {finding_id}\n")).unwrap();
    let runner = FakeRunner {
        result: json!({ "results": [{
        "finding_id": finding_id, "status": "handed_off", "evidence": "verified broad scope",
        "changed_paths": [handoff], "validation": ["marker checked"], "commit_oid": null,
        "canonical_id": null, "handoff_path": handoff
    }] }),
    };
    coordinator.run_findings_triage_with(&run_id, repo.path(), &runner).unwrap();
    let finding = coordinator
        .store()
        .unwrap()
        .finding(&path_text(&crate::host::git_root(repo.path()).unwrap()).unwrap(), &finding_id, 101.0)
        .unwrap()
        .unwrap();
    assert_eq!(finding.state, FindingState::HandedOff);
}

#[test]
fn code_only_batch_launches_without_a_tracked_file_scope() {
    let repo = repository(true);
    let (coordinator, origin) = fixture(repo.path(), 100.0);
    let finding_id = add_finding_at(&coordinator, repo.path(), "code behavior", "src/lib.rs", FindingKind::Bug, 1.0);
    let launcher = FakeLauncher::default();
    let TriageSchedule::Launched { run_id, .. } =
        coordinator.schedule_findings_triage_for(repo.path(), &origin, &launcher).unwrap()
    else {
        panic!()
    };
    let runner = FakeRunner {
        result: json!({ "results": [{
            "finding_id": finding_id, "status": "deferred", "evidence": "verified code behavior",
            "changed_paths": [], "validation": [], "commit_oid": null,
            "canonical_id": null, "handoff_path": null
        }] }),
    };
    coordinator.run_findings_triage_with(&run_id, repo.path(), &runner).unwrap();
    let metadata = read_metadata(&repo.path().join("state/triage-runs").join(&run_id)).unwrap();
    assert!(metadata.authorized_paths.is_empty());
    let actor = triager_identity(&run_id);
    let store = coordinator.store().unwrap();
    assert!(store.work(&actor).unwrap().is_none());
    assert!(store.session(&actor).unwrap().is_none());
    assert_worktree_removed(repo.path(), &run_id);
    assert_eq!(store.triage_run(&run_id).unwrap().unwrap().outcome.as_deref(), Some("partial"));
    let root = path_text(&crate::host::git_root(repo.path()).unwrap()).unwrap();
    assert_eq!(store.finding(&root, &finding_id, 100.0).unwrap().unwrap().state, FindingState::Pending);
}

#[test]
fn runner_failure_finishes_run_and_releases_claims() {
    let repo = repository(true);
    let (coordinator, origin) = fixture(repo.path(), 100.0);
    add_finding(&coordinator, repo.path(), "retry later", 1.0);
    let launcher = FakeLauncher::default();
    let TriageSchedule::Launched { run_id, .. } =
        coordinator.schedule_findings_triage_for(repo.path(), &origin, &launcher).unwrap()
    else {
        panic!()
    };
    let actor = triager_identity(&run_id);
    coordinator.run_findings_triage_with(&run_id, repo.path(), &FailingRunner).unwrap();
    let prompt = fs::read_to_string(repo.path().join("state/triage-runs").join(&run_id).join("prompt.txt")).unwrap();
    assert!(prompt.contains("Authorized editable paths:\n[\n  \"README.md\"\n]"));
    assert!(prompt.contains("Do not run ai-coord lifecycle or status commands"));
    assert!(prompt.contains("$task-handoff"));
    assert!(prompt.contains("FINDING_<UPPERCASE_ID>.md` with its no-clipboard workflow"));
    let store = coordinator.store().unwrap();
    assert_worktree_removed(repo.path(), &run_id);
    assert_eq!(store.triage_run(&run_id).unwrap().unwrap().outcome.as_deref(), Some("runner-failed"));
    assert!(store.triage_claims(&run_id).unwrap().is_empty());
    assert!(store.work(&actor).unwrap().is_none());
    assert!(store.session(&actor).unwrap().is_none());
}

#[test]
fn commit_trailer_is_reconciled_before_retrying_runner() {
    let repo = repository(true);
    let (coordinator, origin) = fixture(repo.path(), 100.0);
    let finding_id = add_finding(&coordinator, repo.path(), "stale prose", 1.0);
    let launcher = FakeLauncher::default();
    let TriageSchedule::Launched { run_id, .. } =
        coordinator.schedule_findings_triage_for(repo.path(), &origin, &launcher).unwrap()
    else {
        panic!()
    };
    let run_dir = repo.path().join("state/triage-runs").join(&run_id);
    let mut metadata = read_metadata(&run_dir).unwrap();
    metadata.authorized_paths = vec!["README.md".to_owned()];
    write_metadata(&run_dir, &metadata).unwrap();
    fs::write(repo.path().join("README.md"), "current prose\n").unwrap();
    assert!(Command::new("git").args(["add", "README.md"]).current_dir(repo.path()).status().unwrap().success());
    assert!(
        Command::new("git")
            .args(["-c", "user.name=test", "-c", "user.email=test@invalid", "commit", "-qm"])
            .arg(format!("docs: refresh prose\n\nFinding-ID: {finding_id}"))
            .current_dir(repo.path())
            .status()
            .unwrap()
            .success()
    );
    coordinator.run_findings_triage_with(&run_id, repo.path(), &FailingRunner).unwrap();
    let store = coordinator.store().unwrap();
    assert_eq!(store.triage_run(&run_id).unwrap().unwrap().outcome.as_deref(), Some("reconciled"));
    let root = path_text(&crate::host::git_root(repo.path()).unwrap()).unwrap();
    assert_eq!(store.finding(&root, &finding_id, 101.0).unwrap().unwrap().state, FindingState::Fixed);
}

#[test]
fn commit_trailer_is_not_reconciled_after_branch_changes() {
    let repo = repository(true);
    let (coordinator, origin) = fixture(repo.path(), 100.0);
    let finding_id = add_finding(&coordinator, repo.path(), "stale prose", 1.0);
    let launcher = FakeLauncher::default();
    let TriageSchedule::Launched { run_id, .. } =
        coordinator.schedule_findings_triage_for(repo.path(), &origin, &launcher).unwrap()
    else {
        panic!()
    };
    let run_dir = repo.path().join("state/triage-runs").join(&run_id);
    let mut metadata = read_metadata(&run_dir).unwrap();
    metadata.authorized_paths = vec!["README.md".to_owned()];
    write_metadata(&run_dir, &metadata).unwrap();
    fs::write(repo.path().join("README.md"), "current prose\n").unwrap();
    assert!(Command::new("git").args(["add", "README.md"]).current_dir(repo.path()).status().unwrap().success());
    assert!(
        Command::new("git")
            .args(["-c", "user.name=test", "-c", "user.email=test@invalid", "commit", "-qm"])
            .arg(format!("docs: refresh prose\n\nFinding-ID: {finding_id}"))
            .current_dir(repo.path())
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new("git").args(["checkout", "-qb", "topic"]).current_dir(repo.path()).status().unwrap().success()
    );

    coordinator.run_findings_triage_with(&run_id, repo.path(), &FailingRunner).unwrap();

    let store = coordinator.store().unwrap();
    assert_eq!(store.triage_run(&run_id).unwrap().unwrap().outcome.as_deref(), Some("branch-changed"));
    let root = path_text(&crate::host::git_root(repo.path()).unwrap()).unwrap();
    assert_eq!(store.finding(&root, &finding_id, 101.0).unwrap().unwrap().state, FindingState::Pending);
}

#[test]
fn fixed_result_cannot_claim_an_unapproved_safe_document() {
    let repo = repository(true);
    let (coordinator, origin) = fixture(repo.path(), 100.0);
    let finding_id = add_finding(&coordinator, repo.path(), "stale prose", 1.0);
    let launcher = FakeLauncher::default();
    let TriageSchedule::Launched { run_id, .. } =
        coordinator.schedule_findings_triage_for(repo.path(), &origin, &launcher).unwrap()
    else {
        panic!()
    };
    fs::write(repo.path().join("README.md"), "current prose\n").unwrap();
    fs::write(repo.path().join("NOTES.md"), "changed other prose\n").unwrap();
    assert!(
        Command::new("git").args(["add", "README.md", "NOTES.md"]).current_dir(repo.path()).status().unwrap().success()
    );
    assert!(
        Command::new("git")
            .args(["-c", "user.name=test", "-c", "user.email=test@invalid", "commit", "-qm"])
            .arg(format!("docs: refresh prose\n\nFinding-ID: {finding_id}"))
            .current_dir(repo.path())
            .status()
            .unwrap()
            .success()
    );
    let oid = git_head_oid(repo.path()).unwrap();
    let runner = FakeRunner {
        result: json!({ "results": [{
            "finding_id": finding_id, "status": "fixed", "evidence": "updated prose",
            "changed_paths": ["README.md", "NOTES.md"], "validation": ["reviewed diff"],
            "commit_oid": oid, "canonical_id": null, "handoff_path": null
        }] }),
    };
    coordinator.run_findings_triage_with(&run_id, repo.path(), &runner).unwrap();
    let store = coordinator.store().unwrap();
    assert_eq!(store.triage_run(&run_id).unwrap().unwrap().outcome.as_deref(), Some("partial"));
    let root = path_text(&crate::host::git_root(repo.path()).unwrap()).unwrap();
    assert_eq!(store.finding(&root, &finding_id, 101.0).unwrap().unwrap().state, FindingState::Pending);
}

#[test]
fn sweep_safe_document_scopes_exclude_tracked_symlinks_to_code() {
    let repo = repository(true);
    fs::remove_file(repo.path().join("README.md")).unwrap();
    std::os::unix::fs::symlink("src/lib.rs", repo.path().join("README.md")).unwrap();
    assert!(Command::new("git").args(["add", "README.md"]).current_dir(repo.path()).status().unwrap().success());
    assert!(
        Command::new("git")
            .args(["-c", "user.name=test", "-c", "user.email=test@invalid", "commit", "-qm", "link"])
            .current_dir(repo.path())
            .status()
            .unwrap()
            .success()
    );
    let (coordinator, _) = fixture(repo.path(), 100.0);
    let finding_id = add_finding(&coordinator, repo.path(), "prose", 1.0);
    let root = crate::host::git_root(repo.path()).unwrap();
    let finding = coordinator.store().unwrap().finding(root.to_str().unwrap(), &finding_id, 100.0).unwrap().unwrap();
    assert!(safe_document_paths(&root, &[finding]).unwrap().is_empty());
}

#[test]
fn sweep_triage_child_is_reaped_when_prompt_delivery_fails() {
    let mut command = Command::new("sh");
    command.args(["-c", "exec 0<&-; exec sleep 30"]).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null());
    configure_triage_process_group(&mut command);
    let child = command.spawn().unwrap();
    let probe = NativeProcessProbe::new();
    let fingerprint = probe.fingerprint(child.id()).unwrap();
    let prompt = "x".repeat(1024 * 1024);
    assert!(run_triage_child(child, &prompt, Duration::from_secs(30), &mut || Ok(())).is_err());
    assert_eq!(probe.liveness(&fingerprint), ProcessLiveness::Dead);
}

#[test]
fn sweep_triage_child_preserves_success_after_prompt_delivery() {
    let mut command = Command::new("sh");
    command.args(["-c", "cat >/dev/null"]).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null());
    configure_triage_process_group(&mut command);
    let child = command.spawn().unwrap();

    let status = run_triage_child_with_limits(
        child,
        "prompt",
        &mut || Ok(()),
        Duration::from_secs(1),
        Duration::from_millis(10),
    )
    .unwrap();

    assert!(status.success());
}

#[test]
fn sweep_triage_deadline_interrupts_blocked_prompt_and_reaps_the_process_group() {
    let mut command = Command::new("sh");
    command.args(["-c", "sleep 30 & wait"]).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null());
    configure_triage_process_group(&mut command);
    let child = command.spawn().unwrap();
    let group_id = i32::try_from(child.id()).unwrap();
    let probe = NativeProcessProbe::new();
    let fingerprint = probe.fingerprint(child.id()).unwrap();
    let prompt = "x".repeat(1024 * 1024);
    let started = Instant::now();

    let error = run_triage_child_with_limits(
        child,
        &prompt,
        &mut || Ok(()),
        Duration::from_millis(100),
        Duration::from_millis(10),
    )
    .unwrap_err();

    assert_eq!(error.to_string(), "Codex triage run exceeded the 30-minute deadline");
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_eq!(probe.liveness(&fingerprint), ProcessLiveness::Dead);
    let process_group = nix::unistd::Pid::from_raw(-group_id);
    let group_is_gone =
        (0..100).any(|_| match nix::sys::signal::kill(process_group, None::<nix::sys::signal::Signal>) {
            Err(nix::errno::Errno::ESRCH) => true,
            _ => {
                thread::sleep(Duration::from_millis(10));
                false
            }
        });
    if !group_is_gone {
        let _ = nix::sys::signal::kill(process_group, nix::sys::signal::Signal::SIGKILL);
    }
    assert!(group_is_gone, "triage process group survived deadline cleanup");
}

#[test]
fn recursion_marker_suppresses_public_scheduler() {
    // The recursion guard is checked before identity resolution, so this must
    // hold regardless of whether the test process has an ambient Codex/Claude
    // session identity to resolve.
    temp_env::with_var("AI_COORD_TRIAGE_ROLE", Some("triager"), || {
        let repo = repository(true);
        let (coordinator, _) = fixture(repo.path(), 100.0);
        assert_eq!(
            coordinator.schedule_findings_triage(repo.path()).unwrap(),
            TriageSchedule::Skipped("triager-lifecycle")
        );
    });
}

fn assert_isolated_worktree(request: &TriageRequest<'_>) {
    let metadata = read_metadata(request.run_dir).unwrap();
    assert_eq!(request.worktree, request.run_dir.join("worktree"));
    assert_eq!(metadata.worktree_path.as_deref(), Some(request.worktree));
    let branch = format!("triage/{}", metadata.run_id);
    assert_eq!(metadata.worktree_branch.as_deref(), Some(branch.as_str()));
    assert_eq!(git_text(request.worktree, &["symbolic-ref", "--short", "HEAD"]).unwrap().trim(), branch);
    assert_ne!(crate::host::git_root(request.worktree).unwrap(), Path::new(&metadata.repo_root));
    assert!(request.prompt.contains(&format!("isolated worktree on branch {branch}")));
}

fn assert_worktree_removed(repo: &Path, run_id: &str) {
    assert!(!repo.join("state/triage-runs").join(run_id).join("worktree").exists());
    assert!(git_text(repo, &["branch", "--list", &format!("triage/{run_id}")]).unwrap().is_empty());
    assert_eq!(git_text(repo, &["worktree", "list", "--porcelain"]).unwrap().matches("worktree ").count(), 1);
}

struct CallbackRunner<F>(F);

impl<F: Fn(&TriageRequest<'_>) -> Result<ExitStatus>> TriageRunner for CallbackRunner<F> {
    fn run(&self, request: &TriageRequest<'_>, heartbeat: &mut dyn FnMut() -> Result<()>) -> Result<ExitStatus> {
        heartbeat()?;
        assert_isolated_worktree(request);
        (self.0)(request)
    }
}

fn schedule_run(coordinator: &Coordinator, origin: &Identity, repo: &Path) -> String {
    let TriageSchedule::Launched { run_id, .. } =
        coordinator.schedule_findings_triage_for(repo, origin, &FakeLauncher::default()).unwrap()
    else {
        panic!("expected triage launch")
    };
    run_id
}

fn commit_file(repo: &Path, path: &str, contents: &str, message: &str) -> String {
    fs::write(repo.join(path), contents).unwrap();
    git_text(repo, &["add", "--", path]).unwrap();
    git_text(repo, &["-c", "user.name=test", "-c", "user.email=test@invalid", "commit", "-qm", message]).unwrap();
    git_head_oid(repo).unwrap()
}

/// Commit a README change that keeps its name but is not a regular-file edit.
fn commit_readme_entry(repo: &Path, change: &str, message: &str) -> String {
    let readme = repo.join("README.md");
    match change {
        "symlink" => {
            fs::remove_file(&readme).unwrap();
            std::os::unix::fs::symlink("NOTES.md", &readme).unwrap();
        }
        "mode-change" => fs::set_permissions(&readme, fs::Permissions::from_mode(0o755)).unwrap(),
        _ => fs::remove_file(&readme).unwrap(),
    }
    git_text(repo, &["add", "-A", "--", "README.md"]).unwrap();
    git_text(repo, &["-c", "user.name=test", "-c", "user.email=test@invalid", "commit", "-qm", message]).unwrap();
    git_head_oid(repo).unwrap()
}

fn fixed_output(finding_id: &str, oid: &str) -> Value {
    json!({ "results": [{
        "finding_id": finding_id, "status": "fixed", "evidence": "corrected stale prose",
        "changed_paths": ["README.md"], "validation": ["reviewed documentation diff"],
        "commit_oid": oid, "canonical_id": null, "handoff_path": null
    }] })
}

fn finding_state(coordinator: &Coordinator, repo: &Path, id: &str) -> FindingSummary {
    coordinator
        .store()
        .unwrap()
        .finding(crate::host::git_root(repo).unwrap().to_str().unwrap(), id, 101.0)
        .unwrap()
        .unwrap()
}

#[test]
fn worktree_document_commit_is_admitted_and_resolved() {
    let repo = repository(true);
    let (coordinator, origin) = fixture(repo.path(), 100.0);
    let id = add_finding(&coordinator, repo.path(), "stale prose", 1.0);
    let start = git_head_oid(repo.path()).unwrap();
    let run_id = schedule_run(&coordinator, &origin, repo.path());
    let committed = Mutex::new(None);
    let runner = CallbackRunner(|request: &TriageRequest<'_>| {
        assert_eq!(git_head_oid(request.worktree).as_deref(), Some(start.as_str()));
        let oid =
            commit_file(request.worktree, "README.md", "correct prose\n", &format!("docs: fix\n\nFinding-ID: {id}"));
        assert_eq!(git_head_oid(repo.path()).as_deref(), Some(start.as_str()));
        assert_eq!(fs::read_to_string(repo.path().join("README.md"))?, "old prose\n");
        write_private(&request.run_dir.join(RESULT_FILE), &serde_json::to_vec(&fixed_output(&id, &oid))?)?;
        *committed.lock().unwrap() = Some(oid);
        Ok(ExitStatus::from_raw(0))
    });
    coordinator.run_findings_triage_with(&run_id, repo.path(), &runner).unwrap();
    let oid = committed.lock().unwrap().clone().unwrap();
    assert_eq!(git_head_oid(repo.path()).as_deref(), Some(oid.as_str()));
    assert_eq!(fs::read_to_string(repo.path().join("README.md")).unwrap(), "correct prose\n");
    let finding = finding_state(&coordinator, repo.path(), &id);
    assert_eq!(finding.state, FindingState::Fixed);
    assert_eq!(finding.commit_oid.as_deref(), Some(oid.as_str()));
    assert_eq!(
        coordinator.store().unwrap().triage_run(&run_id).unwrap().unwrap().outcome.as_deref(),
        Some("completed")
    );
    assert_worktree_removed(repo.path(), &run_id);
}

#[test]
fn worktree_commits_are_admitted_in_ancestry_order() {
    let repo = repository(true);
    let (coordinator, origin) = fixture(repo.path(), 100.0);
    let first = add_finding(&coordinator, repo.path(), "first typo", 1.0);
    let second = add_finding_at(&coordinator, repo.path(), "second typo", "NOTES.md", FindingKind::Docs, 2.0);
    let run_id = schedule_run(&coordinator, &origin, repo.path());
    let runner = CallbackRunner(|request: &TriageRequest<'_>| {
        // Reverse claim order to exercise ancestry ordering at admission.
        let second_oid =
            commit_file(request.worktree, "NOTES.md", "fixed notes\n", &format!("docs: notes\n\nFinding-ID: {second}"));
        let first_oid = commit_file(
            request.worktree,
            "README.md",
            "fixed readme\n",
            &format!("docs: readme\n\nFinding-ID: {first}"),
        );
        let mut output = fixed_output(&first, &first_oid);
        let mut second_result = fixed_output(&second, &second_oid)["results"][0].clone();
        second_result["changed_paths"] = json!(["NOTES.md"]);
        output["results"].as_array_mut().unwrap().push(second_result);
        write_private(&request.run_dir.join(RESULT_FILE), &serde_json::to_vec(&output)?)?;
        Ok(ExitStatus::from_raw(0))
    });
    coordinator.run_findings_triage_with(&run_id, repo.path(), &runner).unwrap();
    for id in [&first, &second] {
        let finding = finding_state(&coordinator, repo.path(), id);
        assert_eq!(finding.state, FindingState::Fixed);
        assert!(finding.commit_oid.is_some());
    }
    assert_eq!(git_head_oid(repo.path()), finding_state(&coordinator, repo.path(), &first).commit_oid);
    assert_worktree_removed(repo.path(), &run_id);
}

#[test]
fn admission_failure_preserves_main_and_pending_finding_without_handoff() {
    for obstruction in
        ["main-moved", "dirty-path", "unsafe-ancestor", "unauthorized-commit", "symlink", "mode-change", "deletion"]
    {
        let repo = repository(true);
        let (coordinator, origin) = fixture(repo.path(), 100.0);
        let id = add_finding(&coordinator, repo.path(), "stale prose", 1.0);
        let run_id = schedule_run(&coordinator, &origin, repo.path());
        let expected_head = Mutex::new(git_head_oid(repo.path()).unwrap());
        let runner = CallbackRunner(|request: &TriageRequest<'_>| {
            match obstruction {
                "main-moved" => {
                    *expected_head.lock().unwrap() =
                        commit_file(repo.path(), "NOTES.md", "main advanced\n", "other work");
                }
                "dirty-path" => fs::write(repo.path().join("README.md"), "uncommitted work\n")?,
                "unsafe-ancestor" => {
                    commit_file(request.worktree, "src/lib.rs", "unsafe code\n", "unapproved code");
                }
                _ => {}
            }
            let changed = if obstruction == "unauthorized-commit" { "NOTES.md" } else { "README.md" };
            let message = format!("docs: fix\n\nFinding-ID: {id}");
            let oid = match obstruction {
                "symlink" | "mode-change" | "deletion" => commit_readme_entry(request.worktree, obstruction, &message),
                _ => commit_file(request.worktree, changed, "correct prose\n", &message),
            };
            let handoff = deterministic_handoff(&id);
            fs::create_dir_all(request.worktree.join(".ai/task-handoffs"))?;
            fs::write(request.worktree.join(&handoff), format!("Source finding: {id}\n"))?;
            // A failed admission must not fall back to even a valid deterministic handoff.
            let mut result = fixed_output(&id, &oid);
            if obstruction == "main-moved" {
                result["results"][0] = json!({
                    "finding_id": id, "status": "handed_off", "evidence": "needs retry",
                    "changed_paths": [handoff], "validation": ["marker checked"], "commit_oid": null,
                    "canonical_id": null, "handoff_path": handoff
                });
            }
            write_private(&request.run_dir.join(RESULT_FILE), &serde_json::to_vec(&result)?)?;
            Ok(ExitStatus::from_raw(0))
        });
        coordinator.run_findings_triage_with(&run_id, repo.path(), &runner).unwrap();
        assert_eq!(finding_state(&coordinator, repo.path(), &id).state, FindingState::Pending, "{obstruction}");
        assert_eq!(git_head_oid(repo.path()).unwrap(), *expected_head.lock().unwrap(), "{obstruction}");
        let expected_prose = if obstruction == "dirty-path" { "uncommitted work\n" } else { "old prose\n" };
        assert_eq!(fs::read_to_string(repo.path().join("README.md")).unwrap(), expected_prose);
        assert_eq!(fs::read_to_string(repo.path().join("src/lib.rs")).unwrap(), "pub fn value() -> u8 { 1 }\n");
        assert!(!repo.path().join(deterministic_handoff(&id)).exists());
        let log =
            fs::read_to_string(repo.path().join("state/triage-runs").join(&run_id).join(RECONCILE_LOG_FILE)).unwrap();
        assert!(log.contains("admission-failed"), "{obstruction}: {log}");
        assert_worktree_removed(repo.path(), &run_id);
    }
}

#[test]
fn worktree_handoff_is_published_before_validation_even_after_runner_failure() {
    for runner_failed in [false, true] {
        let repo = repository(true);
        let (coordinator, origin) = fixture(repo.path(), 100.0);
        let id = add_finding_at(&coordinator, repo.path(), "code behavior", "src/lib.rs", FindingKind::Bug, 1.0);
        let run_id = schedule_run(&coordinator, &origin, repo.path());
        let handoff = deterministic_handoff(&id);
        let contents = format!("# Handoff\n\nSource finding: {id}\n");
        let runner = CallbackRunner(|request: &TriageRequest<'_>| {
            fs::create_dir_all(request.worktree.join(".ai/task-handoffs"))?;
            fs::write(request.worktree.join(&handoff), &contents)?;
            assert!(!repo.path().join(&handoff).exists());
            if runner_failed {
                return Err(AppError::operational("simulated lost worker"));
            }
            let result = json!({ "results": [{
                "finding_id": id, "status": "handed_off", "evidence": "verified broad scope",
                "changed_paths": [handoff], "validation": ["marker checked"], "commit_oid": null,
                "canonical_id": null, "handoff_path": handoff
            }] });
            write_private(&request.run_dir.join(RESULT_FILE), &serde_json::to_vec(&result)?)?;
            Ok(ExitStatus::from_raw(0))
        });
        coordinator.run_findings_triage_with(&run_id, repo.path(), &runner).unwrap();
        assert_eq!(finding_state(&coordinator, repo.path(), &id).state, FindingState::HandedOff);
        assert_eq!(fs::read_to_string(repo.path().join(&handoff)).unwrap(), contents);
        validate_handoff(&crate::host::git_root(repo.path()).unwrap(), &id, &handoff).unwrap();
        assert_worktree_removed(repo.path(), &run_id);
    }
}

#[test]
fn invalid_result_and_nonzero_exit_remove_worktrees() {
    for outcome in ["invalid-result", "runner-failed"] {
        let repo = repository(true);
        let (coordinator, origin) = fixture(repo.path(), 100.0);
        add_finding(&coordinator, repo.path(), "stale prose", 1.0);
        let run_id = schedule_run(&coordinator, &origin, repo.path());
        let runner = CallbackRunner(|request: &TriageRequest<'_>| {
            fs::write(request.run_dir.join(RESULT_FILE), "invalid json")?;
            Ok(ExitStatus::from_raw(if outcome == "runner-failed" { 256 } else { 0 }))
        });
        coordinator.run_findings_triage_with(&run_id, repo.path(), &runner).unwrap();
        assert_eq!(
            coordinator.store().unwrap().triage_run(&run_id).unwrap().unwrap().outcome.as_deref(),
            Some(outcome)
        );
        assert_worktree_removed(repo.path(), &run_id);
    }
}

#[test]
fn branch_change_cleans_a_previous_worktree_without_admission() {
    let repo = repository(true);
    let root = crate::host::git_root(repo.path()).unwrap();
    let (coordinator, origin) = fixture(&root, 100.0);
    let id = add_finding(&coordinator, &root, "stale prose", 1.0);
    let run_id = schedule_run(&coordinator, &origin, &root);
    let run_dir = root.join("state/triage-runs").join(&run_id);
    let mut metadata = read_metadata(&run_dir).unwrap();
    let worktree = TriageWorktree::new(&root, &run_dir, &run_id, 100.0);
    worktree.create(&metadata.start_head).unwrap();
    metadata.worktree_path = Some(worktree.path.clone());
    metadata.worktree_branch = Some(worktree.branch.clone());
    metadata.authorized_paths = vec!["README.md".to_owned()];
    write_metadata(&run_dir, &metadata).unwrap();
    commit_file(&worktree.path, "README.md", "correct prose\n", &format!("docs: fix\n\nFinding-ID: {id}"));
    git_text(&root, &["checkout", "-qb", "topic"]).unwrap();
    coordinator.run_findings_triage_with(&run_id, &root, &FailingRunner).unwrap();
    assert_eq!(git_head_oid(&root).as_deref(), Some(metadata.start_head.as_str()));
    assert_eq!(finding_state(&coordinator, &root, &id).state, FindingState::Pending);
    assert_eq!(
        coordinator.store().unwrap().triage_run(&run_id).unwrap().unwrap().outcome.as_deref(),
        Some("branch-changed")
    );
    assert_worktree_removed(&root, &run_id);
}

#[test]
fn inactive_reconciliation_preserves_live_worktrees_and_removes_lost_or_expired_ones() {
    for expired in [false, true] {
        let repo = repository(true);
        let root = crate::host::git_root(repo.path()).unwrap();
        let (coordinator, origin) = fixture(repo.path(), 100.0);
        let id = add_finding(&coordinator, repo.path(), "stale prose", 1.0);
        let run_id = schedule_run(&coordinator, &origin, repo.path());
        let run_root = root.join("state/triage-runs");
        let run_dir = run_root.join(&run_id);
        let mut metadata = read_metadata(&run_dir).unwrap();
        let worktree = TriageWorktree::new(&root, &run_dir, &run_id, 100.0);
        worktree.create(&metadata.start_head).unwrap();
        metadata.worktree_path = Some(worktree.path.clone());
        metadata.worktree_branch = Some(worktree.branch.clone());
        metadata.authorized_paths = vec!["README.md".to_owned()];
        write_metadata(&run_dir, &metadata).unwrap();
        coordinator.reconcile_inactive_runs(&root, &run_root, 100.0).unwrap();
        assert!(worktree.path.exists(), "live worker must retain its worktree");
        let oid =
            commit_file(&worktree.path, "README.md", "correct prose\n", &format!("docs: fix\n\nFinding-ID: {id}"));
        let current = if expired { 100.0 + RUN_EXPIRY_SECONDS } else { 100.0 + HEARTBEAT_GRACE_SECONDS + 1.0 };
        if expired {
            metadata.heartbeat_at = current;
            write_metadata(&run_dir, &metadata).unwrap();
        }
        coordinator.reconcile_inactive_runs(&root, &run_root, current).unwrap();
        assert_eq!(git_head_oid(&root).as_deref(), Some(oid.as_str()));
        assert_eq!(finding_state(&coordinator, &root, &id).state, FindingState::Fixed);
        assert_eq!(
            coordinator.store().unwrap().triage_run(&run_id).unwrap().unwrap().outcome.as_deref(),
            Some("worker-lost")
        );
        assert_worktree_removed(&root, &run_id);
    }
}

#[test]
fn handoff_copy_never_overwrites_or_traverses_symlinked_parents() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let id = "abc123";
    let relative = deterministic_handoff(id);
    fs::create_dir_all(source.path().join(".ai/task-handoffs")).unwrap();
    fs::write(source.path().join(&relative), format!("Source finding: {id}\n")).unwrap();
    std::os::unix::fs::symlink(outside.path(), destination.path().join(".ai")).unwrap();
    assert!(copy_handoff(Some(source.path()), destination.path(), id).is_err());
    assert!(!outside.path().join("task-handoffs").exists());
    fs::remove_file(destination.path().join(".ai")).unwrap();
    fs::create_dir(destination.path().join(".ai")).unwrap();
    std::os::unix::fs::symlink(outside.path(), destination.path().join(".ai/task-handoffs")).unwrap();
    assert!(copy_handoff(Some(source.path()), destination.path(), id).is_err());
    assert!(!outside.path().join("FINDING_ABC123.md").exists());
    fs::remove_file(destination.path().join(".ai/task-handoffs")).unwrap();
    fs::create_dir(destination.path().join(".ai/task-handoffs")).unwrap();
    fs::write(destination.path().join(&relative), "existing user handoff\n").unwrap();
    copy_handoff(Some(source.path()), destination.path(), id).unwrap();
    assert_eq!(fs::read_to_string(destination.path().join(&relative)).unwrap(), "existing user handoff\n");
    assert!(validate_handoff(destination.path(), id, &relative).is_err());
}

fn sample_metadata(run_id: &str, started_at: f64) -> RunMetadata {
    RunMetadata {
        run_id: run_id.to_owned(),
        repo_root: "/repo".to_owned(),
        state_dir: "/state".to_owned(),
        start_head: "0".repeat(40),
        worktree_path: None,
        worktree_branch: None,
        finding_ids: Vec::new(),
        authorized_paths: Vec::new(),
        started_at,
        heartbeat_at: started_at,
        finished_at: None,
        worker: None,
    }
}

fn register_peer(coordinator: &Coordinator, peer: &Identity, root: &Path) {
    let root = path_text(root).unwrap();
    coordinator
        .store()
        .unwrap()
        .upsert_session(&SessionUpdate {
            identity: peer.clone(),
            cwd: root.clone(),
            repo_root: Some(root),
            state: SessionState::Working,
            source: "test".to_owned(),
            name: None,
            waiting_for: None,
            permission_mode: None,
            update_permission_mode: false,
            coordination_waived: None,
            fingerprint: Some(NativeProcessProbe::new().fingerprint(std::process::id()).unwrap()),
            transcript_path: None,
            started_at: Some(100.0),
            current: 100.0,
        })
        .unwrap();
}

#[test]
fn run_liveness_covers_launch_and_expires_after_the_worker_deadline() {
    let probe = NativeProcessProbe::new();
    let run = TriageRun {
        id: "run".to_owned(),
        repo_root: "/repo".to_owned(),
        origin: Identity { client: Client::Codex, session_id: "origin".to_owned() },
        started_at: 100.0,
        finished_at: None,
        outcome: None,
    };
    // The ledger row exists briefly before the scheduler writes run metadata.
    assert!(run_is_live(&run, None, &probe, 100.0 + HEARTBEAT_GRACE_SECONDS));
    assert!(!run_is_live(&run, None, &probe, 101.0 + HEARTBEAT_GRACE_SECONDS));
    let mut metadata = sample_metadata("run", 100.0);
    assert!(run_is_live(&run, Some(&metadata), &probe, 105.0), "a launching worker has no fingerprint yet");
    assert!(!run_is_live(&run, Some(&metadata), &probe, 101.0 + HEARTBEAT_GRACE_SECONDS));
    metadata.worker = Some(probe.fingerprint(std::process::id()).unwrap());
    let deadline = 100.0 + RUN_DEADLINE_SECONDS;
    metadata.heartbeat_at = deadline;
    assert!(run_is_live(&run, Some(&metadata), &probe, deadline + 1.0), "the worker finalizes after its deadline");
    metadata.heartbeat_at = 100.0 + RUN_EXPIRY_SECONDS;
    assert!(!run_is_live(&run, Some(&metadata), &probe, 100.0 + RUN_EXPIRY_SECONDS));
}

#[test]
fn worker_heartbeats_through_setup_and_shares_the_ledger_deadline() {
    let repo = repository(true);
    let root = crate::host::git_root(repo.path()).unwrap();
    let (coordinator, origin, clock) = clocked_fixture(repo.path(), 100.0, true);
    add_finding(&coordinator, repo.path(), "stale prose", 1.0);
    let run_id = schedule_run(&coordinator, &origin, repo.path());
    let run_root = root.join("state/triage-runs");
    let run_dir = run_root.join(&run_id);
    // The worker starts long after the scheduler's own heartbeat went stale.
    let setup = 100.0 + 10.0 * HEARTBEAT_GRACE_SECONDS;
    clock.set(setup);
    let runner = CallbackRunner(|request: &TriageRequest<'_>| {
        assert_eq!(request.deadline, Duration::from_secs_f64(RUN_DEADLINE_SECONDS - (setup - 100.0)));
        coordinator.reconcile_inactive_runs(&root, &run_root, setup)?;
        assert!(request.worktree.exists(), "a setting-up worker must not look lost");
        let later = setup + 10.0 * HEARTBEAT_GRACE_SECONDS;
        clock.set(later);
        let refreshed = (0..100).any(|_| {
            thread::sleep(Duration::from_millis(50));
            read_metadata(&run_dir).is_ok_and(|metadata| metadata.heartbeat_at == later)
        });
        assert!(refreshed, "the background heartbeat keeps running while the worker is busy");
        coordinator.reconcile_inactive_runs(&root, &run_root, later)?;
        assert!(request.worktree.exists());
        Err(AppError::operational("simulated runner stop"))
    });
    coordinator.run_findings_triage_with(&run_id, repo.path(), &runner).unwrap();
    assert_eq!(
        coordinator.store().unwrap().triage_run(&run_id).unwrap().unwrap().outcome.as_deref(),
        Some("runner-failed")
    );
    assert_worktree_removed(&root, &run_id);
}

#[test]
fn lost_worker_commit_is_not_admitted_over_a_peer_claim() {
    let repo = repository(true);
    let root = crate::host::git_root(repo.path()).unwrap();
    let (coordinator, origin) = fixture(repo.path(), 100.0);
    let id = add_finding(&coordinator, repo.path(), "stale prose", 1.0);
    let run_id = schedule_run(&coordinator, &origin, repo.path());
    let run_root = root.join("state/triage-runs");
    let run_dir = run_root.join(&run_id);
    let mut metadata = read_metadata(&run_dir).unwrap();
    let worktree = TriageWorktree::new(&root, &run_dir, &run_id, 100.0);
    worktree.create(&metadata.start_head).unwrap();
    metadata.worktree_path = Some(worktree.path.clone());
    metadata.worktree_branch = Some(worktree.branch.clone());
    metadata.authorized_paths = vec!["README.md".to_owned()];
    write_metadata(&run_dir, &metadata).unwrap();
    commit_file(&worktree.path, "README.md", "correct prose\n", &format!("docs: fix\n\nFinding-ID: {id}"));
    // The worker died and its claim is gone; a peer has since been granted README.md.
    let peer = Identity { client: Client::Codex, session_id: "peer".to_owned() };
    register_peer(&coordinator, &peer, &root);
    let outcome = coordinator.start_for(peer, "edit readme", &[PathBuf::from("README.md")], &[], &root).unwrap();
    assert_eq!(outcome.kind, OutcomeKind::Ready);

    coordinator.reconcile_inactive_runs(&root, &run_root, 100.0 + HEARTBEAT_GRACE_SECONDS + 1.0).unwrap();

    assert_eq!(git_head_oid(&root).as_deref(), Some(metadata.start_head.as_str()));
    assert_eq!(fs::read_to_string(root.join("README.md")).unwrap(), "old prose\n");
    assert_eq!(finding_state(&coordinator, &root, &id).state, FindingState::Pending);
    let log = fs::read_to_string(run_dir.join(RECONCILE_LOG_FILE)).unwrap();
    assert!(log.contains("claimed by another session"), "{log}");
    assert_worktree_removed(&root, &run_id);
}

#[test]
fn findings_resolved_before_the_prompt_do_not_make_a_run_partial() {
    let repo = repository(true);
    let (coordinator, origin) = fixture(repo.path(), 100.0);
    let resolved = add_finding(&coordinator, repo.path(), "already fixed", 1.0);
    let prompted = add_finding_at(&coordinator, repo.path(), "notes prose", "NOTES.md", FindingKind::Docs, 2.0);
    let run_id = schedule_run(&coordinator, &origin, repo.path());
    let root = path_text(&crate::host::git_root(repo.path()).unwrap()).unwrap();
    coordinator
        .store()
        .unwrap()
        .resolve_finding(
            &root,
            &resolved,
            &FindingResolution {
                state: FindingState::Stale,
                commit_oid: None,
                canonical_id: None,
                actor: Identity { client: Client::Codex, session_id: "source".to_owned() },
                current: 100.0,
            },
        )
        .unwrap();
    let runner = FakeRunner {
        result: json!({ "results": [{
            "finding_id": prompted, "status": "stale", "evidence": "prose is already current",
            "changed_paths": [], "validation": [], "commit_oid": null,
            "canonical_id": null, "handoff_path": null
        }] }),
    };
    coordinator.run_findings_triage_with(&run_id, repo.path(), &runner).unwrap();
    let prompt = fs::read_to_string(repo.path().join("state/triage-runs").join(&run_id).join("prompt.txt")).unwrap();
    assert!(prompt.contains(&prompted) && !prompt.contains(&resolved));
    let store = coordinator.store().unwrap();
    assert_eq!(store.triage_run(&run_id).unwrap().unwrap().outcome.as_deref(), Some("completed"));
    assert_eq!(store.finding(&root, &prompted, 101.0).unwrap().unwrap().state, FindingState::Stale);
}

#[test]
fn worker_setup_error_finishes_the_run_and_releases_claims() {
    let repo = repository(true);
    let (coordinator, origin) = fixture(repo.path(), 100.0);
    add_finding(&coordinator, repo.path(), "stale prose", 1.0);
    let run_id = schedule_run(&coordinator, &origin, repo.path());
    let run_dir = repo.path().join("state/triage-runs").join(&run_id);
    // Writing the prompt fails after the triager registered and claimed README.md.
    fs::create_dir(run_dir.join("prompt.txt")).unwrap();
    assert!(coordinator.run_findings_triage_with(&run_id, repo.path(), &FailingRunner).is_err());
    let actor = triager_identity(&run_id);
    let store = coordinator.store().unwrap();
    assert_eq!(store.triage_run(&run_id).unwrap().unwrap().outcome.as_deref(), Some("worker-failed"));
    assert!(store.triage_claims(&run_id).unwrap().is_empty());
    assert!(store.work(&actor).unwrap().is_none());
    assert!(store.session(&actor).unwrap().is_none());
    assert!(read_metadata(&run_dir).unwrap().finished_at.is_some());
    assert!(fs::read_to_string(run_dir.join(RECONCILE_LOG_FILE)).unwrap().contains("worker-failed"));
    assert_worktree_removed(repo.path(), &run_id);
}

#[test]
fn log_pruning_keeps_open_runs_and_ignores_the_repository_opt_in() {
    let repo = repository(true);
    let (coordinator, origin) = fixture(repo.path(), 100.0);
    add_finding(&coordinator, repo.path(), "stale prose", 1.0);
    let run_id = schedule_run(&coordinator, &origin, repo.path());
    let run_root = repo.path().join("state/triage-runs");
    let run_dir = run_root.join(&run_id);
    let far_future = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs_f64() + 60.0 * 24.0 * 60.0 * 60.0;
    prune_run_logs(&coordinator, &run_root, far_future).unwrap();
    assert!(run_dir.exists(), "an open run keeps its directory past retention");
    coordinator.store().unwrap().finish_triage_run(&run_id, "completed", 100.0).unwrap();
    prune_run_logs(&coordinator, &run_root, far_future).unwrap();
    assert!(!run_dir.exists());

    let disabled = repository(false);
    let (coordinator, origin) = fixture(disabled.path(), 40.0 * 24.0 * 60.0 * 60.0);
    let stale = disabled.path().join("state/triage-runs/stale");
    fs::create_dir_all(&stale).unwrap();
    let mut metadata = sample_metadata("stale", 1.0);
    metadata.finished_at = Some(1.0);
    write_metadata(&stale, &metadata).unwrap();
    assert_eq!(
        coordinator.schedule_findings_triage_for(disabled.path(), &origin, &FakeLauncher::default()).unwrap(),
        TriageSchedule::Skipped("disabled")
    );
    assert!(!stale.exists());
}

#[test]
fn cleanup_deletes_the_branch_when_the_worktree_directory_is_gone() {
    let repo = repository(true);
    let root = crate::host::git_root(repo.path()).unwrap();
    let run_dir = root.join("state/triage-runs/run");
    fs::create_dir_all(&run_dir).unwrap();
    let worktree = TriageWorktree::new(&root, &run_dir, "run", 100.0);
    worktree.create(&git_head_oid(&root).unwrap()).unwrap();
    fs::remove_dir_all(&worktree.path).unwrap();
    drop(worktree);
    assert!(!run_dir.join(RECONCILE_LOG_FILE).exists());
    assert_worktree_removed(&root, "run");
}
