# Dashboard package

`apps/coord-dashboard/` is the local live view of ai-coord coordination state.

## Stack

Use Bun, Vite, React 19, and strict TypeScript. Styling is Tailwind v4 through `@tailwindcss/vite`, with tokens in
`@theme inline`; UI primitives are Base UI, icons are Lucide, and variants use `tailwind-variants`. Tests use Vitest.
There is no router, state library, or React Compiler.

## Conventions

Use kebab-case filenames and the `@` alias for `src`. Use system fonts only. Keep the dashboard local-only: no CDNs,
analytics, or external requests. Support light and dark themes through `prefers-color-scheme`, respect
`prefers-reduced-motion`, and keep desktop-first layouts usable at about 390px wide.

## Data contract

Read snapshots from `GET /api/snapshot` and live updates from `GET /api/events`, supplied by `ai-coord serve`.
`src/lib/sample-snapshot.ts` mirrors that snapshot contract. Use SSE when available and polling as its fallback.

## Verification

Run `bun install --frozen-lockfile` and `bun run check`. Final visual proof for UI changes includes rendered inspection
in both light and dark themes.
