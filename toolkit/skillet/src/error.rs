use std::{
    io,
    path::{Path, PathBuf},
};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum InvocationError {
    #[error(transparent)]
    Arguments(#[from] clap::Error),
    #[error(transparent)]
    Operational(#[from] Error),
}

impl InvocationError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Arguments(error) => error.exit_code() as u8,
            Self::Operational(error) => error.exit_code(),
        }
    }

    pub fn print(&self) {
        match self {
            Self::Arguments(error) => {
                let _ = error.print();
            }
            Self::Operational(error) => eprintln!("ai-skillet: {error}"),
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("{operation} {path}: {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("root does not exist: {0}")]
    RootMissing(PathBuf),
    #[error("root is not a directory: {0}")]
    RootNotDirectory(PathBuf),
    #[error("could not traverse {path}: {message}")]
    Traversal { path: PathBuf, message: String },
    #[error("Git is required for --portfolio-root")]
    GitUnavailable,
    #[error("portfolio root is not inside a Git repository: {0}")]
    PortfolioNotGit(PathBuf),
    #[error("invalid Git output: {0}")]
    GitOutput(String),
    #[error("HOME is not set; pass --root explicitly")]
    HomeUnavailable,
    #[error("invalid skill name filter: {}", .0.join(", "))]
    InvalidSkillFilter(Vec<String>),
    #[error("{0}")]
    MapData(String),
    #[error("could not serialize output: {0}")]
    Serialization(String),
}

impl Error {
    pub(crate) fn io(operation: &'static str, path: &Path, source: io::Error) -> Self {
        Self::Io { operation, path: path.to_path_buf(), source }
    }

    pub fn exit_code(&self) -> u8 {
        2
    }
}
