use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
};

use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub path: PathBuf,
    pub line: Option<u64>,
    pub column: Option<u64>,
    pub message: String,
}

impl Diagnostic {
    pub fn error(
        code: impl Into<String>,
        path: impl AsRef<Path>,
        line: u64,
        column: u64,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Error,
            path: path.as_ref().to_path_buf(),
            line: Some(line),
            column: Some(column),
            message: message.into(),
        }
    }

    pub fn without_location(
        code: impl Into<String>,
        severity: Severity,
        path: impl AsRef<Path>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity,
            path: path.as_ref().to_path_buf(),
            line: None,
            column: None,
            message: message.into(),
        }
    }
}

impl Ord for Diagnostic {
    fn cmp(&self, other: &Self) -> Ordering {
        (
            &self.path,
            self.line.unwrap_or(u64::MAX),
            self.column.unwrap_or(u64::MAX),
            &self.code,
            &self.message,
            self.severity,
        )
            .cmp(&(
                &other.path,
                other.line.unwrap_or(u64::MAX),
                other.column.unwrap_or(u64::MAX),
                &other.code,
                &other.message,
                other.severity,
            ))
    }
}

impl PartialOrd for Diagnostic {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn sort_diagnostics(diagnostics: &mut [Diagnostic]) {
    diagnostics.sort();
}
