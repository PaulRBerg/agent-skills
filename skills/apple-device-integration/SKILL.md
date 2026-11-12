---
compatibility: Requires macOS and Apple's Xcode MCP device-interaction tools.
coordination: exempt
name: apple-device-integration
description:
  Use for device or simulator UI verification through Apple's Xcode MCP after UI changes or an explicit device check.
  Inspect screenshots and UI hierarchies, exercise controls, and report functional or visual defects.
---

# Apple Device Integration

User instructions take precedence over this skill. Follow the target repository's build, fixture, privacy, device
ownership, and cleanup rules. This skill is coordination-exempt: skip the ai-coord gate for its declared work. Its
declared work is runtime verification and captured artifacts, without changes to tracked application source.

Forked from Apple's Xcode 27.2 (27B52) skill export, with local catalog metadata and MCP tool-name corrections.

TRIGGER when: user asks to verify/test/check if the app works on device, after implementing a UI-affecting feature that
needs device verification, user says "does it work", "test this", "check on device", user reports UI doesn't work as
expected, need to debug touch/interaction issues. DO NOT TRIGGER when: user asks about unit tests only, build-only
requests without device testing, code review without device testing, simulator configuration questions, changes that
don't affect UI (e.g. comments, refactors, non-UI logic).

---

# Verification Scope

For a small check, verify the changed flow directly in the parent session. Delegate only when an independent or long
verification task benefits from a separate verifier, or when the user asks for delegation.

Check the changed flow and its affected states. Use one representative task-owned simulator by default, or the specific
device named in the request. Add device variants only when a concrete risk or user or release requirement calls for
them.

Keep one controller per device at a time. Delegation is never required to drive the device.

Xcode approves each agent connection separately. In some harnesses, a subagent opens its own connection and receives
"This agent isn't approved to use Xcode's tools yet". If a delegated verifier receives this error, run the interaction
in the parent session. A verifier without approval can capture screenshots only through the target repository's own
tools.

When you delegate, the parent owns session creation, installation, and cleanup. Give the verifier exclusive use of that
session and pass the workspace identifier when applicable, exact device UUID, simulator flag, session key, requested
checks, and repository constraints. Include these interaction instructions in its brief before input. The verifier must
not change application source, commit, or broaden the authorized device actions.

Verify the session's returned UUID and simulator flag before input. Physical-device installation and live-service
actions require task authorization. Use fixture launch arguments when the repository provides them, and preserve
unrelated devices, sessions, and workspaces.

Verify after a coherent change, not after each edit. Repeat the selected checks after further relevant code changes or
observed failures. Stop when the selected checks pass.

## Session Lifecycle

Use the target repository's build and install driver first. Follow its runtime and device ownership rules.

If the repository already built, installed, and launched the app, start a standalone session with
`DeviceInteractionStartSession` and use the exact task-owned device UUID. Do not rebuild through
`DeviceInteractionInstallAndRun` when the app is already running and unchanged.

If the repository uses Apple's MCP to build and install, start a workspace session and use
`DeviceInteractionInstallAndRun` for the initial build and launch. Reuse the running app for interaction checks when its
code has not changed. Use `DeviceInteractionSynthesize` to interact and observe, then end the session when done.

Preflight the live tool catalog for the capabilities this task needs. Require workspace tools only when the selected
repository workflow needs them.

There are two ways to start a session:

- **DeviceInteractionStartWorkspaceSession** — bound to a workspace. Required for DeviceInteractionInstallAndRun. Use
  this when the repository workflow uses Apple's MCP to build and install the app.
- **DeviceInteractionStartSession** — not bound to any workspace. Use this for an already-installed app, including one
  built and launched by the repository's own driver.

## DeviceInteraction(Workspace)StartSession tools

### Device Discovery

When opening a new device interaction session, pass a device identifier to select a device, or omit it to use the
current destination. Pass an empty string to get a list of available targets.

## DeviceInteractionInstallAndRun tool

### Optional Parameters

- `commandLineArguments` — arguments passed to the app at launch. Use `$(inherited)` as a token to preserve the scheme's
  existing arguments (e.g. `["$(inherited)", "--reset-state"]` to add an extra argument at the end).
- `environmentVariables` — key/value pairs set in the app's environment at launch. Use `"$(inherited)"` as a key to
  preserve the scheme's existing environment variables (e.g. `{"$(inherited)": "", "DEBUG_MODE": "1"}`).

Omit both parameters to leave the scheme's arguments and environment unchanged.

**Prefer these parameters over editing the scheme directly.** They are applied only for that one run and have no lasting
effect on the user's configuration.

---

# For the Verifier

The verifier is the agent that drives the interaction session. This is the parent session or a delegated verifier. A
delegated verifier returns its findings to the parent.

**ALWAYS** report UI issues that might be caused by code: overlapping or unreadable text, unexpectedly cropped
image/text, wrong colors etc.

## DeviceInteractionSynthesize tool

This tool allows performing an interaction and observing the state of a device.

