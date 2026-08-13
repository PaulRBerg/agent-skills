---
coordination: exempt
name: brainstorm
description:
  Brainstorm with the user to generate, combine, and refine non-obvious ideas into a promising concept. Use when the
  user wants to brainstorm, ideate, explore possibilities, escape obvious approaches, or find a creative solution.
---

# Brainstorm

This skill is coordination-exempt: skip the ai-coord gate for its declared work.

Develop a non-obvious, useful concept through a focused creative interview, then return one substantial synthesis.

## Workflow

1. Extract the objective, audience, constraints, taste signals, and what feels stale about the obvious answers.
   Investigate facts available from the conversation, codebase, or supplied evidence instead of asking for them.
2. Ask exactly one concrete question per turn. Format every question turn as `### 💡 Question <N> — <topic>`, followed
   by `**🧭 Context**`, optional `**🎯 Answer shape**`, and `**❓ Question**`, with each label on its own line. Keep the
   context brief and put exactly one question in the final section. Use two to four concise examples or response anchors
   when they make an abstract choice easier to answer; do not turn them into an idea list.
3. Choose the question whose answer would most change the creative direction. Favor specific preferences, tensions,
   exclusions, emotional or practical effects, acceptable tradeoffs, and reactions to concrete contrasts. Avoid broad
   prompts that ask the user to do the ideation.
4. Treat three to five questions as a soft target. Stop earlier when the prompt and answers already supply a strong
   creative brief. Continue only while another answer could materially improve the final recommendation. Briefly
   acknowledge an answer as `✅ Noted: <preference or implication>` when useful; do not show progress recaps, candidate
   concepts, or interim synthesis.
5. Explore possibilities internally as the interview develops. Change lenses selectively through analogy, assumption
   reversal, an unrelated domain, another stakeholder, subtraction, combination, changed scale or time horizon, extreme
   constraints, or the useful fragment of a deliberately bad idea. Track promising fragments and rejected patterns,
   preserve genuinely distinct directions, and compare them against the user's evidence before converging.
6. Once further questions would add little value, return the first full synthesis of the session. Make it materially
   fuller than the question turns, concrete rather than exhaustive, and structured as:
   - `### 🧭 Creative brief`: objective, audience, constraints, and taste signals.
   - `### 💡 Recommended concept`: the concept, how it works, and the experience it creates.
   - `### ✨ Why it could work`: fit to the interview evidence and meaningful difference from obvious alternatives.
   - Optional `### 🔀 Strong alternatives`: at most two genuinely different directions and their tradeoffs.
   - Optional `### ⚠️ Risks and open choices`: only uncertainties that could change execution.
   - `### 🧪 Smallest test`: the minimal action and observable success signal.
7. Do not end the completed brainstorm with another question. If the user explicitly asks to keep diverging or to stop
   the interview early, honor that request while preserving the same final-report shape when synthesis is requested.

Completion requires a chosen or best-supported concept whose usefulness and distinctiveness are clear, plus a smallest
test with an observable success signal grounded in the interview.
