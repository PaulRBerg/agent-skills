mod common;

use std::fs;

use common::{Harness, stderr, stdout};

#[test]
fn creates_single_repository_handoff_and_verifies_clipboard() {
    let harness = Harness::new("single-create");
    let repository = harness.repo("alpha", true);
    let draft = harness.root.join("draft.md");
    fs::write(&draft, "# Implement the feature\n\n## Outcome\n\nShip it.\n\n").unwrap();

    let output = harness.command([
        "create",
        "--repo",
        repository.to_str().unwrap(),
        "--category",
        "implementation",
        "--task",
        "add safe handoffs",
        "--draft",
        draft.to_str().unwrap(),
        "CREATE_HANDOFF.md",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));

    let target = repository.join(".ai/task-handoffs/CREATE_HANDOFF.md");
    let published = fs::read_to_string(&target).unwrap();
    let category = published.find("category:").unwrap();
    let created = published.find("created:").unwrap();
    let launch = published.find("launch_repo:").unwrap();
    let repos = published.find("repos:\n").unwrap();
    let origin = published.find("origin:").unwrap();
    let task = published.find("task:").unwrap();
    assert!(category < created && created < launch && launch < repos && repos < origin && origin < task);
    assert!(published.contains("## Handoff category\n\nCategory: `implementation`"));
    assert!(published.contains("## Execution status\n\nCurrent status: No task attempt has been recorded."));
    assert!(published.contains(&format!(
        "## Handoff cleanup\n\nArchive this handoff only after the requested work is complete and task-scoped validation passes:\n\n```sh\nai-handoff archive '{}'\n```",
        target.display()
    )));

    let report = stdout(&output);
    assert!(report.contains(&format!("handoff\t{}\n", target.display())));
    assert!(report.contains(&format!("launch_repo\t{}\n", repository.display())));
    assert!(report.contains("category\timplementation\n"));
    let command = report
        .strip_prefix(&format!(
            "handoff\t{}\nlaunch_repo\t{}\ncategory\timplementation\ncommand\t",
            target.display(),
            repository.display()
        ))
        .unwrap()
        .trim_end();
    assert_eq!(fs::read_to_string(&harness.clipboard).unwrap(), command);
    assert!(command.starts_with(&format!("codex -C '{}' ", repository.display())));
    assert!(command.contains("under .ai/task-handoffs/CREATE_HANDOFF.md"));
}

#[test]
fn create_abbreviates_home_paths_throughout_the_handoff_file() {
    let harness = Harness::new("home-paths");
    fs::create_dir(harness.home.join("projects")).unwrap();
    let repository = harness.repo("home/projects/repo", true);
    let draft = harness.root.join("draft.md");
    fs::write(
        &draft,
        format!(
            "# Home paths\n\nRepository: `{}`\n\nTarget: `{}/.ai/task-handoffs/HOME_PATHS.md`\n",
            repository.display(),
            repository.display()
        ),
    )
    .unwrap();

    let output = harness.command([
        "create",
        "--repo",
        repository.to_str().unwrap(),
        "--category",
        "implementation",
        "--task",
        "abbreviate home paths",
        "--draft",
        draft.to_str().unwrap(),
        "--no-clipboard",
        "HOME_PATHS.md",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));

    let target = repository.join(".ai/task-handoffs/HOME_PATHS.md");
    let contents = fs::read_to_string(target).unwrap();
    assert!(!contents.contains(harness.home.to_str().unwrap()));
    assert!(contents.contains("launch_repo: '~/projects/repo'"));
    assert!(contents.contains("repos:\n  - '~/projects/repo'"));
    assert!(contents.contains("origin: '~/projects/repo/.ai/task-handoffs/HOME_PATHS.md'"));
    assert!(contents.contains("Repository: `~/projects/repo`"));
    assert!(contents.contains("Target: `~/projects/repo/.ai/task-handoffs/HOME_PATHS.md`"));
    assert!(contents.contains("ai-handoff archive ~/'projects/repo/.ai/task-handoffs/HOME_PATHS.md'"));
    assert!(!contents.contains("archive '~"));
}

