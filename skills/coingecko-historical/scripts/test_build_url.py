from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("build-url.py")
SPEC = importlib.util.spec_from_file_location("build_url", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class BuildUrlTests(unittest.TestCase):
    def test_builds_window_across_year_boundary(self) -> None:
        self.assertEqual(
            MODULE.build_url("bitcoin", "2025-01-01"),
            "https://coingecko.com/en/coins/bitcoin/historical_data?start=2024-12-31&end=2025-01-02",
        )

    def test_rejects_non_slug(self) -> None:
        with self.assertRaises(ValueError):
            MODULE.build_url("BTC/USD", "2025-01-01")


if __name__ == "__main__":
    unittest.main()
