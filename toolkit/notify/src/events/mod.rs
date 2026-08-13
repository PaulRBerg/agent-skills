//! Portable Claude Code and Codex event handling.
//!
//! Handlers consume JSON values, a small session-state view, and a notification
//! sink. This keeps event parsing testable on every platform and lets the binary
//! choose the SQLite and macOS implementations.

use std::{env, path::Path};

use serde_json::{Map, Value};

use crate::{
    config::AppConfig,
    error::{AppError, Result},
    filters::{
        should_send_codex_notification, should_send_completion_notification, should_send_failure_notification,
        should_send_permission_notification,
    },
    model::Client,
    notifier::{
        NotificationSink, completion_notification, failure_notification, permission_notification, question_notification,
    },
};

const CODEX_EVENT_TYPE: &str = "agent-turn-complete";
const DEFAULT_FAILURE_MESSAGE: &str = "Claude Code API error";
const DEFAULT_QUESTION: &str = "Claude is asking a question";

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct JobInfo {
    pub job_number: Option<i64>,
    pub duration_seconds: Option<i64>,
    pub prompt: Option<String>,
}

/// Minimal state required by Claude hook handlers.
pub trait SessionState {
    fn track_prompt(&self, session_id: &str, prompt: &str, cwd: &str);
    fn mark_stopped(&self, session_id: &str);
    fn mark_waiting(&self, session_id: &str);
    fn active_prompt(&self, session_id: &str) -> Option<String>;
    fn job_info(&self, session_id: &str) -> JobInfo;
    fn cleanup_if_due(&self);
}

/// Parses a JSON object and applies the common CLI payload validation.
pub fn parse_payload(payload: &str) -> Result<Value> {
    let value =
        serde_json::from_str(payload).map_err(|error| AppError::usage(format!("Failed to parse JSON: {error}")))?;
    validate_payload(&value)?;
    Ok(value)
}

/// Rejects non-object payloads, traversal-bearing cwd values, and invalid session IDs.
pub fn validate_payload(payload: &Value) -> Result<()> {
    let object = payload.as_object().ok_or_else(|| AppError::usage("JSON payload must be an object"))?;
    if let Some(cwd) = object.get("cwd") {
        let cwd = cwd.as_str().ok_or_else(|| AppError::usage("cwd must be a string"))?;
        if cwd.contains("..") {
            return Err(AppError::usage("Path traversal detected in cwd"));
        }
    }
    if let Some(session_id) = object.get("session_id") {
        let session_id = session_id.as_str().ok_or_else(|| AppError::usage("session_id must be a string"))?;
        if session_id.is_empty() || session_id.len() > 255 {
            return Err(AppError::usage("Invalid session_id"));
        }
    }
    Ok(())
}

/// Records user intent, except for Claude's internal subagent completion envelopes.
pub fn handle_user_prompt(payload: &Value, state: &impl SessionState) -> Result<()> {
    validate_payload(payload)?;
    let object = object(payload)?;
    let session_id = required_session_id(object)?;
    let prompt = string(object, "prompt");
    if !is_internal_agent_notification(prompt) {
        state.track_prompt(session_id, prompt, string(object, "cwd"));
    }
    Ok(())
}

/// Marks a completed turn and emits a filtered completion notification.
pub fn handle_stop(
    payload: &Value,
    state: &impl SessionState,
    config: &AppConfig,
    notifier: &mut impl NotificationSink,
) -> Result<()> {
    validate_payload(payload)?;
    let object = object(payload)?;
    let session_id = required_session_id(object)?;
    if nonempty_collection(object.get("background_tasks")) || nonempty_collection(object.get("session_crons")) {
        return Ok(());
    }
    state.mark_stopped(session_id);
    let job = state.job_info(session_id);
    if let (Some(duration), Some(prompt)) =
        (job.duration_seconds.and_then(|value| u64::try_from(value).ok()), job.prompt) &&
        should_send_completion_notification(&prompt, duration, config)
    {
        let notification = completion_notification(
            Client::Claude,
            project_name(string(object, "cwd")).as_str(),
            &prompt,
            string(object, "last_assistant_message"),
            Some(&format_duration(duration)),
        );
        let _ = notifier.deliver(&notification);
    }
    state.cleanup_if_due();
    Ok(())
}

