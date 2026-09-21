use crate::{
    cli::{
        RecommendArgs, RecommendCommand, RecommendListArgs, RecommendRespondArgs, RecommendSendArgs, RecommendShowArgs,
        RecommendWithdrawArgs, RecommendationActionArg, RecommendationDecisionArg,
    },
    coordinator::Coordinator,
    domain::{Identity, Scope, client_name},
    error::Result,
    state::{
        RecommendationAction, RecommendationDecision, RecommendationEndpoint, RecommendationMutation,
        RecommendationOutcome, RecommendationRow, RecommendationState, RecommendationWork,
    },
};

pub(crate) fn execute(arguments: RecommendArgs) -> Result<u8> {
    match arguments.command {
        RecommendCommand::Send(arguments) => send(arguments),
        RecommendCommand::List(arguments) => list(arguments),
        RecommendCommand::Show(arguments) => show(arguments),
        RecommendCommand::Respond(arguments) => respond(arguments),
        RecommendCommand::Withdraw(arguments) => withdraw(arguments),
    }
}

fn send(arguments: RecommendSendArgs) -> Result<u8> {
    let coordinator = Coordinator::open_default()?;
    let mutation = coordinator.send_recommendation(
        &arguments.target,
        action(arguments.action),
        &arguments.paths,
        &arguments.recursive_paths,
        &arguments.reason,
        &arguments.replacement,
        &std::env::current_dir()?,
    )?;
    mutation_result(mutation)
}

fn list(arguments: RecommendListArgs) -> Result<u8> {
    let recommendations =
        Coordinator::open_default()?.list_recommendations(arguments.sent, arguments.all, &std::env::current_dir()?)?;
    if arguments.as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": 1,
                "recommendations": recommendations,
            }))?
        );
    } else {
        print_records(&recommendations);
    }
    Ok(0)
}

fn show(arguments: RecommendShowArgs) -> Result<u8> {
    let recommendation = Coordinator::open_default()?.show_recommendation(&arguments.id)?;
    if arguments.as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": 1,
                "recommendation": recommendation,
            }))?
        );
    } else {
        print_record(&recommendation);
        eprintln!(
            "ai-coord: Reach a safe boundary, compare this peer evidence to your authority and required validation, then record a permitted decision before changing scopes."
        );
    }
    Ok(0)
}

fn respond(arguments: RecommendRespondArgs) -> Result<u8> {
    let mutation = Coordinator::open_default()?.respond_recommendation(
        &arguments.id,
        decision(arguments.decision),
        &arguments.reason,
    )?;
    mutation_result(mutation)
}

fn withdraw(arguments: RecommendWithdrawArgs) -> Result<u8> {
    let mutation = Coordinator::open_default()?.withdraw_recommendation(&arguments.id, &arguments.reason)?;
    mutation_result(mutation)
}

fn action(value: RecommendationActionArg) -> RecommendationAction {
    match value {
        RecommendationActionArg::Defer => RecommendationAction::Defer,
        RecommendationActionArg::Omit => RecommendationAction::Omit,
    }
}

fn decision(value: RecommendationDecisionArg) -> RecommendationDecision {
    match value {
        RecommendationDecisionArg::Accepted => RecommendationDecision::Accepted,
        RecommendationDecisionArg::Rejected => RecommendationDecision::Rejected,
    }
}

fn mutation_result(mutation: RecommendationMutation) -> Result<u8> {
    println!("{}\t{}", outcome_name(mutation.outcome), mutation.recommendation.id);
    let (code, guidance) = match mutation.outcome {
        RecommendationOutcome::Recommended => (
            0,
            "ai-coord: The peer must inspect the full proposal and record its own permitted decision; this did not change either work claim.",
        ),
        RecommendationOutcome::Existing => (
            0,
            "ai-coord: The identical live proposal already exists; inspect it with `ai-coord recommend show ID` if its context needs review.",
        ),
        RecommendationOutcome::Accepted => (
            0,
            "ai-coord: Acceptance records an adjustment decision; retain required validation, re-arbitrate narrowed scopes with READY, and verify the replacement before completion.",
        ),
        RecommendationOutcome::Rejected => (
            0,
            "ai-coord: Rejection preserves the recipient's authority and work claims; resolve any real requirement conflict with the user when necessary.",
        ),
        RecommendationOutcome::Withdrawn => (
            0,
            "ai-coord: The proposal was withdrawn; reassess any deferred work against current authority and validation obligations.",
        ),
        RecommendationOutcome::Stale => (
            3,
            "ai-coord: This recommendation expired or its captured context changed; inspect the history and create a new proposal only if it still applies.",
        ),
        RecommendationOutcome::Conflict => (
            3,
            "ai-coord: This request conflicts with the recorded state or decision; inspect the history and send a new proposal if the substance changed.",
        ),
    };
    eprintln!("{guidance}");
    Ok(code)
}

