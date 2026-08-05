from __future__ import annotations

import subprocess
from pathlib import Path

import pytest
from hypothesis import example, given, settings
from hypothesis import strategies as st

import ai_coord.util as util_module
from ai_coord.util import (
    MAX_CALLSIGN_CODEPOINTS,
    MAX_SCOPE_CHARS,
    age_label,
    callsign_key,
    first_heading,
    git_dirty_paths,
    normalize_callsign,
    normalize_claim_scopes,
    normalize_scopes,
    overlaps_outside_coverage,
    paths_overlap,
    relevant_dirty,
    sanitize,
    scopes_cover,
)

PATH_SEGMENTS = st.text(
    alphabet=st.characters(whitelist_categories=("Ll", "Lu", "Nd"), whitelist_characters="_-"),
    min_size=1,
    max_size=12,
)
LITERAL_PATHS = st.lists(PATH_SEGMENTS, min_size=1, max_size=5).map("/".join)


@settings(max_examples=100)
@example(text=" a\n b\x00c ", limit=6)
@example(text="abcdefgh", limit=5)
@given(text=st.text(), limit=st.integers(min_value=1, max_value=240))
def test_sanitize_matches_bounded_printable_text_model(text: str, limit: int) -> None:
    result = sanitize(text, limit)
    printable = "".join(character if character.isprintable() else " " for character in text)
    collapsed = " ".join(printable.split())
    expected = collapsed[: limit - 1].rstrip() + "…" if len(collapsed) > limit else collapsed

    assert result == expected
    assert len(result) <= limit
    assert all(character.isprintable() for character in result)
    assert result == " ".join(result.split())
    assert sanitize(result, limit) == result


def test_normalize_callsign_preserves_compound_emoji_and_normalizes_text() -> None:
    callsign = normalize_callsign("  👩‍💻\nCafe\u0301   Captain  ")

    assert callsign == "👩‍💻 Café Captain"
    assert "\u200d" in callsign
    assert callsign_key("✈️  NIGHT Owl") == callsign_key("✈ night owl")


@pytest.mark.parametrize(
    ("callsign", "message"),
    (
        ("plain text", "emoji"),
        ("😀", "letter or number"),
        ("😀" + "a" * MAX_CALLSIGN_CODEPOINTS, "40 Unicode code points"),
        ("😀 pilot\x00", "control"),
    ),
)
def test_normalize_callsign_rejects_invalid_values(callsign: str, message: str) -> None:
    with pytest.raises(ValueError, match=message):
        normalize_callsign(callsign)


def test_normalize_callsign_accepts_emoji_ranges_and_exact_limit() -> None:
    assert normalize_callsign("🇷🇴 Rover") == "🇷🇴 Rover"
    assert normalize_callsign("⚙️ Gearbox") == "⚙️ Gearbox"
    assert normalize_callsign("😀" + "a" * (MAX_CALLSIGN_CODEPOINTS - 1))


@settings(max_examples=100)
@example(left=".", right="src/app.py")
@given(left=LITERAL_PATHS | st.just("."), right=LITERAL_PATHS | st.just("."))
def test_literal_scope_overlap_is_symmetric(left: str, right: str) -> None:
    overlap = paths_overlap(left, right)

    assert overlap == paths_overlap(right, left)
    if left == "." or right == ".":
        assert overlap


@settings(max_examples=100)
@given(path=LITERAL_PATHS | st.just("."))
def test_literal_scope_overlap_is_reflexive(path: str) -> None:
    assert paths_overlap(path, path)


@settings(max_examples=100)
@example(ancestor="src", descendants=["app.py"])
@given(ancestor=LITERAL_PATHS, descendants=st.lists(PATH_SEGMENTS, min_size=1, max_size=4))
def test_literal_scope_overlap_recognizes_ancestry(ancestor: str, descendants: list[str]) -> None:
    descendant = "/".join((ancestor, *descendants))

    assert paths_overlap(ancestor, descendant)


@settings(max_examples=100)
@example(parent="src", segment="a", extension="b")
@given(parent=LITERAL_PATHS, segment=PATH_SEGMENTS, extension=PATH_SEGMENTS)
def test_literal_scope_overlap_respects_segment_boundaries(
    parent: str, segment: str, extension: str
) -> None:
    left = f"{parent}/{segment}"
    right = f"{parent}/{segment}{extension}"

    assert not paths_overlap(left, right)


def test_normalize_scopes_rejects_globs_and_escapes(git_repo: Path, tmp_path: Path) -> None:
    assert normalize_scopes(("src", "src/app.py", "src"), git_repo, git_repo) == (
        "src",
        "src/app.py",
    )
    assert normalize_scopes(("src/..",), git_repo, git_repo) == (".",)
    with pytest.raises(ValueError, match="literal scope"):
        normalize_scopes(("src/**",), git_repo, git_repo)
    with pytest.raises(ValueError, match="outside repository"):
        normalize_scopes((str(tmp_path / "outside"),), git_repo, git_repo)
    with pytest.raises(ValueError, match="outside repository"):
        normalize_scopes(("..",), git_repo, git_repo)


