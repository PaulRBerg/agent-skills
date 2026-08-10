//! macOS notification delivery and portable notification formatting helpers.

use std::{
    env,
    ffi::OsString,
    io::{self, Write},
    path::PathBuf,
    process::Command,
};

use tempfile::NamedTempFile;

use crate::{
    config::AppConfig,
    model::{Client, Notification},
};

pub const TASK_EXCERPT_LENGTH: usize = 100;
pub const DETAIL_EXCERPT_LENGTH: usize = 180;

const CLAUDE_ICON: &[u8] = include_bytes!("../assets/claude-icon.png");
const CODEX_ICON: &[u8] = include_bytes!("../assets/codex-icon.png");

/// Boundary used by handlers so their tests do not depend on macOS.
pub trait NotificationSink {
    /// Returns false when delivery was unavailable or rejected. Event handling treats that as success.
    fn deliver(&mut self, notification: &Notification) -> bool;
}

/// Injectable platform and process boundary for `MacNotifier`.
pub trait NotificationEnvironment {
    fn is_darwin(&self) -> bool;
    fn executable(&self, name: &str) -> Option<PathBuf>;
    fn environment_variable(&self, name: &str) -> Option<OsString>;
    fn run(&self, program: &std::path::Path, arguments: &[OsString]) -> io::Result<bool>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemEnvironment;

impl NotificationEnvironment for SystemEnvironment {
    fn is_darwin(&self) -> bool {
        env::consts::OS == "macos"
    }

    fn executable(&self, name: &str) -> Option<PathBuf> {
        let paths = env::var_os("PATH")?;
        env::split_paths(&paths).map(|directory| directory.join(name)).find(|candidate| candidate.is_file())
    }

    fn environment_variable(&self, name: &str) -> Option<OsString> {
        env::var_os(name)
    }

    fn run(&self, program: &std::path::Path, arguments: &[OsString]) -> io::Result<bool> {
        Command::new(program).args(arguments).status().map(|status| status.success())
    }
}

/// Sends notifications through `terminal-notifier` when macOS support is available.
pub struct MacNotifier<E = SystemEnvironment> {
    config: AppConfig,
    environment: E,
    available: Option<bool>,
}

impl MacNotifier<SystemEnvironment> {
    pub fn new(config: AppConfig) -> Self {
        Self::with_environment(config, SystemEnvironment)
    }
}

impl<E: NotificationEnvironment> MacNotifier<E> {
    pub fn with_environment(config: AppConfig, environment: E) -> Self {
        Self { config, environment, available: None }
    }

    pub fn check_available(&mut self) -> bool {
        if let Some(available) = self.available {
            return available;
        }
        let available = self.environment.is_darwin() && self.environment.executable("terminal-notifier").is_some();
        self.available = Some(available);
        available
    }

    fn focus_command(&self) -> Option<String> {
        let raw_session = self.environment.environment_variable("ITERM_SESSION_ID")?;
        let session = raw_session.to_string_lossy();
        if session.is_empty() {
            return None;
        }
        let it2 = self.environment.executable("it2")?;
        let uuid = session.rsplit_once(':').map_or(session.as_ref(), |(_, value)| value);
        Some(format!(
            "{} session focus {} || open -b {}",
            shell_quote(&it2.to_string_lossy()),
            shell_quote(uuid),
            shell_quote(&self.config.notification.app_bundle)
        ))
    }

    fn arguments(&self, notification: &Notification, icon: Option<&std::path::Path>) -> Vec<OsString> {
        let mut arguments = vec![
            OsString::from("-title"),
            OsString::from(&notification.title),
            OsString::from("-subtitle"),
            OsString::from(&notification.subtitle),
            OsString::from("-message"),
            OsString::from(&notification.message),
            OsString::from("-ignoreDnD"),
        ];
        if let Some(command) = self.focus_command() {
            arguments.extend([OsString::from("-execute"), OsString::from(command)]);
        } else {
            arguments.extend([OsString::from("-activate"), OsString::from(&self.config.notification.app_bundle)]);
        }
        arguments.extend([OsString::from("-sound"), OsString::from(&self.config.notification.sound)]);
        if let Some(icon) = icon {
            arguments.extend([OsString::from("-contentImage"), icon.as_os_str().to_owned()]);
        }
        arguments
    }

    fn materialize_icon(&self, client: Client) -> io::Result<NamedTempFile> {
        let mut icon = NamedTempFile::new()?;
        let bytes = match client {
            Client::Claude => CLAUDE_ICON,
            Client::Codex => CODEX_ICON,
        };
        icon.write_all(bytes)?;
        icon.flush()?;
        Ok(icon)
    }
}

impl<E: NotificationEnvironment> NotificationSink for MacNotifier<E> {
    fn deliver(&mut self, notification: &Notification) -> bool {
        if !self.check_available() {
            return false;
        }
        let Some(program) = self.environment.executable("terminal-notifier") else {
            self.available = Some(false);
            return false;
        };
        let icon = self.materialize_icon(notification.client).ok();
        self.environment
            .run(&program, &self.arguments(notification, icon.as_ref().map(|file| file.path())))
            .unwrap_or(false)
    }
}

/// Normalizes whitespace and bounds a notification excerpt.
pub fn truncate_text(value: &str, limit: usize) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= limit {
        return normalized;
    }
    if limit <= 3 {
        return ".".repeat(limit);
    }
    let excerpt = normalized.chars().take(limit - 3).collect::<String>();
    format!("{}...", excerpt.trim_end())
}

