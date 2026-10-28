import { Tabs } from "@base-ui/react/tabs";
import { ClipboardCheck, FileText, MessageSquareText, PanelsTopLeft } from "lucide-react";
import { AnimatePresence, LayoutGroup, MotionConfig } from "motion/react";
import { lazy, Suspense, useEffect, useRef, useState } from "react";
import { ScrollRestoration, useLocation, useNavigate } from "react-router";
import { tv } from "tailwind-variants";

import { subscribeToSnapshots } from "@/lib/api.js";
import type { ConnectionState } from "@/lib/api.js";
import { countFindings } from "@/lib/findings.js";
import { groupSnapshotByRepo } from "@/lib/group.js";
import type { Snapshot } from "@/lib/types.js";
import { ApiErrorState, EmptySessions } from "@/ui/empty-error-states.js";
import { FindingsPanel } from "@/ui/findings-panel.js";
import { Header } from "@/ui/header.js";
import { MessagesFeed } from "@/ui/messages-feed.js";
import { MessagesPreview } from "@/ui/messages-preview.js";
import { RepoLane } from "@/ui/repo-lane.js";

// Loaded on demand: Markdown rendering and syntax highlighting dominate the bundle.
const HandoffsPanel = lazy(async () => {
  const module = await import("@/ui/handoffs-panel.js");
  return { default: module.HandoffsPanel };
});

const tabs = ["coordination", "findings", "messages", "handoffs"] as const;
type Tab = (typeof tabs)[number];
const navigationTab = tv({
  base: "flex min-h-12 shrink-0 cursor-pointer items-center gap-1.5 border-b-2 border-transparent px-1 py-3 text-[12px] font-medium whitespace-nowrap text-muted transition-colors hover:text-ink focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent data-active:border-accent data-active:text-accent motion-reduce:transition-none sm:gap-2 sm:px-5 sm:text-[13px]",
});

function tabFromPath(pathname: string): Tab {
  const segment = pathname.split("/")[1];
  return tabs.find((tab) => tab !== "coordination" && tab === segment) ?? "coordination";
}