## Reading Hierarchy Files

The hierarchy files include calculated hitPoint positions for each element:

```
UIView {{100, 200}, {60, 30}}, hitPoint: {130.0, 215.0}
  UIButton "Login" {{110, 205}, {30, 20}}, hitPoint: {125.0, 215.0}
  UIButton "Login2" {{140, 205}, {20, 20}}, hitPoint: {150.0, 215.0}, activationBundleId: com.your.app
```

- `{100, 200}` - origin position
- `{50, 30}` - width and height
- `hitPoint: {125.0, 215.0}` - calculated hitPoint point (best for tapping)

**Always prefer the hitPoint coordinates for touch events.** Only after tapping a hitPoint and confirming (via
recapture) it had no effect may you fall back to a raw screenshot-estimated position.

Warning: Elements marked `isRemoteLeafPlaceholder` do not report child elements — interacting with them requires falling
back to screenshot-estimated coordinates.

### Multiple Applications (activationBundleId annotations)

When windows from more than one application overlap on screen, each element line is annotated with a
`activationBundleId: <bundle-identifier>` suffix.

If an element line carries a `activationBundleId`, **any interaction with it requires activating that application
first** by passing `activationBundleId` parameter to the DeviceInteractionSynthesize tool.

Tip: you can pass `activationBundleId` with no `interactionCommand`. **Application activation is expensive so use that
only if necessary.**

#### Example:

```
Device orientation: Landscape Right
------------------------
Application bundle identifier: com.some.app
Application UI orientation: Landscape Left
Application, pid: 123, label: ' '
 Window, {{0.0, 0.0}, {1133.0, 744.0}}, hitPoint: {566.5, 372.0}
  Other, {{0.0, 0.0}, {744.0, 1133.0}}, hitPoint: {372.0, 372.0}
...
------------------------
Application bundle identifier: com.some.other.app
Application UI orientation: Landscape Left
Application, pid: 333, label: ' '
 Window, {{0.0, 0.0}, {1133.0, 744.0}}, hitPoint: {566.5, 372.0}
...
```

## Interaction Command Syntax

The `interactionCommand` parameter accepts a command syntax:

| Command                                                                                | Description                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| -------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `t <x> <y> [duration]`                                                                 | Tap at coordinates with optional hold duration                                                                                                                                                                                                                                                                                                                                                                                                   |
| `d <x> <y>`                                                                            | Double tap                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| `t <x1> <y1> f <x2> <y2> [duration]`                                                   | Swipe from (x1,y1) to (x2,y2)                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `drag <x1> <y1> <x2> <y2> [holdDuration] [moveDuration]`                               | Drag-and-drop: press-and-hold at (x1,y1) then slowly move to (x2,y2). Use for reordering lists or drag-and-drop targets                                                                                                                                                                                                                                                                                                                          |
| `mt [x1 y1, ...] dur [x1 y1, ...] dur ...`                                             | Multi-touch sequence. Each `[...]` keyframe lists touch positions (`x y`). The duration after each block is the travel time to the next keyframe; for the last block it is the hold time before lifting. A finger ends when its slot is an empty comma entry (e.g. `[, x y]`) **or** when the frame has fewer entries than the finger's index — both are equivalent. A finger that reappears in a later keyframe after lifting starts a new tap. |
| `b h/p/u/d [duration]`                                                                 | Hardware button: h=Home, p=Power, u=VolUp, d=VolDown                                                                                                                                                                                                                                                                                                                                                                                             |
| `b c/s/a [duration]`                                                                   | **watchOS only.** c=Digital Crown press, s=side button, a=Action button (some devices only)                                                                                                                                                                                                                                                                                                                                                      |
| `sender keyboard kbd <text>`                                                           | Type text; **must be the last command in the chain** — all content after `kbd ` is taken verbatim (multiple spaces preserved). For special characters use `\u{XXXX}` Unicode escapes: `\u{000A}` (return/newline), `\u{0009}` (tab)                                                                                                                                                                                                              |
| `w duration`                                                                           | Wait for a duration without any work                                                                                                                                                                                                                                                                                                                                                                                                             |
| `orientation faceDown/faceUp/landscapeLeft/landscapeRight/portrait/portraitUpsideDown` | Set device orientation (iOS only)                                                                                                                                                                                                                                                                                                                                                                                                                |
| `c <rotations>`                                                                        | **watchOS only.** Rotate the Digital Crown; sign sets direction, 1.0 = one full revolution                                                                                                                                                                                                                                                                                                                                                       |
| `r up/down/left/right/select/menu/playpause/home`                                      | **tvOS only.** Press a Siri Remote button to move focus, select, or go to the Home screen                                                                                                                                                                                                                                                                                                                                                        |

**Examples:**

