# /// script
# dependencies = ["PyYAML>=6.0.2"]
# ///

from __future__ import annotations

import importlib.util
import io
import json
import os
import re
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest import mock


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = REPO_ROOT / "skills" / "skill-map" / "scripts" / "skill-map.py"
sys.path.insert(0, str(SCRIPT_PATH.parent))
SPEC = importlib.util.spec_from_file_location("skill_map", SCRIPT_PATH)
assert SPEC is not None and SPEC.loader is not None
skill_map = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = skill_map
SPEC.loader.exec_module(skill_map)


def write_skill(
    directory: Path,
    name: str,
    install_targets: str | None = None,
    dependencies: list[object] | object | None = None,
) -> None:
    directory.mkdir(parents=True)
    metadata = f"metadata:\n  install-targets: {install_targets}\n" if install_targets else ""
    if isinstance(dependencies, list):
        dependency_field = "skill-dependencies:\n" + "".join(
            f"  - {json.dumps(value)}\n" for value in dependencies
        )
    elif dependencies is not None:
        dependency_field = f"skill-dependencies: {json.dumps(dependencies)}\n"
    else:
        dependency_field = ""
    (directory / "SKILL.md").write_text(
        f"---\n{metadata}name: {name}\n{dependency_field}description: Test skill.\n---\n\n# {name}\n",
        encoding="utf-8",
    )


class ParseRgMatchesTest(unittest.TestCase):
    def test_parses_chunked_null_delimited_matches(self) -> None:
        class ChunkedStream(io.BytesIO):
            def read(self, size: int = -1) -> bytes:
                return super().read(min(size, 5))

        stream = ChunkedStream(b"./one file\0" b"12:$skill-map\n" b"./line\nbreak\0" b"3: other-test skill\n")

        self.assertEqual(
            list(skill_map.parse_rg_matches(stream)),
            [
                ("./one file", 12, "$skill-map"),
                ("./line\nbreak", 3, " other-test skill"),
            ],
        )

    def test_rejects_truncated_output(self) -> None:
        with redirect_stderr(io.StringIO()):
            with self.assertRaisesRegex(SystemExit, "2"):
                list(skill_map.parse_rg_matches(io.BytesIO(b"./file\0" b"1:$skill-map")))