/// Records a failed turn and emits an all-mode-only failure notification.
pub fn handle_stop_failure(
    payload: &Value,
    state: &impl SessionState,
    config: &AppConfig,
    notifier: &mut impl NotificationSink,
) -> Result<()> {
    validate_payload(payload)?;
    let object = object(payload)?;
    let session_id = required_session_id(object)?;
    let prompt = state.active_prompt(session_id);
    state.mark_stopped(session_id);
    let job = prompt.as_ref().map(|_| state.job_info(session_id));
    if should_send_failure_notification(config) {
        let duration = job
            .as_ref()
            .and_then(|job| job.duration_seconds)
            .and_then(|value| u64::try_from(value).ok())
            .map(format_duration);
        let notification = failure_notification(
            project_name(string(object, "cwd")).as_str(),
            prompt.as_deref().unwrap_or_default(),
            &failure_message(object),
            duration.as_deref(),
        );
        let _ = notifier.deliver(&notification);
    }
    Ok(())
}

/// Tracks Claude's waiting state without issuing a duplicate notification.
pub fn handle_notification(payload: &Value, state: &impl SessionState) -> Result<()> {
    validate_payload(payload)?;
    let object = object(payload)?;
    let session_id = required_session_id(object)?;
    let message = string(object, "message");
    let notification_type = string(object, "notification_type");
    if notification_type == "idle_prompt" ||
        ["waiting for input", "waiting for user", "approval needed"]
            .iter()
            .any(|keyword| message.to_ascii_lowercase().contains(keyword))
    {
        state.mark_waiting(session_id);
    }
    Ok(())
}

/// Emits a permission-request notification when enabled; sessions are optional for this event.
pub fn handle_permission(
    payload: &Value,
    state: &impl SessionState,
    config: &AppConfig,
    notifier: &mut impl NotificationSink,
) -> Result<()> {
    validate_payload(payload)?;
    if !should_send_permission_notification(config) {
        return Ok(());
    }
    let object = object(payload)?;
    let task = optional_session_id(object).and_then(|session_id| state.active_prompt(session_id)).unwrap_or_default();
    let notification = permission_notification(
        project_name(string(object, "cwd")).as_str(),
        &task,
        &permission_request(object.get("tool_name"), object.get("tool_input")),
    );
    let _ = notifier.deliver(&notification);
    Ok(())
}

/// Emits an AskUserQuestion notification when enabled; sessions are optional for this event.
pub fn handle_ask_user_question(
    payload: &Value,
    state: &impl SessionState,
    config: &AppConfig,
    notifier: &mut impl NotificationSink,
) -> Result<()> {
    validate_payload(payload)?;
    if !should_send_permission_notification(config) {
        return Ok(());
    }
    let object = object(payload)?;
    let task = optional_session_id(object).and_then(|session_id| state.active_prompt(session_id)).unwrap_or_default();
    let notification = question_notification(
        project_name(string(object, "cwd")).as_str(),
        &task,
        &question_text(object.get("tool_input")),
    );
    let _ = notifier.deliver(&notification);
    Ok(())
}

