---
name: codebase-design
description:
  Shared vocabulary for designing deep modules. Use when the user wants to design or improve a module's interface, find
  deepening opportunities, decide where a seam goes, make code more testable, AI-navigable, or agent-legible, choose
  which invariants to enforce mechanically, or when another skill needs the deep-module vocabulary.
---

# Codebase Design

Design **deep modules**: substantial behaviour behind a small interface at a clean seam, testable through that
interface. Use this vocabulary when it clarifies a design decision. The aim is leverage for callers, locality for
maintainers, and testability.

## Glossary

Use these terms consistently where their distinctions matter.

**Module** — anything with an interface and an implementation. Deliberately scale-agnostic: a function, class, package,
or tier-spanning slice. _Avoid_: unit, component, service.

**Interface** — everything a caller must know to use the module correctly: the type signature, but also invariants,
ordering constraints, error modes, required configuration, and performance characteristics. _Avoid_: API, signature (too
narrow — they refer only to the type-level surface).

**Implementation** — what is inside a module, its body of code. Distinct from **Adapter**: a thing can be a small
adapter with a large implementation (a Postgres repo) or a large adapter with a small implementation (an in-memory
fake). When the seam is the topic, use "adapter". Otherwise, use "implementation".

**Depth** — leverage at the interface: the amount of behaviour a caller (or test) can exercise per unit of interface
they have to learn. A module is **deep** when a large amount of behaviour sits behind a small interface, **shallow**
when the interface is nearly as complex as the implementation.

**Seam** _(Michael Feathers)_ — a place where you can alter behaviour without editing in that place. It is the
_location_ at which a module's interface lives. Where to put the seam is its own design decision, distinct from what
goes behind it. _Avoid_: boundary (overloaded with DDD's bounded context).

**Adapter** — a concrete thing that satisfies an interface at a seam. Describes _role_ (what slot it fills), not
substance (what is inside).

**Leverage** — what callers get from depth: more capability per unit of interface they learn. One implementation pays
back across N call sites and M tests.

**Locality** — what maintainers get from depth: change, bugs, knowledge, and verification concentrate in one place
rather than spreading across callers. Fix once, fixed everywhere.

## Deep vs shallow

**Deep module** — small interface (few methods, simple params) hiding a lot of implementation.

**Shallow module** (avoid) — interface nearly as large as its thin, pass-through implementation.

When designing an interface, ask:

- Can I reduce the number of methods?
- Can I simplify the parameters?
- Can I hide more complexity inside?

## Principles

- **Depth is a property of the interface, not the implementation.** A deep module can contain small, mockable, swappable
  parts internally. These parts are not part of the interface. A module can have **internal seams** (private to its
  implementation, used by its own tests) as well as the **external seam** at its interface.
- **The deletion test.** Imagine deleting the module. If complexity vanishes, it was a pass-through. If complexity
  reappears across N callers, the module was useful.
- **The interface is the test surface.** Callers and tests cross the same seam. If you want to test _past_ the
  interface, the module is probably the wrong shape.
- **One adapter means a hypothetical seam. Two adapters means a real one.** Do not introduce a seam unless something
  actually varies across it.

## Designing for testability

Good interfaces make testing natural:

- At a real seam, accept the dependency as a parameter instead of constructing it inside the module.
- Prefer returning results to side effects when that keeps the interface simpler.
- Keep the surface small: fewer methods and parameters mean fewer, simpler tests.

## Agent legibility

A codebase is **agent-legible** when an agent can reason about the full business domain directly from the repository.

- **The repository is the agent's only context.** For the agent, knowledge in chat threads, external documents, or
  people's heads effectively does not exist. Record architecture decisions, norms, and plans as versioned artifacts in
  the repository.
- **Enforce invariants, not implementations.** Encode layer dependency direction and the single seam for cross-cutting
  concerns as custom lints and structural tests. Inside those constraints, leave implementation choices free. For
  example, require parsing where data enters the system. Do not prescribe the parsing library.
- **Write custom lint errors as remediation instructions.** These error messages land in agent context.
- **Centralize invariants in shared modules, not in hand-rolled helpers.** This is **locality**. Validate data where it
  enters the system, or use typed SDKs. Never build on guessed data shapes.
- **Prefer dependencies that the agent can fully internalize.** Choose stable, composable technologies that are well
  represented in training data. When a workaround for opaque upstream behavior costs more than a reimplementation,
  reimplement a small, tested subset.
- **Agents copy existing patterns, including bad ones.** Write golden principles as opinionated, mechanical rules in the
  repository. Correct drift against them continuously with small, targeted refactors. When documentation alone does not
  hold a rule, promote the rule into a lint.
- **When an agent struggles, find the missing capability.** Make that capability legible and enforceable for the agent.
  Do not prompt harder.
- **Make current state queryable.** Provide one cheap read-only command that reports current state, so the agent does
  not infer state from raw data.
- **Make every name in an instruction resolve in one step.** A skill, command, or document that an instruction names
  must be reachable from where the instruction loads.
- **Give one command family one default mode.** Do not let some commands write by default while others only check. State
  the default mode, the paths written, and the side effects in help text.

## Relationships

- A **Module** has exactly one **Interface** (the surface it presents to callers and tests).
- **Depth** is a property of a **Module**, measured against its **Interface**.
- A **Seam** is where a **Module**'s **Interface** lives.
- An **Adapter** sits at a **Seam** and satisfies the **Interface**.
- **Depth** produces **Leverage** for callers and **Locality** for maintainers.

## Rejected framings

- **Depth as ratio of implementation-lines to interface-lines** (Ousterhout): rewards padding the implementation. We use
  depth-as-leverage instead.
- **"Interface" as the TypeScript `interface` keyword or a class's public methods**: too narrow — interface here
  includes every fact a caller must know.
- **"Boundary"**: overloaded with DDD's bounded context. Say **seam** or **interface**.

## Output Contract

When applying this vocabulary to a design or review, report the recommended module, interface, and seam. In that design
or review, explain how the result improves depth, leverage, locality, or testability. Identify its material tradeoffs or
unresolved evidence. When this skill is only supporting another requested artifact, incorporate that analysis into the
artifact instead of adding a separate report.

## Going deeper

- **Deepening a cluster given its dependencies** — see [references/DEEPENING.md](references/DEEPENING.md): dependency
  categories, seam discipline, and replace-don't-layer testing.
- **Exploring alternative interfaces** — see [references/DESIGN-IT-TWICE.md](references/DESIGN-IT-TWICE.md): start
  parallel sub-agents to design the interface several radically different ways, then compare on depth, locality, and
  seam placement.
- **Making a codebase agent-legible** — see [references/AGENT-LEGIBILITY.md](references/AGENT-LEGIBILITY.md): feedback
  loops, mechanical architecture enforcement, dependency policy, and continuous garbage collection.

Forked from [mattpocock/skills](https://github.com/mattpocock/skills/tree/main/skills/engineering/codebase-design).
