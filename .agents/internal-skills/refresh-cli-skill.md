---
name: refresh-cli-skill
description:
  Review browser dependencies weekly and refresh cli-* skills when installed versions advance. Use stable upstream
  evidence and preserve local safety rules.
---

# Refresh CLI Skill

Refresh `skills/cli-*` when an installed CLI is newer than its `references/version.txt` marker. Review
`skills/chromium-browser` against Chrome DevTools MCP every week, even when the installed version has not changed.

Circadian's weekly maintenance job runs `jobs/refresh-cli-skills.sh` in an isolated clone of this repository. The job
commits, pushes, and publishes the result after validation.

## Arguments

- Accept arguments as `<skill>=<version>`, for example `cli-just=1.43.0` or `chromium-browser=1.10.1`.
- Treat each version as an already-normalized semver with no leading `v`.
- Accept only `cli-*` and `chromium-browser` skills.
- If no valid skill/version pairs are supplied, stop.

## Workflow

1. Work in the current checkout. In an isolated clone, never touch the shared `~/projects/agent-skills` worktree.
2. Before editing, confirm the worktree is clean. If it is dirty, stop and report the dirty paths.
3. For each requested CLI skill:
   - Confirm `skills/<cli-skill>/SKILL.md` exists.
   - Map `cli-<name>` to binary `<name>`. Map `cli-coingecko` to `cg`.
   - Confirm the binary's `--version` output still reports the requested version. If evidence is missing, stop with
     failure.
   - Read current docs and references that mention versioned features, flags, or upstream links.
   - Inspect official stable release notes and tagged manuals or CLI docs for the requested version.
   - Use installed CLI help to confirm compatibility.
   - Update stale or missing facts. Keep edits terse and preserve local safety rules.
   - Write `skills/<cli-skill>/references/version.txt` with exactly the requested semver and one trailing newline.
4. For `chromium-browser`, complete these steps even when the installed version has not changed:
   - Confirm `skills/chromium-browser/SKILL.md` exists.
   - Read `~/.local/libexec/mcp/chrome-devtools.sh` without changing it. You may read its chezmoi source for context.
   - Verify the wrapper's installed executable version through package metadata or its `--version` output. Never start
     the MCP server for this check.
   - Inspect official stable Chrome DevTools MCP release notes and tagged documentation. Compare them with the installed
     version and wrapper flags.
   - Update compatible skill guidance and link to the installed release's tagged tool reference. No browser version
     marker is required.
   - Report stable upstream releases newer than the installed version and wrapper incompatibilities. Never present
     unreleased or unavailable features as available.
   - Preserve attach-only operation, privacy and header redaction, negotiated-root restrictions, screenshot caps, and
     shared-browser ownership.
   - Treat the wrapper audit as read-only. Never change wrapper flags, settings, client configuration, or installed
     copies.
5. Edit only the requested skill directories. Never edit generated lockfiles, unrelated skills, installed copies under
   `~/.agents`, or global Claude/Codex configuration.
6. Run `just prettier-write <changed Markdown files>`. Then run `just prettier-check <changed files>` and
   `just skill-check`.
7. If the scheduled weekly job invoked you, stop without committing or publishing. The job performs both steps.
   Otherwise, commit and publish per `AGENTS.md`.
8. Report evidence sources, compatible documentation changes, newer upstream releases, and configuration
   incompatibilities. If a required check or evidence source fails, report failure instead of claiming success.

   For the scheduled review, return structured output with `status` and `summary`. Set `status` to `succeeded` only when
   all required review evidence and checks are complete. Otherwise, set `status` to `failed` and give an actionable
   `summary`. The job owns later publication. A publication phase must return `failed` if any required publication work
   is incomplete.

## Version Metadata

`references/version.txt` is the weekly maintenance job's contract. `ai-skillet doctor` also enforces it. The file must
contain exactly one normalized semver:

```text
1.2.3
```

The file must have no leading `v`, prose, comments, ranges, prerelease labels, or extra lines.
