# AI Handoffs

Local-only, read-only viewer for agent task handoffs.

## Invariants

- Bind application servers only to `127.0.0.1`.
- Never accept filesystem paths from HTTP clients.
- Keep discovery depth-bounded to the locations encoded in `src/server/scanner.ts`; do not replace it with recursive home-directory traversal.
- Treat missing roots and individual unreadable files as recoverable scan conditions.
- Do not mutate, move, archive, or delete discovered handoff files.
- Preserve the parser's legacy degradation behavior when evolving frontmatter.

## Workflow

- Use Bun 1.3.14 and the exact dependency pins in `package.json`.
- Prefer the `just` recipes for development, targeted tests, type-checking, builds, and cold-start serving.
- Keep server behavior behind testable parser, scanner, freshness, and request-handler seams.
- Run targeted tests while iterating; use `just check` only for aggregate validation.

