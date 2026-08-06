use std::{
    collections::VecDeque,
    io::Cursor,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use serde_json::{Value, json};
use tempfile::TempDir;

use super::*;

#[test]
fn accepts_supported_versions_and_rejects_old_or_malformed_versions() {
    for output in [
        "codex-cli 0.146.0\n",
        "codex-cli 0.146.0+build.1",
        "codex-cli 0.147.0-alpha.1",
        "codex-cli 0.147.0-alpha-beta",
        "codex-cli 1.0.0",
    ] {
        require_codex_minimum_version(output).unwrap();
    }
    for output in ["codex-cli 0.145.9", "codex-cli 0.146.0-alpha.1"] {
        assert!(
            require_codex_minimum_version(output).unwrap_err().to_string().contains("requires codex-cli >= 0.146.0")
        );
    }
    for output in ["", "codex 0.146.0", "codex-cli latest", "codex-cli 0.146", "codex-cli 0.146.0+"] {
        assert!(require_codex_minimum_version(output).unwrap_err().to_string().contains("could not parse"));
    }
}

#[test]
fn dry_run_validates_the_active_path_without_starting_codex() {
    let active = default_hook_path(Client::Codex);
    assert_eq!(trust_codex_hooks(Some(&active), true).unwrap(), TrustOutcome::Skipped);
    let other = active.with_file_name("other-hooks.json");
    assert!(trust_codex_hooks(Some(&other), true).unwrap_err().to_string().contains("active source"));
}

#[test]
fn unsupported_version_is_rejected_before_connecting() {
    let fixture = Fixture::new();
    let mut runtime = FakeRuntime::new(std::iter::empty()).with_version("codex-cli 0.145.9");
    let error = trust_codex_hooks_with(&mut runtime, &fixture.hooks_path).unwrap_err();
    assert!(error.to_string().contains("requires codex-cli >= 0.146.0"));
    assert_eq!(runtime.connects, 0);
}

#[test]
fn trust_batches_only_exact_owned_hooks_and_verifies_in_a_fresh_server() {
    let fixture = Fixture::new();
    let config_path = fixture.config_path();
    let first = FakeTransport::from_results([
        ok(1, json!({})),
        ok(2, hooks_response(&fixture.hooks_path, "untrusted", "hash")),
        ok(3, config_response(&config_path, "v1")),
        ok(4, json!({ "filePath": config_path, "status": "ok", "version": "v2" })),
    ]);
    let first_writes = first.writes.clone();
    let second =
        FakeTransport::from_results([ok(1, json!({})), ok(2, hooks_response(&fixture.hooks_path, "trusted", "hash"))]);
    let second_writes = second.writes.clone();
    let mut runtime = FakeRuntime::new([first, second]);

    assert_eq!(trust_codex_hooks_with(&mut runtime, &fixture.hooks_path).unwrap(), TrustOutcome::Updated);
    assert_eq!(runtime.connects, 2);

    let writes = locked(&first_writes);
    assert_eq!(writes[0]["method"], "initialize");
    assert_eq!(writes[1]["method"], "initialized");
    assert_eq!(writes[2]["method"], "hooks/list");
    assert_eq!(writes[3]["method"], "config/read");
    assert_eq!(writes[3]["params"], json!({ "includeLayers": true }));
    assert_eq!(writes[4]["method"], "config/batchWrite");
    let params = &writes[4]["params"];
    assert_eq!(params["expectedVersion"], "v1");
    assert_eq!(params["filePath"], config_path.to_string_lossy().as_ref());
    let edits = params["edits"].as_array().unwrap();
    assert_eq!(edits.len(), hook_specs(Client::Codex).len());
    assert!(edits.iter().all(|edit| edit["mergeStrategy"] == "upsert"));
    assert!(edits.iter().all(|edit| edit["keyPath"].as_str().unwrap().starts_with("hooks.state.\"key.")));
    drop(writes);
    assert_eq!(locked(&second_writes)[2]["method"], "hooks/list");
}

#[test]
fn already_trusted_hooks_are_a_read_only_noop() {
    let fixture = Fixture::new();
    let server =
        FakeTransport::from_results([ok(1, json!({})), ok(2, hooks_response(&fixture.hooks_path, "trusted", "hash"))]);
    let writes = server.writes.clone();
    let mut runtime = FakeRuntime::new([server]);

    assert_eq!(trust_codex_hooks_with(&mut runtime, &fixture.hooks_path).unwrap(), TrustOutcome::Unchanged);
    assert_eq!(runtime.connects, 1);
    assert_eq!(locked(&writes).len(), 3);
}

#[test]
fn retries_three_fresh_discoveries_only_on_config_version_conflict() {
    let fixture = Fixture::new();
    let mut servers = Vec::new();
    let mut transcripts = Vec::new();
    for attempt in 1..=3 {
        let server = FakeTransport::from_results([
            ok(1, json!({})),
            ok(2, hooks_response(&fixture.hooks_path, "untrusted", &format!("attempt-{attempt}"))),
            ok(3, config_response(&fixture.config_path(), &format!("v{attempt}"))),
            error(
                4,
                json!({
                    "code": -32600,
                    "data": { "config_write_error_code": "configVersionConflict" }
                }),
            ),
        ]);
        transcripts.push(server.writes.clone());
        servers.push(server);
    }
    let mut runtime = FakeRuntime::new(servers);

    let error = trust_codex_hooks_with(&mut runtime, &fixture.hooks_path).unwrap_err();
    assert!(error.to_string().contains("version conflict"));
    assert_eq!(runtime.connects, 3);
    for (attempt, transcript) in transcripts.iter().enumerate() {
        let writes = locked(transcript);
        let edits = writes[4]["params"]["edits"].as_array().unwrap();
        assert!(
            edits
                .iter()
                .all(|edit| { edit["value"].as_str().unwrap().starts_with(&format!("attempt-{}-", attempt + 1)) })
        );
    }
}

#[test]
fn does_not_retry_a_failed_fresh_verification() {
    let fixture = Fixture::new();
    let config_path = fixture.config_path();
    let first = FakeTransport::from_results([
        ok(1, json!({})),
        ok(2, hooks_response(&fixture.hooks_path, "untrusted", "hash")),
        ok(3, config_response(&config_path, "v1")),
        ok(4, json!({ "filePath": config_path, "status": "ok", "version": "v2" })),
    ]);
    let verifier =
        FakeTransport::from_results([ok(1, json!({})), ok(2, hooks_response(&fixture.hooks_path, "modified", "hash"))]);
    let mut runtime = FakeRuntime::new([first, verifier]);

    let error = trust_codex_hooks_with(&mut runtime, &fixture.hooks_path).unwrap_err();
    assert!(error.to_string().contains("did not verify"));
    assert_eq!(runtime.connects, 2);
}

#[test]
fn inspection_fails_closed_for_untrusted_missing_duplicate_and_malformed_hooks() {
    let fixture = Fixture::new();
    let untrusted = FakeTransport::from_results([
        ok(1, json!({})),
        ok(2, hooks_response(&fixture.hooks_path, "untrusted", "hash")),
    ]);
    let mut runtime = FakeRuntime::new([untrusted]);
    let check = inspect_codex_hook_trust_with(&mut runtime, fixture.hooks_path.clone()).unwrap();
    assert!(!check.ok);
    assert_eq!(check.error.as_deref(), Some("owned Codex hooks are not trusted"));

    let mut missing = hooks_response(&fixture.hooks_path, "trusted", "hash");
    missing["data"][0]["hooks"].as_array_mut().unwrap().pop();
    assert_inventory_error(&fixture, missing, "missing exact");

    let mut duplicate = hooks_response(&fixture.hooks_path, "trusted", "hash");
    let hook = duplicate["data"][0]["hooks"][0].clone();
    duplicate["data"][0]["hooks"].as_array_mut().unwrap().push(hook);
    assert_inventory_error(&fixture, duplicate, "duplicate owned");

    let mut malformed = hooks_response(&fixture.hooks_path, "trusted", "hash");
    malformed["data"][0]["hooks"][0].as_object_mut().unwrap().remove("currentHash");
    assert_inventory_error(&fixture, malformed, "malformed hooks/list");

    let mut duplicate_key = hooks_response(&fixture.hooks_path, "trusted", "hash");
    let key = duplicate_key["data"][0]["hooks"][0]["key"].clone();
    duplicate_key["data"][0]["hooks"][1]["key"] = key;
    assert_inventory_error(&fixture, duplicate_key, "duplicate Codex hook key");
}

#[test]
fn ownership_rejects_every_near_match_field() {
    let fixture = Fixture::new();
    let cases = [
        ("eventName", json!("sessionEnd")),
        ("enabled", json!(false)),
        ("isManaged", json!(true)),
        ("source", json!("project")),
        ("sourcePath", json!("/not-the-active-hooks-file")),
        ("handlerType", json!("prompt")),
        ("command", json!("ai-coord hook codex ")),
        ("matcher", json!("*")),
        ("timeoutSec", json!(6)),
        ("additionalContextLimit", json!(0)),
        ("key", json!("")),
        ("currentHash", json!("")),
        ("trustStatus", json!("managed")),
    ];
    for (field, value) in cases {
        let mut response = hooks_response(&fixture.hooks_path, "trusted", "hash");
        response["data"][0]["hooks"][0][field] = value;
        let error = owned_codex_hooks(response, &fixture.hooks_path).unwrap_err();
        assert!(error.to_string().contains("missing exact"), "field {field}: {error}");
    }
}

#[test]
fn selects_only_the_exact_active_user_config_layer() {
    let fixture = Fixture::new();
    let config_path = fixture.config_path();
    let response = json!({
        "layers": [
            { "name": { "type": "system", "file": config_path }, "version": "system" },
            { "name": { "type": "user", "file": fixture.root.path().join("other.toml") }, "version": "other" },
            { "name": { "type": "user", "file": config_path }, "version": "active" }
        ]
    });
    let selected = user_config_layer(response, &config_path).unwrap();
    assert_eq!(selected.file_path, config_path.to_string_lossy());
    assert_eq!(selected.version, "active");

    let duplicate_path_layers = json!({
        "layers": [
            { "name": { "type": "user", "file": config_path }, "version": "v1" },
            { "name": { "type": "user", "file": config_path }, "version": "v2" }
        ]
    });
    assert_eq!(user_config_layer(duplicate_path_layers, &config_path).unwrap().version, "v1");
}

#[test]
fn quotes_an_opaque_hook_key_as_one_toml_segment() {
    let key = "path.with.dot:\"quote\"\\slash\nline";
    assert_eq!(
        codex_trust_key_path(key).unwrap(),
        r#"hooks.state."path.with.dot:\"quote\"\\slash\nline".trusted_hash"#
    );
}

#[test]
fn config_write_response_must_match_the_path_status_and_version() {
    let expected = Path::new("/active/config.toml");
    for response in [
        json!(null),
        json!({}),
        json!({ "filePath": "/wrong/config.toml", "status": "ok", "version": "v2" }),
        json!({ "filePath": expected, "status": "unexpected", "version": "v2" }),
        json!({ "filePath": expected, "status": "ok", "version": "" }),
    ] {
        assert!(validate_config_write(response, expected).is_err());
    }
}

#[test]
fn jsonl_client_performs_handshake_and_accepts_versionless_responses() {
    let transport =
        FakeTransport::from_results([json!({ "id": 1, "result": {} }), json!({ "id": 2, "result": { "data": [] } })]);
    let writes = transport.writes.clone();
    let mut server = JsonlAppServer::connect(transport).unwrap();
    assert_eq!(server.request("hooks/list", json!({})).unwrap(), json!({ "data": [] }));
    let writes = locked(&writes);
    assert_eq!(writes[0]["method"], "initialize");
    assert_eq!(writes[1], json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }));
    assert_eq!(writes[2]["id"], 2);
}

