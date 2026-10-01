import { Drawer } from "@base-ui/react/drawer";
import { Tabs } from "@base-ui/react/tabs";
import rehypeShiki from "@shikijs/rehype";
import { FileText, FolderGit2, Menu, RefreshCw, X } from "lucide-react";
import { StrictMode, useCallback, useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import { MarkdownHooks } from "react-markdown";
import remarkGfm from "remark-gfm";
import type { PluggableList } from "unified";

import type { HandoffCategory, HandoffRecord, HandoffsResponse } from "../shared/handoff";
import "./styles.css";

type LoadState = "loading" | "ready" | "error";
type HandoffState = HandoffRecord["state"];

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

const monthNames = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

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

/** Formats a timestamp as `21 Sep`, adding the year when it differs from the current one or when forced. */
function formatDate(value: string, withYear = false): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;

  const dayMonth = `${date.getDate()} ${monthNames[date.getMonth()]}`;
  return withYear || date.getFullYear() !== new Date().getFullYear() ? `${dayMonth} ${date.getFullYear()}` : dayMonth;
}

function categoryLabel(category: HandoffCategory | null): string {
  return category ? categoryNames[category] : "Uncategorized";
}

/** Drops a leading `# Title` line that repeats the record title already shown in the reader header. */
function stripDuplicateTitle(markdown: string, title: string): string {
  const lines = markdown.split("\n");
  const index = lines.findIndex((line) => line.trim() !== "");
  const heading = /^#\s+(.+)$/.exec(lines[index]?.trimEnd() ?? "")?.[1];
  if (heading?.trim() !== title.trim()) return markdown;

  const next = lines[index + 1]?.trim() === "" ? index + 2 : index + 1;
  return lines.slice(next).join("\n");
}

function SidebarBrand() {
  return (
    <div className="sidebar-brand">
      <h1>AI Handoffs</h1>
    </div>
  );
}

function HandoffIndex({
  archived,
  live,
  onSelect,
  onTabChange,
  selectedPath,
  tab,
}: {
  archived: HandoffGroup[];
  live: HandoffGroup[];
  onSelect: (path: string) => void;
  onTabChange: (tab: HandoffState) => void;
  selectedPath: string | null;
  tab: HandoffState;
}) {
  const liveCount = live.reduce((total, group) => total + group.handoffs.length, 0);
  const archivedCount = archived.reduce((total, group) => total + group.handoffs.length, 0);

  return (
    <nav aria-label="Handoff index" className="handoff-index">
      <Tabs.Root className="index-tabs" onValueChange={(value) => onTabChange(value as HandoffState)} value={tab}>
        <Tabs.List className="tab-list">
          <Tabs.Tab aria-label={`Live, ${liveCount} handoffs`} className="tab" value="live">
            Live <span className="tab-count">{liveCount}</span>
          </Tabs.Tab>
          <Tabs.Tab aria-label={`Archived, ${archivedCount} handoffs`} className="tab" value="archived">
            Archived <span className="tab-count">{archivedCount}</span>
          </Tabs.Tab>
        </Tabs.List>
        <Tabs.Panel className="tab-panel" value="live">
          <GroupList empty="No live handoffs." groups={live} onSelect={onSelect} selectedPath={selectedPath} />
        </Tabs.Panel>
        <Tabs.Panel className="tab-panel" value="archived">
          <GroupList empty="No archived handoffs." groups={archived} onSelect={onSelect} selectedPath={selectedPath} />
        </Tabs.Panel>
      </Tabs.Root>
    </nav>
  );
}

