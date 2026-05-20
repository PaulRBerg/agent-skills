# Update README Workflow

Workflow for updating human-aimed `README.md` files based on current project metadata. By default it runs on **every** `README.md` in the repository, each scoped to its own directory and nearest manifest. README.md contains description, badges, links, references, license, and a contributing pointer — never CLI commands, `just` recipes, scripts, or project structure trees. Those live in the sibling `AGENTS.md`.

## Guiding Principles

**README.md is for humans browsing the repo on GitHub or a package registry.**

- **Generic information only** — what the project is, where to learn more, how it relates to other work.
- **No CLI commands** — install, build, test, lint, dev, deploy, scripts, `just` recipes all belong in `AGENTS.md`.
- **No project structure trees** — that's a developer concern; AGENTS.md owns it.
- **No API reference** — link to the dedicated docs site or AGENTS.md instead.
- **Show, don't tell** — short, direct prose; if a code snippet is critical, link to a file in `examples/` rather than embedding it.
- **Respect the reader's time** — they want to know what it is, whether it's relevant, and where to go next.

**Target length by mode**:

- `--minimal`: 40-80 lines
- Default: 80-150 lines
- `--thorough`: 150-250 lines (only if the project has many references or related projects worth listing)

## Workflow Steps

### STEP 1: Validate Prerequisites & Enumerate Targets

**CHECK repository state:**

- Run `git rev-parse --show-toplevel` to confirm we're in a git repository
- IF not a git repo: ERROR "Must be run from within a git repository. Initialize with 'git init' first."
- Store the repository root path

**Scope**: By default this command updates every `README.md` in the tree. Enumerate targets (see Recursive Discovery in `SKILL.md` / `references/common-patterns.md`):

```bash
git ls-files --cached --others --exclude-standard -- '**/README.md' 'README.md'
```

Drop paths under excluded dirs (`.git`, `node_modules`, `vendor`, `.venv`, `target`, `dist`, `build`, `out`, `.next`, `coverage`, gitignored, hidden-without-manifest). `--root-only` keeps just the repo-root file; a `path` argument keeps only files inside it.

Run STEP 3 onward **for each target**, scoping metadata reads to the target's own directory and nearest enclosing manifest, and reporting results grouped by path.

**FILL gaps with init**: For any package root (manifest-bearing directory) that has no `README.md`, route that directory to `init-readme` (or, with `--force`, create it from scratch in place).

**CHECK for CONTRIBUTING.md (per target directory):**

```bash
test -f "$dir/CONTRIBUTING.md" && echo "found" || echo "absent"
```

If found, append a `⚠ CONTRIBUTING.md detected` advisory for that directory recommending a merge into the sibling AGENTS.md and deleting the file. Do not edit `CONTRIBUTING.md`.

### STEP 2: Parse Arguments

Interpret arguments for mode flags:

- `path` → Confine the sweep to one directory subtree
- `--root-only` → Update the repo-root `README.md` only (disables recursion)
- `--dry-run` → Preview README content without writing
- `--preserve` → Keep existing custom prose sections (About, Background, Acknowledgments, Why X), only refresh metadata-driven sections (badges, links, license)
- `--minimal` → Title, description, badges, license, contributing/AGENTS.md pointer only
- `--thorough` (alias `--full`) → Comprehensive: include References, Related Projects, Acknowledgments
- **Default** (no flags): Balanced — title, badges, description, links, contributing pointer, license

SET mode based on arguments parsed.

### STEP 3: Gather Project Metadata

Read only the metadata needed for human-facing content. Do **not** scan scripts, lockfiles for install commands, or build configs — those are AGENTS.md territory.

**Language/Stack Detection (for badges only):**

- `package.json` → Node.js / TypeScript / JavaScript
- `Cargo.toml` → Rust
- `pyproject.toml` or `setup.py` → Python
- `foundry.toml` → Solidity / Foundry
- `go.mod` → Go
- `Gemfile` → Ruby
- `composer.json` → PHP

**Extract from detected metadata files:**

