use std::{
    collections::{BTreeMap, HashSet},
    env, fmt, fs,
    io::{self, BufRead, BufReader, Write},
    path::{Component, Path, PathBuf},
    process::{Child, ChildStdin, Command, ExitStatus, Stdio},
    sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use serde::Deserialize;
use serde_json::{Map, Value, json};

use super::{
    config::default_hook_path,
    specs::{Client, HookSpec, hook_specs},
};

const CODEX_TIMEOUT: Duration = Duration::from_secs(10);
const CODEX_MINIMUM_VERSION: (u64, u64, u64) = (0, 146, 0);
const CODEX_MINIMUM_VERSION_TEXT: &str = "0.146.0";
const MAX_CONFIG_ATTEMPTS: usize = 3;
const MAX_JSONL_BYTES: usize = 1024 * 1024;
const MAX_QUEUED_LINES: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TrustOutcome {
    Updated,
    Unchanged,
    Skipped,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HookCheck {
    pub(crate) ok: bool,
    pub(crate) path: PathBuf,
    pub(crate) error: Option<String>,
    pub(crate) details: Value,
}

#[derive(Debug)]
pub(crate) struct CodexTrustError {
    message: String,
    version_conflict: bool,
}

impl CodexTrustError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into(), version_conflict: false }
    }

    fn version_conflict() -> Self {
        Self { message: "Codex config version conflict".to_owned(), version_conflict: true }
    }
}

impl fmt::Display for CodexTrustError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CodexTrustError {}

type Result<T> = std::result::Result<T, CodexTrustError>;

pub(crate) fn trust_codex_hooks(path: Option<&Path>, dry_run: bool) -> Result<TrustOutcome> {
    let active = active_codex_hook_path(path)?;
    if dry_run {
        return Ok(TrustOutcome::Skipped);
    }
    trust_codex_hooks_with(&mut CodexRuntime, &active)
}

pub(crate) fn inspect_codex_hook_trust(path: Option<&Path>) -> HookCheck {
    let requested = path.map(Path::to_path_buf).unwrap_or_else(|| default_hook_path(Client::Codex));
    let result = (|| {
        let active = active_codex_hook_path(path)?;
        inspect_codex_hook_trust_with(&mut CodexRuntime, active)
    })();
    result.unwrap_or_else(|error| HookCheck {
        ok: false,
        path: requested,
        error: Some(error.to_string()),
        details: json!({}),
    })
}

fn trust_codex_hooks_with(runtime: &mut dyn Runtime, hooks_path: &Path) -> Result<TrustOutcome> {
    require_codex_minimum_version(&runtime.codex_version()?)?;
    let mut last_conflict = None;
    for _ in 0..MAX_CONFIG_ATTEMPTS {
        match trust_once(runtime, hooks_path) {
            Ok(AttemptOutcome::Unchanged) => return Ok(TrustOutcome::Unchanged),
            Ok(AttemptOutcome::Written(expected_hashes)) => {
                let mut verifier = runtime.connect()?;
                let verified = owned_codex_hooks(verifier.request("hooks/list", json!({}))?, hooks_path)?;
                let converged = expected_hashes.iter().all(|(key, expected_hash)| {
                    verified
                        .get(key)
                        .is_some_and(|hook| hook.trust_status == "trusted" && hook.current_hash == *expected_hash)
                });
                if converged {
                    return Ok(TrustOutcome::Updated);
                }
                return Err(CodexTrustError::new("Codex did not verify the submitted hook trust state"));
            }
            Err(error) if error.version_conflict => last_conflict = Some(error),
            Err(error) => return Err(error),
        }
    }
    Err(last_conflict.unwrap_or_else(|| CodexTrustError::new("Codex hook trust did not converge")))
}

