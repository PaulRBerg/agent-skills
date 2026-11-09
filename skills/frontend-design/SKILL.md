---
compatibility:
  Designed for Codex and Claude Code. Prefer the chromium-browser skill for rendered verification when available,
  otherwise the host's DevTools/browser tool.
name: frontend-design
skill-dependencies:
  - chromium-browser
description:
  Use when creating or substantially redesigning web interfaces, landing pages, dashboards, components, or other
  frontend UI where visual direction and implementation quality matter. Produces subject-specific art direction,
  accessible code, and rendered visual verification.
---

# Frontend Design

Create a working frontend with a clear, subject-specific point of view. Then prove it in the rendered UI.

## Contract

- For a critique, review, or design plan, inspect the relevant product and report the direction without editing files.
- For a build or redesign, make the requested in-scope changes and run local, non-destructive validation.
- Treat explicit user direction, supplied references, repository instructions, existing product behavior, and the local
  design system as authoritative. Fidelity requests override this skill's defaults.
- Preserve the stack and working behavior. Do not replace the framework or design system, add dependencies, or invent
  features unless the request requires it.
- Preserve distinctive existing artwork and useful visual cues during redesigns. Before replacing them, verify that the
  replacement retains their contribution to identity, comprehension, or interaction.
- Choose original SVG authored in code or stock imagery when it supports the art direction. Generate other media only
  when the request requires it.
- Make the smallest defensible assumption when the subject, audience, or page job is missing. State it before building.
  Ask only when the answer would materially change scope or identity.

## Layout and Theme Defaults

Treat every build as a one-off page for a specific purpose. Apply these defaults unless the user specifies otherwise:

- Build for a desktop browser window only. Do not add a mobile or responsive layout.
- Build one theme only. Do not add a second theme, a theme toggle, or a `prefers-color-scheme` variant.
- Before you choose colors, detect the system appearance. Use a light theme only when the system uses light mode.
  Otherwise, use a dark theme.
- On macOS, run `defaults read -g AppleInterfaceStyle`. It prints `Dark` in dark mode. In light mode, it fails because
  the key does not exist.

When you change an existing surface, keep its working responsive and theme behavior. Do not extend it to new work unless
the user asks.

## Workflow

### 1. Read the product before drawing the page

Inspect the affected route or component, neighboring UI, tokens, typography, assets, dependencies, and repository
validation commands. Identify:

- the concrete subject and audience.
- the screen's single primary job and content hierarchy.
- the existing visual language and interaction conventions.
- the required viewports, themes, states, and accessibility constraints.

Use real product content and supplied assets wherever possible. If copy is missing, write only what the user needs to
understand the interface and act.

### 2. Set one art direction

Before coding, define a compact direction:

- **Thesis:** one sentence connecting the visual idea to the subject and screen job.
- **System:** role-based color tokens, explicit type roles and scale, spacing and density, and deliberate layout and
  alignment choices.
- **Signature:** one memorable element drawn from the domain's objects, workflows, data shapes, environments, or
  language.
- **Risk:** one deliberate departure from the obvious solution, with a reason it serves this brief.
- **Motion and copy:** when motion helps, how the interface speaks, and where restraint matters.

Adapt this direction to existing brand constraints instead of creating a parallel design language. Keep options and
process narration internal unless the user asked to choose among directions.

For builds and substantial redesigns, evaluate imagery case by case. Consider stock photos, artwork, or original SVG for
backgrounds, section transitions, or focal details. Use images only when they strengthen the site's subject, atmosphere,
or content hierarchy. An image-free design is valid. Read [SVG design](references/svg-design.md) when choosing or
building an original SVG motif.