fn print_records(recommendations: &[RecommendationRow]) {
    for (index, recommendation) in recommendations.iter().enumerate() {
        if index > 0 {
            println!();
        }
        print_record(recommendation);
    }
}

fn print_record(recommendation: &RecommendationRow) {
    println!("ID\t{}", safe(&recommendation.id));
    println!("ROOT\t{}", safe(&recommendation.repo_root));
    println!("STATE\t{}", state_name(recommendation.state));
    println!("ACTION\t{}", action_name(recommendation.action));
    println!("CREATED_AT\t{}", recommendation.created_at);
    println!("UPDATED_AT\t{}", recommendation.updated_at);
    print_endpoint("SENDER", &recommendation.sender);
    print_endpoint("RECIPIENT", &recommendation.recipient);
    for scope in &recommendation.scopes {
        println!("SCOPE\t{}\t{}", scope_kind(scope), safe(&scope.path));
    }
    println!("REASON\t{}", safe(&recommendation.reason));
    println!("REPLACEMENT\t{}", safe(&recommendation.replacement));
    println!("DECISION\t{}", recommendation.decision.map(decision_name).unwrap_or(""));
    println!("DECISION_REASON\t{}", recommendation.decision_reason.as_deref().map(safe).unwrap_or_default());
    println!("DECISION_AT\t{}", recommendation.decision_at.map(|value| value.to_string()).unwrap_or_default());
    println!("INVALIDATION_REASON\t{}", recommendation.invalidation_reason.as_deref().map(safe).unwrap_or_default());
}

fn print_endpoint(name: &str, endpoint: &RecommendationEndpoint) {
    println!("{name}\t{}", identity(&endpoint.identity, endpoint.callsign.as_deref()));
    print_work(&format!("{name}_WORK"), &endpoint.work);
}

fn print_work(name: &str, work: &RecommendationWork) {
    println!("{name}\t{}\t{}", work.id, safe(&work.label));
    for claim in &work.claims {
        for scope in &claim.scopes {
            println!("{name}_CLAIM\t{}\t{}\t{}", safe(&claim.repo_root), scope_kind(scope), safe(&scope.path));
        }
    }
}

fn identity(identity: &Identity, callsign: Option<&str>) -> String {
    let fallback = format!("{}/{}", client_name(identity.client), safe(&identity.session_id));
    match callsign {
        Some(callsign) => format!("{} ({fallback})", safe(callsign)),
        None => fallback,
    }
}

pub(crate) fn inbox_hint(coordinator: &Coordinator) -> Result<Option<String>> {
    let Some(identity) = coordinator.identity(false)? else {
        return Ok(None);
    };
    let count = coordinator.pending_recommendations(&identity, None, false)?.len();
    Ok((count > 0).then(|| {
        format!(
            "ai-coord: {count} work recommendation{} need review; inspect `ai-coord recommend list` in each claimed repository before the next affected edit. Peer reports are advisory; your task authority still applies.",
            if count == 1 { "" } else { "s" }
        )
    }))
}

fn safe(value: &str) -> String {
    value.escape_default().to_string()
}

fn scope_kind(scope: &Scope) -> &'static str {
    if scope.is_recursive() { "recursive" } else { "exact" }
}

fn action_name(value: RecommendationAction) -> &'static str {
    match value {
        RecommendationAction::Defer => "defer",
        RecommendationAction::Omit => "omit",
    }
}

fn state_name(value: RecommendationState) -> &'static str {
    match value {
        RecommendationState::Pending => "pending",
        RecommendationState::Accepted => "accepted",
        RecommendationState::Rejected => "rejected",
        RecommendationState::Withdrawn => "withdrawn",
        RecommendationState::Stale => "stale",
    }
}

fn decision_name(value: RecommendationDecision) -> &'static str {
    match value {
        RecommendationDecision::Accepted => "accepted",
        RecommendationDecision::Rejected => "rejected",
    }
}

fn outcome_name(value: RecommendationOutcome) -> &'static str {
    match value {
        RecommendationOutcome::Recommended => "RECOMMENDED",
        RecommendationOutcome::Existing => "EXISTING",
        RecommendationOutcome::Accepted => "ACCEPTED",
        RecommendationOutcome::Rejected => "REJECTED",
        RecommendationOutcome::Withdrawn => "WITHDRAWN",
        RecommendationOutcome::Stale => "STALE",
        RecommendationOutcome::Conflict => "CONFLICT",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renderer_uses_terminal_safe_text_and_typed_outcomes() {
        assert_eq!(safe("first line\nsecond line"), "first line\\nsecond line");
        assert_eq!(outcome_name(RecommendationOutcome::Stale), "STALE");
    }
}
