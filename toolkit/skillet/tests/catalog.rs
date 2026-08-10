mod common;

use std::fs;
use std::path::Path;

use ai_skillet::catalog::Catalog;
use ai_skillet::dependency::{DependencyIdentifier, SkillName};
use ai_skillet::frontmatter::{InstallTargets, parse_skill_file};
use ai_skillet::hash::{sha256_file, sha256_tree};
use ai_skillet::traversal::{ExposureScope, RootRequest};
use tempfile::TempDir;

#[test]
fn parses_only_leading_multiline_frontmatter_with_field_locations() {
    let temporary = TempDir::new().unwrap();
    let skill = temporary.path().join("SKILL.md");
    common::write(
        &skill,
        b"---\nname: alpha\ncompatibility: |\n  macOS\n  and Linux\nmetadata:\n  install-targets: claude-code codex\nskill-dependencies:\n  - beta\ndescription: >\n  Alpha does useful work.\n  Across clients.\n---\nname: this body is not YAML frontmatter\n",
    );
    common::write(temporary.path().join("skills/beta/SKILL.md"), common::skill("beta", ""));

    let catalog = Catalog::load(&[RootRequest::explicit(temporary.path())]).unwrap();
    assert!(catalog.diagnostics.is_empty(), "{:?}", catalog.diagnostics);
    let alpha = catalog
        .skills
        .iter()
        .find(|skill| skill.name.as_ref().map(SkillName::as_str) == Some("alpha"))
        .unwrap();
    let frontmatter = alpha.frontmatter.as_ref().unwrap();
    assert_eq!(
        frontmatter
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field.line))
            .collect::<Vec<_>>(),
        [
            ("name", 2),
            ("compatibility", 3),
            ("metadata", 6),
            ("skill-dependencies", 8),
            ("description", 10),
        ]
    );
    assert_eq!(frontmatter.name.as_ref().unwrap().value, "alpha");
    assert_eq!(
        frontmatter.install_targets.as_ref().unwrap().value,
        Some(InstallTargets::ClaudeCodeAndCodex)
    );
    assert_eq!(alpha.dependencies[0].line, 9);
    assert_eq!(alpha.dependencies[0].identifier.to_string(), "beta");
}

#[test]
fn malformed_non_mapping_and_missing_frontmatter_are_diagnostics() {
    let temporary = TempDir::new().unwrap();
    common::write(temporary.path().join("skills/a/SKILL.md"), "---\nname: [unterminated\n---\n");
    common::write(temporary.path().join("skills/b/SKILL.md"), "---\n- list\n---\n");
    common::write(temporary.path().join("skills/c/SKILL.md"), "name: c\n");
    common::write(temporary.path().join("skills/d/SKILL.md"), "---\nname: d\n");

    let catalog = Catalog::load(&[RootRequest::explicit(temporary.path())]).unwrap();
    assert_eq!(
        catalog.diagnostics.iter().map(|diagnostic| diagnostic.code.as_str()).collect::<Vec<_>>(),
        [
            "FRONTMATTER_INVALID_YAML",
            "FRONTMATTER_NOT_MAPPING",
            "FRONTMATTER_DELIMITER_MISSING",
            "FRONTMATTER_DELIMITER_MISSING",
        ]
    );
    assert!(catalog.diagnostics.iter().all(|diagnostic| diagnostic.line.is_some()));
}