fn trust_once(runtime: &mut dyn Runtime, hooks_path: &Path) -> Result<AttemptOutcome> {
    let mut server = runtime.connect()?;
    let hooks = owned_codex_hooks(server.request("hooks/list", json!({}))?, hooks_path)?;
    if hooks.values().all(|hook| hook.trust_status == "trusted") {
        return Ok(AttemptOutcome::Unchanged);
    }

    let config_path = hooks_path
        .parent()
        .ok_or_else(|| CodexTrustError::new("active Codex hooks path has no parent"))?
        .join("config.toml");
    let config = user_config_layer(server.request("config/read", json!({ "includeLayers": true }))?, &config_path)?;
    let edits = hooks
        .iter()
        .map(|(key, hook)| {
            Ok(json!({
                "keyPath": codex_trust_key_path(key)?,
                "value": hook.current_hash,
                "mergeStrategy": "upsert",
            }))
        })
        .collect::<Result<Vec<_>>>()?;
    let response = server.request(
        "config/batchWrite",
        json!({
            "edits": edits,
            "expectedVersion": config.version,
            "filePath": config.file_path,
        }),
    )?;
    validate_config_write(response, Path::new(&config.file_path))?;
    Ok(AttemptOutcome::Written(hooks.into_iter().map(|(key, hook)| (key, hook.current_hash)).collect()))
}

fn inspect_codex_hook_trust_with(runtime: &mut dyn Runtime, hooks_path: PathBuf) -> Result<HookCheck> {
    require_codex_minimum_version(&runtime.codex_version()?)?;
    let mut server = runtime.connect()?;
    let hooks = owned_codex_hooks(server.request("hooks/list", json!({}))?, &hooks_path)?;
    let mut hook_details = Map::new();
    let mut untrusted = Vec::new();
    for (key, hook) in &hooks {
        hook_details.insert(key.clone(), json!({ "hash": hook.current_hash, "trust": hook.trust_status }));
        if hook.trust_status != "trusted" {
            untrusted.push(Value::String(key.clone()));
        }
    }
    let details = if untrusted.is_empty() {
        json!({ "hooks": hook_details })
    } else {
        json!({ "hooks": hook_details, "untrusted": untrusted })
    };
    Ok(HookCheck {
        ok: untrusted.is_empty(),
        path: hooks_path,
        error: (!untrusted.is_empty()).then(|| "owned Codex hooks are not trusted".to_owned()),
        details,
    })
}

enum AttemptOutcome {
    Unchanged,
    Written(BTreeMap<String, String>),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HookMetadata {
    key: String,
    current_hash: String,
    #[serde(rename = "displayOrder")]
    _display_order: i64,
    enabled: bool,
    event_name: String,
    handler_type: String,
    is_managed: bool,
    source: String,
    source_path: String,
    command: Option<String>,
    matcher: Option<String>,
    timeout_sec: u64,
    additional_context_limit: Option<u64>,
    trust_status: String,
}

#[derive(Deserialize)]
struct HooksListResponse {
    data: Vec<HooksListEntry>,
}

#[derive(Deserialize)]
struct HooksListEntry {
    cwd: String,
    errors: Vec<Value>,
    warnings: Vec<String>,
    hooks: Vec<HookMetadata>,
}

fn owned_codex_hooks(response: Value, hooks_path: &Path) -> Result<BTreeMap<String, HookMetadata>> {
    let response: HooksListResponse =
        serde_json::from_value(response).map_err(|_| CodexTrustError::new("malformed hooks/list response"))?;
    let specs = hook_specs(Client::Codex);
    let mut by_event = BTreeMap::new();
    for entry in response.data {
        if !entry.errors.is_empty() {
            return Err(CodexTrustError::new("Codex reported hook loading errors"));
        }
        let (_cwd, _warnings) = (entry.cwd, entry.warnings);
        for hook in entry.hooks {
            let Some(spec) = matching_codex_spec(&hook, hooks_path, specs)? else {
                continue;
            };
            if by_event.insert(spec.event, hook).is_some() {
                return Err(CodexTrustError::new(format!("duplicate owned Codex hook: {}", spec.event)));
            }
        }
    }
    let missing =
        specs.iter().filter(|spec| !by_event.contains_key(spec.event)).map(|spec| spec.event).collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(CodexTrustError::new(format!("missing exact Codex hooks: {}", missing.join(", "))));
    }

    let mut keys = HashSet::new();
    let mut result = BTreeMap::new();
    for hook in by_event.into_values() {
        if !keys.insert(hook.key.clone()) {
            return Err(CodexTrustError::new("duplicate Codex hook key"));
        }
        result.insert(hook.key.clone(), hook);
    }
    Ok(result)
}

