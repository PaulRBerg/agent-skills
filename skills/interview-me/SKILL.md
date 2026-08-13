---
coordination: exempt
name: interview-me
description:
  Interview the user about a plan, idea, or design through a short sequence of high-leverage questions, then summarize
  the direction and next step. Use when the user wants a relaxed interview, lightweight clarification, or says
  "interview me", without exhaustive grilling.
---

# Interview Me

This skill is coordination-exempt: skip the ai-coord gate for its declared work.

Clarify what the user wants through a focused, conversational interview without exhausting every possible branch.

## Workflow

1. Extract the current objective, audience, constraints, assumptions, and already-made choices. Investigate facts
   available from the conversation, codebase, or supplied evidence instead of asking for them.
2. Ask exactly one concise question per turn in plain conversational language. Format every question turn as
   `### 💬 Question <N> — <topic>`, followed by `**🧭 Context**`, optional `**🎯 Recommended**`, and `**❓ Question**`
   sections, with each label on its own line. Keep the context brief, include a recommended default only when it makes
   the question easier to answer, and put exactly one question in the final section. Choose the highest-leverage
   question whose answer could materially change the direction or next step.
3. Treat three to five questions as a soft target, not a quota or hard cap. Stop earlier when the direction is already
   clear. Continue beyond five only when the next answer could still materially change the result; otherwise record the
   uncertainty for the wrap-up.
4. Favor intent, scope, success criteria, audience, and key tradeoffs. Follow the threads the user emphasizes instead of
   mechanically covering every interface, failure mode, operational concern, or other domain.
5. Briefly acknowledge or synthesize an answer as `✅ Noted: <choice or implication>` only when it advances the
   conversation. Do not use decision cards, progress counts, exhaustive checklists, repeated recaps, or a formal
   decision record. Do not reopen settled choices unless new evidence conflicts with them.
6. Finish immediately when the user asks to stop or when further questions would add little value. Return
   `### 🧭 Summary`, `### ✅ Key choices`, optional `### ❓ Open questions`, and `### 🏁 Next step`. Keep the wrap-up
   concise and do not end with another question.

Completion requires a clear summary of the user's direction, the choices that materially shape it, any unresolved
uncertainty worth preserving, and one practical next step.