export function App() {
  const { pathname } = useLocation();
  const navigate = useNavigate();
  const activeTab = tabFromPath(pathname);
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [connection, setConnection] = useState<ConnectionState>("connecting");
  const [error, setError] = useState<Error | null>(null);
  const [lastUpdated, setLastUpdated] = useState<number | null>(null);
  const [refreshSequence, setRefreshSequence] = useState(0);
  const [now, setNow] = useState(() => Date.now() / 1000);
  const messagesTabRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now() / 1000), 1000);
    return () => window.clearInterval(timer);
  }, []);

  useEffect(
    () =>
      subscribeToSnapshots({
        onSnapshot(nextSnapshot) {
          setSnapshot(nextSnapshot);
          setLastUpdated(Date.now() / 1000);
          setRefreshSequence((sequence) => sequence + 1);
          setError(null);
        },
        onConnectionChange: setConnection,
        onError: setError,
      }),
    []
  );

  const lanes = snapshot ? groupSnapshotByRepo(snapshot) : [];
  const unresolvedCount = snapshot ? countFindings(snapshot.findings).unresolved : 0;

  return (
    <MotionConfig reducedMotion="user">
      <ScrollRestoration />
      <div className="min-h-dvh bg-canvas text-ink">
        <Header
          connection={connection}
          lanes={lanes}
          lastUpdated={lastUpdated}
          now={now}
          refreshSequence={refreshSequence}
          snapshot={snapshot}
        />

        <main className="mx-auto max-w-[1520px] px-4 pt-2 pb-8 sm:px-6 lg:px-8">
          {connection === "disconnected" && error ? (
            <div className="pb-5">
              <ApiErrorState compact={snapshot !== null} detail={error.message} />
            </div>
          ) : null}

          <Tabs.Root
            onValueChange={(tab: Tab) => navigate(tab === "coordination" ? "/" : `/${tab}`)}
            value={activeTab}
          >
            <Tabs.List
              activateOnFocus
              aria-label="Dashboard views"
              className="mb-7 flex w-full gap-0.5 overflow-x-auto border-b border-line sm:gap-2"
            >
              <Tabs.Tab className={navigationTab()} value="coordination">
                <PanelsTopLeft aria-hidden="true" className="hidden size-4 sm:block" />
                Coordination
              </Tabs.Tab>
              <Tabs.Tab className={navigationTab()} ref={messagesTabRef} value="messages">
                <MessageSquareText aria-hidden="true" className="hidden size-4 sm:block" />
                Messages
              </Tabs.Tab>
              <Tabs.Tab className={navigationTab()} value="findings">
                <ClipboardCheck aria-hidden="true" className="hidden size-4 sm:block" />
                Findings
                <span
                  aria-label={`${unresolvedCount} unresolved findings`}
                  className="min-w-5 rounded-sm bg-surface-muted px-1.5 py-0.5 text-center text-[11px]/4 text-ink-secondary tabular-nums"
                  title={`${unresolvedCount} unresolved findings (pending or handed off)`}
                >
                  {unresolvedCount}
                </span>
              </Tabs.Tab>
              <Tabs.Tab className={navigationTab()} value="handoffs">
                <FileText aria-hidden="true" className="hidden size-4 sm:block" />
                Handoffs
              </Tabs.Tab>
            </Tabs.List>
            {snapshot === null && connection !== "disconnected" && activeTab !== "handoffs" ? (
              <div className="rounded-md border border-line bg-surface px-4 py-12 text-center text-sm text-muted">
                Loading coordination snapshot…
              </div>
            ) : null}
            {snapshot ? (
              <>
                <Tabs.Panel
                  keepMounted
                  value="coordination"
                  hidden={activeTab !== "coordination"}
                  className="focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-accent"
                >
                  <div className="mb-5">
                    <h2 className="flex items-center gap-2 text-xl font-semibold tracking-tight">
                      <PanelsTopLeft
                        aria-hidden="true"
                        className="size-4 shrink-0 text-accent"
                        strokeWidth={1.8}
                      />
                      Coordination
                    </h2>
                    <p className="mt-1 text-sm/5 text-muted">
                      {snapshot.sessions.length} session
                      {snapshot.sessions.length === 1 ? "" : "s"} across {lanes.length} repositor
                      {lanes.length === 1 ? "y" : "ies"}
                    </p>
                  </div>
                  <div className="grid items-start gap-7 xl:grid-cols-[minmax(0,1fr)_20rem]">
                    <div className="min-w-0">
                      {snapshot.sessions.length === 0 || lanes.length === 0 ? (
                        <EmptySessions />
                      ) : null}
                      {lanes.length > 0 ? (
                        <div className="flex flex-col gap-7">
                          <LayoutGroup id="coordination-lanes">
                            <AnimatePresence initial={false} mode="popLayout">
                              {lanes.map((lane) => (
                                <RepoLane key={lane.repoRoot} lane={lane} now={now} />
                              ))}
                            </AnimatePresence>
                          </LayoutGroup>
                        </div>
                      ) : null}
                    </div>

                    <aside className="min-w-0 xl:sticky xl:top-6">
                      <MessagesPreview
                        messages={snapshot.messages}
                        now={now}
                        // activateOnFocus makes focusing the tab also select it and update the URL.
                        onOpenMessages={() => messagesTabRef.current?.focus()}
                      />
                    </aside>
                  </div>
                </Tabs.Panel>
                <Tabs.Panel
                  keepMounted
                  value="findings"
                  hidden={activeTab !== "findings"}
                  className="focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-accent"
                >
                  <FindingsPanel findings={snapshot.findings} now={now} />
                </Tabs.Panel>
                <Tabs.Panel
                  keepMounted
                  value="messages"
                  hidden={activeTab !== "messages"}
                  className="focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-accent"
                >
                  <MessagesFeed messages={snapshot.messages} now={now} />
                </Tabs.Panel>
              </>
            ) : null}
            <Tabs.Panel
              value="handoffs"
              hidden={activeTab !== "handoffs"}
              className="focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-accent"
            >
              <Suspense fallback={null}>
                <HandoffsPanel />
              </Suspense>
            </Tabs.Panel>
          </Tabs.Root>
        </main>
      </div>
    </MotionConfig>
  );
}
