---
compatibility:
  Requires PRB's attach-only Chrome DevTools MCP wrapper, bundled as scripts/chrome-devtools-mcp.sh, and a
  remote-debugging Chromium browser. On macOS, scripts/ensure-chromium.sh launches that browser when it is closed.
name: chromium-browser
description:
  Use Chrome DevTools through PRB's shared attach-only Chromium browser for browsing, debugging, automation, visual
  inspection, console or network analysis, performance or memory profiling, screencasts, and Wayback Machine research.
---

# Chromium Browser

Operate the configured Chrome DevTools MCP against the existing shared browser without disrupting unrelated tabs or
authenticated state.

Prefer this skill for rendered web pages in shared Chromium. When they fit, use search, fetch, APIs, CLIs, or connectors
for retrieval. Use host computer-use/CUA tools for native apps, browser chrome, and browser-internal pages such as
`chrome://extensions`. Also use native controls for a user-requested browser without a usable browser integration.
Existing task authorization covers this controller choice without a separate permission request.

Use one controller at a time for each browser. Preserve existing profiles, sessions, and unrelated tabs across
controllers. Native controls do not relax privacy rules or approval requirements for the action itself. If the user
selects another available browser integration, follow its tool contract.

## Environment Contract

- Treat the bundled `scripts/chrome-devtools-mcp.sh` wrapper and the tools exposed in the current session as
  authoritative. The wrapper runs the globally installed `chrome-devtools-mcp` executable and sets flags, logging, and
  browser attachment. Do not run the MCP package directly.
- The MCP attaches to an existing remote-debugging browser and does not launch one.
- If an MCP call cannot attach to the browser, run the bundled `scripts/ensure-chromium.sh` once. The helper launches
  Chromium with the shared profile on the configured port. It does nothing when the endpoint already responds. After it
  succeeds, retry the failed call once. If the call still fails, follow Troubleshooting.
- Never launch Chromium another way, launch a fallback browser, or create another profile.
- Treat the browser as shared, authenticated, and concurrently used by the user and other agents. Inspect only pages
  relevant to the task and do not surface unrelated tab titles or content.
- Trust the live tool inventory. An absent tool is unavailable in this session. Do not advise editing client MCP
  configuration as a troubleshooting shortcut.
- The wrapper returns plain-text responses and permits file tools on every absolute path. Path capability is not write
  authorization.

