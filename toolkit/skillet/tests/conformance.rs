mod common;

use std::collections::BTreeSet;

use ai_skillet::error::InvocationError;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

fn finding_codes(report: &Value) -> BTreeSet<&str> {
    report["findings"].as_array().unwrap().iter().map(|finding| finding["code"].as_str().unwrap()).collect()
}

fn write_skill(root: &std::path::Path, name: &str, fields: &str, description: &str) {
    common::write(
        root.join("skills").join(name).join("SKILL.md"),
        format!(
            "---\n{fields}name: {name}\ndescription: {description}\n---\n\n# {name}\n\n## Completion\n\nReport verification.\n"
        ),
    );
}

#[test]
fn exit_contract_distinguishes_usage_operations_findings_and_fix_failures() {
    let usage = ai_skillet::run_from(["ai-skillet", "map", "--root"]).expect_err("missing value must be rejected");
    assert!(matches!(usage, InvocationError::Arguments(_)));
    assert_eq!(usage.exit_code(), 2);

    let missing = TempDir::new().unwrap().path().join("missing");
    common::ai_skillet()
        .args(["map", "--root"])
        .arg(missing)
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_match(r"^ai-skillet: root does not exist: .+\n$").unwrap());

    let root = TempDir::new().unwrap();
    write_skill(root.path(), "finding", "", "finding");
    common::ai_skillet()
        .args(["doctor", "--root"])
        .arg(root.path())
        .args(["--format", "json"])
        .assert()
        .code(1)
        .stderr(predicate::str::is_empty());
}

#[test]
fn doctor_reports_all_durable_metadata_finding_families() {
    let root = TempDir::new().unwrap();
    common::write(root.path().join("skills/missing/SKILL.md"), "---\nmodel: sonnet\n---\n\n# missing\n");
    common::write(
        root.path().join("skills/typed/SKILL.md"),
        format!(
            "---\nagent: 1\nallowed-tools: {{}}\nargument-hint: false\narguments: false\nbackground: invalid\ncompatibility: {}\ncontext: inline\ncoordination: managed\ndisable-model-invocation: invalid\ndisallowed-tools: {{}}\neffort: extreme\nhooks: []\nlicense: false\nmetadata:\n  author: 1\n  install-targets: other\nmodel: []\nname: typed\npaths: false\nshell: zsh\nunexpected-field: true\nuser-invocable: invalid\nwhen_to_use: 1\ndescription: typed\n---\n\n# typed\n\n## Completion\n\nReport verification.\n",
            "x".repeat(501)
        ),
    );
    write_skill(root.path(), "description-invalid", "", "42");
    write_skill(root.path(), "description-long", "", &format!("|\n  {}", "x".repeat(1025)));
    write_skill(root.path(), "openai-invalid", "", "valid");
    common::write(root.path().join("skills/openai-invalid/agents/openai.yaml"), "policy: [\n");
    write_skill(root.path(), "policy-missing", "", "valid");
    common::write(root.path().join("skills/policy-missing/agents/openai.yaml"), "policy: {}\n");
    write_skill(root.path(), "cli-missing", "", "valid");

    let output =
        common::ai_skillet().args(["doctor", "--root"]).arg(root.path()).args(["--format", "json"]).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let found = finding_codes(&report);
    for expected in [
        "AGENT_INVALID",
        "ALLOWED_TOOLS_INVALID_TYPE",
        "ARGUMENT_HINT_INVALID",
        "ARGUMENTS_INVALID_TYPE",
        "BACKGROUND_INVALID_TYPE",
        "CLI_VERSION_MISSING",
        "COMPATIBILITY_TOO_LONG",
        "CONTEXT_INVALID",
        "COORDINATION_INVALID",
        "DESCRIPTION_INVALID",
        "DESCRIPTION_MISSING",
        "DESCRIPTION_TOO_LONG",
        "DISABLE_MODEL_INVOCATION_INVALID",
        "DISALLOWED_TOOLS_INVALID_TYPE",
        "EFFORT_INVALID_VALUE",
        "FRONTMATTER_UNKNOWN_FIELD",
        "HOOKS_INVALID_TYPE",
        "INSTALL_TARGETS_INVALID",
        "LICENSE_INVALID_TYPE",
        "METADATA_VALUE_INVALID_TYPE",
        "MODEL_INVALID_TYPE",
        "NAME_MISSING",
        "OPENAI_METADATA_INVALID",
        "OPENAI_METADATA_MISSING",
        "OPENAI_POLICY_MISSING",
        "PATHS_INVALID_TYPE",
        "README_MISSING",
        "SHELL_INVALID_VALUE",
        "USER_INVOCABLE_INVALID",
        "WHEN_TO_USE_INVALID_TYPE",
    ] {
        assert!(found.contains(expected), "missing {expected}: {found:?}");
    }
}

#[test]
fn text_paths_are_escaped_and_json_is_valid_for_newline_roots() {
    let base = TempDir::new().unwrap();
    let root = base.path().join("catalog with\nnewline");
    write_skill(&root, "alpha", "", "alpha");

    let map = common::ai_skillet().args(["map", "--root"]).arg(&root).args(["--format", "json"]).output().unwrap();
    assert!(map.status.success());
    let _: Value = serde_json::from_slice(&map.stdout).unwrap();

    common::ai_skillet()
        .args(["doctor", "--root"])
        .arg(&root)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("catalog with\\nnewline"));
}
