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
fn config_uses_exact_canonical_root_and_explicit_format_wins() {
    let harness = Harness::new("config");
    harness.write("intended.txt", "base\n");
    harness.commit_all("base");
    harness.write("intended.txt", "changed\n");
    let config_dir = harness.root.join("config/ai-commit");
    fs::create_dir_all(&config_dir).unwrap();
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&harness.repo, harness.home.join("repo-link")).unwrap();
        fs::write(config_dir.join("config.toml"), "[message]\nnatural_repositories = [\"~/repo-link\"]\n").unwrap();
    }
    #[cfg(not(unix))]
    fs::write(
        config_dir.join("config.toml"),
        format!("[message]\nnatural_repositories = [\"{}\"]\n", harness.repo.display()),
    )
    .unwrap();

    let configured = harness.success(["prepare", "--porcelain", "--", "intended.txt"]);
    assert!(stdout(&configured).contains("FORMAT\tnatural\n"));
    let forced = harness.success(["prepare", "--porcelain", "--conventional", "--", "intended.txt"]);
    assert!(stdout(&forced).contains("FORMAT\tconventional\n"));
    let forced_natural = harness.success(["prepare", "--porcelain", "--natural", "--", "intended.txt"]);
    assert!(stdout(&forced_natural).contains("FORMAT\tnatural\n"));

    let override_path = harness.root.join("override.toml");
    fs::write(&override_path, format!("[message]\nnatural_repositories = [\"{}\"]\n", harness.repo.display())).unwrap();
    let overridden = harness.command_with_env(
        ["prepare", "--porcelain", "--", "intended.txt"],
        [("AI_COMMIT_CONFIG", override_path.to_string_lossy().into_owned())],
    );
    assert!(overridden.status.success(), "{}", stderr(&overridden));
    assert!(stdout(&overridden).contains("FORMAT\tnatural\n"));

    fs::write(config_dir.join("config.toml"), "[message\ninvalid = true\n").unwrap();
    let invalid = harness.command(["prepare", "--", "intended.txt"]);
    assert_eq!(exit_code(&invalid), 2);
    assert!(stderr(&invalid).contains("invalid config"));
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
