mod common;

use std::collections::BTreeSet;
use std::fs;

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
        format!(
            "---\n{fields}name: {name}\ndescription: {name} description.\n---\n\n# {name}\n\n{body}\n"
        ),
    );
}

fn write_metadata(root: &std::path::Path, name: &str, contents: &str) {
    common::write(root.join("skills").join(name).join("agents/openai.yaml"), contents);
}

fn write_readme(root: &std::path::Path, names: &[&str]) {
    let rows = names.iter().map(|name| format!("| {name} | {name} |\n")).collect::<String>();
    common::write(
        root.join("README.md"),
        format!("# Catalog\n\n## Skills\n\n| Skill | Description |\n| --- | --- |\n{rows}"),
    );
}

fn codes(report: &Value) -> BTreeSet<&str> {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|finding| finding["code"].as_str().unwrap())
        .collect()
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
    write_skill(
        root.path(),
        "beta",
        "disable-model-invocation: true\n",
        "## Completion\n\nReport verification.",
    );
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
    write_skill(
        root.path(),
        "alpha",
        "disable-model-invocation: true\n",
        "## Completion\n\nReport verification.",
    );
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
        fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>(),
        [std::ffi::OsString::from("openai.yaml")]
    );
}

#[test]
fn safe_fix_failures_are_isolated_from_successful_atomic_fixes() {
    let root = TempDir::new().unwrap();
    write_skill(root.path(), "alpha", "", "## Completion\n\nReport verification.");
    write_skill(
        root.path(),
        "beta",
        "disable-model-invocation: true\n",
        "## Completion\n\nReport verification.",
    );
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
    let run = || {
        common::ai_skillet()
            .args(["doctor", "--format", "json"])
            .current_dir(&root)
            .output()
            .unwrap()
    };
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
