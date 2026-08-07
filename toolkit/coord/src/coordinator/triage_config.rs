use std::{path::Path, process::Command};

#[cfg(test)]
use std::fs;

use crate::error::Result;

const CONFIG_PATH: &str = ".ai-coord.toml";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum TriageSchedule {
    Launched { run_id: String, finding_count: usize },
    Skipped(&'static str),
}

pub(super) fn auto_triage_enabled(root: &Path) -> Result<bool> {
    let output = Command::new("git").args(["-C"]).arg(root).args(["show", &format!("HEAD:{CONFIG_PATH}")]).output()?;
    if !output.status.success() {
        return Ok(false);
    }
    let text = match String::from_utf8(output.stdout) {
        Ok(text) => text,
        Err(_) => return Ok(false),
    };
    let mut in_findings = false;
    let mut enabled = None;
    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            in_findings = line == "[findings]";
            continue;
        }
        if !in_findings {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == "auto_triage" {
            if enabled.is_some() {
                return Ok(false);
            }
            enabled = Some(value.trim() == "true");
        }
    }
    Ok(enabled == Some(true))
}

pub(super) fn main_branch(root: &Path) -> bool {
    let Ok(output) =
        Command::new("git").args(["-C"]).arg(root).args(["symbolic-ref", "--quiet", "--short", "HEAD"]).output()
    else {
        return false;
    };
    output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "main"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init() -> tempfile::TempDir {
        let temp = tempfile::tempdir().unwrap();
        assert!(
            Command::new("git").args(["init", "-q", "-b", "main"]).current_dir(temp.path()).status().unwrap().success()
        );
        temp
    }

    #[test]
    fn opt_in_must_be_exact_and_tracked() {
        let repo = init();
        fs::write(repo.path().join(CONFIG_PATH), "[findings]\nauto_triage = true\n").unwrap();
        assert!(!auto_triage_enabled(repo.path()).unwrap());
        assert!(Command::new("git").args(["add", CONFIG_PATH]).current_dir(repo.path()).status().unwrap().success());
        assert!(!auto_triage_enabled(repo.path()).unwrap(), "staged policy is not committed authority");
        assert!(
            Command::new("git")
                .args(["-c", "user.name=test", "-c", "user.email=test@invalid", "commit", "-qm", "opt in"])
                .current_dir(repo.path())
                .status()
                .unwrap()
                .success()
        );
        assert!(auto_triage_enabled(repo.path()).unwrap());
        fs::write(repo.path().join(CONFIG_PATH), "[findings]\nauto_triage = \"true\"\n").unwrap();
        assert!(auto_triage_enabled(repo.path()).unwrap(), "dirty policy cannot replace committed authority");
    }

    #[test]
    fn branch_guard_requires_main_exactly() {
        let repo = init();
        assert!(main_branch(repo.path()));
        assert!(
            Command::new("git")
                .args(["checkout", "-q", "-b", "topic"])
                .current_dir(repo.path())
                .status()
                .unwrap()
                .success()
        );
        assert!(!main_branch(repo.path()));
    }
}
