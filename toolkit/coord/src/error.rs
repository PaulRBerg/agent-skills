#![allow(dead_code)]

use thiserror::Error;

pub(crate) type Result<T> = std::result::Result<T, AppError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ErrorKind {
    Operational,
    Usage,
    Retry,
}

impl ErrorKind {
    pub(crate) const fn code(self) -> u8 {
        match self {
            Self::Operational => 1,
            Self::Usage => 2,
            Self::Retry => 3,
        }
    }
}

#[derive(Debug, Error)]
#[error("{message}")]
pub(crate) struct AppError {
    pub(crate) kind: ErrorKind,
    pub(crate) message: String,
}

impl AppError {
    pub(crate) fn operational(message: impl Into<String>) -> Self {
        Self { kind: ErrorKind::Operational, message: message.into() }
    }

    pub(crate) fn usage(message: impl Into<String>) -> Self {
        Self { kind: ErrorKind::Usage, message: message.into() }
    }

    pub(crate) fn retry(message: impl Into<String>) -> Self {
        Self { kind: ErrorKind::Retry, message: message.into() }
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
