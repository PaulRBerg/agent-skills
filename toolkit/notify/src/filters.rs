//! Pure notification filtering rules.

use crate::{config::AppConfig, model::NotificationMode};

/// Applies the duration and case-sensitive prompt-prefix filters.
pub fn should_send_notification(prompt: &str, duration_seconds: u64, config: &AppConfig) -> bool {
    duration_seconds >= config.notification.threshold_seconds &&
        !config.notification.exclude_patterns.iter().any(|pattern| !prompt.is_empty() && prompt.starts_with(pattern))
}

/// Completion notifications are only enabled in `all` mode.
pub fn should_send_completion_notification(prompt: &str, duration_seconds: u64, config: &AppConfig) -> bool {
    config.notification.mode == NotificationMode::All && should_send_notification(prompt, duration_seconds, config)
}

/// Permission and question notifications are enabled unless all notifications are disabled.
pub fn should_send_permission_notification(config: &AppConfig) -> bool {
    config.notification.mode != NotificationMode::Disabled
}

/// API failures intentionally bypass duration and prompt filters.
pub fn should_send_failure_notification(config: &AppConfig) -> bool {
    config.notification.mode == NotificationMode::All
}

/// Codex has no duration; filter internal title requests before applying user preferences.
pub fn should_send_codex_notification(prompt: &str, config: &AppConfig) -> bool {
    config.notification.mode == NotificationMode::All &&
        !is_codex_title_request(prompt) &&
        !config.notification.exclude_patterns.iter().any(|pattern| !prompt.is_empty() && prompt.starts_with(pattern))
}

fn is_codex_title_request(prompt: &str) -> bool {
    // Codex 0.155.1's temporary title threads inherit legacy notify without a source marker.
    // Match both prompt envelopes from tui/src/app/thread_title.rs, not generic title requests.
    let Some((_, context)) = prompt
        .strip_prefix("Generate a concise, single-line task title of at most ")
        .and_then(|instructions| instructions.split_once("Do not answer the request."))
    else {
        return false;
    };
    context.starts_with("\n\nUser prompt:\n") ||
        context.starts_with(
            "\nPrioritize the current task and latest substantive user request.\n\nRecent conversation messages:\n",
        )
}

#[cfg(test)]
mod tests {
    use crate::{config::NotificationConfig, model::NotificationMode};

    use super::*;

    fn config() -> AppConfig {
        AppConfig {
            notification: NotificationConfig {
                threshold_seconds: 10,
                exclude_patterns: vec!["/skip".into()],
                ..NotificationConfig::default()
            },
            ..AppConfig::default()
        }
    }

    #[test]
    fn duration_and_case_sensitive_prefixes_filter_completions() {
        let config = config();
        assert!(!should_send_notification("work", 9, &config));
        assert!(!should_send_notification("/skip work", 10, &config));
        assert!(should_send_notification("/Skip work", 10, &config));
        assert!(should_send_notification("work /skip", 10, &config));
    }

    #[test]
    fn notification_mode_matrix_is_preserved() {
        let mut config = config();
        for mode in [NotificationMode::PermissionOnly, NotificationMode::Disabled] {
            config.notification.mode = mode;
            assert!(!should_send_completion_notification("work", 99, &config));
            assert!(!should_send_codex_notification("work", &config));
        }
        config.notification.mode = NotificationMode::PermissionOnly;
        assert!(should_send_permission_notification(&config));
        assert!(!should_send_failure_notification(&config));
        config.notification.mode = NotificationMode::Disabled;
        assert!(!should_send_permission_notification(&config));
        config.notification.mode = NotificationMode::All;
        assert!(should_send_failure_notification(&config));
    }

    #[test]
    fn failure_ignores_regular_filters() {
        let config = config();
        assert!(should_send_failure_notification(&config));
        assert!(!should_send_completion_notification("/skip", 1, &config));
    }
}
