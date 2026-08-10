mod common;

use ai_commit::cli::{Cli, Command};
use clap::Parser;
use std::{fs, time::Instant};

use common::{Harness, exit_code, stderr, stdout, write_executable};

#[test]
fn commit_messages_accept_hyphen_leading_paragraphs() {
    let transaction = "a9a4f5c260c251a8";
    let subject = "Add shared image uploads to Yeet workflows";
    let body = "- Centralize image validation, uploader fallback, placement, and failure handling.\n- Add full discussion updates and image support.";
    let trailer = "Agent-Session: codex/019fd1e6-c747-7d73-b07d-638c0fab8f3b";

    let cli =
        Cli::try_parse_from(["ai-commit", "commit", transaction, "-m", subject, "-m", body, "-m", trailer, "--push"])
            .unwrap();
    let Command::Commit(args) = cli.command else {
        panic!("expected commit command");
    };

    assert_eq!(args.transaction_id, transaction);
    assert_eq!(args.messages, [subject, body, trailer]);
    assert!(args.push);
}

#[test]
fn local_config_selects_message_format_and_absence_defaults_to_conventional() {
    let harness = Harness::new("local-config");
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.write("intended.txt", "changed\n");

    assert_format(&harness.success(["prepare", "--porcelain", "--", "intended.txt"]), "conventional");

    harness.write(".agents/commit.toml", "[message]\nformat = \"natural\"\n");
    assert_format(&harness.success(["prepare", "--porcelain", "--", "intended.txt"]), "natural");

    harness.write(".agents/commit.toml", "[message]\nformat = \"conventional\"\n");
    assert_format(&harness.success(["prepare", "--porcelain", "--", "intended.txt"]), "conventional");
}

#[test]
fn invalid_local_config_is_a_usage_error_that_identifies_the_path() {
    let harness = Harness::new("invalid-local-config");
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.write("intended.txt", "changed\n");
    let config_path = harness.repo.join(".agents/commit.toml");

    for source in [
        "[message\nformat = \"natural\"\n",
        "[message]\nformat = \"natural\"\nextra = true\n",
        "[message]\n",
        "[message]\nformat = \"verbose\"\n",
    ] {
        harness.write(".agents/commit.toml", source);
        let invalid = harness.command(["prepare", "--", "intended.txt"]);
        assert_eq!(exit_code(&invalid), 2, "{}", stderr(&invalid));
        let error = stderr(&invalid);
        assert!(error.contains("invalid config"), "{error}");
        assert!(error.contains(&config_path.to_string_lossy().into_owned()), "{error}");
    }
}

#[test]
fn explicit_format_overrides_local_config() {
    let harness = Harness::new("config-override");
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.write("intended.txt", "changed\n");

    harness.write(".agents/commit.toml", "[message]\nformat = \"natural\"\n");
    let conventional = harness.success(["prepare", "--porcelain", "--conventional", "--", "intended.txt"]);
    assert_format(&conventional, "conventional");

    harness.write(".agents/commit.toml", "not valid TOML");
    let natural = harness.success(["prepare", "--porcelain", "--natural", "--", "intended.txt"]);
    assert_format(&natural, "natural");
}

#[test]
fn legacy_global_and_environment_config_are_ignored() {
    let harness = Harness::new("legacy-config");
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.write("intended.txt", "changed\n");

    let global_path = harness.root.join("config/ai-commit/config.toml");
    fs::create_dir_all(global_path.parent().unwrap()).unwrap();
    fs::write(&global_path, format!("[message]\nnatural_repositories = [\"{}\"]\n", harness.repo.display())).unwrap();
    let global = harness.success(["prepare", "--porcelain", "--", "intended.txt"]);
    assert_format(&global, "conventional");

    let override_path = harness.root.join("override.toml");
    fs::write(&override_path, "not valid TOML").unwrap();
    let environment = harness.command_with_env(
        ["prepare", "--porcelain", "--", "intended.txt"],
        [("AI_COMMIT_CONFIG", override_path.to_string_lossy().into_owned())],
    );
    assert!(environment.status.success(), "{}", stderr(&environment));
    assert_format(&environment, "conventional");
}

#[test]
fn trailer_is_bounded_and_accepts_only_one_agent_session_line() {
    let harness = Harness::new("trailer");
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.write("intended.txt", "changed\n");
    write_executable(&harness.shim.join("ai-coord"), "#!/bin/sh\nprintf 'Agent-Session: codex:test-session\\n'\n");
    let valid = harness.success(["prepare", "--porcelain", "--", "intended.txt"]);
    assert!(stdout(&valid).contains("TRAILER\tAgent-Session: codex:test-session\n"));

    write_executable(
        &harness.shim.join("ai-coord"),
        "#!/bin/sh\nprintf 'Agent-Session: one\\nAgent-Session: two\\n'\n",
    );
    let invalid = harness.success(["prepare", "--porcelain", "--", "intended.txt"]);
    assert!(!stdout(&invalid).contains("TRAILER\t"));

    write_executable(&harness.shim.join("ai-coord"), "#!/bin/sh\nsleep 5\nprintf 'Agent-Session: late\\n'\n");
    let started = Instant::now();
    let timed_out = harness.success(["prepare", "--porcelain", "--", "intended.txt"]);
    assert!(started.elapsed().as_secs_f32() < 2.5, "trailer command was not bounded");
    assert!(!stdout(&timed_out).contains("TRAILER\t"));
}