#[test]
fn create_rejects_existing_and_non_ignored_targets() {
    let harness = Harness::new("create-rejections");
    let repository = harness.repo("ignored", true);
    let draft = harness.root.join("draft.md");
    fs::write(&draft, "# Existing target\n").unwrap();
    fs::create_dir_all(repository.join(".ai/task-handoffs")).unwrap();
    fs::write(repository.join(".ai/task-handoffs/EXISTING.md"), "keep\n").unwrap();
    let output = harness.command([
        "create",
        "--repo",
        repository.to_str().unwrap(),
        "--category",
        "audit",
        "--task",
        "check existing",
        "--draft",
        draft.to_str().unwrap(),
        "--no-clipboard",
        "EXISTING.md",
    ]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("target already exists"));
    assert_eq!(fs::read_to_string(repository.join(".ai/task-handoffs/EXISTING.md")).unwrap(), "keep\n");

    let unignored = harness.repo("unignored", false);
    let output = harness.command([
        "create",
        "--check",
        "--repo",
        unignored.to_str().unwrap(),
        "--category",
        "audit",
        "--task",
        "check ignore",
        "NOT_IGNORED.md",
    ]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("not ignored by Git"));
}

#[test]
fn cross_repository_create_uses_desktop_and_requires_repository_order() {
    let harness = Harness::new("cross-create");
    let first = harness.repo("first", false);
    let second = harness.repo("second", false);
    let draft = harness.root.join("draft.md");
    fs::write(&draft, "# Cross repository work\n").unwrap();
    let arguments = [
        "create",
        "--repo",
        first.to_str().unwrap(),
        "--repo",
        second.to_str().unwrap(),
        "--launch-repo",
        second.to_str().unwrap(),
        "--category",
        "investigation",
        "--task",
        "trace both repositories",
        "--draft",
        draft.to_str().unwrap(),
        "--no-clipboard",
        "CROSS_REPO.md",
    ];
    let output = harness.command(arguments);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("missing a Repository order section"));

    fs::write(&draft, "# Cross repository work\n\n## Repository order\n\n1. First\n2. Second\n").unwrap();
    let output = harness.command(arguments);
    assert!(output.status.success(), "{}", stderr(&output));
    let target = harness.desktop.join(".ai/task-handoffs/CROSS_REPO.md");
    let published = fs::read_to_string(&target).unwrap();
    assert!(published.contains(&format!("launch_repo: '{}'", second.display())));
    assert!(published.contains(&format!("repos:\n  - '{}'\n  - '{}'", first.display(), second.display())));
    assert!(stdout(&output).contains(&format!("handoff\t{}", target.display())));
}

