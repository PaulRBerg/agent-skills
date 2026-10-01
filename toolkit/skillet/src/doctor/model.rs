use std::{cmp::Ordering, path::PathBuf};

use serde::Serialize;

pub const SCHEMA_VERSION: u8 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Report {
    pub schema_version: u8,
    pub roots: Vec<RootRecord>,
    pub counts: Counts,
    pub findings: Vec<Finding>,
    pub fixes: Vec<Fix>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RootRecord {
    pub path: PathBuf,
    pub active_skills: usize,
    pub readme: Option<PathBuf>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct Counts {
    pub findings: usize,
    pub errors: usize,
    pub warnings: usize,
    pub fixable: usize,
    pub fixes: usize,
    pub fix_errors: usize,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Error,
    Warning,
    FixError,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Finding {
    pub code: String,
    pub severity: Severity,
    pub path: PathBuf,
    pub line: Option<u64>,
    pub fixable: bool,
    pub message: String,
}

impl Finding {
    pub fn new(
        code: impl Into<String>,
        severity: Severity,
        path: impl Into<PathBuf>,
        line: Option<u64>,
        fixable: bool,
        message: impl Into<String>,
    ) -> Self {
        Self { code: code.into(), severity, path: path.into(), line, fixable, message: message.into() }
    }
}

impl Ord for Finding {
    fn cmp(&self, other: &Self) -> Ordering {
        (&self.path, self.line.unwrap_or(u64::MAX), &self.code, &self.message, self.severity).cmp(&(
            &other.path,
            other.line.unwrap_or(u64::MAX),
            &other.code,
            &other.message,
            other.severity,
        ))
    }
}

impl PartialOrd for Finding {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Fix {
    pub code: String,
    pub path: PathBuf,
    pub message: String,
}

impl Fix {
    pub fn new(code: impl Into<String>, path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self { code: code.into(), path: path.into(), message: message.into() }
    }
}