#[test]
fn dependency_validation_covers_shape_uniqueness_self_resolution_and_order() {
    let first = TempDir::new().unwrap();
    let second = TempDir::new().unwrap();
    common::write(
        first.path().join("skills/alpha/SKILL.md"),
        common::skill(
            "alpha",
            "skill-dependencies:\n  - gamma\n  - Acme/Tools#beta\n  - beta\n  - beta\n  - alpha\n  - missing\n  - Bad//Repo#thing\n  - 42\n",
        ),
    );
    common::write(second.path().join("skills/beta/SKILL.md"), common::skill("beta", ""));
    common::write(second.path().join("skills/gamma/SKILL.md"), common::skill("gamma", ""));

    let catalog =
        Catalog::load(&[RootRequest::explicit(first.path()), RootRequest::explicit(second.path())])
            .unwrap();
    let codes: Vec<_> =
        catalog.diagnostics.iter().map(|diagnostic| diagnostic.code.as_str()).collect();
    for expected in [
        "SKILL_DEPENDENCY_DUPLICATE",
        "SKILL_DEPENDENCY_INVALID",
        "SKILL_DEPENDENCY_NOT_STRING",
        "SKILL_DEPENDENCY_SELF",
        "SKILL_DEPENDENCY_UNRESOLVED",
        "SKILL_DEPENDENCIES_ORDER",
    ] {
        assert!(codes.contains(&expected), "missing {expected}: {codes:?}");
    }
    assert_eq!(
        DependencyIdentifier::parse("Acme/Tools#beta").unwrap().target_name().as_str(),
        "beta"
    );
    for invalid in [
        "",
        "two--hyphens",
        "owner/repo",
        "owner/repo#",
        "/repo#skill",
        "owner//repo#skill",
        "owner/repo#Bad",
    ] {
        assert!(DependencyIdentifier::parse(invalid).is_err(), "{invalid}");
    }
}

#[test]
fn valid_dependencies_use_target_name_then_complete_identifier_order() {
    let temporary = TempDir::new().unwrap();
    common::write(
        temporary.path().join("skills/alpha/SKILL.md"),
        common::skill("alpha", "skill-dependencies:\n  - Acme/Tools#beta\n  - beta\n  - gamma\n"),
    );
    common::write(temporary.path().join("skills/beta/SKILL.md"), common::skill("beta", ""));
    common::write(temporary.path().join("skills/gamma/SKILL.md"), common::skill("gamma", ""));
    let catalog = Catalog::load(&[RootRequest::explicit(temporary.path())]).unwrap();
    assert!(catalog.diagnostics.is_empty(), "{:?}", catalog.diagnostics);
}

#[test]
fn dependency_order_ignores_hyphens_in_target_skill_names() {
    let temporary = TempDir::new().unwrap();
    common::write(
        temporary.path().join("skills/alpha/SKILL.md"),
        common::skill(
            "alpha",
            "skill-dependencies:\n  - PaulRBerg/dot-agents#codebase-design\n  - code-polish\n  - commit\n",
        ),
    );
    common::write(
        temporary.path().join("skills/code-polish/SKILL.md"),
        common::skill("code-polish", ""),
    );
    common::write(temporary.path().join("skills/commit/SKILL.md"), common::skill("commit", ""));

    let catalog = Catalog::load(&[RootRequest::explicit(temporary.path())]).unwrap();
    assert!(catalog.diagnostics.is_empty(), "{:?}", catalog.diagnostics);
}

#[test]
fn external_dependency_repository_policy_preserves_case_and_allows_trailing_punctuation() {
    for valid in [
        "Org./Repository-#target-skill",
        "Org_/Repository.#target-skill",
        "Org-/Repository_#target-skill",
    ] {
        let parsed = DependencyIdentifier::parse(valid).unwrap_or_else(|_| panic!("{valid}"));
        assert_eq!(parsed.to_string(), valid);
    }
    for invalid in [
        "_Org/Repository#target-skill",
        "Org/_Repository#target-skill",
        "Org/Repository.git#target-skill",
    ] {
        assert!(DependencyIdentifier::parse(invalid).is_err(), "{invalid}");
    }
}

#[test]
fn dependency_field_must_be_a_non_empty_array() {
    let temporary = TempDir::new().unwrap();
    common::write(
        temporary.path().join("skills/empty/SKILL.md"),
        common::skill("empty", "skill-dependencies: []\n"),
    );
    common::write(
        temporary.path().join("skills/scalar/SKILL.md"),
        common::skill("scalar", "skill-dependencies: beta\n"),
    );
    let catalog = Catalog::load(&[RootRequest::explicit(temporary.path())]).unwrap();
    assert_eq!(
        catalog.diagnostics.iter().map(|diagnostic| diagnostic.code.as_str()).collect::<Vec<_>>(),
        ["SKILL_DEPENDENCIES_EMPTY", "SKILL_DEPENDENCIES_NOT_ARRAY"]
    );
}

