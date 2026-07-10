---
name: grill-me
description:
  Interview the user relentlessly about a plan or design until every branch of the decision tree has an explicit
  recorded decision and no open questions remain. Use when user wants to stress-test a plan, get grilled on their
  design, or mentions "grill me".
---

# Grill Me

Stress-test the plan one decision at a time until every material branch is resolved.

## Workflow

1. Extract the current objective, constraints, assumptions, and already-made decisions. Investigate any answer
   discoverable from the codebase or supplied evidence instead of asking the user.
2. Maintain a running decision record with: decision, chosen option, rationale, dependencies, and consequences. Update
   it after every answer; do not reopen a recorded decision unless new evidence conflicts with it.
3. Ask exactly one question at a time. Choose the highest-leverage unresolved branch whose answer constrains the most
   downstream decisions, and include your recommended answer with the tradeoff that makes it preferable.
4. Continue through scope, users, interfaces, data/state, failure modes, security, operations, migration, testing,
   rollout, ownership, and stopping criteria only where they are material to this design.
5. When no material open branches remain, return the complete decision record, assumptions, rejected alternatives that
   matter, risks, and explicit completion criteria. Do not end with another question.

Completion requires every material branch to have a recorded decision or an explicitly accepted unknown with an owner
and resolution point.
