---
compatibility:
  Requires PRB's attach-only Chrome DevTools MCP wrapper at ~/.local/libexec/mcp/chrome-devtools.sh and an existing
  remote-debugging Chromium browser.
name: chromium-browser
description:
  Use Chrome DevTools through PRB's shared attach-only Chromium browser for browsing, debugging, automation, visual
  inspection, console or network analysis, performance or memory profiling, screencasts, and Wayback Machine research.
---

# Chromium Browser

Operate the configured Chrome DevTools MCP against the existing shared browser without disrupting unrelated tabs or
authenticated state.

This skill owns rendered browser UI interaction, inspection, automation, and verification through shared Chromium. When
they fit, use search, fetch, APIs, CLIs, or connectors for retrieval. Use host native-app tools for non-browser UI. Do
not use native-app tools as a fallback around this skill's shared-browser attachment, page-ownership, or privacy rules.
If the user selects another available browser integration, follow its tool contract without mixing controllers.

## Environment Contract

- Treat `~/.local/libexec/mcp/chrome-devtools.sh` and the tools exposed in the current session as authoritative. The
  wrapper runs the globally installed `chrome-devtools-mcp` executable and sets flags, logging, and browser attachment.
  Do not run the MCP package directly.
- The MCP attaches to an existing remote-debugging browser. Never launch a fallback browser or create another profile
  when attachment fails.
- Treat the browser as shared, authenticated, and concurrently used by the user and other agents. Inspect only pages
  relevant to the task and do not surface unrelated tab titles or content.
- Trust the live tool inventory. An absent tool is unavailable in this session. Do not advise editing client MCP
  configuration as a troubleshooting shortcut.