/// Handles Codex's completion callback. Codex does not read or mutate session state.
pub fn handle_codex_notify(payload: &Value, config: &AppConfig, notifier: &mut impl NotificationSink) -> Result<()> {
    validate_payload(payload)?;
    let object = object(payload)?;
    if let Some(event) = first_value(object, &["type", "event"]).and_then(Value::as_str) &&
        !event.is_empty() &&
        event != CODEX_EVENT_TYPE
    {
        return Ok(());
    }
    let prompt = extract_last_user_message(first_value(object, &["input-messages", "input_messages", "inputMessages"]));
    if should_send_codex_notification(&prompt, config) {
        let cwd = string(object, "cwd");
        let project = if cwd.is_empty() {
            project_name(
                &env::current_dir().ok().and_then(|path| path.into_os_string().into_string().ok()).unwrap_or_default(),
            )
        } else {
            project_name(cwd)
        };
        let result = extract_message_text(first_value(
            object,
            &["last-assistant-message", "last_assistant_message", "lastAssistantMessage"],
        ));
        let notification = completion_notification(Client::Codex, &project, &prompt, &result, None);
        let _ = notifier.deliver(&notification);
    }
    Ok(())
}

pub fn format_duration(seconds: u64) -> String {
    if seconds < 60 {
        return format!("{seconds}s");
    }
    let minutes = seconds / 60;
    let remaining_seconds = seconds % 60;
    if minutes < 60 {
        return if remaining_seconds == 0 { format!("{minutes}m") } else { format!("{minutes}m{remaining_seconds}s") };
    }
    let hours = minutes / 60;
    let remaining_minutes = minutes % 60;
    if remaining_minutes == 0 { format!("{hours}h") } else { format!("{hours}h{remaining_minutes}m") }
}

pub fn permission_request(tool_name: Option<&Value>, tool_input: Option<&Value>) -> String {
    let mut tool_name = tool_name.and_then(Value::as_str).unwrap_or_default().trim().to_owned();
    let mut detail = "";
    if let Some(input) = tool_input.and_then(Value::as_object) {
        if tool_name.is_empty() {
            tool_name = string(input, "name").trim().to_owned();
        }
        detail = ["command", "file_path", "notebook_path", "path", "url", "query", "description"]
            .into_iter()
            .map(|key| string(input, key).trim())
            .find(|value| !value.is_empty())
            .unwrap_or_default();
    }
    match (tool_name.is_empty(), detail.is_empty()) {
        (false, false) => format!("{tool_name} — {detail}"),
        (false, true) => tool_name,
        (true, false) => detail.to_owned(),
        (true, true) => "Permission requested".to_owned(),
    }
}

pub fn question_text(tool_input: Option<&Value>) -> String {
    let Some(questions) =
        tool_input.and_then(Value::as_object).and_then(|input| input.get("questions")).and_then(Value::as_array)
    else {
        return DEFAULT_QUESTION.to_owned();
    };
    let questions = questions
        .iter()
        .filter_map(|question| question.as_object().map(|value| string(value, "question").trim()))
        .filter(|question| !question.is_empty())
        .collect::<Vec<_>>();
    match questions.first() {
        None => DEFAULT_QUESTION.to_owned(),
        Some(first) if questions.len() == 1 => (*first).to_owned(),
        Some(first) => format!("{first} (+{} more)", questions.len() - 1),
    }
}

pub fn failure_message(object: &Map<String, Value>) -> String {
    ["last_assistant_message", "error_details", "error"]
        .into_iter()
        .map(|key| string(object, key).split_whitespace().collect::<Vec<_>>().join(" "))
        .find(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_FAILURE_MESSAGE.to_owned())
}

pub fn extract_last_user_message(messages: Option<&Value>) -> String {
    match messages {
        Some(Value::String(value)) => value.trim().to_owned(),
        Some(Value::Array(messages)) => {
            let mut user = Vec::new();
            let mut other = Vec::new();
            for message in messages {
                let text = extract_message_text(Some(message));
                if text.is_empty() {
                    continue;
                }
                if message.as_object().and_then(|item| item.get("role")).and_then(Value::as_str) == Some("user") {
                    user.push(text);
                } else {
                    other.push(text);
                }
            }
            user.into_iter().last().or_else(|| other.into_iter().last()).unwrap_or_default()
        }
        _ => String::new(),
    }
}

