---
compatibility: Designed for Codex and Claude Code; rendered-browser or screenshot tooling is preferred for verification.
name: frontend-design
description:
  Use when creating or substantially redesigning web interfaces, landing pages, dashboards, components, or other
  frontend UI where visual direction and implementation quality matter. Produces subject-specific art direction,
  accessible responsive code, and rendered visual verification.
---

# Frontend Design

Create a working frontend with a clear, subject-specific point of view, then prove it in the rendered UI.

## Contract

- For a critique, review, or design plan, inspect the relevant product and report the direction without editing files.
- For a build or redesign, make the requested in-scope changes and run local, non-destructive validation.
- Treat explicit user direction, supplied references, repository instructions, existing product behavior, and the local
  design system as authoritative. Fidelity requests override this skill's defaults.
- Preserve the stack and working behavior. Do not replace the framework or design system, add dependencies, invent
  features, or fetch or generate media unless the request requires it.
- Make the smallest defensible assumption when the subject, audience, or page job is missing. State it before building;
  ask only when the answer would materially change scope or identity.

## Workflow

### 1. Read the product before drawing the page

Inspect the affected route or component, neighboring UI, tokens, typography, assets, dependencies, and repository
validation commands. Identify:

- the concrete subject and audience;
- the screen's single primary job and content hierarchy;
- the existing visual language and interaction conventions;
- the required viewports, themes, states, and accessibility constraints.

Use real product content and supplied assets wherever possible. If copy is missing, write only what the user needs to
understand the interface and act.

### 2. Set one art direction

Before coding, define a compact direction:

- **Thesis:** one sentence connecting the visual idea to the subject and screen job.
- **System:** role-based color tokens, explicit type roles and scale, spacing and density, and a layout principle.
- **Signature:** one memorable element drawn from the domain's objects, workflows, data shapes, environments, or
  language.
- **Risk:** one deliberate departure from the obvious solution, with a reason it serves this brief.
- **Motion and copy:** when motion helps, how the interface speaks, and where restraint matters.

Adapt this direction to existing brand constraints instead of creating a parallel design language. Keep options and
process narration internal unless the user asked to choose among directions.

### 3. Run the anti-default critique

Apply the **subject-swap test**: mentally replace the subject with an unrelated one. Any major choice that still works
unchanged is probably a template default; revise it or justify why it is structurally correct.

Make every visual device earn its place. Cards, pills, gradients, numbering, dividers, oversized type, split heroes,
illustrations, and animation must communicate hierarchy, function, sequence, or subject—not merely decorate. Concentrate
expressive force in the signature and make the surrounding system support it.

### 4. Build the actual experience

- Follow local components, tokens, styling architecture, and dependency versions before introducing new primitives.
- Match composition to use: operational tools favor scannable information and efficient repeated actions; editorial or
  marketing surfaces may use a more expressive narrative.
- Make the first viewport establish both identity and primary purpose. For an application or tool, show the real working
  experience rather than wrapping it in an unrequested landing page.
- Use structural devices only when they encode real relationships. Use cards for genuinely grouped or repeated objects,
  not as the default wrapper for every section, and avoid nested cards.
- Give typography deliberate display, body, and utility roles. Use available fonts first; do not add a font dependency
  solely to manufacture novelty.
- Write interface copy from the user's side: specific nouns, active verbs, consistent action names, sentence case, and
  useful empty and error states. A control's label must describe its result.
- Use familiar controls and the project's icon library. Give unfamiliar icon-only controls accessible names and visible
  tooltips where appropriate.
- Keep motion purposeful and concentrated. Respect reduced-motion preferences and ensure the experience remains clear
  without animation.
- Build responsive constraints from content rather than device labels. Prevent overlap, clipping, unreadable wrapping,
  and layout shift at narrow and wide widths.
- Preserve semantic structure, keyboard operation, visible focus, adequate contrast, and target sizes. Match the
  project's accessibility standard when it is stricter.
- Keep selector specificity and component ownership predictable; do not rely on competing selectors or fragile cascade
  order to establish spacing and state.

### 5. Render, inspect, and revise

Run the repository's narrowest relevant formatter, lint, type, test, and build checks. Then use the available browser or
screenshot tooling to inspect the implemented UI at representative narrow and wide viewports and exercise every changed
interaction and state.

Check the rendered result for content hierarchy, subject specificity, asset loading, overflow, overlap, truncation,
contrast, focus, hover, motion, empty/error states, and theme variants in scope. Compare it with the brief and the art
direction. Fix visible defects and remove any element that does not materially serve either.

Do not claim visual verification from source inspection alone when rendering is available. If the environment cannot
render the UI, state that limitation and report exactly what was verified instead.

## Completion

Completion requires the requested artifact or code, a subject-specific direction reflected in the implementation,
passing relevant local checks, and rendered inspection evidence when tooling permits. Report the direction in one
sentence, the checks and viewports exercised, and any remaining limitation.

Finish builds with `### ✨ Built: <surface>`, `🎨 Direction — <one sentence>`, and `### 🧪 Verification`; use a compact
viewport/state/result table when several states were inspected and link screenshots or artifacts when available. Add
`### ⚠️ Remaining` only when non-empty. Agent-report decoration does not authorize emoji or ASCII ornament in shipped
interface copy, snapshots, or source unless the brief or design system calls for it.