fn matching_codex_spec<'a>(
    hook: &HookMetadata,
    hooks_path: &Path,
    specs: &'a [HookSpec],
) -> Result<Option<&'a HookSpec>> {
    if normalized_path(Path::new(&hook.source_path))? != normalized_path(hooks_path)? {
        return Ok(None);
    }
    Ok(specs.iter().find(|spec| {
        hook.event_name == codex_event_name(spec.event).unwrap_or("") &&
            hook.enabled &&
            !hook.is_managed &&
            hook.source == "user" &&
            hook.handler_type == "command" &&
            hook.command.as_deref() == Some(spec.command) &&
            hook.matcher.as_deref() == spec.matcher &&
            Some(hook.timeout_sec) == spec.timeout &&
            hook.additional_context_limit == spec.additional_context_limit &&
            !hook.key.is_empty() &&
            !hook.current_hash.is_empty() &&
            matches!(hook.trust_status.as_str(), "trusted" | "untrusted" | "modified")
    }))
}

fn codex_event_name(event: &str) -> Option<&'static str> {
    match event {
        "SessionStart" => Some("sessionStart"),
        "UserPromptSubmit" => Some("userPromptSubmit"),
        "Stop" => Some("stop"),
        "SessionEnd" => Some("sessionEnd"),
        "SubagentStart" => Some("subagentStart"),
        "SubagentStop" => Some("subagentStop"),
        "PostToolUse" => Some("postToolUse"),
        _ => None,
    }
}

struct UserConfigLayer {
    file_path: String,
    version: String,
}

fn user_config_layer(response: Value, expected_path: &Path) -> Result<UserConfigLayer> {
    let layers = response
        .as_object()
        .and_then(|object| object.get("layers"))
        .and_then(Value::as_array)
        .ok_or_else(|| CodexTrustError::new("malformed config/read response"))?;
    let expected = normalized_path(expected_path)?;
    for layer in layers {
        let object = layer.as_object().ok_or_else(|| CodexTrustError::new("malformed config/read response"))?;
        let name = object
            .get("name")
            .and_then(Value::as_object)
            .ok_or_else(|| CodexTrustError::new("malformed config/read response"))?;
        if name.get("type").and_then(Value::as_str) != Some("user") {
            continue;
        }
        let Some(file_path) = name.get("file").and_then(Value::as_str) else {
            continue;
        };
        if normalized_path(Path::new(file_path))? != expected {
            continue;
        }
        let version = object.get("version").and_then(Value::as_str).unwrap_or_default();
        if !version.is_empty() {
            return Ok(UserConfigLayer { file_path: file_path.to_owned(), version: version.to_owned() });
        }
    }
    Err(CodexTrustError::new(format!("missing active Codex user config layer: {}", expected.display())))
}

fn codex_trust_key_path(key: &str) -> Result<String> {
    let quoted = serde_json::to_string(key)
        .map_err(|error| CodexTrustError::new(format!("could not quote Codex hook key: {error}")))?;
    Ok(format!("hooks.state.{quoted}.trusted_hash"))
}

fn validate_config_write(response: Value, expected_path: &Path) -> Result<()> {
    let object = response.as_object().ok_or_else(|| CodexTrustError::new("malformed config/batchWrite response"))?;
    let file_path = object.get("filePath").and_then(Value::as_str).unwrap_or_default();
    let status = object.get("status").and_then(Value::as_str).unwrap_or_default();
    let version = object.get("version").and_then(Value::as_str).unwrap_or_default();
    let path_matches =
        !file_path.is_empty() && normalized_path(Path::new(file_path))? == normalized_path(expected_path)?;
    if !path_matches || !matches!(status, "ok" | "okOverridden") || version.is_empty() {
        return Err(CodexTrustError::new("malformed config/batchWrite response"));
    }
    Ok(())
}

