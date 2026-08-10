use std::{os::unix::process::ExitStatusExt, sync::Mutex};

use serde_json::{Value, json};
use tempfile::TempDir;

use crate::{
    coordinator::{Clock, inventory::StaticInventory},
    domain::{FindingKind, ProcessFingerprint, WorkState},
    host::NativeProcessProbe,
    state::{FindingAdd, FindingPathObservation},
};

use super::*;

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

struct FakeClock(f64);
impl Clock for FakeClock {
    fn wall(&self) -> f64 {
        self.0
    }
    fn monotonic(&self) -> f64 {
        self.0
    }
    fn sleep(&self, _: Duration) {}
}

struct FakeRunner {
    result: Value,
}
impl TriageRunner for FakeRunner {
    fn run(&self, request: &TriageRequest<'_>, heartbeat: &mut dyn FnMut() -> Result<()>) -> Result<ExitStatus> {
        heartbeat()?;
        let metadata = read_metadata(request.run_dir)?;
        let store = Store::open(request.state_dir.join("state.db"))?;
        let work = store.work(&triager_identity(&metadata.run_id))?;
        if metadata.authorized_paths.is_empty() {
            assert!(work.is_none());
        } else {
            let work = work.expect("safe-document triage owns an exact scope before the model runs");
            assert_eq!(work.state, WorkState::Active);
            assert!(work.scopes.iter().all(|scope| !scope.is_recursive()));
            assert_eq!(
                work.scopes.into_iter().map(|scope| scope.path).collect::<BTreeSet<_>>(),
                metadata.authorized_paths.into_iter().collect()
            );
        }
        write_private(&request.run_dir.join(RESULT_FILE), serde_json::to_vec(&self.result)?.as_slice())?;
        Ok(ExitStatus::from_raw(0))
    }
}

struct FailingRunner;
impl TriageRunner for FailingRunner {
    fn run(&self, _: &TriageRequest<'_>, heartbeat: &mut dyn FnMut() -> Result<()>) -> Result<ExitStatus> {
        heartbeat()?;
        Err(AppError::operational("simulated deadline"))
    }
}

fn repository(auto_triage: bool) -> TempDir {
    let temp = tempfile::tempdir().unwrap();
    assert!(
        Command::new("git").args(["init", "-q", "-b", "main"]).current_dir(temp.path()).status().unwrap().success()
    );
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
    let state = repo.join("state");
    let store = Store::open(state.join("state.db")).unwrap();
    let coordinator = Coordinator::with_components(
        store,
        Box::new(StaticInventory { complete, refreshes: Default::default() }),
        std::sync::Arc::new(NativeProcessProbe::new()),
        std::sync::Arc::new(FakeClock(now)),
    );
    (coordinator, Identity { client: Client::Codex, session_id: "origin".to_owned() })
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
            normalized_summary: summary.to_owned(),
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
fn codex_command_is_ephemeral_sandboxed_offline_and_agentless() {
    let repo = Path::new("/repo");
    let state = Path::new("/state");
    let run = Path::new("/state/triage-runs/a");
    let request = TriageRequest { repo_root: repo, state_dir: state, run_dir: run, prompt: "prompt" };
    let args = codex_args(request.repo_root, request.state_dir, request.run_dir)
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
    let store = coordinator.store().unwrap();
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
fn recursion_marker_suppresses_public_scheduler() {
    // The recursion guard is checked before identity resolution, so this must
    // hold regardless of whether the test process has an ambient Codex/Claude
    // session identity to resolve.
    unsafe { std::env::set_var("AI_COORD_TRIAGE_ROLE", "triager") };
    let repo = repository(true);
    let (coordinator, _) = fixture(repo.path(), 100.0);
    assert_eq!(
        coordinator.schedule_findings_triage(repo.path()).unwrap(),
        TriageSchedule::Skipped("triager-lifecycle")
    );
    unsafe { std::env::remove_var("AI_COORD_TRIAGE_ROLE") };
}
