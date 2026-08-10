use thiserror::Error;

pub type Result<T> = std::result::Result<T, AppError>;

/// Broad error classes used by the binary to keep exit behavior consistent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    Operational,
    Usage,
    Configuration,
    Integration,
}

impl ErrorKind {
    /// Click used exit 2 for command-line usage errors and exit 1 for runtime failures.
    pub const fn code(self) -> u8 {
        match self {
            Self::Usage => 2,
            Self::Operational | Self::Configuration | Self::Integration => 1,
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

    pub fn configuration(message: impl Into<String>) -> Self {
        Self { kind: ErrorKind::Configuration, message: message.into() }
    }

    pub fn integration(message: impl Into<String>) -> Self {
        Self { kind: ErrorKind::Integration, message: message.into() }
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
        Self::configuration(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_preserve_cli_usage_and_runtime_conventions() {
        assert_eq!(ErrorKind::Usage.code(), 2);
        assert_eq!(ErrorKind::Operational.code(), 1);
        assert_eq!(ErrorKind::Configuration.code(), 1);
        assert_eq!(ErrorKind::Integration.code(), 1);
    }
}