#[test]
fn jsonl_client_rejects_timeout_malformed_and_oversized_responses() {
    let timeout = FakeTransport::new([FakeRead::Json(ok(1, json!({}))), FakeRead::Timeout]);
    let mut server = JsonlAppServer::connect(timeout).unwrap();
    assert!(server.request("hooks/list", json!({})).unwrap_err().to_string().contains("timed out"));

    let malformed = FakeTransport::new([FakeRead::Raw(b"not-json".to_vec())]);
    let error = JsonlAppServer::connect(malformed).err().unwrap();
    assert!(error.to_string().contains("malformed JSON"));

    let oversized = vec![b'x'; MAX_JSONL_BYTES + 1];
    assert!(parse_jsonrpc_line(&oversized).unwrap_err().to_string().contains("size limit"));
    assert!(read_bounded_line(&mut Cursor::new([oversized, vec![b'\n']].concat())).is_err());
}

#[test]
fn version_conflicts_are_retryable_only_for_config_batch_write() {
    let conflict = json!({
        "code": -32600,
        "data": { "config_write_error_code": "configVersionConflict" }
    });
    let transport = FakeTransport::from_results([ok(1, json!({})), error(2, conflict.clone())]);
    let mut server = JsonlAppServer::connect(transport).unwrap();
    assert!(!server.request("hooks/list", json!({})).unwrap_err().version_conflict);

    let transport = FakeTransport::from_results([ok(1, json!({})), error(2, conflict)]);
    let mut server = JsonlAppServer::connect(transport).unwrap();
    assert!(server.request("config/batchWrite", json!({})).unwrap_err().version_conflict);
}