class SearchPatternTest(unittest.TestCase):
    def test_closing_search_terminates_its_ripgrep_child(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "references.txt").write_text("$skill-map\n" * 100_000, encoding="utf-8")
            pattern = skill_map.build_known_pattern(["skill-map"])
            assert pattern is not None
            matches = skill_map.search_pattern([root], pattern, include_catalog_sources=False)

            self.assertEqual(next(matches)["match"], "$skill-map")
            matches.close()

    def test_streams_only_the_match_from_a_long_line(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            reference = root / "reference.txt"
            reference.write_text("x" * 1_000_000 + " $skill-map suffix\n", encoding="utf-8")
            pattern = skill_map.build_known_pattern(["skill-map"])
            assert pattern is not None

            matches = list(skill_map.search_pattern([root], pattern, include_catalog_sources=False))

        self.assertEqual(len(matches), 1)
        self.assertEqual(matches[0]["path"], str(reference.resolve()))
        self.assertEqual(matches[0]["line_number"], 1)
        self.assertEqual(matches[0]["match"], "$skill-map")

    def test_extracts_names_with_one_combined_pattern(self) -> None:
        pattern = skill_map.build_known_pattern(["skill-map", "other-test"])
        assert pattern is not None

        self.assertEqual(
            skill_map.matched_names("$skill-map", re.compile(pattern)),
            ["skill-map"],
        )


class IgnorePolicyTest(unittest.TestCase):
    def test_broad_home_scan_ignores_dependency_cache_roots(self) -> None:
        home = Path.home().resolve()
        args = skill_map.rg_base_args(home, include_catalog_sources=False)

        self.assertIn("!.cache/**", args)
        self.assertIn("!.local/share/uv/**", args)
        self.assertIn("!go/pkg/mod/**", args)

    def test_explicit_cache_root_remains_scannable(self) -> None:
        cache = (Path.home() / ".cache").resolve()
        args = skill_map.rg_base_args(cache, include_catalog_sources=False)

        self.assertNotIn("!.cache/**", args)


class QueryPlanningTest(unittest.TestCase):
    def test_missing_filter_uses_only_a_targeted_unresolved_search(self) -> None:
        root = Path("/tmp/skill-map-test")
        with mock.patch.object(skill_map, "search_pattern", return_value=[]) as search:
            skill_map.collect_edges(
                [root],
                skills=[],
                selected={"missing-skill"},
                include_self=False,
                include_snippets=False,
                include_catalog_sources=False,
            )

        search.assert_called_once()
        self.assertIn("missing\\-skill", search.call_args.args[1])
        self.assertNotIn("[a-z0-9]+", search.call_args.args[1])


class DeclaredDependencyTest(unittest.TestCase):
    def test_declaration_precedes_inference_and_keeps_external_target(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "skills" / "source-skill"
            write_skill(
                source,
                "source-skill",
                dependencies=["OrgName/RepositoryName#target-skill"],
            )
            (source / "SKILL.md").write_text(
                (source / "SKILL.md").read_text(encoding="utf-8")
                + "\nUse the target-skill skill.\n",
                encoding="utf-8",
            )
            write_skill(root / "skills" / "target-skill", "target-skill")
            skills = skill_map.discover_skills([root], False)

            edges, unresolved = skill_map.collect_edges(
                [root], skills, set(), False, False, False
            )

        matching = [
            edge
            for edge in edges
            if edge.get("source") == "source-skill"
            and skill_map.dependency_skill_name(edge["target"]) == "target-skill"
        ]
        self.assertEqual(unresolved, [])
        self.assertEqual(len(matching), 1)
        self.assertEqual(matching[0]["target"], "OrgName/RepositoryName#target-skill")
        self.assertIs(matching[0]["declared"], True)

    def test_filters_declarations_by_source_or_target_skill_name(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_skill(
                root / "skills" / "source-skill",
                "source-skill",
                dependencies=["OrgName/RepositoryName#target-skill", "other-skill"],
            )
            write_skill(root / "skills" / "other-skill", "other-skill")
            skills = skill_map.discover_skills([root], False)

            by_target, _ = skill_map.collect_edges(
                [root], skills, {"target-skill"}, False, False, False
            )
            by_source, _ = skill_map.collect_edges(
                [root], skills, {"source-skill"}, False, False, False
            )

        declared_by_target = [edge for edge in by_target if edge.get("declared")]
        declared_by_source = [edge for edge in by_source if edge.get("declared")]
        self.assertEqual(
            [edge["target"] for edge in declared_by_target],
            ["OrgName/RepositoryName#target-skill"],
        )
        self.assertEqual(len(declared_by_source), 2)

    def test_rejects_malformed_declaration_fields_with_path_context(self) -> None:
        invalid_values: list[object] = ["target-skill", [], [1], ["Org/repo/extra#target-skill"]]
        for value in invalid_values:
            with self.subTest(value=value), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                write_skill(root / "source-skill", "source-skill", dependencies=value)
                stderr = io.StringIO()
                with redirect_stderr(stderr), self.assertRaisesRegex(SystemExit, "2"):
                    skill_map.discover_skills([root], False)
                self.assertIn("cannot parse", stderr.getvalue())
                self.assertIn("skill-dependencies", stderr.getvalue())

    def test_rejects_malformed_yaml_with_path_context(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            skill = root / "source-skill"
            write_skill(skill, "source-skill")
            (skill / "SKILL.md").write_text(
                "---\nname: source-skill\nskill-dependencies: [target-skill\n---\n",
                encoding="utf-8",
            )
            stderr = io.StringIO()
            with redirect_stderr(stderr), self.assertRaisesRegex(SystemExit, "2"):
                skill_map.discover_skills([root], False)
        self.assertIn(str(skill / "SKILL.md"), stderr.getvalue())
        self.assertIn("invalid YAML frontmatter", stderr.getvalue())

    def test_declarations_appear_in_text_json_and_dot_outputs(self) -> None:
        edge = {
            "type": "dependency",
            "source": "source-skill",
            "target": "OrgName/RepositoryName#target-skill",
            "path": "/tmp/source-skill/SKILL.md",
            "line": 5,
            "declared": True,
        }
        root = Path("/tmp")

        text_output = io.StringIO()
        with redirect_stdout(text_output):
            skill_map.as_text([root], [], [edge], [], [], False)
        self.assertIn("(declared; /tmp/source-skill/SKILL.md:5)", text_output.getvalue())

        json_output = io.StringIO()
        with redirect_stdout(json_output):
            skill_map.as_json([root], [], [edge], [], [], False)
        payload = json.loads(json_output.getvalue())
        self.assertIs(payload["edges"][0]["declared"], True)
        self.assertEqual(payload["counts"]["declared_dependencies"], 1)

        dot_output = io.StringIO()
        with redirect_stdout(dot_output):
            skill_map.as_dot([edge], [])
        self.assertIn(
            '"source-skill" -> "OrgName/RepositoryName#target-skill";',
            dot_output.getvalue(),
        )


class PortfolioResolutionTest(unittest.TestCase):
    def test_expands_git_root_and_records_missing_optional_user_root(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            home = base / "home"
            repository = base / "repository"
            nested = repository / "nested"
            nested.mkdir(parents=True)
            agents_root = home / ".agents" / "skills"
            agents_root.mkdir(parents=True)
            subprocess.run(
                ["git", "init", "-q", str(repository)],
                check=True,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )

            with mock.patch.object(skill_map.Path, "home", return_value=home):
                portfolio = skill_map.resolve_portfolio(str(nested))
                summary = skill_map.portfolio_as_dict(portfolio)

        self.assertEqual(portfolio.repository_root, repository.resolve())
        self.assertEqual(portfolio.scan_roots, [repository.resolve(), agents_root])
        self.assertEqual(
            [(root.client, root.present) for root in portfolio.user_roots],
            [("codex", True), ("claude-code", False)],
        )
        self.assertEqual(summary["repository_root"], str(repository.resolve()))
        self.assertEqual(summary["user_roots"]["present"], [{"path": str(agents_root), "client": "codex"}])
        self.assertEqual(
            summary["user_roots"]["missing"],
            [{"path": str(home / ".claude" / "skills"), "client": "claude-code"}],
        )

    def test_root_and_portfolio_root_are_mutually_exclusive(self) -> None:
        argv = ["skill-map.py", "--root", ".", "--portfolio-root", "."]
        with mock.patch.object(sys, "argv", argv), redirect_stderr(io.StringIO()):
            with self.assertRaisesRegex(SystemExit, "2"):
                skill_map.parse_args()


class PortfolioInventoryTest(unittest.TestCase):
    def test_classifies_repository_user_and_client_exposures(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            repository = base / "repository"
            agents_root = base / "home" / ".agents" / "skills"
            claude_root = base / "home" / ".claude" / "skills"
            write_skill(repository / "skills" / "repo-skill", "repo-skill", "claude-code")
            write_skill(agents_root / "codex-user", "codex-user")
            write_skill(claude_root / "claude-user", "claude-user")
            portfolio = skill_map.Portfolio(
                repository_root=repository,
                user_roots=(
                    skill_map.UserSkillRoot(agents_root, "codex", True),
                    skill_map.UserSkillRoot(claude_root, "claude-code", True),
                ),
            )

            skills = skill_map.discover_skills(portfolio.scan_roots, True, portfolio)

        by_name = {skill.name: skill for skill in skills}
        self.assertEqual(by_name["repo-skill"].location, "repository")
        self.assertEqual(by_name["repo-skill"].kind, "catalog")
        self.assertEqual(by_name["repo-skill"].clients, ("claude-code",))
        self.assertEqual(by_name["codex-user"].location, "user")
        self.assertEqual(by_name["codex-user"].kind, "install")
        self.assertEqual(by_name["codex-user"].clients, ("codex",))
        self.assertEqual(by_name["claude-user"].clients, ("claude-code",))

    def test_keeps_lexical_symlink_exposures_for_one_real_target(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            repository = base / "repository"
            source = repository / "skills" / "shared-skill"
            agents_root = base / "home" / ".agents" / "skills"
            claude_root = base / "home" / ".claude" / "skills"
            write_skill(source, "shared-skill")
            agents_root.mkdir(parents=True)
            claude_root.mkdir(parents=True)
            (agents_root / "shared-skill").symlink_to(source, target_is_directory=True)
            (claude_root / "shared-skill").symlink_to(source, target_is_directory=True)
            portfolio = skill_map.Portfolio(
                repository_root=repository,
                user_roots=(
                    skill_map.UserSkillRoot(agents_root, "codex", True),
                    skill_map.UserSkillRoot(claude_root, "claude-code", True),
                ),
            )

            skills = skill_map.discover_skills(portfolio.scan_roots, True, portfolio)
            shared = [skill for skill in skills if skill.name == "shared-skill"]

        self.assertEqual(len(shared), 3)
        self.assertEqual(len({skill.path for skill in shared}), 3)
        self.assertEqual({skill.real_directory for skill in shared}, {str(source.resolve())})
        self.assertEqual(sum(bool(skill.is_symlink) for skill in shared), 2)
        self.assertEqual({skill.exposure_path for skill in shared}, {skill.path for skill in shared})
        self.assertTrue(all(skill.symlink_target for skill in shared if skill.is_symlink))
        self.assertEqual(skill_map.duplicate_installs(shared, set()), [])

    def test_hashes_are_stable_and_cover_support_content_and_executable_bits(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            first = base / "first"
            second = base / "second"
            write_skill(first, "hash-skill")
            write_skill(second, "hash-skill")
            first_support = first / "scripts" / "helper.sh"
            second_support = second / "scripts" / "helper.sh"
            first_support.parent.mkdir()
            second_support.parent.mkdir()
            first_support.write_text("#!/bin/sh\necho test\n", encoding="utf-8")
            second_support.write_text("#!/bin/sh\necho test\n", encoding="utf-8")
            first_support.chmod(0o644)
            second_support.chmod(0o644)

            skill_hash = skill_map.sha256_file(first / "SKILL.md")
            initial = skill_map.sha256_tree(first)

            self.assertEqual(skill_hash, skill_map.sha256_file(second / "SKILL.md"))
            self.assertEqual(initial, skill_map.sha256_tree(second))

            second_support.write_text("#!/bin/sh\necho changed\n", encoding="utf-8")
            self.assertNotEqual(initial, skill_map.sha256_tree(second))
            self.assertEqual(skill_hash, skill_map.sha256_file(second / "SKILL.md"))

            second_support.write_text("#!/bin/sh\necho test\n", encoding="utf-8")
            second_support.chmod(os.stat(second_support).st_mode | 0o100)
            self.assertNotEqual(initial, skill_map.sha256_tree(second))


class ExistingJsonCompatibilityTest(unittest.TestCase):
    def test_root_json_fields_and_distinct_location_duplicates_are_preserved(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_skill(root / "one", "same-skill")
            write_skill(root / "two", "same-skill")
            skills = skill_map.discover_skills([root], False)
            duplicates = skill_map.duplicate_installs(skills, set())
            output = io.StringIO()
            with redirect_stdout(output):
                skill_map.as_json([root], skills, [], [], duplicates, False)

        payload = json.loads(output.getvalue())
        self.assertNotIn("portfolio", payload)
        self.assertEqual(
            set(payload["skills"][0]),
            {"name", "path", "realpath", "directory", "real_directory", "scope"},
        )
        self.assertEqual(len(payload["duplicates"]), 1)
        self.assertEqual(len(payload["duplicates"][0]["paths"]), 2)


if __name__ == "__main__":
    unittest.main()
