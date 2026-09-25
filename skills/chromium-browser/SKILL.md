---
compatibility:
  Requires PRB's attach-only Chrome DevTools MCP wrapper at ~/.local/libexec/mcp/chrome-devtools and an existing
  remote-debugging Chromium browser.
name: chromium-browser
description:
  Use Chrome DevTools through PRB's shared attach-only Chromium browser for browsing, debugging, automation, visual
  inspection, console or network analysis, performance or memory profiling, screencasts, and Wayback Machine research.
---

# Chromium Browser

Operate the configured Chrome DevTools MCP against the existing shared browser without disrupting unrelated tabs or
authenticated state.

This skill owns rendered browser UI interaction, inspection, automation, and verification through shared Chromium. Use
search, fetch, APIs, CLIs, or connectors for retrieval when they fit, and host native-app tools for non-browser UI. Do
not use native-app tools as a fallback around this skill's shared-browser attachment, page-ownership, or privacy rules.
If the user selects another available browser integration, follow its tool contract without mixing controllers.

## Environment Contract

- Treat `~/.local/libexec/mcp/chrome-devtools` and the tools exposed in the current session as authoritative. The
  wrapper owns server versioning, flags, logging, and browser attachment; do not run the MCP package directly.
- The MCP attaches to an existing remote-debugging browser. Never launch a fallback browser or create another profile
  when attachment fails.
- Treat the browser as shared, authenticated, and concurrently used by the user and other agents. Inspect only pages
  relevant to the task and do not surface unrelated tab titles or content.
- Trust the live tool inventory; an absent tool is unavailable in this session. Do not advise editing client MCP
  configuration as a troubleshooting shortcut.

## Reference Routing

For Wayback Machine capture discovery or replay research, read `references/wayback-machine.md` before making any Wayback
request. Otherwise do not load it. Discover captures through serialized APIs outside Chromium, then open only the
selected replay when rendered inspection adds evidence.

## Page Ownership

1. Call `list_pages` before interacting and preserve the initial pages as pre-existing state.
2. Prefer `new_page` with `background: true` when a fresh page satisfies the task. Record the exact `pageId` returned by
   every page this task creates; never infer ownership from a later page-list difference.
3. Pass an explicit `pageId` to every page-scoped tool. Do not rely on selected-page state; use `select_page` only when
   deliberately bringing a page to the foreground or recovering the closed-page context described below.
4. Navigate or mutate a pre-existing page only when the task explicitly depends on that page's current state. Never
   close a pre-existing page.
5. Before closing an owned page that is currently selected, `select_page` a previously observed surviving page (for
   example the page selected before this task opened its own) with `bringToFront: false`, then `close_page` the owned
   page by its ID. If the close still returns the closed-page error described in Troubleshooting, treat it as a likely
   success and confirm once using that recovery rather than retrying the close.
6. At completion, close only the recorded pages created by this task unless the user asked to leave one open.

## Interaction and Evidence

- On one page, navigate, wait for a useful known signal, take a fresh snapshot, then interact with identifiers from that
  snapshot. Refresh it after navigation or meaningful DOM changes.
- Prefer `take_snapshot` for structure and automation, `take_screenshot` for visual evidence, and `evaluate_script` for
  information absent from the accessibility tree. Accept wrapper screenshot defaults unless the task requires lossless
  or full-resolution output.
- Keep action responses small with `includeSnapshot: false` unless the updated state is immediately needed. Paginate and
  filter console, network, memory, and other high-volume results.
- When a cookie consent popup appears, select only necessary or essential cookies by default, including through its
  settings when needed. If no such option is available, accept all cookies and continue.
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

- If closing an owned page returns `The selected page has been closed` and `list_pages` repeats it, the close may have
  succeeded while MCP retained a stale selection. Select a previously observed surviving page with
  `bringToFront: false`, then call `list_pages` and confirm the owned page ID is absent. This recovery attempts only to
  repair MCP context: do not navigate, inspect, or close the surviving page, and suppress unrelated titles and content
  from results. Do not repeat the close or create another tab to recover selection.
- If `select_page` also returns the same closed-page error, stop MCP recovery attempts and report the stale session
  context. A read-only `/json/list` request at the configured debugging endpoint can confirm that a task page's known,
  unique, unchanged URL is absent; filter locally and return only that result, never unrelated targets. This is closure
  evidence, not MCP recovery or authorization to control the browser through another route. MCP `pageId` values are not
  CDP target IDs. If the owned page cannot be identified reliably, report cleanup as unverified; do not infer ownership
  from the endpoint list or use it to close tabs.
- On attachment or transport failure, distinguish the browser endpoint from the MCP process: check the debugging
  endpoint at `http://127.0.0.1:${PRB_AGENT_CHROMIUM_PORT:-9222}/json/version`, then inspect the newest per-process log
  under `$XDG_CACHE_HOME/chrome-devtools-mcp/logs/` or, when unset, `~/.cache/chrome-devtools-mcp/logs/`.
- A responsive endpoint proves the browser process is alive, not that renderers can start. If `new_page` times out and
  tabs show a crashed icon or `Untitled`, stop repeated creation attempts and check browser health separately from MCP
  transport health. A failed creation can leave a tab without returning a `pageId`; do not infer ownership from a later
  page list or close unidentified tabs.
- After a macOS Homebrew Chromium upgrade, compare the endpoint's `Browser` version with
  `plutil -extract CFBundleShortVersionString raw /Applications/Chromium.app/Contents/Info.plist`. Confirm the running
  process uses that app bundle and inspect its start time and versioned framework/helper paths. Homebrew can replace the
  bundle while the old process keeps running, removing helpers it needs for new renderers; existing pages and
  `/json/version` can still work. A version mismatch alone is a clue, not proof of the failure.
- Report the failed layer and evidence. Do not change client configuration or wrapper flags without authorization for
  that configuration work. Do not automatically restart the shared browser during ordinary browsing or from the wrapper.
  An explicit request to repair the browser authorizes a graceful restart using the same profile and debugging port.
  Record those arguments first and preserve the session; `chrome://restart` requests a session-restoring restart. If the
  old process exits but cannot relaunch after an upgrade, confirm it has exited, then reopen the installed executable
  with the recorded profile/port arguments and `--restore-last-session`, never a fallback profile. Respect any
  unsaved-work prompt. Recheck the live version, call `list_pages` for fresh IDs, verify pre-existing pages were
  restored, and test creation and navigation on an owned page before closing it.
- When a requested capability is missing, confirm the current tool inventory and wrapper configuration, then report the
  boundary. Do not invent a fallback that weakens the configured privacy or concurrency defaults.

Completion requires fresh tool evidence for the requested outcome and confirmation that task-created pages were either
closed or intentionally left open.
