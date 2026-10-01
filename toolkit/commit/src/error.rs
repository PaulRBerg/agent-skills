use std::{fmt, io};

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    Operational,
    Usage,
    Retry,
}

impl ErrorKind {
    pub const fn code(self) -> i32 {
        match self {
            Self::Operational => 1,
            Self::Usage => 2,
            Self::Retry => 3,
        }
    }
}

#[derive(Debug)]
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

    pub fn retry(message: impl Into<String>) -> Self {
        Self { kind: ErrorKind::Retry, message: message.into() }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for AppError {}

impl From<io::Error> for AppError {
    fn from(error: io::Error) -> Self {
        Self::operational(error.to_string())
    }
}