For released tool behavior, consult the
[upstream tool reference](https://github.com/ChromeDevTools/chrome-devtools-mcp/blob/chrome-devtools-mcp-v1.10.1/docs/tool-reference.md).
The current session's schemas determine which tools and parameters are available.

## Reference Routing

For Wayback Machine capture discovery or replay research, read `references/wayback-machine.md` before making any Wayback
request. Otherwise do not load it. Discover captures through serial API requests outside Chromium. Then open only the
selected replay when rendered inspection adds evidence.

## Page Ownership

1. Call `list_pages` before interacting and preserve the initial pages as pre-existing state.
2. Prefer `new_page` with `background: true` when a fresh page satisfies the task. Record the exact `pageId` returned by
   every page this task creates. Never infer ownership from a later page-list difference.
3. Pass an explicit `pageId` to every page-scoped tool. Do not rely on selected-page state. Use `select_page` only when
   deliberately bringing a page to the foreground or recovering the closed-page context described below.
4. Navigate or mutate a pre-existing page only when the task explicitly depends on that page's current state. Never
   close a pre-existing page.
5. Before closing a selected owned page, use `select_page` with `bringToFront: false` on a previously observed surviving
   page. For example, use the page selected before this task opened its own. Then use `close_page` on the owned page by
   its ID. If the close still returns the closed-page error described in Troubleshooting, treat it as a likely success
   and confirm once using that recovery rather than retrying the close.
6. At completion, close only the recorded pages created by this task unless the user asked to leave one open.

## Interaction and Evidence

- On one page, navigate first. Wait for a useful known signal. Take a fresh snapshot. Then interact with identifiers
  from that snapshot. Refresh the snapshot after navigation or meaningful DOM changes.
- Prefer `take_snapshot` for structure and automation, `take_screenshot` for visual evidence, and `evaluate_script` for
  information absent from the accessibility tree. If exposed, use `waitForStableDom: false` only for scripts that read
  data without changing page state.
- Prefer `fill_form` for batches of inputs, selects, checkboxes, and radio buttons. For `upload_file`, use the current
  `filePaths` array schema with task-authorized files on the browser host.
- For CSS cascade questions, use `get_css_styles` with a UID from a fresh snapshot. Paginate with `pageIdx` and
  `pageSize` to inspect additional matched and inherited rules.
- Accept wrapper screenshot defaults. Request `format: "png"` when lossless output is required. PNG, `fullPage`, and
  `filePath` cannot bypass the wrapper's 1600-pixel width and height caps. Full-resolution output requires an authorized
  wrapper configuration change.
- Keep action responses small. Set `includeSnapshot: false` on `click` and `fill` unless the updated state is
  immediately needed. Do not pass `includeSnapshot` to `navigate_page`, which does not accept it. A snapshot in an
  action response can add about 10 KB to the context.
- Bound every `evaluate_script` return value to a summary, a count, or at most about 50 items. Never return a whole DOM,
  HTML document, or large array.
- Paginate and filter console, network, memory, and other high-volume results.
- Before `resize_page`, confirm the window is in the normal state. A maximized or full-screen window returns
  `Restore window to normal state before setting content size`. On that error, use a viewport emulation tool if the
  session exposes one. Otherwise, skip the resize and tell the user.
- When a cookie consent popup appears, select only necessary or essential cookies by default, including through its
  settings when needed. If no such option is available, accept all cookies and continue.
- Use `filePath` for screenshots, snapshots, traces, recordings, and response bodies. Write only to a task-authorized
  absolute path. Prefer a git-ignored `.ai/` directory under the current repository, or another root the client
  negotiated. The Claude Code scratchpad under `/private/tmp` and the `/tmp` directory are not permitted roots. A write
  there returns `Access denied`. On that error, retry once under `<cwd>/.ai/`. Path capability is not write
  authorization.
- Calls within one MCP server are serialized, even with explicit page routing. Separate agent servers still share
  browser state. Preserve causal order and page ownership across concurrent work.

## Human Gates

A human gate is a captcha, a login, a 2FA prompt, a phone verification, or an external approval page. An example of an
external approval page is an npm trusted-publisher prompt. Ending the turn at a human gate is a premature stop.

1. Stop all writes on the gated page. Never try to bypass the gate.
2. Tell the user the exact page and the action the gate requires.
3. Allow ten minutes in total unless the user states another response window. Use sequential `wait_for` calls on the
   gated `pageId`, with each `timeout` at most 10000. Match text that appears only after the gate clears. Examples are
   the post-login heading or the next form label. If no distinctive text exists, use snapshots separated by short,
   interruptible host waits. Do not queue another browser call behind an outstanding `wait_for`. Canceling a host wait
   does not prove that its MCP call stopped.
4. After a user reply, take a fresh snapshot before waiting again. A cleared gate can leave the form awaiting another
   action. When the gate clears, resume from the same page state. Re-verify the form fields first, because sites clear
   them.
5. End the turn only after the total response window expires. Then report the exact step the user must complete and the
   step that resumes the task.

## Audits and Profiling

Use these workflows only when the tools are exposed in the current session.

- For Lighthouse audits, choose `mode: "snapshot"` to analyze current page state. `mode: "navigation"` reloads the page.
  Lighthouse excludes performance audits. Use performance traces for performance questions.
- Navigate before `performance_start_trace`. Set `reload: false` to profile an interaction without reloading. When
  `autoStop: false`, stop the trace with `performance_stop_trace` after the measured interaction, including during
  cleanup after a failed task. Stop only traces this task started.
- For heap analysis, capture with `take_heapsnapshot`, then start with `get_heapsnapshot_summary`. Use targeted,
  paginated queries for objects or retainers. Call `close_heapsnapshot` for each snapshot this task loaded to release
  server memory, including during cleanup after a failed task.
- Each server permits one active screencast. Record whether this task started it, then call `screencast_stop` during
  cleanup. Never stop another task's recording to make room.

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
  repair MCP context.

  Do not navigate, inspect, or close the surviving page. Suppress unrelated titles and content from results. Do not
  repeat the close or create another tab to recover selection.

- If `select_page` also returns the same closed-page error, stop MCP recovery attempts and report the stale session
  context. A read-only `/json/list` request at the configured debugging endpoint can confirm that a task page's known,
  unique, unchanged URL is absent. Filter locally and return only that result. Never return unrelated targets. This is
  closure evidence, not MCP recovery or authorization to control the browser through another route. MCP `pageId` values
  are not CDP target IDs.

  If you cannot identify the owned page reliably, report cleanup as unverified. Do not infer ownership from the endpoint
  list or use it to close tabs.

- On attachment or transport failure, distinguish the browser endpoint from the MCP process. First, check the debugging
  endpoint at `http://127.0.0.1:${PRB_AGENT_CHROMIUM_PORT:-9222}/json/version`. Then inspect the newest per-process log
  under `$XDG_CACHE_HOME/chrome-devtools-mcp/logs/` or, when unset, `~/.cache/chrome-devtools-mcp/logs/`.
- A responsive endpoint proves the browser process is alive, not that renderers can start. If `new_page` times out and
  tabs show a crashed icon or `Untitled`, stop repeated creation attempts and check browser health separately from MCP
  transport health. A failed creation can leave a tab without returning a `pageId`. Do not infer ownership from a later
  page list or close unidentified tabs.
- After a macOS Homebrew Chromium upgrade, compare the endpoint's `Browser` version with
  `plutil -extract CFBundleShortVersionString raw /Applications/Chromium.app/Contents/Info.plist`. Confirm the running
  process uses that app bundle and inspect its start time and versioned framework/helper paths. Homebrew can replace the
  bundle while the old process keeps running. This can remove helpers it needs for new renderers. Existing pages and
  `/json/version` can still work. A version mismatch alone is a clue, not proof of the failure.
- Report the failed layer and evidence. Do not change client configuration or wrapper flags without authorization for
  that configuration work. Do not automatically restart the shared browser during ordinary browsing or from the wrapper.

  An explicit request to repair the browser authorizes a graceful restart using the same profile and debugging port.
  Record those arguments first. Preserve the session. `chrome://restart` requests a session-restoring restart. If the
  old process exits but cannot relaunch after an upgrade, confirm it has exited, then reopen the installed executable
  with the recorded profile/port arguments and `--restore-last-session`, never a fallback profile. Respect any
  unsaved-work prompt.

  Recheck the live version. Call `list_pages` for fresh IDs. Verify that pre-existing pages were restored. Test creation
  and navigation on an owned page before closing it.

- When a requested capability is missing, confirm the current tool inventory and wrapper configuration, then report the
  boundary. Do not invent a fallback that weakens the configured privacy or concurrency defaults.

Completion requires fresh tool evidence for the requested outcome and confirmation that task-created pages were either
closed or intentionally left open.