pub fn extract_message_text(message: Option<&Value>) -> String {
    match message {
        Some(Value::String(value)) => value.trim().to_owned(),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| extract_message_text(Some(item)))
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
        Some(Value::Object(object)) => ["content", "text", "message"]
            .into_iter()
            .find_map(|key| object.get(key))
            .map_or_else(String::new, |value| extract_message_text(Some(value))),
        _ => String::new(),
    }
}

fn object(payload: &Value) -> Result<&Map<String, Value>> {
    payload.as_object().ok_or_else(|| AppError::usage("JSON payload must be an object"))
}
fn string<'a>(object: &'a Map<String, Value>, key: &str) -> &'a str {
    object.get(key).and_then(Value::as_str).unwrap_or_default()
}
fn required_session_id(object: &Map<String, Value>) -> Result<&str> {
    optional_session_id(object).ok_or_else(|| AppError::usage("Missing session_id in input"))
}
fn optional_session_id(object: &Map<String, Value>) -> Option<&str> {
    object.get("session_id").and_then(Value::as_str).filter(|value| !value.is_empty())
}
fn first_value<'a>(object: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|key| object.get(*key))
}
fn nonempty_collection(value: Option<&Value>) -> bool {
    value.is_some_and(|value| match value {
        Value::Array(values) => !values.is_empty(),
        Value::Object(values) => !values.is_empty(),
        Value::Null => false,
        _ => true,
    })
}
fn project_name(cwd: &str) -> String {
    Path::new(cwd).file_name().and_then(|name| name.to_str()).unwrap_or_default().to_owned()
}

fn is_internal_agent_notification(prompt: &str) -> bool {
    let clean = strip_terminal_controls(prompt);
    let trimmed = clean.trim_start().to_ascii_lowercase();
    ["<subagent_notification", "<task-notification"].iter().any(|prefix| {
        trimmed.strip_prefix(prefix).is_some_and(|rest| rest.starts_with('>') || rest.starts_with(char::is_whitespace))
    })
}

