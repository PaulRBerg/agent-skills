# Writing Great Skills

Compact authoring guidance adapted from Matt Pocock's
[writing-great-skills](https://github.com/mattpocock/skills/tree/main/skills/productivity/writing-great-skills)
(SKILL.md + GLOSSARY.md).

A skill makes a stochastic system reliably satisfy an observable contract. Predictability means reaching the intended
outcome while preserving the contract, not following an identical path.

## Contents

- [Observable contract](#define-the-observable-contract)
- [Self-containment](#keep-independently-installed-skills-self-contained)
- [Context and invocation](#spend-context-deliberately)
- [Frontmatter dialect](#frontmatter-dialect)
- [Representation and routing](#choose-the-smallest-useful-representation)
- [Execution](#write-for-reliable-execution)
- [Background reporting](#design-background-reporting)
- [Presentation](#keep-presentation-substantive)

## Define the Observable Contract

Write the smallest interface that makes success checkable:

- **Outcome**: the state or artifact the user should receive.
- **Invariants**: rules that must hold on every valid path.
- **Preferred defaults**: opinionated choices that explicit user intent or repository evidence may override.
- **Authority**: which reads, local writes, external writes, and destructive actions are allowed or gated.
- **Routing**: prerequisites, tools, scripts, and conditional references needed for each branch.
- **Stop conditions**: states that require a different workflow, missing authority, or user-owned input.
- **User communication**: the kickoff, progress, decision, blocker, and completion events worth surfacing, with the
  smallest useful output shape for each.
- **Completion evidence**: the command, inspection, or artifact that proves the outcome.

Let the agent choose the path inside that contract. Prescribe a sequence only when ordering is safety-critical, a
prerequisite determines the next action, or a deterministic helper is the simpler interface.

Make completion criteria both checkable and demanding enough to force the required legwork. Prefer criteria such as
"every modified model accounted for and the scoped check passes" over subjective states such as "understanding reached."

## Keep Independently Installed Skills Self-Contained

Put reusable guidance in the owning skill and discover target-project conventions at runtime. Do not share references
across skills or depend on another repository unless that repository is required to perform the task.

## Spend Context Deliberately

Every skill pays one of two costs:

- **Context load** — a model-invoked skill's `description` sits in the agent's context window every turn, spending
  tokens and attention.
- **Cognitive load** — a user-invoked skill is invisible to the agent; the human must remember when to invoke it. Spend
  this where human judgment matters.

Choose:

- **Model-invoked** (omit `disable-model-invocation`): the agent and other skills can reach it. Write a model-facing
  description with one trigger phrase per distinct branch.
- **User-invoked** (`disable-model-invocation: true`): disable automatic model selection. Its description becomes a
  human-facing one-line summary. An explicitly authorized workflow may still load its instructions through supported
  host mechanisms; absence from automatic discovery does not prove it is unavailable.

Inline what every branch needs. Route conditional detail directly from `SKILL.md`; the wording of the link must say when
to read it. Co-locate a concept's definition, rules, and caveats. Keep each meaning in one authoritative place.

Judge context economy relative to the current target models: remove a sentence when it does not change their behavior,
and resolve disagreements with representative runs rather than intuition. Prune descriptions hardest because they may
load on every turn.

Prefer domain-first capability names such as `large-file-refactor`; keep memorable verb-based exceptions when clearer.
Use familiar domain terms from the user's prompts, docs, and code. Give a model-facing description one trigger per
distinct branch, without repeating identity already in the body.

## Frontmatter Dialect

`ai-skillet doctor` accepts an extended top-level field union:

- Portable: `name`, `description`, `license`, `compatibility`, `metadata`, `allowed-tools`.
- Claude Code: `when_to_use`, `argument-hint`, `arguments`, `disable-model-invocation`, `user-invocable`,
  `disallowed-tools`, `model`, `effort`, `context`, `agent`, `background`, `hooks`, `paths`, `shell`.
- Repository: `coordination`, `skill-dependencies`.

Unknown fields are errors. `metadata` maps strings to strings; tool, argument, and path fields accept strings or string
lists; `hooks` is a mapping. `effort` accepts `low`, `medium`, `high`, `xhigh`, or `max`; `context` accepts only `fork`;
`shell` accepts `bash` or `powershell`. `agent` and `background` require `context: fork`. The entrypoint defines
invocation defaults and dependency policy.

Use portable-only validators such as `skills-ref` or `agentskills` only when a distribution target explicitly requires
the strict portable format; they do not replace the canonical local gate.

## Choose the Smallest Useful Representation

| Content                                                              | Put it in                    | Decision rule                                                                |
| -------------------------------------------------------------------- | ---------------------------- | ---------------------------------------------------------------------------- |
| Intent, authority, routing, exceptions, evidence judgment            | `SKILL.md`                   | The model must interpret it on every applicable path.                        |
| Deterministic computation, parsing, formatting, validation, recovery | `scripts/`                   | Repetition, exactness, or failure handling justifies code.                   |
| Closed structural shape                                              | Schema plus validator        | Types, required fields, enums, and relationships are mechanically checkable. |
| Conditional detail, long examples, API or schema documentation       | `references/` or `examples/` | Only some branches need the context.                                         |
| Templates, media, or files copied into output                        | `assets/`                    | Runtime uses the file without loading it as instructions.                    |

Prefer one deep helper with a small interface over scripts that mirror prose steps. Keep the caller-visible invariant in
prose, document the CLI and compact result, and leave the implementation algorithm in code. A helper earns its place
when logic is repeated, deterministic, exact, or recovery-heavy; a shell pipeline past roughly five lines, real error
handling, or a recurring long heredoc is a strong signal.

Use TypeScript through `bun run scripts/<name>.ts` by default. Use Python through `uv run scripts/<name>.py` when it
better fits data, text, or file processing. Keep Bash compatible with macOS `/bin/bash` 3.2. Scripts save context only
when normal runs do not require reading their source and stdout stays compact.

Aim for `SKILL.md` under 500 lines; move sections past roughly 50 lines when they are not core workflow. Move prose,
examples, or schema documentation past roughly 100 lines into a reference when not core to every branch. Link references
directly from `SKILL.md`, one level deep, with a routing sentence. Add a table of contents to references over 100 lines;
for files over 10,000 words, give targeted search patterns in `SKILL.md`. Keep a required rule inline if a direct
pointer still fails to route reliably.

Bundle a schema only with a real validation route. Keep semantic meaning and permissions in prose. Put output templates
in `assets/`, and omit repository-style support files, scratch artifacts, and authoring notes that runtime agents do not
use.

## Write for Reliable Execution

State outcomes, invariants, and completion evidence as positive, observable acceptance criteria. Keep a negative
instruction only for an explicit user exclusion or when it is the clearest concise guard against a consequential safety,
authority, destructive-action, scope, or likely model-failure boundary.

If a workflow completes prematurely, sharpen its completion criterion first; split later steps behind a real context
boundary only when observed behavior still justifies it.

Make authority and follow-through concrete:

- State that explicit user instructions take precedence over skill defaults. Carry established approvals forward;
  preserve host restrictions and required approval for destructive or external actions.
- Resolve discoverable facts before asking. Use stated assumptions for routine reversible choices; ask when missing
  user-owned input would change scope, safety, or the intended outcome, and continue independent work.
- Before requesting approval, prepare the concrete result the user must review within existing authority. When a skill
  rule causes a pause, identify the rule and the action still lacking authorization.
- Finish authorized work through its required validation. Progress summaries, offers to continue, and milestones do not
  establish completion. Name real stop conditions, including read-only scope, hard limits, and missing authority.

Choose the smallest checks that prove the changed behavior and satisfy repository requirements. Add regression tests for
meaningful failure modes, not tests that mirror a reversible text or configuration edit. Reuse passing evidence while
its inputs remain unchanged; broaden or repeat checks only for new edits, failures, or unresolved risk.

Specify delegation when it helps: independent scopes, dependencies, write ownership, and one owner for aggregate checks.
Use the smallest effective team, honor the user's delegation preference and host limits, and keep dependent mutations
sequential. A file-count threshold alone is not a reason to delegate.

## Design Background Reporting

When a skill launches background jobs or agents, decide whether the wait may be long or opaque from expected runtime and
uncertainty, fan-out or waves, meaningful milestones, and the state already visible in the host. Do not use a universal
time cutoff.

Make the main agent own monitoring and user reporting until every required unit settles. Announce the units or scope in
flight and the evidence that will prove completion. Reuse a host-native progress surface only when it exposes meaningful
live state; otherwise report observed phase changes and milestones, with sparse factual updates during quiet periods.

Never infer completion from elapsed time, event counts, or activity. Use a progress bar or percentage only with an exact
settled/total denominator. Finish with a compact report distinguishing completed, blocked, failed, and timed-out units,
their evidence, and the next action.

## Keep Presentation Substantive

Lead with the outcome and keep the output shape proportional to the information:

- Prefer concise, direct prose for simple results. Add headings, lists, and tables when they clarify the information; do
  not require a multi-section report for a one-line outcome. Honor the user's requested format.
- Use `🔎` preview/read-only, `⏳` running, `✅` verified success, `⚠️` caveat/approval/risk, `⛔` blocked/not written,
  `❓` unknown, and `↩` reverted/rolled back consistently; pair every status symbol with text.
- Use at most one non-status domain icon per heading. Reserve tables for repeated fields, trees for real structure, and
  progress bars for measured numerators and denominators.
- Keep commands, machine-readable output, identifiers, confirmation tokens, diagnostics, safety wording, and copied
  downstream content undecorated.
- Keep decoration in the reporting wrapper unless the requested artifact calls for it; do not inject emoji into code,
  product copy, user prose, external contributions, or structured data by default.