#[test]
fn discovers_direct_catalog_client_and_recognized_symlink_exposures() {
    let temporary = TempDir::new().unwrap();
    common::write(temporary.path().join("SKILL.md"), common::skill("direct", ""));
    for (relative, name) in [
        ("skills/catalog", "catalog"),
        (".agents/skills/agents", "agents"),
        (".claude/skills/claude", "claude"),
        (".codex/skills/codex", "codex"),
    ] {
        common::write(temporary.path().join(relative).join("SKILL.md"), common::skill(name, ""));
    }
    let target = TempDir::new().unwrap();
    common::write(target.path().join("SKILL.md"), common::skill("shared", ""));
    #[cfg(unix)]
    std::os::unix::fs::symlink(target.path(), temporary.path().join(".claude/skills/shared"))
        .unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(target.path(), temporary.path().join("arbitrary-link")).unwrap();

    let catalog = Catalog::load(&[RootRequest::explicit(temporary.path())]).unwrap();
    let scopes: Vec<_> = catalog.skills.iter().map(|skill| skill.exposure.scope).collect();
    assert!(scopes.contains(&ExposureScope::Direct));
    assert!(scopes.contains(&ExposureScope::Catalog));
    assert!(scopes.contains(&ExposureScope::Agents));
    assert!(scopes.contains(&ExposureScope::Claude));
    assert!(scopes.contains(&ExposureScope::Codex));
    let catalog_root =
        Catalog::load(&[RootRequest::explicit(temporary.path().join("skills"))]).unwrap();
    assert_eq!(catalog_root.skill_names().len(), 1);
    let client_root =
        Catalog::load(&[RootRequest::explicit(temporary.path().join(".agents/skills"))]).unwrap();
    assert_eq!(client_root.skill_names().len(), 1);
    #[cfg(unix)]
    {
        assert_eq!(catalog.skills.len(), 6);
        let shared = catalog
            .skills
            .iter()
            .find(|skill| skill.name.as_ref().map(SkillName::as_str) == Some("shared"))
            .unwrap();
        assert_ne!(shared.skill_path(), shared.resolved_skill_path());
        assert!(shared.exposure.directory_symlink_target.is_some());
    }
}

#[test]
fn ignored_catalog_entry_is_found_when_requested_directly() {
    let temporary = TempDir::new().unwrap();
    fs::create_dir(temporary.path().join(".git")).unwrap();
    common::write(temporary.path().join(".gitignore"), "skills/ignored/\n");
    common::write(temporary.path().join("skills/visible/SKILL.md"), common::skill("visible", ""));
    let ignored = temporary.path().join("skills/ignored");
    common::write(ignored.join("SKILL.md"), common::skill("ignored", ""));

    let parent = Catalog::load(&[RootRequest::explicit(temporary.path())]).unwrap();
    assert_eq!(parent.skill_names().len(), 1);
    assert!(parent.skill_names().iter().any(|name| name.as_str() == "visible"));

    let direct = Catalog::load(&[RootRequest::explicit(&ignored)]).unwrap();
    assert!(direct.skill_names().iter().any(|name| name.as_str() == "ignored"));
}

#[test]
fn broad_scan_streams_unrecognized_roots_but_prunes_dependency_trees() {
    let temporary = TempDir::new().unwrap();
    common::write(temporary.path().join("workspace/tool/SKILL.md"), common::skill("tool", ""));
    let dependency = temporary.path().join("node_modules/dependency");
    common::write(dependency.join("SKILL.md"), common::skill("dependency", ""));

    let broad = Catalog::load(&[RootRequest::broad(temporary.path())]).unwrap();
    assert!(broad.skill_names().iter().any(|name| name.as_str() == "tool"));
    assert!(!broad.skill_names().iter().any(|name| name.as_str() == "dependency"));

    let direct = Catalog::load(&[RootRequest::explicit(&dependency)]).unwrap();
    assert!(direct.skill_names().iter().any(|name| name.as_str() == "dependency"));
}