- Project name
- Version (only if shown in a version badge)
- Description / summary
- License identifier
- Homepage / documentation URL
- Repository URL (also check `git remote get-url origin`)
- Author / maintainer (for acknowledgment, optional)
- Keywords (helps categorize, optional)

**Find human-facing key files:**

- `LICENSE` or `LICENSE.md`
- `CITATION.cff`, `CITATIONS.bib`, or papers/ directory → drives References section
- `CHANGELOG.md` → link from README
- `CODE_OF_CONDUCT.md` → link from README
- `.github/FUNDING.yml` → drives Sponsors / Funding mention
- `examples/`, `demo/`, `docs/` directories → link to them
- `assets/`, `screenshots/`, `media/` directories → drive a banner or screenshot section

**Discover documentation site:**

- `package.json` `homepage` field
- `Cargo.toml` `documentation` field
- A `docs/` directory with a published site (look for `mkdocs.yml`, `docusaurus.config.js`, `vitepress.config.js`, `nextra.config.js`)
- Mention of GitHub Pages in CI (`.github/workflows/*.yml` deploying to `gh-pages`)

Do not run package managers, build tools, or scripts at any point.

### STEP 4: Read Existing README (If Preserving)

IF `--preserve` flag is set AND README.md exists:

- READ current README.md
- PARSE sections by extracting `##` and `###` headings with their content
- IDENTIFY sections to preserve (custom prose):
  - "About" / "Overview"
  - "Background" / "Motivation"
  - "Why {Project}"
  - "Acknowledgments" / "Credits" / "Thanks"
  - "References" / "Citations"
  - "Related Projects" / "See Also"
  - Any custom-headed prose blocks
- IDENTIFY sections to regenerate (metadata-driven):
  - Title + Badges
  - Description
  - Links section
  - License
  - Contributing pointer
- IDENTIFY sections to **remove** (technical content that no longer belongs in README):
  - Installation / Install / Getting Started
  - Usage / Quick Start
  - Scripts / Commands / Available Scripts
  - Project Structure / Structure
  - API Reference / API
  - Configuration / Config
  - Testing / Tests
  - Deployment

When removing technical sections, leave a single line directing the reader to AGENTS.md (see STEP 5 → Contributing pointer).

### STEP 5: Generate README Sections

**Section order (all project types):**

01. Title + Badges
02. Description
03. Optional banner / screenshot (if assets exist)
04. Links
05. About / Background (preserved or generated if `--thorough`)
06. References (if applicable)
07. Related Projects (if `--thorough` and detected)
08. Acknowledgments (preserved)
09. Contributing
10. License

**Generate each section based on mode:**

**REMEMBER**: Keep all sections concise. README is a landing page, not a manual.

**Title + Badges:**

- Extract project name from metadata or git repo name
- Add relevant badges based on what exists:
  - License badge (if LICENSE file found)
  - Version badge (from package version on the relevant registry)
  - CI status badge (if `.github/workflows/*.yml` exists)
  - Package registry badge (npm, crates.io, PyPI, etc.)
  - Stack/framework badge (only one or two, not a wall)
- Format: `# {Project Name}` followed by badges on the next line.

**Description:**

- IF `--preserve` AND existing description exists: keep it
- ELSE: extract from `package.json` / `Cargo.toml` / `pyproject.toml` description field
- ELSE: generate a 1-2 sentence description from the project metadata and any guided description provided
- **Length**: 1-3 sentences. Answer "What is this?" and stop.

**Banner / Screenshot:**

- IF an obvious banner asset exists (`assets/banner.{png,svg}`, `docs/banner.*`): embed it.
- IF screenshots are present and the project is a UI/app: embed one or two thumbnails.
- Skip otherwise. Do not invent images.

**Links:**