def test_normalize_scopes_preserves_literal_leaf_symlinks(git_repo: Path, tmp_path: Path) -> None:
    outside_target = tmp_path / "outside.py"
    outside_target.write_text("value = 1\n")
    outbound_link = git_repo / "src" / "outbound_link.py"
    outbound_link.symlink_to(outside_target)
    internal_link = git_repo / "src" / "internal_link.py"
    internal_link.symlink_to(git_repo / "src" / "app.py")

    assert normalize_scopes(
        ("src/outbound_link.py", "src/internal_link.py"), git_repo, git_repo
    ) == ("src/outbound_link.py", "src/internal_link.py")


def test_normalize_scopes_rejects_external_leaf_and_symlinked_ancestor(
    git_repo: Path, tmp_path: Path
) -> None:
    incoming_link = tmp_path / "incoming_link.py"
    incoming_link.symlink_to(git_repo / "src" / "app.py")
    outside_dir = tmp_path / "outside-dir"
    outside_dir.mkdir()
    outbound_dir = git_repo / "src" / "outbound-dir"
    outbound_dir.symlink_to(outside_dir, target_is_directory=True)

    with pytest.raises(ValueError, match="outside repository"):
        normalize_scopes((str(incoming_link),), git_repo, git_repo)
    with pytest.raises(ValueError, match="outside repository"):
        normalize_scopes(("src/outbound-dir/child.py",), git_repo, git_repo)


def test_normalize_scopes_preserves_spaces_and_rejects_unsafe_lengths(git_repo: Path) -> None:
    assert normalize_scopes(("src/two  spaces.py",), git_repo, git_repo) == ("src/two  spaces.py",)
    with pytest.raises(ValueError, match=f"exceeds {MAX_SCOPE_CHARS}"):
        normalize_scopes(("x" * (MAX_SCOPE_CHARS + 1),), git_repo, git_repo)
    with pytest.raises(ValueError, match="non-printable"):
        normalize_scopes(("src/line\nbreak.py",), git_repo, git_repo)


def test_claim_scopes_require_explicit_recursion_for_existing_directories(
    git_repo: Path,
) -> None:
    with pytest.raises(ValueError, match=r"directory scope requires --recursive: src"):
        normalize_claim_scopes(("src",), (), git_repo, git_repo)

    assert normalize_claim_scopes(
        ("src/app.py", "planned.py"), ("src", "planned-dir"), git_repo, git_repo
    ) == ("src/app.py", "planned.py", "src", "planned-dir")


def test_claim_scopes_preserve_symlink_leaves_but_reject_recursive_symlinks(
    git_repo: Path,
) -> None:
    link = git_repo / "src-link"
    link.symlink_to(git_repo / "src", target_is_directory=True)

    assert normalize_claim_scopes(("src-link",), (), git_repo, git_repo) == ("src-link",)
    with pytest.raises(ValueError, match=r"recursive scope cannot be a symlink: src-link"):
        normalize_claim_scopes((), ("src-link",), git_repo, git_repo)


def test_claim_scopes_reject_recursive_files_and_cross_class_duplicates(git_repo: Path) -> None:
    with pytest.raises(ValueError, match=r"recursive scope is not a directory: src/app.py"):
        normalize_claim_scopes((), ("src/app.py",), git_repo, git_repo)
    with pytest.raises(ValueError, match=r"scope cannot be both exact and recursive: planned"):
        normalize_claim_scopes(("planned",), ("planned",), git_repo, git_repo)


def test_scope_coverage_distinguishes_refinement_from_expansion() -> None:
    assert scopes_cover(("src", "docs/readme.md"), ("src/app.py", "docs/readme.md"))
    assert not scopes_cover(("src/app.py",), ("src",))
    assert overlaps_outside_coverage(
        ("src/app.py", "docs/readme.md"), ("docs",), ("src/app.py",)
    ) == ("docs/readme.md",)
    assert overlaps_outside_coverage(("src/app.py",), ("src",), ("src/app.py",)) == ()


def test_git_dirty_paths_includes_renames_and_untracked(git_repo: Path) -> None:
    (git_repo / "src" / "app.py").rename(git_repo / "src" / "main.py")
    (git_repo / "src" / "new.py").write_text("value = 1\n")
    dirty = git_dirty_paths(git_repo)
    assert "src/app.py" in dirty
    assert "src/main.py" in dirty
    assert "src/new.py" in dirty
    assert set(relevant_dirty(("src",), dirty)) == set(dirty)
    assert relevant_dirty(("docs",), dirty) == ()


def test_git_dirty_paths_replaces_invalid_utf8_bytes(
    git_repo: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setattr(
        util_module.subprocess,
        "run",
        lambda *args, **kwargs: subprocess.CompletedProcess(args, 0, b"?? bad\xff\0", b""),
    )

    assert git_dirty_paths(git_repo) == ("bad\ufffd",)


def test_first_heading_ignores_body() -> None:
    assert first_heading("preface\n# Implement queue\nsecret body") == "Implement queue"
    assert first_heading("## No H1") is None


def test_age_label_honors_epoch_reference() -> None:
    assert age_label(0, current=0) == "0s"
