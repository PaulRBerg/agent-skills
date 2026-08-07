use crate::{domain::FindingSummary, error::Result};

pub(super) fn triage_prompt(start_head: &str, findings: &[FindingSummary]) -> Result<String> {
    let payload = serde_json::to_string_pretty(findings)?;
    Ok(format!(
        "You are the autonomous ai-coord findings triager. Work only on the supplied findings.\n\
         Verify every claim against the repository at current HEAD; the run began at {start_head}. Group semantic \
         duplicates and select one canonical finding. Close a record as stale or rejected only with concrete proof.\n\
         Auto-fix only unambiguous typos, broken local links, and clearly stale factual prose in ordinary hand-authored \
         documentation. Never modify code or comments, generated or canonical evidence, configuration, schemas, API \
         contracts, legal or release text, AGENTS.md, CLAUDE.md, skills, policies, or guidance.\n\
         Use ordinary ai-coord path ownership before editing. Run narrow validation. For each canonical finding you fix, \
         create exactly one local commit on main through ai-commit with a `Finding-ID: <ID>` trailer. Never push.\n\
         For everything outside the safe tier, explicitly invoke `$task-handoff` and publish the exact repository-relative \
         `.ai/task-handoffs/FINDING_<UPPERCASE_ID>.md` with its `finalize --no-clipboard` workflow. Preserve the ledger ID's \
         original spelling in the exact line `Source finding: <ID>`, and never overwrite a mismatched existing file. Leave \
         uncertain work deferred. Do not spawn or delegate to agents.\n\
         Return only the required structured result. Evidence must explain verification; changed_paths and validation \
         must be exact. fixed requires its commit OID, duplicate requires canonical_id, and handed_off requires the exact \
         handoff_path.\n\nFindings:\n{payload}\n"
    ))
}
