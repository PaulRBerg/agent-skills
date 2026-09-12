#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# ///
"""Print Taze arguments for the inherited Bun release-age policy."""

import argparse
import os
import sys
import tomllib
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--exclude-supported", action="store_true")
    args = parser.parse_args()

    global_root = os.environ.get("XDG_CONFIG_HOME") or os.environ.get("HOME")
    configs = [Path(global_root) / ".bunfig.toml"] if global_root else []
    configs.append(Path("bunfig.toml"))
    policy = {}
    for config in configs:
        try:
            with config.open("rb") as source:
                install = tomllib.load(source).get("install", {})
        except FileNotFoundError:
            continue
        except (OSError, ValueError):
            # Configs can contain registry credentials; do not echo parser input.
            raise ValueError(f"cannot read Bun configuration: {config}") from None
        if not isinstance(install, dict):
            raise ValueError(f"expected an install table in {config}")
        policy.update(install)

    age = policy.get("minimumReleaseAge", 0)
    excludes = policy.get("minimumReleaseAgeExcludes", [])
    if type(age) is not int or age < 0:
        raise ValueError("minimumReleaseAge must be a nonnegative integer")
    if not isinstance(excludes, list) or any(
        not isinstance(name, str) or not name or any(c in name for c in ",\r\n")
        for name in excludes
    ):
        raise ValueError("minimumReleaseAgeExcludes must be an array of package names")
    if age:
        print("--maturity-period")
        print((age + 86399) // 86400)
        if excludes and args.exclude_supported:
            print("--maturity-period-exclude")
            print(",".join(excludes))


if __name__ == "__main__":
    try:
        main()
    except ValueError as error:
        print(f"ERROR: {error}", file=sys.stderr)
        sys.exit(1)