fn require_codex_minimum_version(output: &str) -> Result<()> {
    let version_text = output
        .trim()
        .strip_prefix("codex-cli ")
        .ok_or_else(|| CodexTrustError::new("could not parse `codex --version` output"))?;
    let (without_build, build) = split_optional(version_text, '+')?;
    if let Some(build) = build {
        validate_identifiers(build)?;
    }
    let (numbers, prerelease) = split_optional(without_build, '-')?;
    if let Some(prerelease) = prerelease {
        validate_identifiers(prerelease)?;
    }
    let mut parts = numbers.split('.');
    let version =
        (parse_version_part(parts.next())?, parse_version_part(parts.next())?, parse_version_part(parts.next())?);
    if parts.next().is_some() {
        return Err(CodexTrustError::new("could not parse `codex --version` output"));
    }
    if version < CODEX_MINIMUM_VERSION || (version == CODEX_MINIMUM_VERSION && prerelease.is_some()) {
        return Err(CodexTrustError::new(format!(
            "Codex hook trust requires codex-cli >= {CODEX_MINIMUM_VERSION_TEXT}; found {version_text}"
        )));
    }
    Ok(())
}

fn split_optional(text: &str, separator: char) -> Result<(&str, Option<&str>)> {
    let (before, after) = text.split_once(separator).map_or((text, None), |(before, after)| (before, Some(after)));
    if before.is_empty() || after == Some("") {
        return Err(CodexTrustError::new("could not parse `codex --version` output"));
    }
    Ok((before, after))
}

fn validate_identifiers(text: &str) -> Result<()> {
    if text
        .split('.')
        .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'))
    {
        return Err(CodexTrustError::new("could not parse `codex --version` output"));
    }
    Ok(())
}

fn parse_version_part(part: Option<&str>) -> Result<u64> {
    let part = part
        .filter(|part| !part.is_empty())
        .ok_or_else(|| CodexTrustError::new("could not parse `codex --version` output"))?;
    if !part.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(CodexTrustError::new("could not parse `codex --version` output"));
    }
    part.parse().map_err(|_| CodexTrustError::new("could not parse `codex --version` output"))
}

trait Runtime {
    fn codex_version(&mut self) -> Result<String>;
    fn connect(&mut self) -> Result<Box<dyn RpcClient>>;
}

struct CodexRuntime;

impl Runtime for CodexRuntime {
    fn codex_version(&mut self) -> Result<String> {
        let mut child = Command::new("codex")
            .arg("--version")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| CodexTrustError::new(format!("could not determine Codex version: {error}")))?;
        let status = wait_for_child(&mut child, CODEX_TIMEOUT)
            .map_err(|error| CodexTrustError::new(format!("could not determine Codex version: {error}")))?;
        if status.is_none() {
            let _ = child.kill();
        }
        let output = child
            .wait_with_output()
            .map_err(|error| CodexTrustError::new(format!("could not determine Codex version: {error}")))?;
        let Some(status) = status else {
            return Err(CodexTrustError::new("could not determine Codex version: timed out"));
        };
        if !status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            let detail = if stderr.is_empty() { format!("exit {}", status.code().unwrap_or(-1)) } else { stderr };
            return Err(CodexTrustError::new(format!("could not determine Codex version: {detail}")));
        }
        String::from_utf8(output.stdout).map_err(|_| CodexTrustError::new("could not parse `codex --version` output"))
    }

    fn connect(&mut self) -> Result<Box<dyn RpcClient>> {
        Ok(Box::new(JsonlAppServer::connect(ChildTransport::spawn()?)?))
    }
}

trait RpcClient {
    fn request(&mut self, method: &str, params: Value) -> Result<Value>;
}

struct JsonlAppServer<T: JsonlTransport> {
    transport: T,
    next_id: u64,
}

impl<T: JsonlTransport> JsonlAppServer<T> {
    fn connect(transport: T) -> Result<Self> {
        let mut server = Self { transport, next_id: 1 };
        let initialized = server.request(
            "initialize",
            json!({
                "clientInfo": {
                    "name": "ai-coord",
                    "version": env!("CARGO_PKG_VERSION"),
                }
            }),
        )?;
        if !initialized.is_object() {
            return Err(CodexTrustError::new("malformed initialize response"));
        }
        server.notify("initialized", json!({}))?;
        Ok(server)
    }

    fn notify(&mut self, method: &str, params: Value) -> Result<()> {
        self.write_message(&json!({ "jsonrpc": "2.0", "method": method, "params": params }))
    }

