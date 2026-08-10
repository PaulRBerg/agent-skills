use std::path::Path;

pub(super) fn deterministic_handoff(finding_id: &str) -> String {
    format!(".ai/task-handoffs/FINDING_{}.md", finding_id.to_ascii_uppercase())
}

pub(super) fn safe_document_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let filename = Path::new(&lower).file_name().and_then(|name| name.to_str()).unwrap_or("");
    let extension = Path::new(&lower).extension().and_then(|value| value.to_str()).unwrap_or("");
    let protected_file =
        matches!(filename, "agents.md" | "claude.md" | "changelog.md" | "security.md" | "code_of_conduct.md") ||
            filename.starts_with("license") ||
            filename.starts_with("copying") ||
            filename.starts_with("notice");
    matches!(extension, "md" | "mdx" | "rst" | "adoc" | "txt") &&
        !protected_file &&
        !lower.split('/').any(|part| {
            matches!(
                part,
                ".agents" |
                    ".claude" |
                    ".codex" |
                    "skills" |
                    "generated" |
                    "schema" |
                    "schemas" |
                    "api" |
                    "legal" |
                    "release" |
                    "releases" |
                    "policy" |
                    "policies" |
                    "guidance"
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handoff_path_matches_task_handoff_publication() {
        assert_eq!(deterministic_handoff("a1b2c3d4"), ".ai/task-handoffs/FINDING_A1B2C3D4.md");
    }

    #[test]
    fn safe_document_tier_excludes_policy_legal_and_generated_text() {
        for path in ["README.md", "docs/guide.mdx", "manual/setup.rst"] {
            assert!(safe_document_path(path), "expected safe path: {path}");
        }
        for path in [
            "AGENTS.md",
            "SECURITY.md",
            "CODE_OF_CONDUCT.md",
            "LICENSE.txt",
            "COPYING.md",
            "NOTICE",
            "docs/generated/reference.md",
            "policies/release.md",
            "src/lib.rs",
        ] {
            assert!(!safe_document_path(path), "expected protected path: {path}");
        }
    }
}