fn assert_inventory_error(fixture: &Fixture, response: Value, expected: &str) {
    let server = FakeTransport::from_results([ok(1, json!({})), ok(2, response)]);
    let mut runtime = FakeRuntime::new([server]);
    let error = inspect_codex_hook_trust_with(&mut runtime, fixture.hooks_path.clone()).unwrap_err();
    assert!(error.to_string().contains(expected), "{error}");
}

struct Fixture {
    root: TempDir,
    hooks_path: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        let hooks_path = root.path().join("hooks.json");
        Self { root, hooks_path }
    }

    fn config_path(&self) -> PathBuf {
        self.root.path().join("config.toml")
    }
}

fn hooks_response(path: &Path, trust: &str, hash_prefix: &str) -> Value {
    let hooks = hook_specs(Client::Codex)
        .iter()
        .enumerate()
        .map(|(index, spec)| {
            json!({
                "key": format!("key.{}.\"quoted\"", spec.event),
                "currentHash": format!("{hash_prefix}-{}", spec.event),
                "displayOrder": index,
                "enabled": true,
                "eventName": codex_event_name(spec.event).unwrap(),
                "handlerType": "command",
                "isManaged": false,
                "source": "user",
                "sourcePath": path,
                "command": spec.command,
                "matcher": spec.matcher,
                "timeoutSec": spec.timeout,
                "additionalContextLimit": spec.additional_context_limit,
                "trustStatus": trust,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "data": [{ "cwd": path.parent().unwrap(), "errors": [], "warnings": [], "hooks": hooks }]
    })
}

fn config_response(path: &Path, version: &str) -> Value {
    json!({
        "layers": [{ "name": { "type": "user", "file": path }, "version": version }]
    })
}

fn ok(id: u64, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error(id: u64, error: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": error })
}

struct FakeRuntime {
    servers: VecDeque<FakeTransport>,
    connects: usize,
    version: String,
}

impl FakeRuntime {
    fn new(servers: impl IntoIterator<Item = FakeTransport>) -> Self {
        Self { servers: servers.into_iter().collect(), connects: 0, version: "codex-cli 0.146.1".to_owned() }
    }

    fn with_version(mut self, version: &str) -> Self {
        self.version = version.to_owned();
        self
    }
}

impl Runtime for FakeRuntime {
    fn codex_version(&mut self) -> Result<String> {
        Ok(self.version.clone())
    }

    fn connect(&mut self) -> Result<Box<dyn RpcClient>> {
        self.connects += 1;
        let transport =
            self.servers.pop_front().ok_or_else(|| CodexTrustError::new("unexpected fake server connection"))?;
        Ok(Box::new(JsonlAppServer::connect(transport)?))
    }
}

enum FakeRead {
    Json(Value),
    Raw(Vec<u8>),
    Timeout,
    Eof,
}

struct FakeTransport {
    reads: VecDeque<FakeRead>,
    writes: Arc<Mutex<Vec<Value>>>,
}

impl FakeTransport {
    fn new(reads: impl IntoIterator<Item = FakeRead>) -> Self {
        Self { reads: reads.into_iter().collect(), writes: Arc::new(Mutex::new(Vec::new())) }
    }

    fn from_results(results: impl IntoIterator<Item = Value>) -> Self {
        Self::new(results.into_iter().map(FakeRead::Json))
    }
}

impl JsonlTransport for FakeTransport {
    fn write_line(&mut self, line: &[u8]) -> Result<()> {
        let value = serde_json::from_slice(line)
            .map_err(|error| CodexTrustError::new(format!("fake received malformed JSON: {error}")))?;
        self.writes.lock().unwrap().push(value);
        Ok(())
    }

    fn read_line(&mut self, _timeout: Duration) -> Result<ReadOutcome> {
        match self.reads.pop_front().unwrap_or(FakeRead::Eof) {
            FakeRead::Json(value) => Ok(ReadOutcome::Line(serde_json::to_vec(&value).unwrap())),
            FakeRead::Raw(line) => Ok(ReadOutcome::Line(line)),
            FakeRead::Timeout => Ok(ReadOutcome::Timeout),
            FakeRead::Eof => Ok(ReadOutcome::Eof),
        }
    }
}

fn locked<T>(mutex: &Arc<Mutex<T>>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap()
}