    fn write_message(&mut self, message: &Value) -> Result<()> {
        let mut encoded = serde_json::to_vec(message)
            .map_err(|error| CodexTrustError::new(format!("could not encode JSON-RPC: {error}")))?;
        encoded.push(b'\n');
        if encoded.len() > MAX_JSONL_BYTES {
            return Err(CodexTrustError::new("Codex app-server request exceeded size limit"));
        }
        self.transport.write_line(&encoded)
    }
}

impl<T: JsonlTransport> RpcClient for JsonlAppServer<T> {
    fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        let request_id = self.next_id;
        self.next_id += 1;
        self.write_message(&json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method,
            "params": params,
        }))?;
        let deadline = Instant::now() + CODEX_TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(CodexTrustError::new("Codex app-server response timed out"));
            }
            let line = match self.transport.read_line(remaining)? {
                ReadOutcome::Line(line) => line,
                ReadOutcome::Timeout => {
                    return Err(CodexTrustError::new("Codex app-server response timed out"));
                }
                ReadOutcome::Eof => {
                    return Err(CodexTrustError::new("Codex app-server closed stdout"));
                }
            };
            let response = parse_jsonrpc_line(&line)?;
            if response.get("id") != Some(&json!(request_id)) {
                continue;
            }
            if let Some(error) = response.get("error") {
                if method == "config/batchWrite" && is_config_version_conflict(error) {
                    return Err(CodexTrustError::version_conflict());
                }
                return Err(CodexTrustError::new(format!("Codex {method} failed: {error}")));
            }
            return response
                .get("result")
                .cloned()
                .ok_or_else(|| CodexTrustError::new(format!("malformed {method} response")));
        }
    }
}

fn parse_jsonrpc_line(line: &[u8]) -> Result<Map<String, Value>> {
    if line.len() > MAX_JSONL_BYTES {
        return Err(CodexTrustError::new("Codex app-server response exceeded size limit"));
    }
    let response: Value =
        serde_json::from_slice(line).map_err(|_| CodexTrustError::new("Codex app-server emitted malformed JSON"))?;
    let object =
        response.as_object().ok_or_else(|| CodexTrustError::new("Codex app-server emitted malformed JSON-RPC"))?;
    if !object.get("jsonrpc").is_none_or(|version| version == "2.0") ||
        (!object.contains_key("id") && !object.contains_key("method"))
    {
        return Err(CodexTrustError::new("Codex app-server emitted malformed JSON-RPC"));
    }
    Ok(object.clone())
}

fn is_config_version_conflict(error: &Value) -> bool {
    error.get("code").and_then(Value::as_i64) == Some(-32600) &&
        error.get("data").and_then(|data| data.get("config_write_error_code")).and_then(Value::as_str) ==
            Some("configVersionConflict")
}

trait JsonlTransport {
    fn write_line(&mut self, line: &[u8]) -> Result<()>;
    fn read_line(&mut self, timeout: Duration) -> Result<ReadOutcome>;
}

enum ReadOutcome {
    Line(Vec<u8>),
    Timeout,
    Eof,
}

struct ChildTransport {
    child: Child,
    stdin: Option<ChildStdin>,
    reader: Receiver<ReaderEvent>,
    reader_thread: Option<JoinHandle<()>>,
}

impl ChildTransport {
    fn spawn() -> Result<Self> {
        let mut child = Command::new("codex")
            .args(["app-server", "--stdio"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| CodexTrustError::new(format!("could not start Codex app-server: {error}")))?;
        let stdin = child.stdin.take().ok_or_else(|| CodexTrustError::new("Codex app-server stdin is unavailable"))?;
        let stdout =
            child.stdout.take().ok_or_else(|| CodexTrustError::new("Codex app-server stdout is unavailable"))?;
        let (sender, receiver) = mpsc::sync_channel(MAX_QUEUED_LINES);
        let reader_thread = thread::spawn(move || pump_stdout(BufReader::new(stdout), sender));
        Ok(Self { child, stdin: Some(stdin), reader: receiver, reader_thread: Some(reader_thread) })
    }
}

impl JsonlTransport for ChildTransport {
    fn write_line(&mut self, line: &[u8]) -> Result<()> {
        let stdin = self.stdin.as_mut().ok_or_else(|| CodexTrustError::new("Codex app-server stdin is unavailable"))?;
        stdin
            .write_all(line)
            .and_then(|()| stdin.flush())
            .map_err(|error| CodexTrustError::new(format!("could not write Codex app-server request: {error}")))
    }