fn strip_terminal_controls(value: &str) -> String {
    strip_ansi_escapes::strip_str(value).chars().filter(|character| !character.is_control()).collect()
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use serde_json::json;

    use crate::{config::NotificationConfig, model::Notification};

    use super::*;

    #[derive(Default)]
    struct State {
        prompts: RefCell<Vec<String>>,
        stopped: RefCell<Vec<String>>,
        waiting: RefCell<Vec<String>>,
        active: Option<String>,
        job: JobInfo,
    }
    impl SessionState for State {
        fn track_prompt(&self, session: &str, prompt: &str, _: &str) {
            self.prompts.borrow_mut().push(format!("{session}:{prompt}"));
        }
        fn mark_stopped(&self, session: &str) {
            self.stopped.borrow_mut().push(session.into());
        }
        fn mark_waiting(&self, session: &str) {
            self.waiting.borrow_mut().push(session.into());
        }
        fn active_prompt(&self, _: &str) -> Option<String> {
            self.active.clone()
        }
        fn job_info(&self, _: &str) -> JobInfo {
            self.job.clone()
        }
        fn cleanup_if_due(&self) {}
    }
    #[derive(Default)]
    struct Sink(RefCell<Vec<Notification>>);
    impl NotificationSink for Sink {
        fn deliver(&mut self, notification: &Notification) -> bool {
            self.0.borrow_mut().push(notification.clone());
            true
        }
    }
    fn config() -> AppConfig {
        AppConfig {
            notification: NotificationConfig { threshold_seconds: 10, ..Default::default() },
            ..Default::default()
        }
    }

    #[test]
    fn validation_rejects_invalid_generic_payload_fields() {
        assert!(validate_payload(&json!({"cwd":"/tmp/../etc"})).is_err());
        assert!(validate_payload(&json!({"session_id":""})).is_err());
        assert!(parse_payload("[]").is_err());
    }
    #[test]
    fn user_prompt_tracks_intent_but_ignores_internal_envelopes() {
        let state = State::default();
        handle_user_prompt(&json!({"session_id":"s", "prompt":"<task-notification>done", "cwd":"/tmp/p"}), &state)
            .unwrap();
        handle_user_prompt(&json!({"session_id":"s", "prompt":"fix this", "cwd":"/tmp/p"}), &state).unwrap();
        assert_eq!(*state.prompts.borrow(), ["s:fix this"]);
    }
    #[test]
    fn stop_defers_pending_work_and_filters_normal_completions() {
        let state = State {
            job: JobInfo { job_number: Some(1), duration_seconds: Some(11), prompt: Some("work".into()) },
            ..Default::default()
        };
        let mut sink = Sink::default();
        handle_stop(&json!({"session_id":"s", "background_tasks":[{}]}), &state, &config(), &mut sink).unwrap();
        assert!(state.stopped.borrow().is_empty());
        handle_stop(
            &json!({"session_id":"s", "cwd":"/tmp/project", "last_assistant_message":"done"}),
            &state,
            &config(),
            &mut sink,
        )
        .unwrap();
        assert_eq!(sink.0.borrow()[0].subtitle, "Claude completed in 11s");
    }
    #[test]
    fn failure_bypasses_duration_and_prefix_filters_but_requires_all() {
        let state = State {
            active: Some("/skip work".into()),
            job: JobInfo { job_number: Some(1), duration_seconds: Some(1), prompt: Some("/skip work".into()) },
            ..Default::default()
        };
        let mut sink = Sink::default();
        let config = AppConfig {
            notification: NotificationConfig {
                threshold_seconds: 999,
                exclude_patterns: vec!["/skip".into()],
                ..Default::default()
            },
            ..Default::default()
        };
        handle_stop_failure(&json!({"session_id":"s", "error":"nope"}), &state, &config, &mut sink).unwrap();
        assert_eq!(sink.0.borrow()[0].message, "Task: /skip work\nError: nope");
    }
    #[test]
    fn idle_permission_and_question_contracts_are_preserved() {
        let state = State { active: Some("task".into()), ..Default::default() };
        let mut sink = Sink::default();
        handle_notification(&json!({"session_id":"s", "notification_type":"idle_prompt"}), &state).unwrap();
        handle_permission(
            &json!({"session_id":"s", "tool_name":"Bash", "tool_input":{"command":"ls"}}),
            &state,
            &config(),
            &mut sink,
        )
        .unwrap();
        handle_ask_user_question(
            &json!({"session_id":"s", "tool_input":{"questions":[{"question":"Continue?"},{"question":"Why?"}]}}),
            &state,
            &config(),
            &mut sink,
        )
        .unwrap();
        assert_eq!(*state.waiting.borrow(), ["s"]);
        assert_eq!(sink.0.borrow()[0].message, "Task: task\nRequest: Bash — ls");
        assert_eq!(sink.0.borrow()[1].message, "Task: task\nQuestion: Continue? (+1 more)");
    }
    #[test]
    fn codex_aliases_choose_last_user_message_and_have_no_duration_filter() {
        let mut sink = Sink::default();
        handle_codex_notify(&json!({"event":"agent-turn-complete", "cwd":"/tmp/p", "inputMessages":[{"role":"user", "content":[{"text":"first"}]},{"role":"user", "content":"last"}], "lastAssistantMessage":{"content":"done"}}), &config(), &mut sink).unwrap();
        assert_eq!(sink.0.borrow()[0].message, "Task: last\nResult: done");
    }
    #[test]
    fn duration_and_text_extractors_keep_historical_behavior() {
        assert_eq!(format_duration(3661), "1h1m");
        assert_eq!(failure_message(json!({"error_details":" a\n b "}).as_object().unwrap()), "a b");
        assert_eq!(extract_message_text(Some(&json!([{"text":"a"}, {"content":"b"}]))), "a b");
    }
}
