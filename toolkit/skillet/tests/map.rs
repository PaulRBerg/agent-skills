mod common;

use std::{fs, process::Command as ProcessCommand};

use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

fn fixture_catalog() -> std::path::PathBuf {
    common::fixture("map/catalog")
}

fn json_map(arguments: &[&str]) -> Value {
    let output = common::ai_skillet().args(arguments).output().unwrap();
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn json_schema_retains_declared_and_inferred_evidence() {
    let root = fixture_catalog();
    let report = json_map(&["map", "--root", root.to_str().unwrap(), "--format", "json", "--include-snippets"]);

    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["counts"]["skills"], 2);
    assert_eq!(report["counts"]["declared_dependencies"], 2);
    assert_eq!(report["counts"]["inferred_dependencies"], 1);
    assert_eq!(report["counts"]["external_references"], 2);
    assert_eq!(report["counts"]["unresolved"], 1);
    assert!(report["roots"][0]["exposure_path"].is_string());

    let edges = report["edges"].as_array().unwrap();
    let beta_evidence: Vec<_> =
        edges.iter().filter(|edge| edge["source"] == "alpha" && edge["target"] == "beta").collect();
    assert_eq!(beta_evidence.len(), 2);
    assert!(beta_evidence.iter().any(|edge| edge["provenance"] == "declared"));
    assert!(beta_evidence.iter().any(|edge| edge["provenance"] == "inferred"));
    let external = edges.iter().find(|edge| edge["identifier"] == "Acme/Tools#external-tool").unwrap();
    assert_eq!(external["target"], "Acme/Tools#external-tool");
    assert_eq!(external["target_repository"], "Acme/Tools");
    assert!(edges.iter().all(|edge| edge["snippet"].is_string()));

    let alpha = report["skills"].as_array().unwrap().iter().find(|skill| skill["name"] == "alpha").unwrap();
    assert_eq!(alpha["skill_dependencies"][0], "beta");
    assert_eq!(alpha["skill_sha256"].as_str().unwrap().len(), 64);
    assert_eq!(alpha["tree_sha256"].as_str().unwrap().len(), 64);
}

