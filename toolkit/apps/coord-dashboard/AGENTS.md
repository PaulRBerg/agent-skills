# Dashboard package

`apps/coord-dashboard/` is the local live view of ai-coord coordination state. It also contains the read-only task
handoff viewer.

## Stack

Use Bun, Vite, React 19 with React Compiler, React Router, and strict TypeScript. Keep the router error boundary and its
reload action. Effect 4 Schema validates incoming snapshots. Keep pure helpers and existing Promise/SSE boundaries
outside Effect unless a concrete requirement calls for a runtime change.

Styling uses Tailwind v4 through `@tailwindcss/vite`, with tokens in `@theme inline`. UI primitives use Base UI, icons
use Lucide, and variants use `tailwind-variants`. Tests use Vitest in Node. There is no separate state library.

## Conventions

Use kebab-case filenames, type aliases, and explicit `.js` extensions on local TypeScript imports. The `@` alias
resolves to `src`. Let React Compiler handle ordinary memoization. Write complete static Tailwind class strings. Use
`tailwind-variants` for variants and overrides.

Use system fonts only. Keep the dashboard local-only: no CDNs, analytics, or external requests. Omit React Grab because
its bundled telemetry and external fonts conflict with this constraint. Support light and dark themes through
`prefers-color-scheme`, respect `prefers-reduced-motion`, and keep layouts usable at about 390px wide.

Keep `src/start.ts`, `src/freshness.ts`, and `src/server/` out of the browser module graph. Development and preview bind
to `127.0.0.1:5173` with `strictPort`. The Bun production server defaults to port 4173. Reject requests whose Host is
not a loopback name (`localhost`, `127.0.0.1`, `[::1]`) as a DNS-rebinding guard.

Open the dashboard at `https://coord.localhost` and the handoffs view at `https://coord.localhost/handoffs`. Circadian's
Caddy configuration proxies this URL to `127.0.0.1:4173` and rewrites the upstream Host header to preserve the loopback
guard.

Oxlint/Oxfmt owns code. ESLint owns Tailwind classes and React hooks. Prettier owns Markdown and YAML. The package-local
lint-staged configuration uses the repository's existing Husky hook. Do not install a separate package hook.
`@typescript/native` supplies the TypeScript 7 compiler. The `typescript` alias supplies the TypeScript 6 API for
ESLint.

## Data contract

Read snapshots from `GET /api/snapshot` and live updates from `GET /api/events`, supplied by `ai-coord serve`.
`src/lib/snapshot-schema.ts` validates that contract. Preserve schema version 8, additive fields, input identity, and
cross-field claim and draft invariants. `src/lib/sample-snapshot.ts` mirrors the contract. Use SSE when available and
polling as its fallback.

The URL path selects the tab: `/`, `/findings`, `/messages`, or `/handoffs`. Keep the snapshot subscription above the
Coordination, Findings, and Messages tabs. The Findings badge and default Unresolved filter include pending and
handed-off findings. Triaging is an independent overlay, not another finding state.

Render full message history directly in the Messages tab with 25 messages per page. Keep search and filters in the tab.

## Handoffs

The Bun server answers `GET /api/handoffs` from `src/server/handoffs/` and proxies all other `/api` routes to
`ai-coord serve`. In development, Vite proxies `/api/handoffs` to the always-on server on port 4173.

- Never accept filesystem paths from HTTP clients.
- Keep discovery depth-bounded to the locations in `src/server/handoffs/scanner.ts`. Do not replace it with recursive
  home-directory traversal.
- Scan the Desktop in an isolated worker process with a timeout.
- Treat missing roots and individual unreadable files as recoverable scan conditions.
- Do not mutate, move, archive, or delete discovered handoff files.
- Preserve the parser's legacy degradation behavior when evolving frontmatter.

## Verification

Run `bun install --frozen-lockfile` and `just check`. The gate includes Oxlint/Oxfmt, ESLint, Markdown/YAML formatting,
types, tests, and the production build. Use path arguments with `just ox-check`, `just eslint-check`, and `just test`
during iteration. Final visual proof for UI changes includes rendered inspection in both light and dark themes.