When stock imagery suits the direction, use `$chromium-browser` to search Google or [Unsplash](https://unsplash.com/).
Open the original source to verify usage rights and any attribution requirements before using an image. Search results
alone do not establish permission to reuse an asset.

Compose imagery and colorful backgrounds as one palette. Choose complementary colors, deliberate crops, and quiet space
behind text and controls. Avoid generic stock images that could fit any unrelated site.

### 3. Run the anti-default critique

Apply the **subject-swap test**: mentally replace the subject with an unrelated one. Any major choice that still works
unchanged is probably a template default. Revise that choice or justify why it is structurally correct.

Make every visual device earn its place. Cards, pills, gradients, numbering, dividers, oversized type, split heroes,
illustrations, and animation must communicate hierarchy, function, sequence, or subject—not merely decorate. Concentrate
expressive force in the signature and make the surrounding system support it.

Clear delineation is a functional requirement. Borders and containers are valid when they make grouping and ownership
easier to scan. Do not remove useful boundaries merely to make a design flatter or less conventional.

Use concrete patterns as critique prompts, not absolute bans. Question cream backgrounds, italic headline accents,
`01/02/03` section labels, monospace labels, and pill-shaped buttons when the brief does not support them. Apply the
same test to repeated decorative cards, shadows, and section entrance effects. Preserve these choices when they serve
the brief or a clear function. After the first render, identify other unexamined defaults and revise them.

### 4. Build the actual experience

- Follow local components, tokens, styling architecture, and dependency versions before introducing new primitives.
- Match composition to use: operational tools favor scannable information and efficient repeated actions. Editorial or
  marketing surfaces may use a more expressive narrative.
- Make the first viewport establish both identity and primary purpose. For an application or tool, show the real working
  experience rather than wrapping it in an unrequested landing page.
- Make projects, records, and independent panels easy to distinguish with visible borders, surface contrast, and
  spacing. Preserve useful group borders unless a rendered replacement makes the boundaries at least as clear.
- Use cards for related or repeated objects. Use nested containers only when they communicate a real hierarchy.
- Give typography deliberate display, body, and utility roles. Use available fonts first. Do not add a font dependency
  solely for novelty. One or two font families can provide these roles.
- Keep body text readable, usually below 80 characters per line. Adjust the measure and line height to the typeface and
  content.
- Write interface copy from the user's side: specific nouns, active verbs, consistent action names, sentence case, and
  useful empty and error states. A control's label must describe its result.
- Use familiar controls and the project's icon library. Give unfamiliar icon-only controls accessible names and visible
  tooltips where appropriate.
- Motion is optional. When using it, concentrate it where it explains hierarchy or gives useful interaction feedback.
  Respect reduced-motion preferences and ensure the experience remains clear without animation.
- Tune motion for perceived speed. Use ease-out curves instead of ease-in for feedback to user input, so the change
  starts at full speed and settles. Keep most interface animations under 300 ms. When an interaction must feel snappy,
  use a custom `cubic-bezier()` curve instead of the browser's `ease` keyword. Make loading spinners rotate quickly. A
  slow spinner makes the same wait feel longer.
- Prevent overlap, clipping, unreadable wrapping, and layout shift at every width in scope. By default, the scope is one
  desktop width. When the user requests responsive behavior, build responsive constraints from content rather than
  device labels, and include narrow and wide widths.
- For images, choose responsive crops and delivery sizes that preserve the focal point without unnecessary download
  weight. Give meaningful images useful alt text and decorative images empty alt text.
- Preserve semantic structure, keyboard operation, visible focus, adequate contrast, and target sizes. Match the
  project's accessibility standard when it is stricter.
- Keep selector specificity and component ownership predictable. Do not rely on competing selectors or fragile cascade
  order to establish spacing and state.

### 5. Render, inspect, and revise

Run repository-required checks and the narrowest additional checks that prove the changed behavior. Select among
formatting, lint, typecheck, tests, and build according to the affected surface. Reuse passing evidence until edits or
unresolved concerns invalidate it. Then do rendered verification with the chromium-browser skill when it is available in
this session. Otherwise, use the host's DevTools/browser tool. Do not fall back to Computer Use or ad-hoc Playwright
scripts while a DevTools browser tool is available.

Scale the inspection matrix to the change. Repository instructions may reduce the viewport/state matrix, including
waiving multi-viewport checks. Follow those instructions. Absent such instructions, a small edit to existing UI needs
verification only of the changed states at one representative viewport. A new or substantially redesigned surface needs
one desktop viewport in its single theme and every changed interaction and state. When the user requests responsive
behavior or several themes, add representative narrow and wide viewports and each requested theme. Behavior belongs in
the repository's automated tests where they exist. Use the browser for visual, layout, and theme questions.

Check the rendered result for content hierarchy, group boundaries, subject specificity, asset loading, overflow,
overlap, truncation, contrast, focus, hover, motion, empty/error states, and theme variants in scope. Confirm that each
item's group and each group's extent remain obvious at the tested widths and in each theme. Compare the result with the
brief and the art direction. Fix visible defects and remove any element that does not materially serve either.

Do not claim visual verification from source inspection alone when rendering is available. If the environment cannot
render the UI, state that limitation and report exactly what was verified instead.

## Completion

Completion requires the requested artifact or code, a subject-specific direction reflected in the implementation,
passing relevant local checks, and rendered inspection evidence when tooling permits. Report the direction in one
sentence, the checks and viewports exercised, and any remaining limitation.

Finish builds with `### ✨ Built: <surface>`, `🎨 Direction — <one sentence>`, and `### 🧪 Verification`. When several
states were inspected, use a compact viewport/state/result table. When screenshots or artifacts are available, link
them. Add `### ⚠️ Remaining` only when non-empty. Agent-report decoration does not authorize emoji or ASCII ornament in
shipped interface copy, snapshots, or source unless the brief or design system calls for it.