#[test]
fn filter_selects_skill_and_inbound_edges_while_missing_filters_warn_and_succeed() {
    let root = fixture_catalog();
    let root = root.to_str().unwrap();
    let filtered = json_map(&["map", "--root", root, "--skill", "beta", "--format", "json"]);
    assert_eq!(filtered["skills"].as_array().unwrap().len(), 1);
    assert_eq!(filtered["skills"][0]["name"], "beta");
    assert!(
        filtered["edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| { edge["source"] == "alpha" && edge["target"] == "beta" })
    );

    common::ai_skillet()
        .args(["map", "--root", root, "--skill", "definitely-missing", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"skills\": []"))
        .stderr(predicate::str::contains("ai-skillet: warning: no discovered skill named definitely-missing\n"));
}

#[test]
fn snippets_are_opt_in_and_skipped_policy_is_optional() {
    let root = fixture_catalog();
    let root = root.to_str().unwrap();
    let ordinary = json_map(&["map", "--root", root, "--format", "json"]);
    assert!(ordinary.get("skipped").is_none());
    assert!(ordinary["edges"].as_array().unwrap().iter().all(|edge| edge.get("snippet").is_none()));
    let verbose = json_map(&["map", "--root", root, "--format", "json", "--show-skipped", "--include-snippets"]);
    assert!(verbose["skipped"]["directories"].as_array().unwrap().len() >= 10);
    assert!(verbose["unresolved"][0]["snippet"].is_string());
}

#[test]
fn dot_deduplicates_pairs_and_keeps_full_external_identifiers() {
    let root = fixture_catalog();
    let output =
        common::ai_skillet().args(["map", "--root", root.to_str().unwrap(), "--format", "dot"]).output().unwrap();
    assert!(output.status.success());
    let dot = String::from_utf8(output.stdout).unwrap();
    assert_eq!(dot.matches("\"alpha\" -> \"beta\";").count(), 1);
    assert!(dot.contains("\"alpha\" -> \"Acme/Tools#external-tool\";"));
    assert!(dot.starts_with("digraph skill_map {\n"));
    assert!(dot.ends_with("}\n"));
}

#[test]
fn default_broad_root_excludes_agent_homes_and_catalog_sources_unless_enabled() {
    let home = TempDir::new().unwrap();
    common::write(home.path().join("workspace/tool/SKILL.md"), common::skill("tool", ""));
    common::write(home.path().join(".agents/skills/installed/SKILL.md"), common::skill("installed", ""));
    common::write(home.path().join("projects/agent-skills/skills/source/SKILL.md"), common::skill("source", ""));

    let output = common::ai_skillet().args(["map", "--format", "json"]).env("HOME", home.path()).output().unwrap();
    assert!(output.status.success());
    let ordinary: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(ordinary["roots"][0]["mode"], "broad");
    assert_eq!(ordinary["skills"].as_array().unwrap().len(), 1);
    assert_eq!(ordinary["skills"][0]["name"], "tool");

    let output = common::ai_skillet()
        .args(["map", "--format", "json", "--include-catalog-sources"])
        .env("HOME", home.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let included: Value = serde_json::from_slice(&output.stdout).unwrap();
    let names: Vec<_> =
        included["skills"].as_array().unwrap().iter().map(|skill| skill["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["source", "tool"]);
}

#[test]
fn explicit_ignored_skill_root_is_still_mapped() {
    let catalog = fixture_catalog();
    let parent = json_map(&["map", "--root", catalog.to_str().unwrap(), "--format", "json"]);
    assert!(parent["skills"].as_array().unwrap().iter().all(|skill| skill["name"] != "ignored"));

    let ignored = catalog.join("skills/ignored");
    let direct = json_map(&["map", "--root", ignored.to_str().unwrap(), "--format", "json"]);
    assert_eq!(direct["skills"].as_array().unwrap().len(), 1);
    assert_eq!(direct["skills"][0]["name"], "ignored");
}

#[cfg(unix)]
#[test]
fn duplicate_detection_requires_distinct_real_directories() {
    let root = TempDir::new().unwrap();
    let first = TempDir::new().unwrap();
    let second = TempDir::new().unwrap();
    common::write(first.path().join("SKILL.md"), common::skill("shared", ""));
    common::write(second.path().join("SKILL.md"), common::skill("shared", ""));
    fs::create_dir_all(root.path().join(".agents/skills")).unwrap();
    fs::create_dir_all(root.path().join(".claude/skills")).unwrap();
    fs::create_dir_all(root.path().join("skills")).unwrap();
    std::os::unix::fs::symlink(first.path(), root.path().join(".agents/skills/shared")).unwrap();
    std::os::unix::fs::symlink(first.path(), root.path().join(".claude/skills/shared")).unwrap();
    std::os::unix::fs::symlink(second.path(), root.path().join("skills/shared")).unwrap();

    let report = json_map(&["map", "--root", root.path().to_str().unwrap(), "--format", "json"]);
    assert_eq!(report["skills"].as_array().unwrap().len(), 3);
    assert_eq!(report["duplicates"].as_array().unwrap().len(), 1);
    assert_eq!(report["duplicates"][0]["exposure_paths"].as_array().unwrap().len(), 3);
    assert_eq!(report["duplicates"][0]["resolved_directories"].as_array().unwrap().len(), 2);
}

#[test]
fn portfolio_resolves_git_root_and_adds_present_user_roots() {
    let repository = TempDir::new().unwrap();
    assert!(ProcessCommand::new("git").args(["init", "-q"]).current_dir(repository.path()).status().unwrap().success());
    common::write(repository.path().join("skills/repository-skill/SKILL.md"), common::skill("repository-skill", ""));
    fs::create_dir(repository.path().join("nested")).unwrap();
    let home = TempDir::new().unwrap();
    common::write(home.path().join(".agents/skills/codex-skill/SKILL.md"), common::skill("codex-skill", ""));
    common::write(home.path().join(".claude/skills/claude-skill/SKILL.md"), common::skill("claude-skill", ""));

    let output = common::ai_skillet()
        .args(["map", "--portfolio-root", repository.path().join("nested").to_str().unwrap(), "--format", "json"])
        .env("HOME", home.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["portfolio"]["repository_root"], fs::canonicalize(repository.path()).unwrap().to_str().unwrap());
    assert_eq!(report["portfolio"]["user_roots"].as_array().unwrap().len(), 2);
    assert_eq!(report["roots"].as_array().unwrap().len(), 3);
    let user_skills: Vec<_> =
        report["skills"].as_array().unwrap().iter().filter(|skill| skill["location"] == "user").collect();
    assert_eq!(user_skills.len(), 2);
    assert!(user_skills.iter().all(|skill| skill["kind"] == "install"));
}

#[test]
fn newline_paths_large_lines_and_repeated_output_are_supported() {
    let root = TempDir::new().unwrap();
    let skill_directory = root.path().join("skills/space and\nnewline");
    common::write(skill_directory.join("SKILL.md"), common::skill("odd-path", ""));
    let mut long_line = vec![b'x'; 8 * 1024 * 1024];
    long_line.extend_from_slice(b" $odd-path");
    common::write(root.path().join("large notes.md"), long_line);
    common::write(root.path().join("line numbers.md"), format!("{}\n$missing-skill\n$0 $1\n", "x".repeat(128 * 1024)));

    let run = || {
        common::ai_skillet()
            .args(["map", "--root"])
            .arg(root.path())
            .args(["--format", "json", "--include-snippets"])
            .output()
            .unwrap()
    };
    let first = run();
    let second = run();
    assert!(first.status.success());
    assert_eq!(first.stdout, second.stdout);
    let report: Value = serde_json::from_slice(&first.stdout).unwrap();
    assert!(report["skills"][0]["exposure_path"].as_str().unwrap().contains('\n'));
    assert_eq!(report["counts"]["external_references"], 1);
    assert_eq!(report["counts"]["unresolved"], 1);
    assert_eq!(report["unresolved"][0]["line"], 2);
    assert_eq!(report["unresolved"][0]["target"], "missing-skill");
    assert_eq!(report["unresolved"][0]["snippet"], "$missing-skill");
    let inferred = report["edges"].as_array().unwrap().iter().find(|edge| edge["target"] == "odd-path").unwrap();
    assert_eq!(inferred["snippet"], "$odd-path");
}

#[test]
fn operational_git_and_filter_errors_exit_two() {
    let missing = TempDir::new().unwrap().path().join("missing");
    common::ai_skillet()
        .args(["map", "--root"])
        .arg(&missing)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("root does not exist:"));

    let not_git = TempDir::new().unwrap();
    common::ai_skillet()
        .args(["map", "--portfolio-root"])
        .arg(not_git.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains("not inside a Git repository"));

    common::ai_skillet()
        .args(["map", "--skill", "Not_Canonical"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("invalid skill name filter"));
}