#[test]
fn check_validates_without_writing() {
    let harness = Harness::new("check");
    let repository = harness.repo("repo", true);
    let output = harness.command([
        "create",
        "--check",
        "--repo",
        repository.to_str().unwrap(),
        "--category",
        "research",
        "--task",
        "validate only",
        "CHECK_ONLY.md",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(!repository.join(".ai").exists());
    let report = stdout(&output);
    assert_eq!(report.lines().count(), 3);
    assert!(report.contains(&format!("target\t{}/.ai/task-handoffs/CHECK_ONLY.md", repository.display())));
}

#[test]
fn archive_moves_handoff_under_home_archive() {
    let harness = Harness::new("archive");
    let source = harness.root.join("origin/.ai/task-handoffs/ARCHIVE_ME.md");
    fs::create_dir_all(source.parent().unwrap()).unwrap();
    fs::write(&source, "# Finished\n").unwrap();

    let output = harness.command(["archive", source.to_str().unwrap()]);
    assert!(output.status.success(), "{}", stderr(&output));
    let destination = harness.home.join(".local/share/task-handoffs/archive/origin/ARCHIVE_ME.md");
    assert!(!source.exists());
    assert_eq!(fs::read_to_string(&destination).unwrap(), "# Finished\n");
    assert_eq!(stdout(&output), format!("ARCHIVED\t{}\n", destination.display()));
}

#[test]
fn create_rejects_invalid_drafts_filenames_categories_and_tasks_without_publication() {
    let harness = Harness::new("create-input-rejections");
    let repository = harness.repo("repo", true);
    let draft = harness.root.join("draft.md");

    for (name, contents, expected) in [
        ("EMPTY_DRAFT.md", " \n\t", "handoff draft is empty"),
        ("FRONTMATTER.md", "---\n# Body\n", "must not start with YAML frontmatter"),
        ("NO_H1.md", "Body\n", "must start with an H1 heading"),
        ("CATEGORY.md", "# Body\n\n## Handoff category\n", "contains reserved heading"),
        ("STATUS.md", "# Body\n\n## Execution status\n", "contains reserved heading"),
        ("CLEANUP.md", "# Body\n\n## Handoff cleanup\n", "contains reserved heading"),
    ] {
        fs::write(&draft, contents).unwrap();
        let output = harness.command([
            "create",
            "--repo",
            repository.to_str().unwrap(),
            "--category",
            "audit",
            "--task",
            "validate input",
            "--draft",
            draft.to_str().unwrap(),
            "--no-clipboard",
            name,
        ]);
        assert!(!output.status.success(), "{name} unexpectedly succeeded");
        assert!(stderr(&output).contains(expected), "{}", stderr(&output));
        assert!(!repository.join(".ai/task-handoffs").join(name).exists());
    }

    fs::write(&draft, "# Valid body\n").unwrap();
    for filename in ["lowercase.md", "1LEADING.md", "_LEADING.md", "DOUBLE__UNDERSCORE.md", "NO_EXTENSION"] {
        let output = harness.command([
            "create",
            "--repo",
            repository.to_str().unwrap(),
            "--category",
            "audit",
            "--task",
            "validate input",
            "--draft",
            draft.to_str().unwrap(),
            "--no-clipboard",
            filename,
        ]);
        assert!(!output.status.success(), "{filename} unexpectedly succeeded");
        assert!(stderr(&output).contains("invalid handoff filename"));
    }

    let unknown_category = harness.command([
        "create",
        "--check",
        "--repo",
        repository.to_str().unwrap(),
        "--category",
        "unknown",
        "--task",
        "validate input",
        "VALID.md",
    ]);
    assert!(!unknown_category.status.success());
    assert!(stderr(&unknown_category).contains("invalid value 'unknown'"));
    for task in ["", "first line\nsecond line"] {
        let output = harness.command([
            "create",
            "--check",
            "--repo",
            repository.to_str().unwrap(),
            "--category",
            "audit",
            "--task",
            task,
            "VALID.md",
        ]);
        assert!(!output.status.success());
        assert!(stderr(&output).contains("task must"));
    }
}

#[test]
fn create_enforces_repository_topology_and_cross_repository_placement() {
    let harness = Harness::new("create-topology");
    let first = harness.repo("first", true);
    let second = harness.repo("second", true);
    let draft = harness.root.join("draft.md");
    fs::write(&draft, "# Cross repository work\n\n## Repository order\n\n1. First\n2. Second\n").unwrap();

    let non_worktree = harness.root.join("not-a-worktree");
    fs::create_dir(&non_worktree).unwrap();
    let output = harness.command([
        "create",
        "--check",
        "--repo",
        non_worktree.to_str().unwrap(),
        "--category",
        "audit",
        "--task",
        "not a worktree",
        "INVALID.md",
    ]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("not a Git worktree"));

    let missing_launch = harness.command([
        "create",
        "--check",
        "--repo",
        first.to_str().unwrap(),
        "--repo",
        second.to_str().unwrap(),
        "--category",
        "audit",
        "--task",
        "missing launch",
        "MISSING_LAUNCH.md",
    ]);
    assert!(!missing_launch.status.success());
    assert!(stderr(&missing_launch).contains("--launch-repo is required"));

    let outside_launch = harness.repo("outside", true);
    let output = harness.command([
        "create",
        "--check",
        "--repo",
        first.to_str().unwrap(),
        "--repo",
        second.to_str().unwrap(),
        "--launch-repo",
        outside_launch.to_str().unwrap(),
        "--category",
        "audit",
        "--task",
        "outside launch",
        "OUTSIDE_LAUNCH.md",
    ]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("not an involved repository"));

    let alias = harness.root.join("first-alias");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&first, &alias).unwrap();
    let duplicate = harness.command([
        "create",
        "--repo",
        first.to_str().unwrap(),
        "--repo",
        alias.to_str().unwrap(),
        "--category",
        "audit",
        "--task",
        "deduplicate repos",
        "--draft",
        draft.to_str().unwrap(),
        "--no-clipboard",
        "DEDUPED.md",
    ]);
    assert!(duplicate.status.success(), "{}", stderr(&duplicate));
    assert!(first.join(".ai/task-handoffs/DEDUPED.md").exists());
    assert!(!harness.desktop.join(".ai/task-handoffs/DEDUPED.md").exists());

    let cross = harness.command([
        "create",
        "--repo",
        first.to_str().unwrap(),
        "--repo",
        second.to_str().unwrap(),
        "--launch-repo",
        second.to_str().unwrap(),
        "--category",
        "implementation",
        "--task",
        "coordinate both repositories",
        "--draft",
        draft.to_str().unwrap(),
        "--no-clipboard",
        "CROSS_PLACEMENT.md",
    ]);
    assert!(cross.status.success(), "{}", stderr(&cross));
    let target = harness.desktop.join(".ai/task-handoffs/CROSS_PLACEMENT.md");
    let contents = fs::read_to_string(&target).unwrap();
    assert!(contents.contains(&format!("repos:\n  - '{}'\n  - '{}'", first.display(), second.display())));
    assert!(!first.join(".ai/task-handoffs/CROSS_PLACEMENT.md").exists());
    assert!(!second.join(".ai/task-handoffs/CROSS_PLACEMENT.md").exists());
}

#[test]
fn create_output_frontmatter_footer_and_clipboard_contracts_are_exact() {
    let harness = Harness::new("create-contract");
    let repository = harness.repo("repo", true);
    let draft = harness.root.join("draft.md");
    fs::write(&draft, "# Contract body\n\n## Outcome\n\nShip it.\n").unwrap();
    let task = "fix Bob's parser";
    let output = harness.command([
        "create",
        "--repo",
        repository.to_str().unwrap(),
        "--category",
        "investigation",
        "--task",
        task,
        "--draft",
        draft.to_str().unwrap(),
        "CONTRACT.md",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    let target = repository.join(".ai/task-handoffs/CONTRACT.md");
    let prompt = "A previous agent prepared a investigation task handoff for fix Bob's parser under .ai/task-handoffs/CONTRACT.md. Read the handoff, then complete its requested investigation task. Follow its stated outcome, boundaries, authority constraints, and validation requirements.";
    let command = format!("codex -C '{}' '{}'", repository.display(), prompt.replace('\'', "'\\''"));
    assert_eq!(
        stdout(&output),
        format!(
            "handoff\t{}\nlaunch_repo\t{}\ncategory\tinvestigation\ncommand\t{command}\n",
            target.display(),
            repository.display()
        )
    );
    assert_eq!(fs::read_to_string(&harness.clipboard).unwrap(), command);

    let contents = fs::read_to_string(&target).unwrap();
    let lines = contents.lines().collect::<Vec<_>>();
    assert_eq!(lines[0], "---");
    assert_eq!(lines[1], "category: 'investigation'");
    assert!(lines[2].starts_with("created: '") && lines[2].ends_with("Z'"));
    assert_eq!(lines[3], format!("launch_repo: '{}'", repository.display()));
    assert_eq!(lines[4], "repos:");
    assert_eq!(lines[5], format!("  - '{}'", repository.display()));
    assert_eq!(lines[6], format!("origin: '{}'", target.display()));
    assert_eq!(lines[7], "task: 'fix Bob''s parser'");
    assert_eq!(lines[8], "---");
    for heading in ["## Handoff category", "## Execution status", "## Handoff cleanup"] {
        assert_eq!(contents.matches(heading).count(), 1, "{heading}");
    }
    let cleanup = contents.split("```sh\n").nth(1).unwrap().split("\n```").next().unwrap();
    assert_eq!(cleanup, format!("ai-handoff archive '{}'", target.display()));
}

#[test]
fn create_rolls_back_on_clipboard_mismatch_and_no_clipboard_needs_no_tools() {
    let harness = Harness::new("clipboard-rollback");
    let repository = harness.repo("repo", true);
    let draft = harness.root.join("draft.md");
    fs::write(&draft, "# Clipboard\n").unwrap();
    common::write_executable(&harness.root.join("shim/pbpaste"), "#!/bin/sh\nprintf wrong\n");
    let output = harness.command([
        "create",
        "--repo",
        repository.to_str().unwrap(),
        "--category",
        "audit",
        "--task",
        "mismatch clipboard",
        "--draft",
        draft.to_str().unwrap(),
        "MISMATCH.md",
    ]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("clipboard verification failed"));
    let handoffs = repository.join(".ai/task-handoffs");
    assert!(!repository.join(".ai").exists());
    assert!(!handoffs.join("MISMATCH.md").exists());
    assert!(
        fs::read_dir(&repository).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".ai-handoff."))
    );

    let output = harness.command_without_clipboard([
        "create",
        "--repo",
        repository.to_str().unwrap(),
        "--category",
        "audit",
        "--task",
        "skip clipboard",
        "--draft",
        draft.to_str().unwrap(),
        "NO_CLIPBOARD.md",
        "--no-clipboard",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(repository.join(".ai/task-handoffs/NO_CLIPBOARD.md").exists());
}

#[test]
fn finding_create_and_check_are_deterministic_and_do_not_write() {
    let harness = Harness::new("finding-and-check");
    let repository = harness.repo("repo", true);
    let check = harness.command([
        "create",
        "--check",
        "--repo",
        repository.to_str().unwrap(),
        "--category",
        "audit",
        "--task",
        "check only",
        "CHECK.md",
    ]);
    assert!(check.status.success(), "{}", stderr(&check));
    assert_eq!(
        stdout(&check),
        format!(
            "target\t{}/.ai/task-handoffs/CHECK.md\nlaunch_repo\t{}\ncategory\taudit\n",
            repository.display(),
            repository.display()
        )
    );
    assert!(!repository.join(".ai").exists());

    let draft = harness.root.join("finding.md");
    fs::write(&draft, "# Finding\n\nSource finding: deadbeef\n").unwrap();
    let output = harness.command([
        "create",
        "--repo",
        repository.to_str().unwrap(),
        "--category",
        "audit",
        "--task",
        "triage finding deadbeef",
        "--draft",
        draft.to_str().unwrap(),
        "--no-clipboard",
        "FINDING_DEADBEEF.md",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        fs::read_to_string(repository.join(".ai/task-handoffs/FINDING_DEADBEEF.md"))
            .unwrap()
            .contains("Source finding: deadbeef")
    );
}

#[test]
fn archive_validates_location_and_uses_desktop_origin_and_collision_suffix() {
    let harness = Harness::new("archive-boundaries");
    let source = harness.home.join("Desktop/.ai/task-handoffs/ARCHIVE_ME.md");
    fs::create_dir_all(source.parent().unwrap()).unwrap();
    fs::write(&source, "# Finished\n").unwrap();
    let archive_directory = harness.home.join(".local/share/task-handoffs/archive/Desktop");
    fs::create_dir_all(&archive_directory).unwrap();
    fs::write(archive_directory.join("ARCHIVE_ME.md"), "prior\n").unwrap();
    let output = harness.command(["archive", source.to_str().unwrap()]);
    assert!(output.status.success(), "{}", stderr(&output));
    let destination = stdout(&output).trim_end().strip_prefix("ARCHIVED\t").unwrap().to_owned();
    assert!(destination.starts_with(&format!("{}/ARCHIVE_ME_", archive_directory.display())));
    assert!(destination.ends_with(".md"));
    assert_eq!(fs::read_to_string(destination).unwrap(), "# Finished\n");

    let nested = harness.root.join("repo/.ai/task-handoffs/nested/NOT_DIRECT.md");
    fs::create_dir_all(nested.parent().unwrap()).unwrap();
    fs::write(&nested, "# No\n").unwrap();
    let invalid = harness.command(["archive", nested.to_str().unwrap()]);
    assert!(!invalid.status.success());
    assert!(stderr(&invalid).contains("not directly inside .ai/task-handoffs"));
    let missing = harness.command(["archive", harness.root.join("missing.md").to_str().unwrap()]);
    assert!(!missing.status.success());
    assert!(stderr(&missing).contains("cannot inspect handoff"));
}