function GroupList({
  empty,
  groups,
  onSelect,
  selectedPath,
}: {
  empty: string;
  groups: HandoffGroup[];
  onSelect: (path: string) => void;
  selectedPath: string | null;
}) {
  if (groups.length === 0) return <p className="index-empty">{empty}</p>;

  return groups.map((group) => (
    <section className="repository-group" key={`${group.root}\u0000${group.repository}`}>
      <header className="repository-header" title={`${group.root}/${group.repository}`}>
        <FolderGit2 aria-hidden="true" size={14} strokeWidth={1.8} />
        <h2 className="repository-path">
          <span className="repository-name">{group.repository}</span>
          <span className="repository-root">{friendlyRoot(group.root)}</span>
        </h2>
        <span
          aria-label={`${group.handoffs.length} handoff${group.handoffs.length === 1 ? "" : "s"}`}
          className="repository-count"
        >
          {group.handoffs.length}
        </span>
      </header>
      <ul>
        {group.handoffs.map((handoff) => {
          const selected = selectedPath === handoff.path;
          const category = categoryLabel(handoff.category);
          return (
            <li key={handoff.path}>
              <button
                aria-current={selected ? "page" : undefined}
                className="handoff-row"
                data-selected={selected || undefined}
                onClick={() => onSelect(handoff.path)}
                title={`${category} · ${handoff.title}`}
                type="button"
              >
                <span aria-hidden="true" className={`row-dot category-${handoff.category ?? "none"}`} />
                <span className="row-title">
                  <span className="sr-only">{category}: </span>
                  {handoff.title}
                </span>
                <time className="row-date" dateTime={handoff.modifiedAt}>
                  {formatDate(handoff.modifiedAt)}
                </time>
              </button>
            </li>
          );
        })}
      </ul>
    </section>
  ));
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

function HandoffLocation({ handoff }: { handoff: HandoffRecord }) {
  const remainder = handoff.path.startsWith(handoff.root) ? handoff.path.slice(handoff.root.length) : null;
  if (remainder === null) return <p className="document-location">{handoff.path}</p>;

  const repositoryPrefix = `/${handoff.repository}`;
  const hasRepository = remainder.startsWith(`${repositoryPrefix}/`);

  return (
    <p className="document-location" title={handoff.path}>
      {friendlyRoot(handoff.root)}
      {hasRepository ? (
        <>
          /<strong>{handoff.repository}</strong>
          {remainder.slice(repositoryPrefix.length)}
        </>
      ) : (
        remainder
      )}
    </p>
  );
}

function HandoffArticle({ handoff }: { handoff: HandoffRecord }) {
  const recordedAt = handoff.created ?? handoff.modifiedAt;
  const markdown = useMemo(() => stripDuplicateTitle(handoff.markdown, handoff.title), [handoff.markdown, handoff.title]);

  return (
    <article aria-labelledby="handoff-title" className="document">
      <header className="document-header">
        <p className="document-state">
          <span className="state-dot" data-state={handoff.state} aria-hidden="true" />
          <span>{handoff.state === "live" ? "Live" : "Archived"}</span>
          <span aria-hidden="true" className="separator">·</span>
          <span className={`category-${handoff.category ?? "none"}`}>{categoryLabel(handoff.category)}</span>
        </p>
        <h2 id="handoff-title">{handoff.title}</h2>
        <p className="document-meta">
          {handoff.created ? "Created" : "Modified"} <time dateTime={recordedAt}>{formatDate(recordedAt, true)}</time>
          <span aria-hidden="true"> · </span>
          <code>{handoff.filename}</code>
          <span aria-hidden="true"> · </span>
          {handoff.format === "frontmatter" ? "Structured frontmatter" : "Legacy handoff"}
        </p>
        <HandoffLocation handoff={handoff} />
      </header>
      <MarkdownDocument markdown={markdown} />
    </article>
  );
}

function StatusState({ children, role, title }: { children: React.ReactNode; role?: "alert" | "status"; title: string }) {
  return (
    <div className="status-state" data-error={role === "alert" || undefined} role={role}>
      <FileText aria-hidden="true" size={22} />
      <h2>{title}</h2>
      {children}
    </div>
  );
}

function App() {
  const [handoffs, setHandoffs] = useState<HandoffRecord[]>([]);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [tab, setTab] = useState<HandoffState | null>(null);
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
  // Until the reader picks a tab, follow the state of the selected record.
  const activeTab = tab ?? selected?.state ?? "live";
  const liveGroups = useMemo(() => groupByProvenance(handoffs.filter((handoff) => handoff.state === "live")), [handoffs]);
  const archivedGroups = useMemo(
    () => groupByProvenance(handoffs.filter((handoff) => handoff.state === "archived")),
    [handoffs],
  );

  const renderIndex = (onSelect: (path: string) => void) =>
    loadState === "ready" ? (
      <HandoffIndex
        archived={archivedGroups}
        live={liveGroups}
        onSelect={onSelect}
        onTabChange={setTab}
        selectedPath={selectedPath}
        tab={activeTab}
      />
    ) : null;

  return (
    <main className="app-shell">
      <aside className="desktop-sidebar">
        <SidebarBrand />
        {renderIndex(setSelectedPath)}
      </aside>

      <div className="reader-shell">
        <header className="mobile-bar">
          <Drawer.Root onOpenChange={setDrawerOpen} open={drawerOpen} swipeDirection="left">
            <Drawer.Trigger aria-label="Open handoff index" className="drawer-trigger" title="Open handoff index">
              <Menu aria-hidden="true" size={18} />
            </Drawer.Trigger>
            <span className="mobile-title">{selected?.title ?? "AI Handoffs"}</span>
            <Drawer.Portal>
              <Drawer.Backdrop className="drawer-backdrop" />
              <Drawer.Viewport className="drawer-viewport">
                <Drawer.Popup aria-label="Handoff index" className="drawer-popup">
                  <Drawer.Content className="drawer-content">
                    <div className="drawer-header">
                      <SidebarBrand />
                      <Drawer.Close aria-label="Close handoff index" className="drawer-close" title="Close handoff index">
                        <X aria-hidden="true" size={18} />
                      </Drawer.Close>
                    </div>
                    {renderIndex((path) => {
                      setSelectedPath(path);
                      setDrawerOpen(false);
                    })}
                  </Drawer.Content>
                </Drawer.Popup>
              </Drawer.Viewport>
            </Drawer.Portal>
          </Drawer.Root>
        </header>

        <section aria-live="polite" className="reader-content">
          {loadState === "loading" ? (
            <StatusState role="status" title="Opening the index">
              <p>Reading the available handoff records.</p>
            </StatusState>
          ) : null}
          {loadState === "error" ? (
            <StatusState role="alert" title="Unable to reach the handoff index">
              <p>{error}</p>
              <button className="retry-button" onClick={() => void loadHandoffs()} type="button">
                <RefreshCw aria-hidden="true" size={14} />
                Try again
              </button>
            </StatusState>
          ) : null}
          {loadState === "ready" && handoffs.length === 0 ? (
            <StatusState title="No handoffs found">
              <p>The watched locations are empty. This viewer does not create or change handoffs.</p>
            </StatusState>
          ) : null}
          {loadState === "ready" && handoffs.length > 0 && !selected ? (
            <StatusState title="Select a handoff">
              <p>Choose a record from the index to read its complete brief.</p>
            </StatusState>
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
