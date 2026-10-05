# Agent Legibility

This reference distills OpenAI's
[Harness engineering: leveraging Codex in an agent-first world](https://openai.com/index/harness-engineering/). It uses
the vocabulary in [SKILL.md](../SKILL.md). Centralized invariants give **locality**. The single interface through which
cross-cutting concerns enter is a **seam**. In the post, a team shipped about a million lines in five months with no
manually written code. The repository therefore had to carry everything that an agent needs.

## Feedback loops

Make the running application legible to the agent, not only its code.

- Boot one application instance per git worktree. The agent can then launch and drive one instance per change.
- Expose the Chrome DevTools Protocol to the agent, with skills for DOM snapshots, screenshots, and navigation. The
  agent can then reproduce bugs and validate fixes.
- Run an ephemeral observability stack for each worktree, with logs, metrics, and traces. The agent can query logs with
  LogQL and metrics with PromQL. When the task is complete, remove the stack. With this context, measurable prompts
  become tractable, such as "ensure service startup completes in under 800ms" and "no span in these four critical user
  journeys exceeds two seconds".

When an agent fails, the fix is almost never "try harder." Ask which capability is missing. Then make that capability
both legible and enforceable for the agent.

## Enforce architecture mechanically

Divide each business domain into a fixed set of layers, with strictly validated dependency direction and a limited set
of permissible edges. In the post, code in a domain depends only forward:
`Types → Config → Repo → Service → Runtime → UI`. Cross-cutting concerns (auth, connectors, telemetry, feature flags)
enter through one explicit interface, called Providers. Custom linters and structural tests enforce these edges. Teams
usually postpone this architecture until they have hundreds of engineers. With coding agents, it is an early
prerequisite, because the constraints allow speed without decay or architectural drift.

Also encode "taste invariants" as custom lints. In the post, custom lints enforce structured logging, naming conventions
for schemas and types, file size limits, and platform-specific reliability requirements. Write the error messages of
custom lints as remediation instructions, because these messages go into agent context.

Enforce constraints centrally and allow autonomy locally. Output meets the bar when it is correct, maintainable, and
legible to future agent runs, even if it does not match human style preferences. Record human taste in the repository
continuously. Convert review comments, refactoring pull requests, and user-facing bugs into documentation updates or
tooling. When documentation is not sufficient, promote the rule into code.

## Dependencies

Prefer dependencies and abstractions that the agent can fully internalize and reason about in the repository.
Technologies often called "boring" tend to be easier for agents to model. The reasons are composability, API stability,
and representation in the training set. Sometimes a reimplemented subset costs less than a workaround for opaque
upstream behavior. In the post, the team wrote its own map-with-concurrency helper instead of a generic `p-limit`-style
package. The helper integrates tightly with the team's OpenTelemetry instrumentation and has 100% test coverage.

## Entropy and garbage collection

Agents copy patterns that already exist in the repository, including uneven or suboptimal ones, so drift is inevitable.
The team in the post first spent every Friday, 20% of the week, on manual cleanup. That approach did not scale. Instead,
write "golden principles" in the repository. These are opinionated, mechanical rules that keep the codebase legible and
consistent for future agent runs. The post gives two examples:

- Prefer shared utility packages to hand-rolled helpers, so that invariants stay centralized.
- Do not probe data "YOLO-style." Validate data where it enters the system, or use typed SDKs. Then the agent cannot
  accidentally build on guessed shapes.

On a regular cadence, background agent tasks scan for deviations, update quality grades, and open targeted refactoring
pull requests. Most of these pull requests need less than a minute of review and can merge automatically. Repaying
technical debt continuously in small increments is almost always better than painful bursts.

## Throughput and merge gates

When agent throughput far exceeds human attention, corrections are cheap and waiting is expensive. Under that condition,
the post's repository uses minimal blocking merge gates and short-lived pull requests. It often handles test flakes with
follow-up runs instead of blocking progress. Use this trade only under that high-throughput condition, where it is often
correct. In a low-throughput environment, it would be irresponsible.
