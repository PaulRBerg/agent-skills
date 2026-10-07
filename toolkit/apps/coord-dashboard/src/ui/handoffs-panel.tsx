import rehypeShiki from "@shikijs/rehype";
import { FileText, FolderGit2, RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import type { ComponentProps, ReactNode } from "react";
import { MarkdownHooks } from "react-markdown";
import remarkGfm from "remark-gfm";
import { tv } from "tailwind-variants";

import { displayPath } from "@/lib/format.js";
import type { HandoffCategory, HandoffRecord } from "@/lib/handoff-types.js";
import {
  categoryLabel,
  fetchHandoffs,
  formatHandoffDate,
  groupByProvenance,
  stripDuplicateTitle,
} from "@/lib/handoffs.js";
import type { HandoffGroup, HandoffState } from "@/lib/handoffs.js";

const filters: { id: HandoffState; label: string }[] = [
  { id: "live", label: "Live" },
  { id: "archived", label: "Archived" },
];

const categoryDot = tv({
  base: "size-1.75 shrink-0 rounded-full",
  variants: {
    category: {
      implementation: "bg-claude",
      investigation: "bg-codex",
      research: "bg-positive",
      audit: "bg-draft-ink",
      operations: "bg-queued-ink",
      none: "border border-muted",
    },
  },
});

const rehypePlugins: ComponentProps<typeof MarkdownHooks>["rehypePlugins"] = [
  [
    rehypeShiki,
    {
      defaultColor: "light",
      fallbackLanguage: "text",
      langs: ["bash", "javascript", "json", "markdown", "shellscript", "tsx", "typescript", "yaml"],
      onError: () => undefined,
      themes: { dark: "github-dark", light: "github-light" },
    },
  ],
];

function CategoryDot({ category }: { category: HandoffCategory | null }) {
  return <span aria-hidden="true" className={categoryDot({ category: category ?? "none" })} />;
}

function HandoffIndex({
  groups,
  onSelect,
  selectedPath,
}: {
  groups: HandoffGroup[];
  onSelect: (path: string) => void;
  selectedPath: string | null;
}) {
  if (groups.length === 0) {
    return <p className="p-4 text-xs/5 text-muted">No handoffs in this view.</p>;
  }

  return groups.map((group) => (
    <section key={`${group.root}\u0000${group.repository}`}>
      <h3
        className="sticky top-0 z-1 flex items-center gap-2 border-b border-line-muted bg-surface-muted px-3 py-2 text-xs"
        title={`${group.root}/${group.repository}`}
      >
        <FolderGit2 aria-hidden="true" className="size-3.5 shrink-0 text-muted" strokeWidth={1.8} />
        <span className="shrink-0 font-semibold text-ink">{group.repository}</span>
        <span className="min-w-0 truncate font-mono text-[11px] text-muted">
          {displayPath(group.root)}
        </span>
        <span className="ml-auto shrink-0 text-muted tabular-nums">{group.handoffs.length}</span>
      </h3>
      <ul>
        {group.handoffs.map((handoff) => {
          const selected = selectedPath === handoff.path;
          const category = categoryLabel(handoff.category);
          return (
            <li className="border-b border-line-muted last:border-b-0" key={handoff.path}>
              <button
                aria-current={selected ? "page" : undefined}
                className="grid min-h-11 w-full grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-2 px-3 py-2 text-left transition-colors hover:bg-surface-muted focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent aria-[current=page]:bg-accent-wash aria-[current=page]:shadow-[inset_2px_0_0_var(--color-accent)] motion-reduce:transition-none"
                onClick={() => onSelect(handoff.path)}
                title={`${category} · ${handoff.title}`}
                type="button"
              >
                <CategoryDot category={handoff.category} />
                <span className="truncate text-[13px] font-medium text-ink">
                  <span className="sr-only">{category}: </span>
                  {handoff.title}
                </span>
                <time
                  className="text-[11px] whitespace-nowrap text-muted tabular-nums"
                  dateTime={handoff.modifiedAt}
                >
                  {formatHandoffDate(handoff.modifiedAt)}
                </time>
              </button>
            </li>
          );
        })}
      </ul>
    </section>
  ));
}

function HandoffLocation({ handoff }: { handoff: HandoffRecord }) {
  const repositoryPrefix = `${handoff.root}/${handoff.repository}/`;
  if (!handoff.path.startsWith(repositoryPrefix)) {
    return <>{displayPath(handoff.path)}</>;
  }
  return (
    <>
      {displayPath(handoff.root)}/
      <strong className="font-semibold text-ink">{handoff.repository}</strong>/
      {handoff.path.slice(repositoryPrefix.length)}
    </>
  );
}

function HandoffArticle({ handoff }: { handoff: HandoffRecord }) {
  const recordedAt = handoff.created ?? handoff.modifiedAt;

  return (
    <article aria-labelledby="handoff-title" className="min-w-0">
      <header className="border-b border-line-muted pb-5">
        <p className="flex flex-wrap items-center gap-2 text-xs font-semibold">
          <span
            aria-hidden="true"
            className="size-1.75 rounded-full bg-positive data-[state=archived]:bg-muted"
            data-state={handoff.state}
          />
          {handoff.state === "live" ? "Live" : "Archived"}
          <span aria-hidden="true" className="text-muted">
            ·
          </span>
          <CategoryDot category={handoff.category} />
          {categoryLabel(handoff.category)}
        </p>
        <h2
          className="mt-2 text-xl/tight font-semibold tracking-tight text-balance wrap-break-word"
          id="handoff-title"
        >
          {handoff.title}
        </h2>
        <p className="mt-2 text-xs/5 text-muted">
          {handoff.created ? "Created" : "Modified"}{" "}
          <time dateTime={recordedAt}>{formatHandoffDate(recordedAt, true)}</time>
          <span aria-hidden="true"> · </span>
          <code className="font-mono break-all">{handoff.filename}</code>
          <span aria-hidden="true"> · </span>
          {handoff.format === "frontmatter" ? "Structured frontmatter" : "Legacy handoff"}
        </p>
        <p className="mt-1 font-mono text-xs/5 break-all text-muted" title={handoff.path}>
          <HandoffLocation handoff={handoff} />
        </p>
      </header>
      <div className="markdown-body pt-6">
        <MarkdownHooks
          fallback={<p className="text-sm text-muted">Preparing document…</p>}
          rehypePlugins={rehypePlugins}
          remarkPlugins={[remarkGfm]}
          skipHtml
        >
          {stripDuplicateTitle(handoff.markdown, handoff.title)}
        </MarkdownHooks>
      </div>
    </article>
  );
}

function StatusMessage({ children, title }: { children: ReactNode; title: string }) {
  return (
    <div className="rounded-2xl border border-line bg-surface px-4 py-12 text-center">
      <FileText aria-hidden="true" className="mx-auto size-6 text-muted" strokeWidth={1.5} />
      <h3 className="mt-3 text-sm font-semibold">{title}</h3>
      <div className="mx-auto mt-1 max-w-md text-xs/5 text-muted">{children}</div>
    </div>
  );
}

// Fetches on mount, so each visit to the tab rescans the handoff locations.
export function HandoffsPanel() {
  const [handoffs, setHandoffs] = useState<HandoffRecord[] | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [reloadSequence, setReloadSequence] = useState(0);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [filter, setFilter] = useState<HandoffState | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    async function load() {
      setLoadError(null);
      try {
        const next = await fetchHandoffs(controller.signal);
        setHandoffs(next);
        setSelectedPath((current) =>
          current && next.some((handoff) => handoff.path === current)
            ? current
            : ((next.find((handoff) => handoff.state === "live") ?? next[0])?.path ?? null)
        );
      } catch (error) {
        if (!controller.signal.aborted) {
          setLoadError(error instanceof Error ? error.message : "Unable to load handoffs");
        }
      }
    }
    load();
    return () => controller.abort();
  }, [reloadSequence]);

  const all = handoffs ?? [];
  const selected = all.find((handoff) => handoff.path === selectedPath) ?? null;
  // Until the reader picks a filter, follow the state of the selected record.
  const activeFilter = filter ?? selected?.state ?? "live";
  const counts: Record<HandoffState, number> = {
    live: all.filter((handoff) => handoff.state === "live").length,
    archived: all.filter((handoff) => handoff.state === "archived").length,
  };
  const groups = groupByProvenance(all.filter((handoff) => handoff.state === activeFilter));

  const selectHandoff = (path: string) => {
    setSelectedPath(path);
    // The reader sits below the index on narrow layouts.
    if (!window.matchMedia("(min-width: 64rem)").matches) {
      document.querySelector("#handoff-reader")?.scrollIntoView({ block: "start" });
    }
  };

  return (
    <section aria-labelledby="handoffs-heading" className="min-w-0">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h2 className="flex items-center gap-2 text-base font-semibold" id="handoffs-heading">
            <FileText aria-hidden="true" className="size-4 text-accent" strokeWidth={1.8} />
            Handoffs
          </h2>
          <p className="mt-1 text-xs/5 text-muted">
            Read-only task handoffs from ~/projects, ~/work, the Desktop, and the archive
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-1">
          <div aria-label="Handoff state" className="flex gap-1" role="group">
            {filters.map(({ id, label }) => (
              <button
                aria-pressed={activeFilter === id}
                className="flex min-h-9 items-center gap-2 rounded-lg border border-transparent px-2.5 py-1.5 text-xs font-medium text-muted transition-colors hover:bg-surface-muted hover:text-ink focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent aria-pressed:border-accent aria-pressed:bg-accent-wash aria-pressed:text-ink motion-reduce:transition-none"
                key={id}
                onClick={() => setFilter(id)}
                type="button"
              >
                {label}
                <span className="font-mono text-[11px] tabular-nums">{counts[id]}</span>
              </button>
            ))}
          </div>
          <button
            aria-label="Rescan handoffs"
            className="flex size-9 items-center justify-center rounded-lg text-muted transition-colors hover:bg-surface-muted hover:text-ink focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent motion-reduce:transition-none"
            onClick={() => setReloadSequence((sequence) => sequence + 1)}
            title="Rescan handoffs"
            type="button"
          >
            <RefreshCw aria-hidden="true" className="size-4" />
          </button>
        </div>
      </div>

      <div aria-live="polite" className="mt-5">
        {loadError ? (
          <StatusMessage title="Unable to load handoffs">
            <p className="font-mono">{loadError}</p>
            <button
              className="mt-3 min-h-9 px-2 font-medium text-accent hover:underline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
              onClick={() => setReloadSequence((sequence) => sequence + 1)}
              type="button"
            >
              Try again
            </button>
          </StatusMessage>
        ) : null}
        {!loadError && handoffs === null ? (
          <StatusMessage title="Reading handoffs">Scanning the watched locations…</StatusMessage>
        ) : null}
        {!loadError && handoffs?.length === 0 ? (
          <StatusMessage title="No handoffs found">
            The watched locations are empty. This view never creates or changes handoffs.
          </StatusMessage>
        ) : null}
        {!loadError && handoffs && handoffs.length > 0 ? (
          <div className="grid items-start gap-6 lg:grid-cols-[20rem_minmax(0,1fr)]">
            <nav
              aria-label="Handoff index"
              className="max-h-80 overflow-y-auto overscroll-contain rounded-2xl border border-line bg-surface shadow-panel lg:sticky lg:top-4 lg:max-h-[calc(100dvh-2rem)]"
            >
              <HandoffIndex groups={groups} onSelect={selectHandoff} selectedPath={selectedPath} />
            </nav>
            <div
              className="min-w-0 scroll-mt-4 rounded-2xl border border-line bg-surface p-5 shadow-panel sm:p-8"
              id="handoff-reader"
            >
              {selected ? (
                <HandoffArticle handoff={selected} />
              ) : (
                <p className="text-sm text-muted">Choose a handoff to read its brief.</p>
              )}
            </div>
          </div>
        ) : null}
      </div>
    </section>
  );
}
