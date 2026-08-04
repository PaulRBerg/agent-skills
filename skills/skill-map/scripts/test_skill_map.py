from __future__ import annotations

import importlib.util
import io
import re
import sys
import tempfile
import unittest
from contextlib import redirect_stderr
from pathlib import Path
from unittest import mock


SCRIPT_PATH = Path(__file__).with_name("skill-map.py")
SPEC = importlib.util.spec_from_file_location("skill_map", SCRIPT_PATH)
assert SPEC is not None and SPEC.loader is not None
skill_map = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = skill_map
SPEC.loader.exec_module(skill_map)


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


if __name__ == "__main__":
    unittest.main()