- Bullet list pointing readers wherever they need to go next:
  - Documentation site (if discovered)
  - Package on npm / crates.io / PyPI / etc.
  - Live demo / homepage
  - Changelog — only if `CHANGELOG.md` exists at the repo root
  - Releases page on GitHub — only if the repo actually publishes releases. Verify with `gh release list --limit 1` (non-empty output) or by checking that the GitHub remote has at least one release. Do not link to an empty `/releases` page just because the repo is on GitHub.
  - Discussion / community channels (Discord, Matrix, mailing list) — only if linked from existing project metadata
- Each link is a single line: `- [Documentation](https://example.com/docs)`
- Omit any bullet whose target does not exist; never include a placeholder URL or a link that would 404.
- This is the section that replaces Install/Usage from the old template.

**About / Background:**

- IF `--preserve`: keep existing prose verbatim
- IF `--thorough` AND no existing prose: write a short paragraph (3-5 sentences) explaining the problem the project solves and why it exists
- Default mode without existing prose: skip
- `--minimal`: skip

**References:**

- IF `CITATION.cff`, `CITATIONS.bib`, `papers/`, or related work files exist: list the citations as bullet points
- IF the description names papers or specifications: list them with links
- IF `--preserve`: keep existing References section verbatim
- ELSE: skip

**Related Projects:**

- IF `--thorough` AND README has a "Related" / "See Also" section: keep it
- IF `--thorough` AND `package.json` `keywords` clearly map to a known ecosystem (e.g. "viem", "foundry"): suggest related ecosystem links
- ELSE: skip

**Acknowledgments:**

- IF `--preserve`: keep existing section verbatim (this is one of the most important sections to never touch)
- ELSE: skip — do not auto-generate acknowledgment text

**Contributing:**

- One short paragraph plus a pointer:

  ```markdown
  ## Contributing

  Contributions are welcome. See [`AGENTS.md`](AGENTS.md) for the development workflow, commands, and conventions.
  ```

- IF `--preserve` AND existing Contributing section is custom prose: keep it but ensure the AGENTS.md link is present.

- IF `CONTRIBUTING.md` was detected: still point to AGENTS.md here, and surface the merge advisory in the final report.

**License:**

- Extract license type from `LICENSE` file or package metadata
- Format: `This project is licensed under the {LICENSE} — see the [LICENSE](LICENSE) file for details.`
- IF no license found: suggest adding one in the report (do not invent a license)

### STEP 6: Compose Final README

BUILD complete markdown content:

**Structure:**

```markdown
# {project-name}

{badges row}

{description paragraph}

{optional banner/screenshot}

## Links

- [Documentation]({url})
- [Package]({url})
- [Changelog](CHANGELOG.md)
- [Discussions]({url})

{about/background section if applicable}

{references section if applicable}

{related projects section if applicable}

{acknowledgments section if preserved}

## Contributing

Contributions are welcome. See [`AGENTS.md`](AGENTS.md) for the development workflow, commands, and conventions.

## License

{license information}
```

**Formatting rules:**

- Use plain `##` headings; no emoji decoration on section headers (the audience is humans, but emoji-noise reads as 2018 marketing copy).

- Code blocks are rare — README should not show CLI commands at all, and embedded code should be a short illustrative snippet at most. Prefer linking to `examples/`.

- Tables only for things like a feature comparison with related projects.

- Use admonitions sparingly:

  ```markdown
  > [!NOTE]
  > Helpful context

  > [!WARNING]
  > Breaking changes or critical notices
  ```

- Line length ~100-120 chars in paragraphs.

- Blank lines between sections.

**IF `--preserve` mode:**

- MERGE preserved sections (About, References, Related, Acknowledgments) with regenerated metadata sections.
- Maintain section order from original README where it makes sense.
- Insert regenerated Title/Badges/Description/Links/License/Contributing at standard positions.
- **Remove** any installation/usage/scripts/structure/API sections from the original; surface their removal in the report so the user knows where the content went (AGENTS.md).

### STEP 7: Write Updated README

WRITE the new README at the target's own path (`$dir/README.md`):

- Use the Write tool to create/overwrite `$dir/README.md` with the composed content
- Ensure content is complete and properly formatted

IF write succeeds:

