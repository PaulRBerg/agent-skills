import { Drawer } from "@base-ui/react/drawer";
import rehypeShiki from "@shikijs/rehype";
import { Archive, FileText, FolderOpen, Menu, RefreshCw, X } from "lucide-react";
import { StrictMode, useCallback, useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import { MarkdownHooks } from "react-markdown";
import remarkGfm from "remark-gfm";
import type { PluggableList } from "unified";

import type { HandoffCategory, HandoffRecord, HandoffsResponse } from "../shared/handoff";
import "./styles.css";

type LoadState = "loading" | "ready" | "error";

interface HandoffGroup {
  root: string;
  repository: string;
  handoffs: HandoffRecord[];
}

const categoryNames: Record<HandoffCategory, string> = {
  implementation: "Implementation",
  investigation: "Investigation",
  research: "Research",
  audit: "Audit",
  operations: "Operations",
};

const markdownRehypePlugins: PluggableList = [
  [
    rehypeShiki,
    {
      fallbackLanguage: "text",
      langs: ["bash", "javascript", "json", "markdown", "shellscript", "tsx", "typescript", "yaml"],
      onError: () => undefined,
      theme: "github-dark",
    },
  ],
];

function groupByProvenance(handoffs: HandoffRecord[]): HandoffGroup[] {
  const groups = new Map<string, HandoffGroup>();

  for (const handoff of handoffs) {
    const key = `${handoff.root}\u0000${handoff.repository}`;
    const group = groups.get(key);
    if (group) {
      group.handoffs.push(handoff);
    } else {
      groups.set(key, { handoffs: [handoff], repository: handoff.repository, root: handoff.root });
    }
  }

  return [...groups.values()];
}

function friendlyRoot(root: string): string {
  return root.replace(/^\/Users\/[^/]+/, "~");
}

function formatMoment(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;

  return new Intl.DateTimeFormat(undefined, {
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    month: "short",
    year: "numeric",
  }).format(date);
}

function categoryLabel(category: HandoffCategory | null): string {
  return category ? categoryNames[category] : "Uncategorized";
}

function HandoffNavigation({
  archived,
  live,
  onSelect,
  selectedPath,
}: {
  archived: HandoffGroup[];
  live: HandoffGroup[];
  onSelect: (path: string) => void;
  selectedPath: string | null;
}) {
  return (
    <nav aria-label="Handoff index" className="handoff-index">
      <IndexSection groups={live} icon={<FolderOpen aria-hidden="true" size={15} />} label="Live" onSelect={onSelect} selectedPath={selectedPath} />
      <IndexSection
        archived
        groups={archived}
        icon={<Archive aria-hidden="true" size={15} />}
        label="Archived"
        onSelect={onSelect}
        selectedPath={selectedPath}
      />
    </nav>
  );
}

function IndexSection({
  archived = false,
  groups,
  icon,
  label,
  onSelect,
  selectedPath,
}: {
  archived?: boolean;
  groups: HandoffGroup[];
  icon: React.ReactNode;
  label: string;
  onSelect: (path: string) => void;
  selectedPath: string | null;
}) {
  const count = groups.reduce((total, group) => total + group.handoffs.length, 0);

  return (
    <section className="index-section" data-archived={archived || undefined}>
      <h2 className="index-heading">
        {icon}
        <span>{label}</span>
        <span className="index-count" aria-label={`${count} ${label.toLowerCase()} handoffs`}>
          {count}
        </span>
      </h2>
      {groups.length === 0 ? (
        <p className="index-empty">No {label.toLowerCase()} handoffs.</p>
      ) : (
        groups.map((group) => (
          <section className="repository-group" key={`${group.root}\u0000${group.repository}`}>
            <h3>
              <span>{group.repository}</span>
              <span className="repository-count">{group.handoffs.length}</span>
            </h3>
            <p className="repository-root" title={group.root}>
              {friendlyRoot(group.root)}
            </p>
            <ul>
              {group.handoffs.map((handoff) => (
                <li key={handoff.path}>
                  <button
                    aria-current={selectedPath === handoff.path ? "page" : undefined}
                    className="handoff-link"
                    data-selected={selectedPath === handoff.path || undefined}
                    onClick={() => onSelect(handoff.path)}
                    type="button"
                  >
                    <span className="handoff-link-title">{handoff.title}</span>
                    <span className="handoff-link-meta">
                      {handoff.category ? categoryLabel(handoff.category) : handoff.filename}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          </section>
        ))
      )}
    </section>
  );
}

function SidebarBrand() {
  return (
    <div className="sidebar-brand">
      <p className="eyebrow">Local dispatch</p>
      <h1>AI Handoffs</h1>
      <p>Read-only task records, arranged by their point of origin.</p>
    </div>
  );
}

function MarkdownDocument({ markdown }: { markdown: string }) {
  return (
    <div className="markdown-body">
      <MarkdownHooks fallback={<p className="markdown-loading">Preparing document…</p>} rehypePlugins={markdownRehypePlugins} remarkPlugins={[remarkGfm]} skipHtml>
        {markdown}
      </MarkdownHooks>
    </div>
  );
}

function HandoffArticle({ handoff }: { handoff: HandoffRecord }) {
  const recordedAt = handoff.created ?? handoff.modifiedAt;
  const recordedLabel = handoff.created ? "Created" : "Modified";

  return (
    <article aria-labelledby="handoff-title" className="document-sheet">
      <header className="document-header">
        <div className="document-kicker">
          <span className={`category-mark category-${handoff.category ?? "uncategorized"}`}>{categoryLabel(handoff.category)}</span>
          <span className="record-state">{handoff.state}</span>
        </div>
        <h2 id="handoff-title">{handoff.title}</h2>
        <p className="document-summary">
          {recordedLabel} <time dateTime={recordedAt}>{formatMoment(recordedAt)}</time>
        </p>
      </header>

      <div className="document-layout">
        <MarkdownDocument markdown={handoff.markdown} />
        <aside aria-label="Handoff provenance" className="provenance-rail">
          <p className="eyebrow">Provenance</p>
          <dl>
            <div>
              <dt>Filename</dt>
              <dd>{handoff.filename}</dd>
            </div>
            <div>
              <dt>Repository</dt>
              <dd>{handoff.repository}</dd>
            </div>
            <div>
              <dt>Root</dt>
              <dd className="path-value" title={handoff.root}>{friendlyRoot(handoff.root)}</dd>
            </div>
            <div>
              <dt>Location</dt>
              <dd className="path-value" title={handoff.path}>{handoff.path}</dd>
            </div>
            <div>
              <dt>Record</dt>
              <dd>{handoff.format === "frontmatter" ? "Structured frontmatter" : "Legacy handoff"}</dd>
            </div>
          </dl>
        </aside>
      </div>
    </article>
  );
}

function App() {
  const [handoffs, setHandoffs] = useState<HandoffRecord[]>([]);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [loadState, setLoadState] = useState<LoadState>("loading");
  const [error, setError] = useState<string | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);

  const loadHandoffs = useCallback(async (signal?: AbortSignal) => {
    setLoadState("loading");
    setError(null);

    try {
      const response = await fetch("/api/handoffs", { cache: "no-store", signal });
      if (!response.ok) throw new Error(`Request failed (${response.status})`);
      const payload = (await response.json()) as HandoffsResponse;
      setHandoffs(payload.handoffs);
      setSelectedPath((current) => {
        if (current && payload.handoffs.some((handoff) => handoff.path === current)) return current;
        return (
          payload.handoffs.find((handoff) => handoff.state === "live")?.path ??
          payload.handoffs[0]?.path ??
          null
        );
      });
      setLoadState("ready");
    } catch (reason: unknown) {
      if (reason instanceof DOMException && reason.name === "AbortError") return;
      setError(reason instanceof Error ? reason.message : "Unable to load handoffs");
      setLoadState("error");
    }
  }, []);

  useEffect(() => {
    const controller = new AbortController();
    void loadHandoffs(controller.signal);
    return () => controller.abort();
  }, [loadHandoffs]);

  const selected = handoffs.find((handoff) => handoff.path === selectedPath) ?? null;
  const liveGroups = useMemo(() => groupByProvenance(handoffs.filter((handoff) => handoff.state === "live")), [handoffs]);
  const archivedGroups = useMemo(
    () => groupByProvenance(handoffs.filter((handoff) => handoff.state === "archived")),
    [handoffs],
  );
  const selectHandoff = (path: string) => setSelectedPath(path);

  return (
    <main className="app-shell">
      <aside className="desktop-sidebar">
        <SidebarBrand />
        {loadState === "ready" ? (
          <HandoffNavigation archived={archivedGroups} live={liveGroups} onSelect={selectHandoff} selectedPath={selectedPath} />
        ) : null}
      </aside>

      <div className="reader-shell">
        <header className="mobile-bar">
          <Drawer.Root onOpenChange={setDrawerOpen} open={drawerOpen} swipeDirection="left">
            <Drawer.Trigger aria-label="Open handoff index" className="drawer-trigger" title="Open handoff index">
              <Menu aria-hidden="true" size={20} />
            </Drawer.Trigger>
            <span className="mobile-title">AI Handoffs</span>
            <Drawer.Portal>
              <Drawer.Backdrop className="drawer-backdrop" />
              <Drawer.Viewport className="drawer-viewport">
                <Drawer.Popup aria-label="Handoff index" className="drawer-popup">
                  <Drawer.Content>
                    <div className="drawer-header">
                      <SidebarBrand />
                      <Drawer.Close aria-label="Close handoff index" className="drawer-close" title="Close handoff index">
                        <X aria-hidden="true" size={20} />
                      </Drawer.Close>
                    </div>
                    {loadState === "ready" ? (
                      <HandoffNavigation
                        archived={archivedGroups}
                        live={liveGroups}
                        onSelect={(path) => {
                          selectHandoff(path);
                          setDrawerOpen(false);
                        }}
                        selectedPath={selectedPath}
                      />
                    ) : null}
                  </Drawer.Content>
                </Drawer.Popup>
              </Drawer.Viewport>
            </Drawer.Portal>
          </Drawer.Root>
        </header>

        <section aria-live="polite" className="reader-content">
          {loadState === "loading" ? (
            <div className="status-state" role="status">
              <FileText aria-hidden="true" size={28} />
              <h2>Opening the index</h2>
              <p>Reading the available handoff records.</p>
            </div>
          ) : null}
          {loadState === "error" ? (
            <div className="status-state status-error" role="alert">
              <FileText aria-hidden="true" size={28} />
              <h2>Unable to reach the handoff index</h2>
              <p>{error}</p>
              <button className="retry-button" onClick={() => void loadHandoffs()} type="button">
                <RefreshCw aria-hidden="true" size={16} />
                Try again
              </button>
            </div>
          ) : null}
          {loadState === "ready" && handoffs.length === 0 ? (
            <div className="status-state">
              <FileText aria-hidden="true" size={28} />
              <h2>No handoffs found</h2>
              <p>The watched locations are empty. This viewer does not create or change handoffs.</p>
            </div>
          ) : null}
          {loadState === "ready" && handoffs.length > 0 && !selected ? (
            <div className="status-state">
              <FileText aria-hidden="true" size={28} />
              <h2>Select a handoff</h2>
              <p>Choose a record from the index to read its complete brief.</p>
            </div>
          ) : null}
          {loadState === "ready" && selected ? <HandoffArticle handoff={selected} /> : null}
        </section>
      </div>
    </main>
  );
}

const root = document.getElementById("root");
if (!root) throw new Error("Missing #root element");
createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
