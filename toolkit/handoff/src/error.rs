use thiserror::Error;

pub(crate) type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ErrorKind {
    Operational,
    Usage,
}

#[derive(Debug, Error)]
#[error("{message}")]
pub struct Error {
    kind: ErrorKind,
    message: String,
}

impl Error {
    pub(crate) fn operational(message: impl Into<String>) -> Self {
        Self { kind: ErrorKind::Operational, message: message.into() }
    }

    pub(crate) fn usage(message: impl Into<String>) -> Self {
        Self { kind: ErrorKind::Usage, message: message.into() }
    }

    pub const fn exit_code(&self) -> u8 {
        match self.kind {
            ErrorKind::Operational => 1,
            ErrorKind::Usage => 2,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::operational(error.to_string())
    }
}