#[test]
fn porcelain_is_stable_tsv_with_exact_evidence() {
    let harness = Harness::new("porcelain");
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.write("intended.txt", "changed\n");
    let output = harness.success(["prepare", "--porcelain", "--diff", "full", "--", "intended.txt"]);
    let output = stdout(&output);
    let id = output.lines().find_map(|line| line.strip_prefix("PREPARED\t")).expect("prepared ID");
    assert_eq!(id.len(), 16);
    assert!(id.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()));
    for prefix in ["FORMAT\t", "RULE\t", "BRANCH\t", "CHANGE\t", "SHORTSTAT\t", "DIFF\t", "PATH\t"] {
        assert!(output.lines().any(|line| line.starts_with(prefix)), "missing {prefix}:\n{output}");
    }
    assert!(output.contains("CHANGE\tM\\tintended.txt\n"));
    assert!(output.contains("PATH\tintended.txt\n"));
    assert!(output.contains("Subject line (\\\\<= 50 chars)"));
}

#[test]
fn staged_and_all_modes_capture_their_documented_scope() {
    let staged = Harness::new("staged");
    staged.write("one.txt", "base\n");
    staged.write("two.txt", "base\n");
    staged.commit_all("base");
    staged.write("one.txt", "staged snapshot\n");
    staged.git(["add", "one.txt"]);
    staged.write("one.txt", "later worktree\n");
    staged.write("two.txt", "unstaged\n");
    let output = staged.success(["prepare", "--staged", "--porcelain"]);
    let transaction = prepared_id(&stdout(&output));
    assert!(stdout(&output).contains("PATH\tone.txt\n"));
    assert!(!stdout(&output).contains("PATH\ttwo.txt\n"));
    staged.success(["commit", &transaction, "-m", "test: staged exact"]);
    assert_eq!(staged.git(["show", "HEAD:one.txt"]), "staged snapshot");
    assert_eq!(staged.git(["show", "HEAD:two.txt"]), "base");
    assert_eq!(staged.read("one.txt"), "later worktree\n");

    let all = Harness::new("all");
    all.write("one.txt", "base\n");
    all.commit_all("base");
    all.write("one.txt", "changed\n");
    all.write("two.txt", "untracked\n");
    let output = all.success(["prepare", "--all", "--porcelain"]);
    let transaction = prepared_id(&stdout(&output));
    assert!(stdout(&output).contains("PATH\tone.txt\n"));
    assert!(stdout(&output).contains("PATH\ttwo.txt\n"));
    all.success(["commit", &transaction, "-m", "test: all changes"]);
    assert_eq!(all.git(["show", "HEAD:two.txt"]), "untracked");
}

#[test]
fn repository_root_paths_and_flag_errors_are_validated() {
    let harness = Harness::new("paths");
    harness.write("intended.txt", "base\n");
    harness.write("nested/keep.txt", "base\n");
    harness.commit_all("base");
    harness.write("intended.txt", "changed\n");
    let from_subdir =
        harness.command_at(&harness.repo.join("nested"), ["prepare", "--porcelain", "--", "../intended.txt"]);
    assert!(from_subdir.status.success(), "{}", stderr(&from_subdir));
    assert!(stdout(&from_subdir).contains("PATH\tintended.txt\n"));

    let no_paths = harness.command(["prepare"]);
    assert_eq!(exit_code(&no_paths), 2);
    let outside = harness.command(["prepare", "--", "../outside.txt"]);
    assert_eq!(exit_code(&outside), 2);
    let metadata = harness.command(["prepare", "--", ".git/index"]);
    assert_eq!(exit_code(&metadata), 2);
    let controls = harness.command(["prepare", "--", "bad\nname"]);
    assert_eq!(exit_code(&controls), 2);
    let incompatible = harness.command(["prepare", "--all", "--", "intended.txt"]);
    assert_eq!(exit_code(&incompatible), 2);
    let formats = harness.command(["prepare", "--natural", "--conventional", "--", "intended.txt"]);
    assert_eq!(exit_code(&formats), 2);
}

#[test]
fn signing_failure_is_retryable_and_explicit_bypass_succeeds() {
    let harness = Harness::new("signing");
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.write("intended.txt", "changed\n");
    let (transaction, _) = harness.prepare(&["intended.txt"]);
    let failing_signer = harness.root.join("failing-gpg");
    write_executable(&failing_signer, "#!/bin/sh\nprintf 'intentional signing failure\\n' >&2\nexit 1\n");
    harness.git(["config", "commit.gpgsign", "true"]);
    harness.git(["config", "gpg.program", failing_signer.to_str().unwrap()]);
    let failed = harness.command(["commit", &transaction, "-m", "test: signed"]);
    assert_eq!(exit_code(&failed), 1);
    assert!(stderr(&failed).contains("failed to sign"));
    let bypassed = harness.success(["commit", &transaction, "-m", "test: unsigned", "--no-gpg-sign"]);
    assert!(stdout(&bypassed).contains("COMMITTED"));
}

fn prepared_id(output: &str) -> String {
    output.lines().find_map(|line| line.strip_prefix("PREPARED\t")).expect("PREPARED record").to_owned()
}

fn assert_format(output: &std::process::Output, expected: &str) {
    assert!(stdout(output).contains(&format!("FORMAT\t{expected}\n")), "{}", stdout(output));
}
