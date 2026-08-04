import { useEffect, useMemo, useState } from "react";
import { AnimatePresence, LayoutGroup, MotionConfig } from "motion/react";
import { subscribeToSnapshots, type ConnectionState } from "@/lib/api";
import { groupSnapshotByRepo } from "@/lib/group";
import type { Snapshot } from "@/lib/types";
import { ApiErrorState, EmptySessions } from "@/ui/empty-error-states";
import { Header } from "@/ui/header";
import { MessagesFeed } from "@/ui/messages-feed";
import { NotesPanel } from "@/ui/notes-panel";
import { RepoLane } from "@/ui/repo-lane";

export function App() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [connection, setConnection] = useState<ConnectionState>("connecting");
  const [error, setError] = useState<Error | null>(null);
  const [lastUpdated, setLastUpdated] = useState<number | null>(null);
  const [refreshSequence, setRefreshSequence] = useState(0);
  const [now, setNow] = useState(() => Date.now() / 1000);

  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now() / 1000), 1_000);
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
    [],
  );

  const lanes = useMemo(
    () => (snapshot ? groupSnapshotByRepo(snapshot) : []),
    [snapshot],
  );

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
                <ApiErrorState
                  compact={snapshot !== null}
                  detail={error.message}
                />
              </div>
            ) : null}

            {snapshot === null ? (
              connection !== "disconnected" ? (
                <div className="border-y border-line-strong bg-surface px-4 py-12 text-center text-sm text-muted">
                  Loading coordination snapshot…
                </div>
              ) : null
            ) : (
              <div className="grid items-start gap-8 xl:grid-cols-[minmax(0,1fr)_22rem]">
                <div className="min-w-0">
                  {snapshot.sessions.length === 0 ? <EmptySessions /> : null}
                  {lanes.length > 0 ? (
                    <div className="flex flex-col gap-5">
                      <AnimatePresence initial={false} mode="popLayout">
                        {lanes.map((lane) => (
                          <RepoLane key={lane.repoRoot} lane={lane} now={now} />
                        ))}
                      </AnimatePresence>
                    </div>
                  ) : snapshot.sessions.length > 0 ? (
                    <EmptySessions />
                  ) : null}
                </div>

                <aside className="flex min-w-0 flex-col gap-8">
                  <MessagesFeed messages={snapshot.messages} now={now} />
                  <NotesPanel notes={snapshot.notes} now={now} />
                </aside>
              </div>
            )}
          </main>
        </div>
      </LayoutGroup>
    </MotionConfig>
  );
}
