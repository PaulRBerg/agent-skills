---
compatibility: Requires Git and local command and edit access.
disable-model-invocation: true
name: fresh-eyes-sweep
user-invocable: true
description:
  Audit an entire repository with fresh eyes for correctness errors, bugs, omissions, duplication, inconsistencies, and
  other evidenced mistakes; fix every safe issue and verify the result.
---

# Fresh Eyes Sweep

Relentlessly inspect the current repository for anything wrong, then fix every safe, evidenced issue.

## Workflow

1. Read the applicable repository instructions. Map every tracked and non-ignored file; classify generated, vendored,
   binary, and data artifacts; discover the repository's checks; and snapshot existing worktree changes so they remain
   untouched.
2. Account for every mapped file. Inspect code, tests, configuration, automation, schemas, and documentation plus their
   relationships. Trace important control, data, and error paths end to end. Hunt for bugs, omissions, invalid
   assumptions, edge cases, security and reliability failures, inconsistencies, duplication, dead code, misleading
   documentation, and needless complexity.
3. Confirm each suspected issue with concrete evidence. Fix it when intent is clear and the result can be verified;
   prefer the smallest root-cause change and add a focused regression test when warranted. Preserve unrelated edits,
   avoid speculative or cosmetic churn, and report unsafe or ambiguous fixes instead of guessing.
4. Run the narrowest proving check after each fix, then the relevant aggregate checks. Reinspect affected paths and
   repeat until no new evidenced issue appears.

Completion requires every mapped file to be accounted for, every discovered issue to be fixed and verified or reported
with its evidence and blocker, and every relevant check to pass or have its failure explained. Report the outcome,
changed files, exact validation results, unresolved findings, and residual risk. A verified no-op is complete.
