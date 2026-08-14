mod common;

use std::{collections::BTreeSet, fs};

use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

fn run_json(root: &std::path::Path, extra: &[&str]) -> (std::process::Output, Value) {
    let output = common::ai_skillet()
        .args(["doctor", "--root"])
        .arg(root)
        .args(["--format", "json"])
        .args(extra)
        .output()
        .unwrap();
    let report = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("invalid JSON: {error}; stdout={:?}", output.stdout));
    (output, report)
}

fn write_skill(root: &std::path::Path, name: &str, fields: &str, body: &str) {
    common::write(
        root.join("skills").join(name).join("SKILL.md"),
        format!("---\n{fields}name: {name}\ndescription: {name} description.\n---\n\n# {name}\n\n{body}\n"),
    );
}

fn write_dependent_skill(root: &std::path::Path, name: &str, dependencies: &[&str], body: &str) {
    let dependencies = dependencies.iter().map(|dependency| format!("  - {dependency}\n")).collect::<String>();
    common::write(
        root.join("skills").join(name).join("SKILL.md"),
        format!(
            "---\nname: {name}\nskill-dependencies:\n{dependencies}description: {name} description.\n---\n\n# {name}\n\n{body}\n"
        ),
    );
}

fn write_metadata(root: &std::path::Path, name: &str, contents: &str) {
    common::write(root.join("skills").join(name).join("agents/openai.yaml"), contents);
}

fn write_readme(root: &std::path::Path, names: &[&str]) {
    let rows =
        names.iter().map(|name| format!("| {name} | [SKILL.md](/skills/{name}/SKILL.md) |\n")).collect::<String>();
    common::write(
        root.join("README.md"),
        format!("# Catalog\n\n## Skills\n\n| Skill | Entry point |\n| --- | --- |\n{rows}"),
    );
}

fn codes(report: &Value) -> BTreeSet<&str> {
    report["findings"].as_array().unwrap().iter().map(|finding| finding["code"].as_str().unwrap()).collect()
}

fn finding<'a>(report: &'a Value, code: &str) -> &'a Value {
    report["findings"].as_array().unwrap().iter().find(|finding| finding["code"] == code).unwrap()
}

#[test]
fn clean_fixture_has_schema_v1_valid_json_and_text() {
    let root = common::fixture("doctor/catalog");
    let (output, report) = run_json(&root, &[]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["counts"]["findings"], 0);
    assert_eq!(report["roots"][0]["active_skills"], 1);
    assert_eq!(report["findings"], serde_json::json!([]));
    assert_eq!(report["fixes"], serde_json::json!([]));

    common::ai_skillet()
        .args(["doctor", "--root"])
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains("ai-skillet doctor: 0 error(s)"))
        .stdout(predicate::str::contains("Roots:"));
}

