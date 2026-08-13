---
coordination: exempt
name: brainstorm
description:
  Brainstorm with the user to generate, combine, and refine non-obvious ideas into a promising concept. Use when the
  user wants to brainstorm, ideate, explore possibilities, escape obvious approaches, or find a creative solution.
---

# Brainstorm

This skill is coordination-exempt: skip the ai-coord gate for its declared work.

Co-create surprising but useful ideas, then converge on a concept concrete enough to test.

## Workflow

1. Frame the real objective, audience, constraints, and what feels stale or insufficient about the obvious answers.
   Investigate facts available from the conversation, codebase, or supplied evidence instead of asking for them. Start
   exploring immediately unless one missing constraint would materially change the direction.
2. Work in short, mixed rounds. Contribute substantive idea seeds before asking the user to react. In each round, offer
   a useful synthesis or reframe, a few materially different possibilities, and one low-friction response hook such as a
   choice, reaction, ranking, completion, or playful constraint. Do not turn the session into a questionnaire or make
   the user supply all the creativity.
3. Change lenses as the conversation develops. Draw selectively from analogy, assumption reversal, an unrelated domain,
   a different stakeholder, subtraction, combination, changed scale or time horizon, extreme constraints, or salvaging
   the useful part of a deliberately bad idea. Do not mechanically exhaust a checklist or use randomness detached from
   the objective.
4. Track promising fragments, tensions, constraints, and rejected patterns in the conversation. Treat rejection as
   directional evidence: mutate, combine, or replace the idea instead of defending it. When momentum stalls, name the
   pattern causing the stall and switch lenses.
5. Alternate divergence with convergence. Preserve genuinely distinct directions long enough to avoid settling on the
   first plausible answer, then combine compatible fragments and compare finalists against the user's actual criteria.
   Novelty must improve the outcome, not merely make the idea unusual.
6. When the user endorses a direction or one direction is clearly strongest, sharpen it into a coherent concept. For a
   one-shot request, perform the full loop internally and recommend the best-supported direction without forcing another
   turn. If the user explicitly wants continued divergence or a shortlist, honor that instead of converging.
7. Finish with `### Chosen concept`, a concise description, `### Why it could work`, including what makes it
   meaningfully different from the obvious alternatives, and `### Smallest test`, naming the minimal action and
   observable success signal. Do not end the completed brainstorm with another question.

Completion requires a chosen or best-supported concept whose usefulness and distinctiveness are clear, plus a smallest
test with an observable success signal.