#[test]
fn paths_with_spaces_and_newlines_are_not_line_split() {
    let temporary = TempDir::new().unwrap();
    let directory = temporary.path().join("skills/space and\nnewline");
    common::write(directory.join("SKILL.md"), common::skill("odd-path", ""));
    let catalog = Catalog::load(&[RootRequest::explicit(temporary.path())]).unwrap();
    assert_eq!(catalog.skills.len(), 1);
    assert_eq!(catalog.skills[0].skill_path(), directory.join("SKILL.md"));
}

#[test]
fn large_body_after_frontmatter_is_not_buffered_as_frontmatter() {
    let temporary = TempDir::new().unwrap();
    let skill = temporary.path().join("SKILL.md");
    let file = fs::File::create(&skill).unwrap();
    let mut writer = std::io::BufWriter::new(file);
    use std::io::Write;
    writer.write_all(b"---\nname: large\ndescription: large\n---\n").unwrap();
    for _ in 0..10 {
        writer.write_all(&vec![b'x'; 1024 * 1024]).unwrap();
    }
    writer.flush().unwrap();

    let parsed = parse_skill_file(&skill);
    assert!(parsed.diagnostics.is_empty());
    assert_eq!(parsed.frontmatter.unwrap().name.unwrap().value, "large");
}

#[test]
fn file_and_tree_hashes_are_deterministic_streamed_and_metadata_sensitive() {
    let first = TempDir::new().unwrap();
    let second = TempDir::new().unwrap();
    create_hash_tree(first.path(), false);
    create_hash_tree(second.path(), true);
    assert_eq!(sha256_tree(first.path()).unwrap(), sha256_tree(first.path()).unwrap());
    assert_eq!(sha256_tree(first.path()).unwrap(), sha256_tree(second.path()).unwrap());
    assert_eq!(
        sha256_file(&first.path().join("a file\nwith newline")).unwrap(),
        sha256_file(&second.path().join("a file\nwith newline")).unwrap()
    );

    let before_ignored = sha256_tree(first.path()).unwrap();
    common::write(first.path().join(".git/ignored"), "not hashed");
    assert_eq!(before_ignored, sha256_tree(first.path()).unwrap());

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let executable = first.path().join("script");
        let before_mode = sha256_tree(first.path()).unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
        assert_ne!(before_mode, sha256_tree(first.path()).unwrap());
    }
}

#[cfg(unix)]
#[test]
fn tree_hash_records_symlink_target_without_following_it() {
    let tree = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    let first_target = outside.path().join("first");
    let second_target = outside.path().join("second");
    common::write(&first_target, "first contents");
    common::write(&second_target, "second contents");
    let link = tree.path().join("link");
    std::os::unix::fs::symlink(&first_target, &link).unwrap();

    let before = sha256_tree(tree.path()).unwrap();
    common::write(&first_target, "changed outside contents");
    assert_eq!(before, sha256_tree(tree.path()).unwrap());

    fs::remove_file(&link).unwrap();
    std::os::unix::fs::symlink(&second_target, &link).unwrap();
    assert_ne!(before, sha256_tree(tree.path()).unwrap());
}

fn create_hash_tree(root: &Path, reverse: bool) {
    let entries = [
        ("a file\nwith newline", vec![b'a'; 2 * 1024 * 1024]),
        ("nested/value", b"value".to_vec()),
        ("script", b"#!/bin/sh\n".to_vec()),
    ];
    if reverse {
        for (path, contents) in entries.iter().rev() {
            common::write(root.join(path), contents);
        }
    } else {
        for (path, contents) in entries {
            common::write(root.join(path), contents);
        }
    }
}