    fn read_line(&mut self, timeout: Duration) -> Result<ReadOutcome> {
        match self.reader.recv_timeout(timeout) {
            Ok(ReaderEvent::Line(line)) => Ok(ReadOutcome::Line(line)),
            Ok(ReaderEvent::Eof) => Ok(ReadOutcome::Eof),
            Ok(ReaderEvent::Error(error)) => Err(CodexTrustError::new(error)),
            Err(RecvTimeoutError::Timeout) => Ok(ReadOutcome::Timeout),
            Err(RecvTimeoutError::Disconnected) => Ok(ReadOutcome::Eof),
        }
    }
}

impl Drop for ChildTransport {
    fn drop(&mut self) {
        self.stdin.take();
        if wait_for_child(&mut self.child, CODEX_TIMEOUT).ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        if let Some(reader_thread) = self.reader_thread.take() {
            let _ = reader_thread.join();
        }
    }
}

enum ReaderEvent {
    Line(Vec<u8>),
    Eof,
    Error(String),
}

fn pump_stdout<R: BufRead>(mut reader: R, sender: SyncSender<ReaderEvent>) {
    loop {
        let event = match read_bounded_line(&mut reader) {
            Ok(Some(line)) => ReaderEvent::Line(line),
            Ok(None) => ReaderEvent::Eof,
            Err(error) => ReaderEvent::Error(error.to_string()),
        };
        let terminal = !matches!(event, ReaderEvent::Line(_));
        match sender.try_send(event) {
            Ok(()) => {}
            Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => return,
        }
        if terminal {
            return;
        }
    }
}

fn read_bounded_line<R: BufRead>(reader: &mut R) -> io::Result<Option<Vec<u8>>> {
    let mut line = Vec::new();
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok((!line.is_empty()).then_some(line));
        }
        let consumed =
            available.iter().position(|byte| *byte == b'\n').map_or(available.len(), |position| position + 1);
        if line.len() + consumed > MAX_JSONL_BYTES {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "JSONL line exceeded size limit"));
        }
        line.extend_from_slice(&available[..consumed]);
        reader.consume(consumed);
        if line.last() == Some(&b'\n') {
            line.pop();
            return Ok(Some(line));
        }
    }
}

fn wait_for_child(child: &mut Child, timeout: Duration) -> io::Result<Option<ExitStatus>> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(None);
        }
        thread::sleep(remaining.min(Duration::from_millis(10)));
    }
}

fn active_codex_hook_path(path: Option<&Path>) -> Result<PathBuf> {
    let active = normalized_path(&default_hook_path(Client::Codex))?;
    let selected = normalized_path(path.unwrap_or(&active))?;
    if selected != active {
        return Err(CodexTrustError::new(format!("Codex hooks path must be the active source: {}", active.display())));
    }
    Ok(active)
}

fn expand_tilde(path: &Path) -> Result<PathBuf> {
    let text = path.as_os_str().to_string_lossy();
    if text == "~" || text.starts_with("~/") {
        let home = env::var_os("HOME")
            .filter(|value| !value.is_empty())
            .ok_or_else(|| CodexTrustError::new("could not determine Codex home"))?;
        return Ok(PathBuf::from(home).join(text.trim_start_matches("~/")));
    }
    Ok(path.to_path_buf())
}

fn normalized_path(path: &Path) -> Result<PathBuf> {
    let expanded = expand_tilde(path)?;
    let absolute = if expanded.is_absolute() {
        expanded
    } else {
        env::current_dir()
            .map_err(|error| CodexTrustError::new(format!("could not resolve path: {error}")))?
            .join(expanded)
    };
    for ancestor in absolute.ancestors() {
        if let Ok(canonical) = fs::canonicalize(ancestor) {
            let suffix = absolute
                .strip_prefix(ancestor)
                .map_err(|error| CodexTrustError::new(format!("could not resolve path: {error}")))?;
            return Ok(lexically_normalized(canonical.join(suffix)));
        }
    }
    Ok(lexically_normalized(absolute))
}

fn lexically_normalized(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

#[cfg(test)]
#[path = "trust/tests.rs"]
mod tests;