- Note which sections were updated, kept, or removed (especially removed technical sections so the user can verify those moved to the sibling AGENTS.md).

IF write fails:

- Show specific error message
- Suggest fixes:
  - Check file permissions: `ls -la "$dir/README.md"`
  - Check disk space: `df -h .`
  - Verify write access to directory

Loop to the next target; emit the grouped summary (STEP 8) after the last one.

### STEP 8: Display Summary

For recursive runs, repeat this block per target under a `### {path}` sub-header and end with a tally (`Updated N files, skipped M`). Single-target template:

```markdown
✓ Updated README.md

**Mode**: {minimal/default/thorough/preserve}

**Project Type**: {Library/Application/Smart Contract/etc.}

**Sections Generated:**
- Title + Badges
- Description
- Links
- {others}

{IF --preserve mode:}
**Sections Preserved:**
- {list preserved custom sections}

{IF technical sections removed:}
**Sections Removed (now in AGENTS.md):**
- Installation
- Usage
- Scripts
- {others}

{IF CONTRIBUTING.md detected:}
⚠ CONTRIBUTING.md detected
  - Recommend merging its contents into AGENTS.md (which now owns the development workflow).
  - Delete CONTRIBUTING.md after the merge.

**Next Steps:**
1. Review README.md for accuracy
2. Move any removed technical content into AGENTS.md if not already there
3. Commit changes: `git add README.md && git commit -m "docs: update README"`
```

IF errors occurred during generation:

- List specific issues encountered
- Suggest running with different flags (e.g., `--minimal` for simpler output)
- Note which sections may need manual review

## Usage Examples

**Basic update (default mode, recursive):**

```bash
/md-docs:update-readme
```

Updates every `README.md` in the repo, each scoped to its own package — title, badges, description, links, contributing pointer, license.

**Root README only:**

```bash
/md-docs:update-readme --root-only
```

**Limit to one package subtree:**

```bash
/md-docs:update-readme packages/cli
```

**Preserve custom prose:**

```bash
/md-docs:update-readme --preserve
```

Keeps About / Background / References / Acknowledgments; refreshes badges, description, links, license.

**Minimal README (fast):**

```bash
/md-docs:update-readme --minimal
```

Title, badges, description, license, AGENTS.md pointer.

**Thorough analysis:**

```bash
/md-docs:update-readme --thorough
```

Includes References, Related Projects, About/Background prose where applicable.

**Create new README from scratch:**

```bash
/md-docs:update-readme  # in a repo without README.md → routed to init-readme
```

Or invoke `init-readme` directly.

## Key Characteristics

**Audience-strict**: README never contains CLI commands, scripts, or developer workflows. All such content is removed and recorded in the summary so the user can verify it lives in AGENTS.md.

**Language-agnostic**: Works with Node.js, Rust, Python, Solidity, Go, Ruby, PHP, and other common stacks (only for badge selection and metadata extraction).

**Non-destructive in spirit**: Overwrites `README.md` in place; rely on git to restore prior content if needed.

**Smart defaults**: Automatically detects project metadata and language for badge selection.

**Preserves manual prose**: `--preserve` keeps custom About / Background / References / Acknowledgments sections.

**Idempotent**: Running multiple times produces consistent results (same input → same output).

**No git operations**: Only updates `README.md` file, never auto-commits.

**Monorepo handling**: Recurses by default, updating every `README.md` in the tree (root and each package). Each file is scoped to its own directory and nearest manifest, and links to its sibling `AGENTS.md`. Use `--root-only` to update only the repo-root README, or a `path` argument to limit the sweep to one package.

**CONTRIBUTING.md handling**: Detected but never edited. Recommendation surfaced to merge into AGENTS.md and delete.

**Edge cases handled**:

- Non-standard project structures
- Multiple languages in one repo
- Private repositories (omits public-only badges)
- Missing metadata files (graceful degradation)
- No license file (suggests adding one)

**Restoration**: If you don't like the generated README, restore via git:

```bash
git checkout README.md
```