pub fn context_message(task: &str, detail_label: &str, detail: &str) -> String {
    let task = truncate_text(task, TASK_EXCERPT_LENGTH);
    let detail = truncate_text(detail, DETAIL_EXCERPT_LENGTH);
    [
        (!task.is_empty()).then(|| format!("Task: {task}")),
        (!detail.is_empty()).then(|| format!("{detail_label}: {detail}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("\n")
}

pub fn completion_notification(
    client: Client,
    project: &str,
    task: &str,
    result: &str,
    duration: Option<&str>,
) -> Notification {
    let mut subtitle = format!("{} completed", client.display_name());
    if let Some(duration) = duration.filter(|value| !value.is_empty()) {
        subtitle.push_str(" in ");
        subtitle.push_str(duration);
    }
    let message = context_message(task, "Result", result);
    Notification::new(
        client,
        if project.is_empty() { client.display_name() } else { project },
        subtitle,
        if message.is_empty() { "Result: Turn completed.".into() } else { message },
    )
}

pub fn permission_notification(project: &str, task: &str, request: &str) -> Notification {
    Notification::new(
        Client::Claude,
        if project.is_empty() { "Claude Code" } else { project },
        "Claude needs approval",
        context_message(task, "Request", if request.is_empty() { "Permission requested" } else { request }),
    )
}

pub fn failure_notification(project: &str, task: &str, error: &str, duration: Option<&str>) -> Notification {
    let mut subtitle = "Claude failed".to_owned();
    if let Some(duration) = duration.filter(|value| !value.is_empty()) {
        subtitle.push_str(" after ");
        subtitle.push_str(duration);
    }
    Notification::new(
        Client::Claude,
        if project.is_empty() { "Claude Code" } else { project },
        subtitle,
        context_message(task, "Error", error),
    )
}

pub fn question_notification(project: &str, task: &str, question: &str) -> Notification {
    Notification::new(
        Client::Claude,
        if project.is_empty() { "Claude Code" } else { project },
        "Claude needs input",
        context_message(task, "Question", if question.is_empty() { "Claude is asking a question" } else { question }),
    )
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, path::Path};

    use super::*;

    #[derive(Default)]
    struct FakeEnvironment {
        darwin: bool,
        terminal_notifier: bool,
        it2: bool,
        session: Option<OsString>,
        runs: RefCell<Vec<Vec<OsString>>>,
        successful: bool,
    }

    impl NotificationEnvironment for FakeEnvironment {
        fn is_darwin(&self) -> bool {
            self.darwin
        }
        fn executable(&self, name: &str) -> Option<PathBuf> {
            match name {
                "terminal-notifier" if self.terminal_notifier => Some(PathBuf::from("/bin/terminal-notifier")),
                "it2" if self.it2 => Some(PathBuf::from("/bin/it2")),
                _ => None,
            }
        }
        fn environment_variable(&self, name: &str) -> Option<OsString> {
            (name == "ITERM_SESSION_ID").then(|| self.session.clone()).flatten()
        }
        fn run(&self, _: &Path, arguments: &[OsString]) -> io::Result<bool> {
            self.runs.borrow_mut().push(arguments.to_vec());
            Ok(self.successful)
        }
    }

    #[test]
    fn availability_requires_darwin_and_terminal_notifier() {
        let mut notifier = MacNotifier::with_environment(AppConfig::default(), FakeEnvironment::default());
        assert!(!notifier.check_available());
        let mut notifier = MacNotifier::with_environment(
            AppConfig::default(),
            FakeEnvironment { darwin: true, terminal_notifier: true, ..FakeEnvironment::default() },
        );
        assert!(notifier.check_available());
    }

    #[test]
    fn mac_command_includes_all_delivery_options_and_focuses_iterm_session() {
        let environment = FakeEnvironment {
            darwin: true,
            terminal_notifier: true,
            it2: true,
            session: Some("w0t0p0:UUID".into()),
            successful: true,
            ..FakeEnvironment::default()
        };
        let mut notifier = MacNotifier::with_environment(AppConfig::default(), environment);
        assert!(notifier.deliver(&Notification::new(Client::Claude, "title", "subtitle", "message")));
        let args = notifier.environment.runs.borrow()[0]
            .iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(args.windows(2).any(|pair| pair == ["-title", "title"]));
        assert!(args.contains(&"-ignoreDnD".into()));
        assert!(
            args.iter().any(|value| value.contains("it2' session focus 'UUID' || open -b 'com.googlecode.iterm2'"))
        );
        assert!(args.contains(&"-sound".into()));
        assert!(args.contains(&"-contentImage".into()));
    }

    #[test]
    fn contextual_messages_normalize_bound_and_fall_back() {
        assert_eq!(truncate_text(" word\nword ", 100), "word word");
        assert_eq!(truncate_text(&"é".repeat(101), 100).chars().count(), 100);
        assert_eq!(completion_notification(Client::Codex, "", "", "", None).message, "Result: Turn completed.");
        let message = context_message(&("task ".repeat(30)), "Result", &("detail ".repeat(40)));
        let lines = message.lines().collect::<Vec<_>>();
        assert_eq!(lines[0].strip_prefix("Task: ").unwrap().len(), TASK_EXCERPT_LENGTH);
        assert_eq!(lines[1].strip_prefix("Result: ").unwrap().len(), DETAIL_EXCERPT_LENGTH);
    }

    #[test]
    fn configured_activation_and_sound_are_used_without_iterm() {
        let config = AppConfig {
            notification: crate::config::NotificationConfig {
                app_bundle: "example.app".into(),
                sound: "Glass".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        let notifier = MacNotifier::with_environment(config, FakeEnvironment::default());
        let args = notifier
            .arguments(&Notification::new(Client::Codex, "title", "subtitle", "message"), None)
            .iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(args[args.iter().position(|value| value == "-activate").unwrap() + 1], "example.app");
        assert_eq!(args[args.iter().position(|value| value == "-sound").unwrap() + 1], "Glass");
    }
}
