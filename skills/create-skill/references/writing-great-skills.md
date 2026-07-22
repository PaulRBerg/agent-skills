# Writing Great Skills

Predictability levers for skill authoring. Adapted from Matt Pocock's
[writing-great-skills](https://github.com/mattpocock/skills/tree/main/skills/productivity/writing-great-skills)
(SKILL.md + GLOSSARY.md).

A skill exists to make a stochastic system reliably satisfy an observable contract. **Predictability** means reaching
the intended outcome while respecting the same invariants, authority boundaries, and completion evidence; it does not
require an identical execution path. A brainstorming skill should vary its ideas while consistently honoring its scope
and stopping criteria.

## Observable contract

Write the smallest interface that makes success checkable:

- **Outcome**: the state or artifact the user should receive.
- **Invariants**: rules that must hold on every valid path.
- **Preferred defaults**: opinionated choices to use when repository or user evidence does not override them.
- **Authority**: which reads, local writes, external writes, and destructive actions are allowed or gated.
- **Routing**: prerequisites, tools, scripts, and conditional references needed for each branch.
- **User communication**: the kickoff, progress, decision, blocker, and completion events worth surfacing, with the
  smallest useful output shape for each.
- **Completion evidence**: the command, inspection, or artifact that proves the outcome.

Let the agent choose the path inside that contract. Prescribe a sequence only when ordering is safety-critical, a
prerequisite determines the next action, or a deterministic helper is the simpler interface.

## User-facing presentation

Design presentation as part of the observable contract when a skill reports meaningful state:

- Lead with the outcome and use a small semantic status vocabulary consistently: `🔎` preview/read-only, `⏳` running,
  `✅` verified success, `⚠️` caveat/approval/risk, `⛔` blocked/not written, `❓` unknown, and `↩` reverted/rolled
  back. Pair every status symbol with text.
- Use at most one non-status domain icon per heading for identity. Use tables only for repeated fields, trees only for
  real structure, and progress bars only from a measured numerator and denominator.
- Keep exact commands, machine-readable output, identifiers, confirmation tokens, diagnostics, safety wording, and
  copied downstream content undecorated.
- Keep decoration in the agent's wrapper unless the requested artifact itself calls for it. Do not inject emoji into
  code, product copy, user-authored prose, external contributions, or structured data by default.

## Invocation: two loads

Every skill pays one of two costs:

- **Context load** — a model-invoked skill's `description` sits in the agent's context window every turn, spending
  tokens and attention.
- **Cognitive load** — a user-invoked skill is invisible to the agent; the human is the index and must remember it
  exists and when to reach for it. Not a cost to minimize: it is the price of human agency. Spend it where human
  judgment matters.

Choose:

- **Model-invoked** (omit `disable-model-invocation`): the agent can fire the skill autonomously, and other skills can
  reach it. Write a model-facing description with rich trigger phrasing. Pick this only when the agent must reach the
  skill on its own, or another skill must.
- **User-invoked** (`disable-model-invocation: true`): only the human, typing its name, can invoke it — no other skill
  can. Zero context load. The `description` becomes human-facing: a one-line summary, trigger lists stripped.

When user-invoked skills multiply past what the human can remember, add a **router skill**: one user-invoked skill
naming the others and when to reach for each. It can only hint, never fire them.

## Writing the description

A model-invoked description does two jobs — state what the skill is, and list the **branches** (distinct ways of being
invoked) that should trigger it. Every word adds context load, so the description earns harder pruning than the body:

- **Front-load the skill's leading word** — the description is where it does its invocation work.
- **One trigger per branch.** Synonyms renaming a single branch are duplication ("build features using TDD … asks for
  test-first development" is one branch written twice). Collapse them; keep only genuinely distinct branches.
- **Cut identity already in the body.** Keep the description to triggers, plus any "when another skill needs…" reach
  clause.
- Word the description with the leading words actually used in the user's prompts, docs, and code — the agent links that
  shared language to the skill and fires it more reliably.

## Information hierarchy

Skill content is **steps** (ordered actions) and **reference** (definitions, rules, facts consulted on demand); a skill
can be all of either, or both. Rank each piece on a ladder by how immediately the agent needs it:

1. **In-skill step** — an ordered action in `SKILL.md`, the primary tier.
2. **In-skill reference** — consulted on demand. A flat peer-set (every rule of a review on one rung) is a fine
   arrangement, not a smell.
3. **Disclosed reference** — pushed into a linked file, reached by a context pointer, loaded only when the pointer
   fires.

Push too little down and the top bloats; push too much and you hide material the agent actually needs. **Branching** is
the cleanest disclosure test: inline what every branch needs; disclose what only some branches reach.

A **context pointer**'s _wording_ — not its target — decides when, and how reliably, the agent reaches disclosed
material. A must-have target behind a weakly worded pointer is a variance bug: sharpen the wording first; inline the
material only if that fails.

**Co-location**: the hierarchy ranks how far _down_ a piece sits; co-location decides what sits _beside_ it. Keep a
concept's definition, rules, and caveats under one heading rather than scattered — a skill should read like
documentation written for the agent.

### Completion criteria

Every workflow and safety-critical phase ends on a **completion criterion** — the condition telling the agent the work
is done. Two properties make it a lever:

- **Clarity** (checkable: can the agent tell done from not-done?) resists premature completion. "Understanding reached"
  gives way; "every test passing" holds.
- **Demand** (how much it requires) sets **legwork** — the digging the agent does within a step. "Every modified model
  accounted for" forces thorough work; "produce a change list" does not. Demand binds flat reference too: "every rule
  applied" is how a step-free skill still carries an exhaustiveness bar.

The strongest criteria are both checkable and exhaustive.

## Leading words

A **leading word** is a compact concept already living in the model's pretraining that the agent thinks with while
running the skill (_lesson_, _fog of war_, _tracer bullets_). Repeated as a token — never as a sentence — it accumulates
a distributed definition and anchors a whole region of behavior in the fewest tokens by recruiting priors the model
already holds. Coining a new word recruits no priors and costs definition tokens; reach for an existing word first.

It serves predictability twice: in the body it anchors _execution_ (the agent reaches for the same behavior every time
the word appears); in the description it anchors _invocation_.

Hunt for restatements a leading word retires — each wins twice, fewer tokens and a sharper hook:

- "fast, deterministic, low-overhead" → a _tight_ loop.
- "a loop you believe in" → the loop goes _red_ on the bug (fuzzy gate → binary observable state).

## When to split skills

**Granularity** — how finely skills are divided — spends one of the two loads per cut, so split only when the cut earns
it:

- **By invocation**: split off a model-invoked skill when a distinct leading word should trigger it on its own, or
  another skill must reach it. The new always-loaded description costs context load; the independent reach has to be
  worth it.
- **By sequence**: split a run of steps when the steps still ahead (**post-completion steps**) tempt the agent to rush
  the one in front of it. Hiding only works across a real context boundary — a user-invoked hand-off or a subagent
  dispatch; an inline call leaves the later steps in context and clears nothing. Beware the reverse: merging sequences
  exposes every step's followers.

## Pruning

- **Single source of truth**: each meaning in exactly one authoritative place, so a behavior change is a one-place edit.
- **Relevance**: does the line still bear on what the skill does? Lines lose relevance by never bearing on the task, or
  by going stale as the behavior or world drifts. Shorter skills are cheaper to keep relevant.
- **No-op hunt**: test each sentence in isolation — does it change behavior versus the model's default? When one fails,
  delete the whole sentence rather than trim words from it. Most failing prose should go, not be rewritten.

## Failure modes

Diagnose skill misbehavior against these:

- **Premature completion** — ending a step before it is genuinely done; attention slips to _being done_ rather than the
  work. Defense in order: sharpen the completion criterion first (cheap, local); only if it is irreducibly fuzzy _and_
  the rush is observed, hide the post-completion steps by splitting the sequence.
- **Duplication** — the same meaning in more than one place. Costs maintenance and tokens, and inflates the meaning's
  rank on the ladder past its real prominence. The accidental inverse of a leading word, which repeats a token, never
  the meaning.
- **Sediment** — stale layers that settle because adding feels safe and removing feels risky. The default fate of any
  skill without a pruning discipline.
- **Sprawl** — a skill simply too long, even when every line is live and unique. Cure with the ladder: disclose
  reference behind pointers; split by branch or sequence so each path carries only what it needs.
- **No-op** — a line the model already obeys by default; load paid to say nothing. A weak leading word (_be thorough_)
  is a no-op; the fix is a stronger word (_relentless_), not a different technique. The test is model-relative: settle
  disagreements by running the skill, not by debate.