- `"t 100 200"` - Tap at (100, 200)
- `"d 200 300"` - Double tap at (200, 300)
- `"t 200 600 f 200 200 0.3"` - Swipe up (scroll to the content below)
- `"t 200 200 f 200 600 0.3"` - Swipe down (scroll to the content above)
- `"drag 100 300 100 100"` - Drag-and-drop from (100,300) to (100,100) with default durations
- `"drag 100 300 100 100 0.5 1.5"` - Drag-and-drop with 0.5s hold, 1.5s move
- `"mt [100 200] 0.5 [100 200] 1.0 [300 400] 0.2"` - Drag: hold at (100,200) for 0.5s, move to (300,400) over 1.0s, hold
  0.2s then lift
- `"mt [100 300, 300 300] 0.5 [175 300, 225 300] 0.5"` - Two-finger pinch: both fingers start 200px apart and move
  toward each other
- `"b h"` - Press home button
- `"b h b h"` - Press home button twice to go to the app switcher
- `"b h w 0 b h"` - Wake and unlock a device (non-passcode devices only)
- `"b c"` - watchOS: press the Digital Crown
- `"b s"` - watchOS: press the side button
- `"b a"` - watchOS: press the Action button (some devices only)
- `"sender keyboard kbd hello world"` - Type text with spaces
- `"sender keyboard kbd hello   world"` - Type text preserving multiple spaces
- `"sender keyboard kbd submit\u{000A}"` - Type text then press Return/submit
- `"w 0.3"` - Wait for 0.3s
- `"orientation landscapeLeft"` - Rotate device to landscape
- `"c 1.0"` - watchOS: rotate the Digital Crown one full turn (e.g. scroll a list)
- `"c -0.5"` - watchOS: rotate the crown half a turn the other way
- `"r down"` - tvOS: move focus down
- `"r select"` - tvOS: press Select on the focused element
- `"r home"` - tvOS: go to the Home screen

## Standard Verification Workflow

Before any interaction, always capture and read the hierarchy (and screenshot). After any interaction, capture again and
verify the result. For complex components (like toggles or switches), look at nested elements (like `Switch` or
`Slider`) — nearby elements might correspond to the actual control. When done, report the findings.

- To capture without interacting, use DeviceInteractionSynthesize with an empty interactionCommand.
- Never guess positions from screenshots alone — use hierarchy hitPoint coordinates; screenshot estimation is only a
  fallback after a hitPoint is tried and fails.
- If not confident or thumbnail resolution is insufficient, analyze the full-size screenshot.

## tvOS (Apple TV)

tvOS is **focus-based**: there is no touchscreen, so coordinate taps/swipes do not apply. Exactly one element is focused
at a time, and you can only activate whatever currently has focus. Drive it with the Siri Remote instead:

- Read the hierarchy to see all focusable elements and which one is marked `Focused`.
- Move focus toward the target with `r up`/`r down`/`r left`/`r right`, then activate it with `r select`. Use `r menu`
  to go back.
- **Chain multiple presses in one command and capture once**, rather than capturing after every single press. The
  hierarchy lists element order, so compute how many steps to the target and send them together, e.g.
  `r right r right r right`, then capture to confirm focus landed.

## Timing and Retries

- **App launch**: After starting a session, the app may take a few seconds to load. Capture the hierarchy and check it
  has meaningful UI elements before interacting. If the hierarchy is mostly empty or shows a launch screen, capture
  again before proceeding.
- **After interaction**: If a tap or swipe doesn't produce the expected change, recapture the hierarchy and retry the
  interaction once (the element may have shifted during an animation). If it still fails after one retry, report the
  failure rather than retrying indefinitely.
- **Loading states**: If the hierarchy shows a spinner or loading indicator, capture again after a brief pause. Do not
  interact with elements that are still loading.
- **Performance**: Avoid adding wait delays that might slow down the process. Tools are designed to complete once
  animations are done.

## Judging Success vs Failure

When verifying, distinguish between these categories:

- **Functional bug** (always report): element doesn't respond to tap, navigation goes to wrong screen, crash, data not
  displayed, missing expected UI element.
- **Visual/layout bug** (always report): overlapping text, truncated labels, elements rendered off-screen, wrong colors,
  broken alignment.
- **Transient state** (do NOT report as bug): loading spinners, brief animations, keyboard appearing/dismissing. Capture
  again after the transition completes.
- **Unexpected exits** (always report): crashes, application exits. To identify, track process id and capture process's
  standard output.
- **Expected behavior** (do NOT report as bug): empty states with placeholder text, disabled buttons when form is
  incomplete, permission dialogs.

## Error Handling

- If application is not visible, retry once, as this might be caused by a slow device.
- If tap target unclear, re-read hierarchy data for correct hitPoint coordinates.
- Inspect runtime logs only within the target repository's privacy rules. Report only safe diagnostic evidence.
- Report issues with details and suggestions.

## Completion Report

Report the device UUID, simulator flag, checked states, observed results, and screenshot or hierarchy paths. Name
functional and visual defects separately from expected disabled, empty, or transient states. State untested behavior and
concrete blockers. A build or simulator launch does not prove physical-device installation. The parent must end its
interaction session on success or failure and stop task-owned processes under repository rules.
