---
compatibility:
  Requires PRB's attach-only Chrome DevTools MCP wrapper at ~/.local/libexec/mcp/chrome-devtools and an existing
  remote-debugging Chromium browser.
name: chrome-devtools
description:
  Use Chrome DevTools through PRB's shared attach-only MCP browser for page debugging, browser automation, visual
  inspection, console or network analysis, performance or memory profiling, and screencasts.
---

# Chrome DevTools

Operate the configured Chrome DevTools MCP against the existing shared browser without disrupting unrelated tabs or
authenticated state.

## Environment Contract

- Treat `~/.local/libexec/mcp/chrome-devtools` and the tools exposed in the current session as authoritative. The
  wrapper owns server versioning, flags, logging, and browser attachment; do not run the MCP package directly.
- The MCP attaches to an existing remote-debugging browser. Never launch a fallback browser or create another profile
  when attachment fails.
- Treat the browser as shared, authenticated, and concurrently used by the user and other agents. Inspect only pages
  relevant to the task and do not surface unrelated tab titles or content.
- Trust the live tool inventory over remembered server features. An absent tool is unavailable in this session; do not
  advise editing client MCP configuration as a troubleshooting shortcut.

## Page Ownership

1. Call `list_pages` before interacting and preserve the initial pages as pre-existing state.
2. Prefer `new_page` with `background: true` when a fresh page satisfies the task. Record the exact `pageId` returned by
   every page this task creates; never infer ownership from a later page-list difference.
3. Pass an explicit `pageId` to every page-scoped tool. Do not rely on selected-page state; use `select_page` only when
   deliberately bringing a page to the foreground.
4. Navigate or mutate a pre-existing page only when the task explicitly depends on that page's current state. Never
   close a pre-existing page.
5. At completion, close only the recorded pages created by this task unless the user asked to leave one open.

## Interaction and Evidence

- Preserve order on a single page: navigate, wait for a known signal when useful, take a fresh snapshot, then interact
  with identifiers from that snapshot. Refresh the snapshot after navigation or meaningful DOM changes.
- Prefer `take_snapshot` for structure and automation, `take_screenshot` for visual evidence, and `evaluate_script` for
  information absent from the accessibility tree. Accept wrapper screenshot defaults unless the task requires lossless
  or full-resolution output.
- Keep action responses small with `includeSnapshot: false` unless the updated state is immediately needed. Paginate and
  filter console, network, memory, and other high-volume results.
- Use `filePath` for large screenshots, snapshots, traces, recordings, or response bodies, writing only to a
  task-authorized workspace path or a temporary location. Unrestricted path capability is not write authorization.
- Parallelize independent pages when useful, but preserve causal order for calls targeting the same page.

## Authority and Privacy

- Read-only inspection of task-relevant authenticated state is allowed when the task calls for it. Submitting forms,
  changing accounts, installing extensions, making purchases, or causing another external mutation requires the same
  authority that action would require outside the browser.
- Network-header redaction is an intentional server boundary. Do not bypass it or seek credentials through page or
  process introspection.

## Troubleshooting

- On attachment or transport failure, distinguish the browser endpoint from the MCP process: check the debugging
  endpoint at `http://127.0.0.1:${PRB_AGENT_CHROMIUM_PORT:-9222}/json/version`, then inspect the newest per-process log
  under `$XDG_CACHE_HOME/chrome-devtools-mcp/logs/` or, when unset, `~/.cache/chrome-devtools-mcp/logs/`.
- Expect Chromium to remain healthy when an MCP transport drops. Report which layer failed and the supporting evidence;
  do not launch another browser, edit client configuration, or change wrapper flags unless the user explicitly requests
  configuration work.
- When a requested capability is missing, confirm the current tool inventory and wrapper configuration, then report the
  boundary. Do not invent a fallback that weakens the configured privacy or concurrency defaults.

Completion requires fresh tool evidence for the requested outcome and confirmation that task-created pages were either
closed or intentionally left open.
