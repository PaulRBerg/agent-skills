use thiserror::Error;

pub type Result<T> = std::result::Result<T, AppError>;

/// Broad error classes used by the binary to keep exit behavior consistent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    Operational,
    Usage,
    /// An invalid hook/callback payload. Hook commands read from an external tool's stdin or
    /// argv, and their host (Claude Code, Codex) treats exit 2 as a blocking decision, so these
    /// errors must exit 1 like other runtime failures instead of exit 2 like a CLI usage error.
    HookPayload,
}

impl ErrorKind {
    /// Command-line usage errors exit 2, like clap's own parse errors; runtime failures exit 1.
    pub const fn code(self) -> u8 {
        match self {
            Self::Usage => 2,
            Self::Operational | Self::HookPayload => 1,
        }
    }
}

#[derive(Debug, Error)]
#[error("{message}")]
pub struct AppError {
    pub kind: ErrorKind,
    pub message: String,
}

impl AppError {
    pub fn operational(message: impl Into<String>) -> Self {
        Self { kind: ErrorKind::Operational, message: message.into() }
    }

    pub fn usage(message: impl Into<String>) -> Self {
        Self { kind: ErrorKind::Usage, message: message.into() }
    }

    pub fn hook_payload(message: impl Into<String>) -> Self {
        Self { kind: ErrorKind::HookPayload, message: message.into() }
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        Self::operational(error.to_string())
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(error: rusqlite::Error) -> Self {
        Self::operational(error.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        Self::operational(error.to_string())
    }
}

impl From<serde_yaml_ng::Error> for AppError {
    fn from(error: serde_yaml_ng::Error) -> Self {
        Self::operational(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_preserve_cli_usage_and_runtime_conventions() {
        assert_eq!(ErrorKind::Usage.code(), 2);
        assert_eq!(ErrorKind::Operational.code(), 1);
        assert_eq!(ErrorKind::HookPayload.code(), 1);
    }
}
