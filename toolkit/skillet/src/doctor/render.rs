use std::fmt::Write as _;
use std::path::Path;

use crate::cli::DoctorFormat;
use crate::error::Error;

use super::model::{Report, Severity};

pub fn render(report: &Report, format: DoctorFormat) -> Result<String, Error> {
    match format {
        DoctorFormat::Json => serde_json::to_string_pretty(report)
            .map(|mut output| {
                output.push('\n');
                output
            })
            .map_err(|error| Error::Serialization(error.to_string())),
        DoctorFormat::Text => Ok(render_text(report)),
    }
}

fn render_text(report: &Report) -> String {
    let mut output = String::new();
    writeln!(
        output,
        "ai-skillet doctor: {} error(s), {} warning(s), {} fix(es)",
        report.counts.errors, report.counts.warnings, report.counts.fixes
    )
    .unwrap();

    if !report.roots.is_empty() {
        output.push_str("\nRoots:\n");
        for root in &report.roots {
            writeln!(output, "- {}: {} active", display_path(&root.path), root.active_skills)
                .unwrap();
        }
    }
    if !report.fixes.is_empty() {
        output.push_str("\nFixes:\n");
        for fix in &report.fixes {
            writeln!(output, "- {}: {}: {}", fix.code, display_path(&fix.path), fix.message)
                .unwrap();
        }
    }
    if !report.findings.is_empty() {
        output.push_str("\nFindings:\n");
        for finding in &report.findings {
            let location = match finding.line {
                Some(line) => format!("{}:{line}", display_path(&finding.path)),
                None => display_path(&finding.path),
            };
            let fixable = if finding.fixable { " fixable" } else { "" };
            writeln!(
                output,
                "- [{}] {}{}: {}: {}",
                severity(finding.severity),
                finding.code,
                fixable,
                location,
                finding.message
            )
            .unwrap();
        }
    }
    output
}

fn severity(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::FixError => "fix-error",
    }
}

fn display_path(path: &Path) -> String {
    serde_json::to_string(&path.to_string_lossy()).expect("serializing a path cannot fail")
}
