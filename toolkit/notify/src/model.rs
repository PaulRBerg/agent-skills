use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationMode {
    #[default]
    All,
    PermissionOnly,
    Disabled,
}

impl fmt::Display for NotificationMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::All => "all",
            Self::PermissionOnly => "permission_only",
            Self::Disabled => "disabled",
        })
    }
}

impl FromStr for NotificationMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "all" => Ok(Self::All),
            "permission_only" => Ok(Self::PermissionOnly),
            "disabled" => Ok(Self::Disabled),
            _ => Err("notification mode must be one of all, permission_only, disabled".to_owned()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Client {
    Claude,
    Codex,
}

impl Client {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Claude => "Claude",
            Self::Codex => "Codex",
        }
    }

    pub const fn icon_name(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }
}

/// Fully rendered input to the platform-specific notification boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Notification {
    pub title: String,
    pub subtitle: String,
    pub message: String,
    pub client: Client,
}

impl Notification {
    pub fn new(
        client: Client,
        title: impl Into<String>,
        subtitle: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self { client, title: title.into(), subtitle: subtitle.into(), message: message.into() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_modes_use_persisted_values() {
        for (text, expected) in [
            ("all", NotificationMode::All),
            ("permission_only", NotificationMode::PermissionOnly),
            ("disabled", NotificationMode::Disabled),
        ] {
            assert_eq!(text.parse(), Ok(expected));
            assert_eq!(expected.to_string(), text);
        }
    }
}
