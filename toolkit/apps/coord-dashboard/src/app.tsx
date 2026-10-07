import { Tabs } from "@base-ui/react/tabs";
import { ClipboardCheck, PanelsTopLeft } from "lucide-react";
import { AnimatePresence, LayoutGroup, MotionConfig } from "motion/react";
import { useEffect, useState } from "react";

import { subscribeToSnapshots } from "@/lib/api.js";
import type { ConnectionState } from "@/lib/api.js";
import { countFindings } from "@/lib/findings.js";
import { groupSnapshotByRepo } from "@/lib/group.js";
import type { Snapshot } from "@/lib/types.js";
import { ApiErrorState, EmptySessions } from "@/ui/empty-error-states.js";
import { FindingsPanel } from "@/ui/findings-panel.js";
import { Header } from "@/ui/header.js";
import { MessagesFeed } from "@/ui/messages-feed.js";
import { RepoLane } from "@/ui/repo-lane.js";

export function App() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [connection, setConnection] = useState<ConnectionState>("connecting");
  const [error, setError] = useState<Error | null>(null);
  const [lastUpdated, setLastUpdated] = useState<number | null>(null);
  const [refreshSequence, setRefreshSequence] = useState(0);
  const [now, setNow] = useState(() => Date.now() / 1000);

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
      <LayoutGroup id="coordination-dashboard">
        <div className="min-h-dvh bg-canvas text-ink">
          <Header
            connection={connection}
            lanes={lanes}
            lastUpdated={lastUpdated}
            now={now}
            refreshSequence={refreshSequence}
            snapshot={snapshot}
          />

          <main className="px-3 py-5 sm:px-6 lg:px-8">
            {connection === "disconnected" && error ? (
              <div className="pb-5">
                <ApiErrorState compact={snapshot !== null} detail={error.message} />
              </div>
            ) : null}

            <Tabs.Root defaultValue="coordination">
              <Tabs.List
                activateOnFocus
                aria-label="Dashboard views"
                className="mb-6 flex gap-1 border-b border-line-strong"
              >
                <Tabs.Tab
                  className="flex min-h-11 items-center gap-2 border-b-2 border-transparent px-3 py-2 text-sm font-medium text-muted transition-colors hover:bg-surface-muted hover:text-ink focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent data-active:border-accent data-active:bg-accent-wash data-active:text-ink motion-reduce:transition-none sm:px-4"
                  value="coordination"
                >
                  <PanelsTopLeft aria-hidden="true" className="size-4" />
                  Coordination
                </Tabs.Tab>
                <Tabs.Tab
                  className="flex min-h-11 items-center gap-2 border-b-2 border-transparent px-3 py-2 text-sm font-medium text-muted transition-colors hover:bg-surface-muted hover:text-ink focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent data-active:border-accent data-active:bg-accent-wash data-active:text-ink motion-reduce:transition-none sm:px-4"
                  value="findings"
                >
                  <ClipboardCheck aria-hidden="true" className="size-4" />
                  Findings
                  <span
                    aria-label={`${unresolvedCount} unresolved findings`}
                    className="min-w-5 border border-line bg-surface px-1.5 py-0.5 text-center font-mono text-[11px]/4 tabular-nums"
                    title={`${unresolvedCount} unresolved findings (pending or handed off)`}
                  >
                    {unresolvedCount}
                  </span>
                </Tabs.Tab>
              </Tabs.List>
              {snapshot === null && connection !== "disconnected" ? (
                <div className="border-y border-line-strong bg-surface px-4 py-12 text-center text-sm text-muted">
                  Loading coordination snapshot…
                </div>
              ) : null}
              {snapshot ? (
                <>
                  <Tabs.Panel
                    keepMounted
                    value="coordination"
                    className="focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-accent"
                  >
                    <div className="grid items-start gap-8 xl:grid-cols-[minmax(0,1fr)_22rem]">
                      <div className="min-w-0">
                        {snapshot.sessions.length === 0 || lanes.length === 0 ? (
                          <EmptySessions />
                        ) : null}
                        {lanes.length > 0 ? (
                          <div className="flex flex-col gap-5">
                            <AnimatePresence initial={false} mode="popLayout">
                              {lanes.map((lane) => (
                                <RepoLane key={lane.repoRoot} lane={lane} now={now} />
                              ))}
                            </AnimatePresence>
                          </div>
                        ) : null}
                      </div>

                      <aside className="flex min-w-0 flex-col gap-8">
                        <MessagesFeed messages={snapshot.messages} now={now} />
                      </aside>
                    </div>
                  </Tabs.Panel>
                  <Tabs.Panel
                    keepMounted
                    value="findings"
                    className="focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-accent"
                  >
                    <FindingsPanel findings={snapshot.findings} now={now} />
                  </Tabs.Panel>
                </>
              ) : null}
            </Tabs.Root>
          </main>
        </div>
      </LayoutGroup>
    </MotionConfig>
  );
}