#[test]
fn readme_inventory_accepts_a_skill_only_table() {
    let root = TempDir::new().unwrap();
    write_skill(root.path(), "alpha", "", "## Completion\n\nReport verification.");
    write_metadata(root.path(), "alpha", "policy:\n  allow_implicit_invocation: true\n");
    common::write(root.path().join("README.md"), "# Catalog\n\n## Skills\n\n| Skill |\n| --- |\n| alpha |\n");

    let (output, report) = run_json(root.path(), &[]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(report["findings"], serde_json::json!([]));
}

#[test]
fn installed_root_does_not_require_catalog_readme_inventory() {
    let parent = TempDir::new().unwrap();
    for root_name in [".agents", ".claude", ".codex"] {
        let root = parent.path().join(root_name);
        write_skill(&root, "alpha", "", "## Completion\n\nReport verification.");
        write_metadata(&root, "alpha", "policy:\n  allow_implicit_invocation: true\n");
        common::write(root.join("README.md"), "# Installed agent state\n");

        let (output, report) = run_json(&root, &[]);
        assert!(output.status.success(), "{root_name}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(report["roots"][0]["active_skills"], 1);
        assert_eq!(report["findings"], serde_json::json!([]));
    }
}

#[test]
fn installed_exposures_allow_missing_openai_metadata() {
    let parent = TempDir::new().unwrap();
    for root_name in [".agents", ".claude", ".codex"] {
        let root = parent.path().join(root_name);
        write_skill(&root, "alpha", "", "## Completion\n\nReport verification.");

        let (output, report) = run_json(&root, &[]);
        assert!(output.status.success(), "{root_name}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(report["findings"], serde_json::json!([]), "{root_name}");
    }
}

#[test]
fn directly_requested_installed_skill_allows_missing_openai_metadata() {
    let parent = TempDir::new().unwrap();
    let root = parent.path().join(".agents");
    write_skill(&root, "alpha", "", "## Completion\n\nReport verification.");

    let (output, report) = run_json(&root.join("skills/alpha"), &[]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(report["findings"], serde_json::json!([]));
}

#[test]
fn source_catalog_requires_and_safely_creates_openai_metadata() {
    let root = TempDir::new().unwrap();
    write_skill(root.path(), "alpha", "", "## Completion\n\nReport verification.");
    write_readme(root.path(), &["alpha"]);
    let skill = root.path().join("skills/alpha");
    let metadata = skill.join("agents/openai.yaml");

    for scan_root in [root.path(), skill.as_path()] {
        let (output, report) = run_json(scan_root, &[]);
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(codes(&report), BTreeSet::from(["OPENAI_METADATA_MISSING"]));
        assert_eq!(finding(&report, "OPENAI_METADATA_MISSING")["fixable"], true);
        assert!(!metadata.exists());
    }

    let (output, report) = run_json(root.path(), &["--fix-safe"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(report["counts"]["findings"], 0);
    assert_eq!(report["counts"]["fixes"], 1);
    assert_eq!(report["fixes"][0]["code"], "OPENAI_METADATA_CREATED");
    assert_eq!(fs::read_to_string(metadata).unwrap(), "policy:\n  allow_implicit_invocation: true\n");
}

#[test]
fn installed_exposures_validate_and_safely_update_declared_metadata() {
    let parent = TempDir::new().unwrap();
    let root = parent.path().join(".codex");
    write_skill(&root, "alpha", "", "## Completion\n\nReport verification.");
    write_skill(&root, "beta", "disable-model-invocation: true\n", "## Completion\n\nReport verification.");
    write_metadata(&root, "alpha", "policy:\n  allow_implicit_invocation: not-a-boolean\n");
    write_metadata(&root, "beta", "policy:\n  allow_implicit_invocation: true\n");

    let (output, report) = run_json(&root, &[]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(codes(&report), BTreeSet::from(["OPENAI_POLICY_MISMATCH", "OPENAI_POLICY_MISSING"]));

    let (output, report) = run_json(&root, &["--fix-safe"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(codes(&report), BTreeSet::from(["OPENAI_POLICY_MISSING"]));
    assert_eq!(report["counts"]["fixes"], 1);
    assert_eq!(report["fixes"][0]["code"], "OPENAI_POLICY_UPDATED");
    assert_eq!(
        fs::read_to_string(root.join("skills/beta/agents/openai.yaml")).unwrap(),
        "policy:\n  allow_implicit_invocation: false\n"
    );
}

#[test]
fn fix_safe_leaves_missing_installed_metadata_absent() {
    let parent = TempDir::new().unwrap();
    let root = parent.path().join(".claude");
    write_skill(&root, "alpha", "", "## Completion\n\nReport verification.");
    let agents = root.join("skills/alpha/agents");

    let (output, report) = run_json(&root, &["--fix-safe"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(report["counts"]["findings"], 0);
    assert_eq!(report["counts"]["fixes"], 0);
    assert!(!agents.exists());
    assert!(!agents.join("openai.yaml").exists());
}

#[test]
fn complete_portable_claude_and_repository_dialect_is_accepted() {
    let root = TempDir::new().unwrap();
    common::write(
        root.path().join("skills/alpha/SKILL.md"),
        "---\nagent: Explore\nallowed-tools:\n  - Read\n  - Grep\nargument-hint: \"[issue] [branch]\"\narguments: issue branch\nbackground: false\ncompatibility: Requires Git.\ncontext: fork\ncoordination: exempt\ndisable-model-invocation: true\ndisallowed-tools:\n  - AskUserQuestion\neffort: max\nhooks:\n  PreToolUse:\n    - matcher: Bash\n      hooks: []\nlicense: MIT\nmetadata:\n  author: test-suite\n  install-targets: claude-code codex\n  version: \"1\"\nmodel: inherit\nname: alpha\npaths:\n  - \"src/**\"\nshell: bash\nskill-dependencies:\n  - beta\nuser-invocable: false\nwhen_to_use: Use for complete-dialect fixtures.\ndescription: alpha description.\n---\n\n# alpha\n\nThis skill is coordination-exempt: skip the ai-coord gate for its declared work.\n\n## Completion\n\nReport verification.\n",
    );
    write_skill(root.path(), "beta", "", "## Completion\n\nReport verification.");
    write_metadata(root.path(), "alpha", "policy:\n  allow_implicit_invocation: false\n");
    write_metadata(root.path(), "beta", "policy:\n  allow_implicit_invocation: true\n");
    write_readme(root.path(), &["alpha", "beta"]);

    let (output, report) = run_json(root.path(), &[]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stdout));
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["findings"], serde_json::json!([]));
}

#[test]
fn unknown_fields_invalid_enums_and_fork_cross_fields_are_located_and_deterministic() {
    let root = TempDir::new().unwrap();
    common::write(
        root.path().join("skills/invalid/SKILL.md"),
        "---\nagent: Explore\nbackground: true\neffort: extreme\nname: invalid\nshell: zsh\nunknown-option: enabled\ndescription: invalid description.\n---\n\n# invalid\n\n## Completion\n\nReport verification.\n",
    );
    write_metadata(root.path(), "invalid", "policy:\n  allow_implicit_invocation: true\n");
    write_readme(root.path(), &["invalid"]);

    let first =
        common::ai_skillet().args(["doctor", "--root"]).arg(root.path()).args(["--format", "json"]).output().unwrap();
    let second =
        common::ai_skillet().args(["doctor", "--root"]).arg(root.path()).args(["--format", "json"]).output().unwrap();
    assert_eq!(first.status.code(), Some(1));
    assert_eq!(first.stdout, second.stdout);
    let report: Value = serde_json::from_slice(&first.stdout).unwrap();
    for expected in [
        "AGENT_CONTEXT_REQUIRED",
        "BACKGROUND_CONTEXT_REQUIRED",
        "EFFORT_INVALID_VALUE",
        "FRONTMATTER_UNKNOWN_FIELD",
        "SHELL_INVALID_VALUE",
    ] {
        assert!(codes(&report).contains(expected), "missing {expected}: {}", String::from_utf8_lossy(&first.stdout));
    }
    assert_eq!(finding(&report, "AGENT_CONTEXT_REQUIRED")["line"], 2);
    assert_eq!(finding(&report, "BACKGROUND_CONTEXT_REQUIRED")["line"], 3);
    assert_eq!(finding(&report, "FRONTMATTER_UNKNOWN_FIELD")["line"], 7);
    assert_eq!(
        finding(&report, "FRONTMATTER_UNKNOWN_FIELD")["message"],
        "unknown frontmatter field \"unknown-option\""
    );

    common::ai_skillet()
        .args(["doctor", "--root"])
        .arg(root.path())
        .assert()
        .code(1)
        .stdout(predicate::str::contains("FRONTMATTER_UNKNOWN_FIELD"))
        .stdout(predicate::str::contains("AGENT_CONTEXT_REQUIRED"));
}

#[test]
fn explicit_claude_defaults_warn_without_changing_effective_policy() {
    let root = TempDir::new().unwrap();
    common::write(
        root.path().join("skills/defaults/SKILL.md"),
        "---\ndisable-model-invocation: false\nname: defaults\nuser-invocable: true\ndescription: defaults description.\n---\n\n# defaults\n\n## Completion\n\nReport verification.\n",
    );
    write_metadata(root.path(), "defaults", "policy:\n  allow_implicit_invocation: true\n");
    write_readme(root.path(), &["defaults"]);

    let (output, report) = run_json(root.path(), &[]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(report["counts"]["errors"], 0);
    assert_eq!(report["counts"]["warnings"], 2);
    assert_eq!(
        codes(&report),
        BTreeSet::from(["DISABLE_MODEL_INVOCATION_REDUNDANT_DEFAULT", "USER_INVOCABLE_REDUNDANT_DEFAULT",])
    );
    assert_eq!(finding(&report, "DISABLE_MODEL_INVOCATION_REDUNDANT_DEFAULT")["line"], 2);
    assert_eq!(finding(&report, "USER_INVOCABLE_REDUNDANT_DEFAULT")["line"], 4);
}

#[test]
fn coordination_declarations_ignore_markdown_code_quotes_and_example_sections() {
    let root = TempDir::new().unwrap();
    let sentence = "This skill is coordination-exempt: skip the ai-coord gate for its declared work.";
    let fixtures = [
        ("plain", "", format!("{sentence}\n\n## Completion\n\nReport verification.")),
        ("inline", "", format!("Documentation: `{sentence}`\n\n## Completion\n\nReport verification.")),
        ("fenced", "", format!("```markdown\n{sentence}\n```\n\n## Completion\n\nReport verification.")),
        ("quote", "", format!("> {sentence}\n\n## Completion\n\nReport verification.")),
        (
            "example",
            "",
            format!("## Example: coordination policy\n\n{sentence}\n\n## Completion\n\nReport verification."),
        ),
        ("exempt", "coordination: exempt\n", format!("{sentence}\n\n## Completion\n\nReport verification.")),
        (
            "exempt-code",
            "coordination: exempt\n",
            format!("```text\n{sentence}\n```\n\n## Completion\n\nReport verification."),
        ),
    ];
    for (name, fields, body) in &fixtures {
        write_skill(root.path(), name, fields, body);
        write_metadata(root.path(), name, "policy:\n  allow_implicit_invocation: true\n");
    }
    write_readme(root.path(), &fixtures.iter().map(|(name, _, _)| *name).collect::<Vec<_>>());

    let (output, report) = run_json(root.path(), &[]);
    assert_eq!(output.status.code(), Some(1));
    let coordination = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|finding| finding["code"].as_str().unwrap().starts_with("COORDINATION_"))
        .collect::<Vec<_>>();
    assert_eq!(coordination.len(), 2, "{coordination:#?}");
    assert!(
        finding(&report, "COORDINATION_EXEMPT_FRONTMATTER_MISSING")["path"]
            .as_str()
            .unwrap()
            .ends_with("skills/plain/SKILL.md")
    );
    assert!(
        finding(&report, "COORDINATION_EXEMPT_SENTENCE_MISSING")["path"]
            .as_str()
            .unwrap()
            .ends_with("skills/exempt-code/SKILL.md")
    );
}

#[test]
fn full_audit_covers_metadata_coordination_versions_links_readme_and_hygiene() {
    let root = TempDir::new().unwrap();
    let oversized = (0..400).map(|_| "reference\n").collect::<String>();
    common::write(root.path().join("outside.md"), "outside\n");
    common::write(root.path().join("skills/demo/references/large.md"), oversized);
    common::write(
        root.path().join("skills/demo/SKILL.md"),
        "---\nname: Wrong_Name\nmodel: opus\nmetadata: nope\ncoordination: exempt\ncompatibility: 42\ndescription: Demo.\n---\n\n# Demo\n\nAlways delete generated files. Never delete generated files.\nAlways read [large](references/large.md) before work.\nSee [missing](scripts/missing.sh).\nDo not follow [outside](references/../../../outside.md).\n",
    );
    write_metadata(root.path(), "demo", "policy:\n  allow_implicit_invocation: false\n");
    write_skill(root.path(), "cli-tool", "", "## Completion\n\nReport verification.");
    write_metadata(root.path(), "cli-tool", "policy:\n  allow_implicit_invocation: true\n");
    common::write(root.path().join("skills/cli-tool/references/version.txt"), "v1.2.3\n");
    write_readme(root.path(), &["demo", "ghost"]);

    let (output, report) = run_json(root.path(), &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let found = codes(&report);
    for expected in [
        "CLI_VERSION_INVALID",
        "COMPATIBILITY_INVALID",
        "COMPLETION_EVIDENCE_MISSING",
        "CONFLICTING_AUTHORITY",
        "COORDINATION_EXEMPT_SENTENCE_MISSING",
        "FRONTMATTER_FIELD_ORDER",
        "METADATA_INVALID",
        "NAME_DIRECTORY_MISMATCH",
        "NAME_INVALID",
        "OPENAI_POLICY_MISMATCH",
        "README_LISTS_MISSING",
        "README_SKILL_MISSING",
        "RESOURCE_LINK_OUTSIDE_SKILL",
        "RESOURCE_LINK_MISSING",
        "STALE_MODEL_PIN",
        "UNCONDITIONAL_REFERENCE_OVERSIZED",
    ] {
        assert!(found.contains(expected), "missing {expected}: {found:?}");
    }
}

#[cfg(unix)]
#[test]
fn resource_symlink_escape_is_rejected_without_reclassifying_missing_target() {
    use std::os::unix::fs::symlink;

    let root = TempDir::new().unwrap();
    let outside = root.path().join("outside.md");
    let outside_directory = root.path().join("outside-references");
    common::write(&outside, (0..400).map(|_| "outside\n").collect::<String>());
    fs::create_dir_all(&outside_directory).unwrap();
    write_skill(
        root.path(),
        "alpha",
        "",
        "Always read [outside](references/outside.md) before work.\nSee [missing](references/external/missing.md).\n\n## Completion\n\nReport verification.",
    );
    let references = root.path().join("skills/alpha/references");
    fs::create_dir_all(&references).unwrap();
    symlink(outside, references.join("outside.md")).unwrap();
    symlink(outside_directory, references.join("external")).unwrap();
    write_metadata(root.path(), "alpha", "policy:\n  allow_implicit_invocation: true\n");
    write_readme(root.path(), &["alpha"]);

    let (output, report) = run_json(root.path(), &[]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(codes(&report), BTreeSet::from(["RESOURCE_LINK_MISSING", "RESOURCE_LINK_OUTSIDE_SKILL"]));
    assert_eq!(
        finding(&report, "RESOURCE_LINK_OUTSIDE_SKILL")["message"],
        "resource link must stay inside its skill directory: references/outside.md"
    );
    assert_eq!(
        finding(&report, "RESOURCE_LINK_MISSING")["message"],
        "referenced resource does not exist: references/external/missing.md"
    );
}

#[test]
fn dependencies_only_uses_all_roots_and_suppresses_unrelated_findings() {
    let first = TempDir::new().unwrap();
    let second = TempDir::new().unwrap();
    common::write(
        first.path().join("skills/alpha/SKILL.md"),
        "---\nname: alpha\nskill-dependencies:\n  - Acme/Tools#beta\n  - beta\ndescription: alpha description.\n---\n\n# alpha\n\nNo completion contract.\n",
    );
    write_skill(second.path(), "beta", "", "No completion contract.");

    let output = common::ai_skillet()
        .args(["doctor", "--root"])
        .arg(first.path())
        .args(["--root"])
        .arg(second.path())
        .args(["--dependencies-only", "--format", "json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["findings"], serde_json::json!([]));

    common::write(
        first.path().join("skills/broken/SKILL.md"),
        "---\nname: broken\nskill-dependencies: beta\ndescription: broken\n---\n",
    );
    let (output, report) = run_json(first.path(), &["--dependencies-only"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(codes(&report).contains("SKILL_DEPENDENCIES_NOT_ARRAY"));
    assert!(!codes(&report).contains("OPENAI_METADATA_MISSING"));
    assert!(!codes(&report).contains("COMPLETION_EVIDENCE_MISSING"));
    assert!(!codes(&report).contains("README_MISSING"));
}

#[test]
fn targeted_catalog_and_direct_skill_resolve_siblings_without_auditing_them() {
    let root = TempDir::new().unwrap();
    write_dependent_skill(root.path(), "alpha", &["beta"], "## Completion\n\nReport verification.");
    write_metadata(root.path(), "alpha", "policy:\n  allow_implicit_invocation: true\n");
    common::write(root.path().join("skills/beta/SKILL.md"), "# malformed sibling\n");

    let (catalog_output, catalog_report) = run_json(root.path(), &["--skill", "alpha"]);
    assert!(catalog_output.status.success(), "{}", String::from_utf8_lossy(&catalog_output.stderr));
    assert_eq!(catalog_report["findings"], serde_json::json!([]));
    assert_eq!(catalog_report["roots"][0]["active_skills"], 1);

    let direct = root.path().join("skills/alpha");
    let (direct_output, direct_report) = run_json(&direct, &[]);
    assert!(direct_output.status.success(), "{}", String::from_utf8_lossy(&direct_output.stderr));
    assert_eq!(direct_report["findings"], serde_json::json!([]));
    assert_eq!(direct_report["roots"][0]["path"], serde_json::json!(direct));
    assert_eq!(direct_report["roots"][0]["active_skills"], 1);

    let (unfiltered_output, unfiltered_report) = run_json(root.path(), &[]);
    assert_eq!(unfiltered_output.status.code(), Some(1));
    assert!(codes(&unfiltered_report).contains("FRONTMATTER_DELIMITER_MISSING"));
}

#[test]
fn targeted_dependency_resolution_still_reports_missing_dependencies() {
    let root = TempDir::new().unwrap();
    write_dependent_skill(root.path(), "alpha", &["missing"], "");

    let (catalog_output, catalog_report) = run_json(root.path(), &["--skill", "alpha", "--dependencies-only"]);
    assert_eq!(catalog_output.status.code(), Some(1));
    assert_eq!(codes(&catalog_report), BTreeSet::from(["SKILL_DEPENDENCY_UNRESOLVED"]));

    let (direct_output, direct_report) = run_json(&root.path().join("skills/alpha"), &["--dependencies-only"]);
    assert_eq!(direct_output.status.code(), Some(1));
    assert_eq!(codes(&direct_report), BTreeSet::from(["SKILL_DEPENDENCY_UNRESOLVED"]));
}

#[test]
fn standalone_skill_root_does_not_infer_a_sibling_resolution_context() {
    let root = TempDir::new().unwrap();
    let standalone = root.path().join("alpha");
    common::write(
        standalone.join("SKILL.md"),
        "---\nname: alpha\nskill-dependencies:\n  - beta\ndescription: alpha description.\n---\n# alpha\n",
    );
    common::write(root.path().join("beta/SKILL.md"), common::skill("beta", ""));

    let (output, report) = run_json(&standalone, &["--dependencies-only"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(codes(&report), BTreeSet::from(["SKILL_DEPENDENCY_UNRESOLVED"]));
}

#[test]
fn ignored_direct_skill_remains_auditable_with_sibling_resolution() {
    let root = TempDir::new().unwrap();
    fs::create_dir(root.path().join(".git")).unwrap();
    common::write(root.path().join(".gitignore"), "skills/alpha/\n");
    write_dependent_skill(root.path(), "alpha", &["beta"], "");
    write_skill(root.path(), "beta", "", "");

    let (output, report) = run_json(&root.path().join("skills/alpha"), &["--dependencies-only"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(report["findings"], serde_json::json!([]));
    assert_eq!(report["roots"][0]["active_skills"], 1);
}

#[test]
fn skill_filter_targets_directory_names_despite_frontmatter_mismatch() {
    let root = TempDir::new().unwrap();
    common::write(
        root.path().join("skills/alpha/SKILL.md"),
        "---\nname: wrong-name\ndescription: alpha description.\n---\n\n# alpha\n\n## Completion\n\nReport verification.\n",
    );
    write_metadata(root.path(), "alpha", "policy:\n  allow_implicit_invocation: true\n");

    let (output, report) = run_json(root.path(), &["--skill", "alpha"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(codes(&report), BTreeSet::from(["NAME_DIRECTORY_MISMATCH"]));
    assert!(finding(&report, "NAME_DIRECTORY_MISMATCH")["path"].as_str().unwrap().ends_with("skills/alpha/SKILL.md"));
}

#[test]
fn invalid_and_undiscovered_doctor_filters_are_operational_errors() {
    let root = TempDir::new().unwrap();
    write_skill(root.path(), "alpha", "", "");

    common::ai_skillet()
        .args(["doctor", "--root"])
        .arg(root.path())
        .args(["--skill", "Bad_Name"])
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("ai-skillet: invalid skill name filter: Bad_Name\n"));
    common::ai_skillet()
        .args(["doctor", "--root"])
        .arg(root.path())
        .args(["--skill", "missing"])
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "ai-skillet: doctor skill filter did not match a discovered directory: missing\n",
        ));
}

#[test]
fn dependencies_only_skill_filter_does_not_recursively_audit_dependencies() {
    let root = TempDir::new().unwrap();
    write_dependent_skill(root.path(), "alpha", &["beta"], "");
    write_dependent_skill(root.path(), "beta", &["missing"], "");

    let (alpha_output, alpha_report) = run_json(root.path(), &["--skill", "alpha", "--dependencies-only"]);
    assert!(alpha_output.status.success(), "{}", String::from_utf8_lossy(&alpha_output.stderr));
    assert_eq!(alpha_report["findings"], serde_json::json!([]));

    let (beta_output, beta_report) = run_json(root.path(), &["--skill", "beta", "--dependencies-only"]);
    assert_eq!(beta_output.status.code(), Some(1));
    assert_eq!(codes(&beta_report), BTreeSet::from(["SKILL_DEPENDENCY_UNRESOLVED"]));
}

#[test]
fn fix_safe_skill_filter_changes_only_selected_metadata() {
    let root = TempDir::new().unwrap();
    write_skill(root.path(), "alpha", "", "## Completion\n\nReport verification.");
    write_skill(root.path(), "beta", "", "## Completion\n\nReport verification.");

    let (output, report) = run_json(root.path(), &["--skill", "alpha", "--fix-safe"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(report["counts"]["fixes"], 1);
    assert!(root.path().join("skills/alpha/agents/openai.yaml").is_file());
    assert!(!root.path().join("skills/beta/agents/openai.yaml").exists());
}

#[test]
fn mixed_direct_and_catalog_roots_apply_scope_independently() {
    let direct_catalog = TempDir::new().unwrap();
    let full_catalog = TempDir::new().unwrap();
    write_skill(direct_catalog.path(), "alpha", "", "");
    common::write(direct_catalog.path().join("skills/hidden/SKILL.md"), "# hidden malformed sibling\n");
    write_skill(full_catalog.path(), "visible", "", "");
    common::write(full_catalog.path().join("skills/broken/SKILL.md"), "# audited malformed sibling\n");

    let run = || {
        common::ai_skillet()
            .args(["doctor", "--root"])
            .arg(direct_catalog.path().join("skills/alpha"))
            .args(["--root"])
            .arg(full_catalog.path())
            .args(["--dependencies-only", "--format", "json"])
            .output()
            .unwrap()
    };
    let first = run();
    let second = run();
    assert_eq!(first.status.code(), Some(1));
    assert_eq!(first.stdout, second.stdout);
    let report: Value = serde_json::from_slice(&first.stdout).unwrap();
    let findings = report["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1, "{findings:#?}");
    assert!(findings[0]["path"].as_str().unwrap().ends_with("skills/broken/SKILL.md"));
}

#[test]
fn a_skill_filter_audits_every_matching_exposure() {
    let first = TempDir::new().unwrap();
    let second = TempDir::new().unwrap();
    write_dependent_skill(first.path(), "alpha", &["missing"], "");
    write_dependent_skill(second.path(), "alpha", &["missing"], "");

    let output = common::ai_skillet()
        .args(["doctor", "--root"])
        .arg(first.path())
        .args(["--root"])
        .arg(second.path())
        .args(["--skill", "alpha", "--dependencies-only", "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["findings"].as_array().unwrap().len(), 2);
    assert!(report["findings"].as_array().unwrap().iter().all(|finding| {
        finding["code"] == "SKILL_DEPENDENCY_UNRESOLVED" &&
            finding["path"].as_str().unwrap().ends_with("skills/alpha/SKILL.md")
    }));
    assert!(report["roots"].as_array().unwrap().iter().all(|root| root["active_skills"] == 1));
}

#[cfg(unix)]
#[test]
fn repeated_symlinked_direct_and_catalog_roots_are_deterministic() {
    use std::os::unix::fs::symlink;

    let source = TempDir::new().unwrap();
    let installed = TempDir::new().unwrap();
    write_dependent_skill(source.path(), "alpha", &["beta"], "");
    write_skill(source.path(), "beta", "", "");
    let installed_root = installed.path().join(".agents");
    fs::create_dir_all(installed_root.join("skills")).unwrap();
    symlink(source.path().join("skills/alpha"), installed_root.join("skills/alpha")).unwrap();
    symlink(source.path().join("skills/beta"), installed_root.join("skills/beta")).unwrap();
    let direct = installed_root.join("skills/alpha");

    let (direct_output, direct_report) = run_json(&direct, &["--dependencies-only"]);
    assert!(direct_output.status.success(), "{}", String::from_utf8_lossy(&direct_output.stderr));
    assert_eq!(direct_report["findings"], serde_json::json!([]));

    let run = || {
        common::ai_skillet()
            .args(["doctor", "--root"])
            .arg(&direct)
            .args(["--root"])
            .arg(&installed_root)
            .args(["--root"])
            .arg(&direct)
            .args(["--skill", "alpha", "--skill", "alpha", "--dependencies-only", "--format", "json"])
            .output()
            .unwrap()
    };
    let first = run();
    let second = run();
    assert!(first.status.success(), "{}", String::from_utf8_lossy(&first.stderr));
    assert_eq!(first.stdout, second.stdout);
    let report: Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(report["findings"], serde_json::json!([]));
    assert_eq!(report["roots"].as_array().unwrap().len(), 2);
    assert!(report["roots"].as_array().unwrap().iter().all(|root| root["active_skills"] == 1));
}

#[test]
fn dependencies_report_every_validation_family_and_malformed_frontmatter() {
    let root = TempDir::new().unwrap();
    common::write(root.path().join("skills/no-frontmatter/SKILL.md"), "# Missing\n");
    write_skill(
        root.path(),
        "alpha",
        "skill-dependencies:\n  - missing\n  - alpha\n  - beta\n  - beta\n  - Bad/Shape\n  - 42\n",
        "",
    );
    write_skill(root.path(), "beta", "", "");
    let (_, report) = run_json(root.path(), &["--dependencies-only"]);
    let found = codes(&report);
    for expected in [
        "FRONTMATTER_DELIMITER_MISSING",
        "SKILL_DEPENDENCIES_ORDER",
        "SKILL_DEPENDENCY_DUPLICATE",
        "SKILL_DEPENDENCY_INVALID",
        "SKILL_DEPENDENCY_NOT_STRING",
        "SKILL_DEPENDENCY_SELF",
        "SKILL_DEPENDENCY_UNRESOLVED",
    ] {
        assert!(found.contains(expected), "missing {expected}: {found:?}");
    }
}

#[test]
fn fix_safe_creates_and_updates_metadata_without_other_byte_changes() {
    let root = TempDir::new().unwrap();
    write_skill(root.path(), "alpha", "", "## Completion\n\nReport verification.");
    write_skill(root.path(), "beta", "disable-model-invocation: true\n", "## Completion\n\nReport verification.");
    write_metadata(
        root.path(),
        "beta",
        "interface:\n  allow_implicit_invocation: true\npolicy:\n  note: keep-me\n  allow_implicit_invocation: true # retained\nui:\n  title: Beta\n",
    );
    write_readme(root.path(), &["alpha", "beta"]);
    common::write(root.path().join("unrelated.bin"), b"\0unchanged\xff");

    let skill_before = fs::read(root.path().join("skills/beta/SKILL.md")).unwrap();
    let readme_before = fs::read(root.path().join("README.md")).unwrap();
    let unrelated_before = fs::read(root.path().join("unrelated.bin")).unwrap();
    let (output, report) = run_json(root.path(), &["--fix-safe"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(report["counts"]["fixes"], 2);
    assert_eq!(report["counts"]["findings"], 0);
    assert_eq!(
        fs::read_to_string(root.path().join("skills/alpha/agents/openai.yaml")).unwrap(),
        "policy:\n  allow_implicit_invocation: true\n"
    );
    assert_eq!(
        fs::read_to_string(root.path().join("skills/beta/agents/openai.yaml")).unwrap(),
        "interface:\n  allow_implicit_invocation: true\npolicy:\n  note: keep-me\n  allow_implicit_invocation: false # retained\nui:\n  title: Beta\n"
    );
    assert_eq!(fs::read(root.path().join("skills/beta/SKILL.md")).unwrap(), skill_before);
    assert_eq!(fs::read(root.path().join("README.md")).unwrap(), readme_before);
    assert_eq!(fs::read(root.path().join("unrelated.bin")).unwrap(), unrelated_before);
}

#[test]
fn failed_fix_is_exit_three_and_leaves_target_and_directories_unchanged() {
    let root = TempDir::new().unwrap();
    write_skill(root.path(), "alpha", "disable-model-invocation: true\n", "## Completion\n\nReport verification.");
    write_metadata(root.path(), "alpha", "policy: { allow_implicit_invocation: true }\n");
    write_readme(root.path(), &["alpha"]);
    let path = root.path().join("skills/alpha/agents/openai.yaml");
    let before = fs::read(&path).unwrap();

    let (output, report) = run_json(root.path(), &["--fix-safe"]);
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stderr.is_empty());
    assert!(codes(&report).contains("OPENAI_METADATA_FIX_FAILED"));
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(
        fs::read_dir(path.parent().unwrap()).unwrap().map(|entry| entry.unwrap().file_name()).collect::<Vec<_>>(),
        [std::ffi::OsString::from("openai.yaml")]
    );
}

#[test]
fn safe_fix_failures_are_isolated_from_successful_atomic_fixes() {
    let root = TempDir::new().unwrap();
    write_skill(root.path(), "alpha", "", "## Completion\n\nReport verification.");
    write_skill(root.path(), "beta", "disable-model-invocation: true\n", "## Completion\n\nReport verification.");
    write_metadata(root.path(), "beta", "policy: { allow_implicit_invocation: true }\n");
    write_readme(root.path(), &["alpha", "beta"]);
    let failed_path = root.path().join("skills/beta/agents/openai.yaml");
    let failed_before = fs::read(&failed_path).unwrap();

    let (output, report) = run_json(root.path(), &["--fix-safe"]);
    assert_eq!(output.status.code(), Some(3));
    assert!(codes(&report).contains("OPENAI_METADATA_FIX_FAILED"));
    assert_eq!(report["counts"]["fixes"], 1);
    assert_eq!(
        fs::read_to_string(root.path().join("skills/alpha/agents/openai.yaml")).unwrap(),
        "policy:\n  allow_implicit_invocation: true\n"
    );
    assert_eq!(fs::read(&failed_path).unwrap(), failed_before);
}

#[test]
fn output_is_deterministic_default_root_works_and_operational_errors_exit_two() {
    let root = common::fixture("doctor/catalog");
    let run = || common::ai_skillet().args(["doctor", "--format", "json"]).current_dir(&root).output().unwrap();
    let first = run();
    let second = run();
    assert!(first.status.success());
    assert_eq!(first.stdout, second.stdout);
    assert!(first.stderr.is_empty());

    let missing = TempDir::new().unwrap().path().join("missing");
    common::ai_skillet()
        .args(["doctor", "--root"])
        .arg(missing)
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("ai-skillet: root does not exist:"));
}
