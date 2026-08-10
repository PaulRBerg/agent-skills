mod common;

use std::path::PathBuf;

use ai_skillet::cli::{Cli, Command, DoctorFormat, MapFormat};
use clap::Parser;
use predicates::prelude::*;

#[test]
fn version_reports_the_package_version() {
    common::ai_skillet()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^ai-skillet 0\.1\.0\n$").unwrap());
}

#[test]
fn map_help_lists_its_supported_options() {
    common::ai_skillet()
        .args(["map", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--root <PATH>"))
        .stdout(predicate::str::contains("--portfolio-root <PATH>"))
        .stdout(predicate::str::contains("--include-catalog-sources"))
        .stdout(predicate::str::contains("--include-self"))
        .stdout(predicate::str::contains("--include-snippets"))
        .stdout(predicate::str::contains("--show-skipped"))
        .stdout(predicate::str::contains("--format <FORMAT>"));
}

#[test]
fn doctor_help_lists_its_supported_options() {
    common::ai_skillet()
        .args(["doctor", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--root <PATH>"))
        .stdout(predicate::str::contains("--dependencies-only"))
        .stdout(predicate::str::contains("--fix-safe"))
        .stdout(predicate::str::contains("--format <FORMAT>"));
}

#[test]
fn map_supports_repeatable_roots_and_skills() {
    let cli = Cli::try_parse_from([
        "ai-skillet",
        "map",
        "--root",
        "first",
        "--root",
        "second",
        "--skill",
        "alpha",
        "--skill",
        "beta",
        "--format",
        "json",
    ])
    .expect("map arguments should parse");

    let Command::Map(args) = cli.command else {
        panic!("expected map command");
    };
    assert_eq!(args.root, [PathBuf::from("first"), PathBuf::from("second")]);
    assert_eq!(args.skill, ["alpha", "beta"]);
    assert_eq!(args.format, MapFormat::Json);
}

#[test]
fn doctor_supports_repeatable_roots() {
    let cli = Cli::try_parse_from([
        "ai-skillet",
        "doctor",
        "--root",
        "first",
        "--root",
        "second",
        "--format",
        "json",
    ])
    .expect("doctor arguments should parse");

    let Command::Doctor(args) = cli.command else {
        panic!("expected doctor command");
    };
    assert_eq!(args.root, [PathBuf::from("first"), PathBuf::from("second")]);
    assert_eq!(args.format, DoctorFormat::Json);
}

#[test]
fn map_root_and_portfolio_root_are_mutually_exclusive() {
    common::ai_skillet()
        .args(["map", "--root", "catalog", "--portfolio-root", "portfolio"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn doctor_fix_safe_and_dependencies_only_are_mutually_exclusive() {
    common::ai_skillet()
        .args(["doctor", "--fix-safe", "--dependencies-only"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn missing_option_values_exit_with_usage_error() {
    common::ai_skillet()
        .args(["map", "--root"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("a value is required"));
}

#[test]
fn unknown_subcommands_fail_with_a_clap_error() {
    common::ai_skillet()
        .arg("unknown")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("unrecognized subcommand 'unknown'"));
}
