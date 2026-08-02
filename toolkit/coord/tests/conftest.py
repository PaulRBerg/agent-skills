from __future__ import annotations

import subprocess
from pathlib import Path

import pytest


@pytest.fixture
def git_repo(tmp_path: Path) -> Path:
    root = tmp_path / "repo"
    root.mkdir()
    subprocess.run(["git", "init", "-b", "main"], cwd=root, check=True, capture_output=True)
    subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=root, check=True)
    subprocess.run(["git", "config", "user.name", "Test"], cwd=root, check=True)
    (root / "src").mkdir()
    (root / "src" / "app.py").write_text("print('ok')\n")
    (root / "docs").mkdir()
    (root / "docs" / "readme.md").write_text("# Docs\n")
    subprocess.run(["git", "add", "src/app.py", "docs/readme.md"], cwd=root, check=True)
    subprocess.run(
        ["git", "-c", "commit.gpgsign=false", "commit", "--no-verify", "-m", "initial"],
        cwd=root,
        check=True,
        capture_output=True,
    )
    return root