For released tool behavior, consult the
[upstream tool reference](https://github.com/ChromeDevTools/chrome-devtools-mcp/blob/chrome-devtools-mcp-v1.12.0/docs/tool-reference.md).
The current session's schemas determine which tools and parameters are available.

## Reference Routing

For Wayback Machine capture discovery or replay research, read `references/wayback-machine.md` before making any Wayback
request. Otherwise do not load it. Discover captures through serial API requests outside Chromium. Then open only the
selected replay when rendered inspection adds evidence.

## Call Budget

Every browser call costs a round trip, and every response costs context. Spend both deliberately.

- When the host loads tool schemas on demand, request the core set in one call: `list_pages`, `new_page`,
  `navigate_page`, `take_snapshot`, `evaluate_script`, `click`, `fill`, `type_text`, `press_key`, `wait_for`,
  `take_screenshot`, and `close_page`. Request other tools only when a step needs them.
- Every response from `list_pages`, `new_page`, `select_page`, `close_page`, and `navigate_page` lists all open pages.
  Do not call `list_pages` to confirm what one of those responses already shows.
- Choose the cheapest evidence that answers the question. `evaluate_script` returns only the values you select.
  `take_snapshot` returns the whole accessibility tree. `take_screenshot` returns an image, which costs more context and
  more model time than either.
- An inline snapshot is typically about 8 KB, and one in ten exceeds 20 KB. A long article, repository page, or feed can
  produce hundreds of kilobytes, which exceeds the host's result limit and returns an error instead of the snapshot. For
  such a page, extract the needed data with `evaluate_script`, or save the snapshot with `filePath` and search the file
  with `rg`.
- `evaluate_script` is exclusive. While it runs, every other call on this server waits, including calls on other pages.
  Keep scripts short and batch several reads into one script.

## Page Ownership

The numbered steps below apply to MCP pages. With native controls, track task-created tabs through the native UI. Verify
the requested result, then close only those tabs unless the user asked to leave them open.

1. When a fresh page satisfies the task, start with `new_page` and `background: true`. Its response lists every open
   page and marks the created page `[selected]`. Record that page ID as owned by this task. Treat every other listed
   page as pre-existing. Call `list_pages` first only when the task must act on a pre-existing page.
2. When an input action response lists `## Opened pages`, record those page IDs as created by this task. Never infer
   ownership from a later page-list difference.
3. Pass an explicit `pageId` to every page-scoped tool. Do not rely on selected-page state. Use `select_page` only when
   deliberately bringing a page to the foreground.
4. Navigate or mutate a pre-existing page only when the task explicitly depends on that page's current state. Never
   close a pre-existing page.
5. At completion, close only the recorded pages created by this task unless the user asked to leave one open. Call
   `close_page` with each owned ID directly. The server selects another page itself, and the response lists the
   remaining pages. Use that list as closure evidence. Do not call `select_page` before the close or `list_pages` after
   it.

## Interaction and Evidence

- On one page, navigate first. Wait for a useful known signal only when the page renders content after load. Take a
  fresh snapshot. Then interact with identifiers from that snapshot. Refresh the snapshot after navigation or meaningful
  DOM changes.
- Prefer `take_snapshot` for structure and automation, `take_screenshot` for visual evidence, and `evaluate_script` for
  information absent from the accessibility tree. Keep the default non-verbose snapshot. `verbose: true` roughly doubles
  its size.
- Set `waitForStableDom: false` on every `evaluate_script` call that only reads. The default waits up to three seconds
  for DOM mutations to stop, which a read does not need. Return a compact JSON object. Bound every array to about 50
  items. Never return a whole DOM, HTML document, or large array.
- `wait_for` returns the full snapshot when the text appears and a short error on timeout. Do not call `take_snapshot`
  after a successful `wait_for` on the same page. On a large page, check for the signal with a read-only
  `evaluate_script` instead.
- Input actions omit the snapshot by default. Pass `includeSnapshot: true` to `click`, `fill`, or another input tool
  only when the next step needs the updated structure immediately. That replaces a separate `take_snapshot`. Do not pass
  `includeSnapshot` to `navigate_page`, which does not accept it.
- When a call returns `A dialog is open`, the open dialog blocks `evaluate_script` and other dialog-sensitive tools on
  that page. Use `handle_dialog` only on a page that this task owns or depends on, within the task's authority.
- Prefer `fill_form` for batches of inputs, selects, checkboxes, and radio buttons. For `upload_file`, use the current
  `filePaths` array schema with task-authorized files on the browser host.
- For CSS cascade questions, use `get_css_styles` with a UID from a fresh snapshot. Paginate with `pageIdx` and
  `pageSize` to inspect additional matched and inherited rules.
- For a screenshot, pass `uid` to capture one element when only that element matters. Pass `filePath` when the image is
  evidence for the user rather than input for your next decision. Accept wrapper screenshot defaults. Request
  `format: "png"` when lossless output is required. PNG, `fullPage`, and `filePath` cannot bypass the wrapper's
  1600-pixel width and height caps. Full-resolution output requires an authorized wrapper configuration change.
- Paginate and filter console, network, memory, and other high-volume results.
- To change the rendered size, use `emulate` with `viewport`. Use `resize_page` only when the OS window size itself
  matters. It fails with `Restore window to normal state before setting content size` on a maximized or full-screen
  window. On that error, use `emulate`. If `emulate` is absent, skip the resize and tell the user.
- When a cookie consent popup appears, select only necessary or essential cookies by default, including through its
  settings when needed. If no such option is available, accept all cookies and continue.
- Use `filePath` for screenshots, snapshots, traces, recordings, response bodies, and large script results. Write only
  to a task-authorized absolute path, such as the host's scratch directory or a git-ignored `.ai/` directory under the
  current repository.
- Within one MCP server, page-scoped calls on different pages can run in parallel. Calls on the same page run in order.
  Exclusive tools, such as `list_pages`, `new_page`, `select_page`, `close_page`, `resize_page`, and `evaluate_script`,
  wait for running calls and block later calls. Separate agent servers still share browser state. Preserve causal order
  and page ownership across concurrent work.

## Human Gates

A human gate is a captcha, a login, a 2FA prompt, a phone verification, or an external approval page. An example of an
external approval page is an npm trusted-publisher prompt. Ending the turn at a human gate is a premature stop.

1. Stop all writes on the gated page. Never try to bypass the gate.
2. Tell the user the exact page and the action the gate requires.
3. Allow ten minutes in total unless the user states another response window. Use sequential `wait_for` calls on the
   gated `pageId`, with each `timeout` at most 10000. Each timed-out call returns a short error, and the successful call
   returns the snapshot you need next. Match text that appears only after the gate clears. Examples are the post-login
   heading or the next form label. Set `visible: true` when the text can exist hidden before the gate clears. If no
   distinctive text exists, use snapshots separated by short, interruptible host waits. Do not queue another browser
   call behind an outstanding `wait_for`. Canceling a host wait does not prove that its MCP call stopped.
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
  `reload: true`, set `ignoreCache: true` to measure a cold load. When `autoStop: false`, stop the trace with
  `performance_stop_trace` after the measured interaction, including during cleanup after a failed task. Stop only
  traces this task started.
- For heap analysis, capture with `take_heapsnapshot`, then start with `get_heapsnapshot_summary`. Use targeted,
  paginated queries for objects or retainers. Use `compare_heapsnapshots` for growth between two snapshots and
  `analyze_heapsnapshot_contexts` for closures that retain dead captured values. Call `close_heapsnapshot` for each
  snapshot this task loaded to release server memory, including during cleanup after a failed task.
- Each server permits one active screencast. Record whether this task started it, then call `screencast_stop` during
  cleanup. Never stop another task's recording to make room.

## Authority and Privacy

- Read-only inspection of task-relevant authenticated state is allowed when the task calls for it. Submitting forms,
  changing accounts, installing extensions, making purchases, or causing another external mutation requires the same
  authority that action would require outside the browser.
- Network-header redaction is an intentional server boundary. Do not bypass it or seek credentials through page or
  process introspection.

## Troubleshooting

- If a page-scoped call returns `No page found`, or a response notes that the browser was restarted or reconnected, the
  page ID is stale. Take IDs from the latest page-listing response. Do not infer ownership from that list or close
  unidentified pages. If you cannot identify an owned page reliably, report cleanup as unverified.
- On attachment or transport failure, distinguish the browser endpoint from the MCP process. First, check the debugging
  endpoint at `http://127.0.0.1:${PRB_AGENT_CHROMIUM_PORT:-9222}/json/version`. `scripts/ensure-chromium.sh` performs
  this check. Then inspect the newest per-process log under `$XDG_CACHE_HOME/chrome-devtools-mcp/logs/` or, when unset,
  `~/.cache/chrome-devtools-mcp/logs/`.
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
  A launch of a closed browser with `scripts/ensure-chromium.sh` is not a restart.

  An explicit request to repair the browser authorizes a graceful restart using the same profile and debugging port.
  Record those arguments first. Preserve the session. `chrome://restart` requests a session-restoring restart. If the
  old process exits but cannot relaunch after an upgrade, confirm it has exited, then reopen the installed executable
  with the recorded profile/port arguments and `--restore-last-session`, never a fallback profile. Respect any
  unsaved-work prompt.

  Recheck the live version. Call `list_pages` for fresh IDs. Verify that pre-existing pages were restored. Test creation
  and navigation on an owned page before closing it.

- When a requested capability is missing, confirm the current tool inventory and wrapper configuration. Use native
  controls for the browser UI cases described above, including an unsupported `chrome://extensions` URL. Otherwise,
  report the boundary. Do not weaken the configured privacy or concurrency defaults.

Completion requires fresh tool evidence for the requested outcome and confirmation that task-created pages were either
closed or intentionally left open.
