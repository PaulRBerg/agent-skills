from __future__ import annotations

from pathlib import Path

import pytest

from ai_coord.util import (
    MAX_SCOPE_CHARS,
    age_label,
    first_heading,
    git_dirty_paths,
    normalize_scopes,
    paths_overlap,
    relevant_dirty,
    sanitize,
)


def test_sanitize_collapses_and_caps() -> None:
    assert sanitize(" a\n b\x00c ", 6) == "a b c"
    assert sanitize("abcdefgh", 5) == "abcd…"


@pytest.mark.parametrize(
    ("left", "right", "expected"),
    [
        (".", "src/app.py", True),
        ("src", "src/app.py", True),
        ("src/app.py", "src/app.py", True),
        ("src/a", "src/ab", False),
        ("src", "docs", False),
    ],
)
def test_literal_scope_overlap(left: str, right: str, expected: bool) -> None:
    assert paths_overlap(left, right) is expected


def test_normalize_scopes_rejects_globs_and_escapes(git_repo: Path, tmp_path: Path) -> None:
    assert normalize_scopes(("src", "src/app.py", "src"), git_repo, git_repo) == (
        "src",
        "src/app.py",
    )
    with pytest.raises(ValueError, match="literal scope"):
        normalize_scopes(("src/**",), git_repo, git_repo)
    with pytest.raises(ValueError, match="outside repository"):
        normalize_scopes((str(tmp_path / "outside"),), git_repo, git_repo)


def test_normalize_scopes_preserves_spaces_and_rejects_unsafe_lengths(git_repo: Path) -> None:
    assert normalize_scopes(("src/two  spaces.py",), git_repo, git_repo) == ("src/two  spaces.py",)
    with pytest.raises(ValueError, match=f"exceeds {MAX_SCOPE_CHARS}"):
        normalize_scopes(("x" * (MAX_SCOPE_CHARS + 1),), git_repo, git_repo)
    with pytest.raises(ValueError, match="non-printable"):
        normalize_scopes(("src/line\nbreak.py",), git_repo, git_repo)


def test_git_dirty_paths_includes_renames_and_untracked(git_repo: Path) -> None:
    (git_repo / "src" / "app.py").rename(git_repo / "src" / "main.py")
    (git_repo / "src" / "new.py").write_text("value = 1\n")
    dirty = git_dirty_paths(git_repo)
    assert "src/app.py" in dirty
    assert "src/main.py" in dirty
    assert "src/new.py" in dirty
    assert set(relevant_dirty(("src",), dirty)) == set(dirty)
    assert relevant_dirty(("docs",), dirty) == ()


def test_first_heading_ignores_body() -> None:
    assert first_heading("preface\n# Implement queue\nsecret body") == "Implement queue"
    assert first_heading("## No H1") is None


def test_age_label_honors_epoch_reference() -> None:
    assert age_label(0, current=0) == "0s"
