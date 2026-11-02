---
coordination: exempt
name: grill-me
description:
  Interview the user relentlessly about a plan or design until every branch of the decision tree has an explicit
  recorded decision and no open questions remain. Use when user wants to stress-test a plan, get grilled on their
  design, or mentions "grill me".
---

# Grill Me

This skill is coordination-exempt: skip the ai-coord gate for its declared work.

Stress-test the plan in question rounds until every material branch is resolved and the user confirms shared
understanding. User instructions take precedence over this skill.

## Workflow

1. Extract the current objective, constraints, assumptions, and already-made decisions. Map the design tree: each
   decision branches into dependent decisions. Investigate facts discoverable from the codebase, environment, or
   supplied evidence instead of asking the user. Put decisions to the user and wait for their answers.
2. Maintain a running decision record with: decision, chosen option, rationale, dependencies, and consequences. Update
   it after every answer. Reopen a recorded decision only when new evidence conflicts with it or the user corrects it.
3. When fact-finding benefits from delegation and subagents are available, delegate the investigation to a subagent that
   starts with a fresh context window. Otherwise, investigate directly. Treat pending fact-finding as an unsettled
   prerequisite. Defer only dependent questions while asking the rest of the currently answerable questions.
4. Work in rounds. The frontier contains every unresolved decision whose prerequisites are settled. Ask the entire
   frontier in each round. A question that depends on another open question belongs in a later round. Number each
   question as `### 🔥 Decision <N> — <topic>`, then give `Recommended`, `Why`, `Tradeoff`, and `Question`. Separate
   question cards with a horizontal rule. Word each `Question` so "yes" accepts its `Recommended` choice. Wait for the
   user's answers before the next round.
5. After each answer, acknowledge `✅ Recorded: <choice> — <consequence>`. Recompute the frontier after each round. When
   useful, show settled and currently open counts. Never show a percentage because the decision tree can grow.
6. Continue through scope, users, interfaces, data/state, failure modes, security, operations, migration, testing,
   rollout, ownership, and stopping criteria only where they are material to this design.
7. When no material open branches or required investigations remain, return `### ✅ Decision record` with a
   decision/choice/rationale/consequence table. Then give `### ⚠️ Risks`, accepted unknowns with owner/resolution point,
   rejected alternatives that matter, and `### 🏁 Completion criteria`. Ask the user to explicitly confirm that the
   report reflects shared understanding. Wait for that confirmation before acting on the plan. If the user corrects the
   report, reopen the affected branches and continue the interview.

Completion requires every material branch to have a recorded decision or an explicitly accepted unknown with an owner
and resolution point, followed by the user's explicit confirmation of shared understanding. The final report alone does
not complete the workflow. After confirmation, act only within the user's authorized scope.
