import { TriangleAlert } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import type { ConnectionState } from "@/lib/api";
import { formatUpdatedTime } from "@/lib/format";
import { MOTION_DURATION, MOTION_EASE } from "@/lib/motion";
import type { RepoLaneModel, Snapshot } from "@/lib/types";
import { AnimatedValue } from "@/ui/animated-value";
import { ConnectionIndicator } from "@/ui/connection-indicator";

interface HeaderProps {
  snapshot: Snapshot | null;
  lanes: RepoLaneModel[];
  connection: ConnectionState;
  lastUpdated: number | null;
  now: number;
  refreshSequence: number;
}

export function Header({
  snapshot,
  lanes,
  connection,
  lastUpdated,
  now,
  refreshSequence,
}: HeaderProps) {
  const blockedCount =
    snapshot?.claims.filter(
      (claim) => claim.state === "queued" || claim.blocked_reason !== null,
    ).length ?? 0;
  const partialProviders =
    snapshot?.providers.filter(
      (provider) => provider.enabled && (!provider.ok || provider.dropped > 0),
    ) ?? [];
  const showCoverageWarning =
    snapshot !== null && (!snapshot.complete || partialProviders.length > 0);

  return (
    <header className="border-b border-line-strong bg-surface">
      <div className="px-4 py-4 sm:px-6 lg:px-8">
        <div className="flex flex-col gap-4 xl:flex-row xl:items-center xl:justify-between">
          <div className="flex items-baseline gap-3">
            <h1 className="font-mono text-lg font-semibold tracking-tight">
              ai-coord
            </h1>
            <p className="text-xs text-muted">Machine control tower</p>
          </div>

          <div className="flex flex-wrap items-center gap-x-5 gap-y-3">
            <dl className="flex flex-wrap items-baseline gap-x-5 gap-y-2 text-xs">
              <div className="flex items-baseline gap-1.5">
                <dd className="font-mono text-sm font-semibold tabular-nums">
                  <AnimatedValue value={snapshot?.sessions.length ?? 0}>
                    {snapshot?.sessions.length ?? 0}
                  </AnimatedValue>
                </dd>
                <dt className="text-muted">live sessions</dt>
              </div>
              <div className="flex items-baseline gap-1.5">
                <dd className="font-mono text-sm font-semibold tabular-nums">
                  <AnimatedValue value={lanes.length}>
                    {lanes.length}
                  </AnimatedValue>
                </dd>
                <dt className="text-muted">active repos</dt>
              </div>
              <div className="flex items-baseline gap-1.5">
                <dd className="font-mono text-sm font-semibold tabular-nums">
                  <AnimatedValue value={blockedCount}>
                    {blockedCount}
                  </AnimatedValue>
                </dd>
                <dt className="text-muted">queued / blocked</dt>
              </div>
            </dl>

            <div className="relative flex items-center gap-3 border-l border-line pl-4">
              {lastUpdated !== null ? (
                <span
                  aria-hidden="true"
                  className="refresh-sweep"
                  key={refreshSequence}
                />
              ) : null}
              <AnimatedValue value={connection}>
                <ConnectionIndicator state={connection} />
              </AnimatedValue>
              <span className="font-mono text-xs text-muted tabular-nums">
                updated{" "}
                {lastUpdated === null
                  ? "never"
                  : formatUpdatedTime(lastUpdated, now)}
              </span>
            </div>
          </div>
        </div>

        <AnimatePresence initial={false}>
          {showCoverageWarning ? (
            <motion.div
              animate={{ opacity: 1, y: 0 }}
              className="mt-4 flex items-start gap-2 border-l-2 border-warning bg-warning-subtle px-3 py-2 text-xs text-warning-ink"
              data-motion-item
              exit={{ opacity: 0, y: -4 }}
              initial={{ opacity: 0, y: 4 }}
              role="status"
              transition={{
                duration: MOTION_DURATION.row,
                ease: MOTION_EASE,
              }}
            >
              <TriangleAlert
                aria-hidden="true"
                className="mt-0.5 size-3.5 shrink-0"
              />
              <p>
                Provider coverage is partial. Session ownership may be
                incomplete
                {partialProviders.length > 0
                  ? `: ${partialProviders
                      .map((provider) =>
                        provider.dropped > 0
                          ? `${provider.client} dropped ${provider.dropped}`
                          : `${provider.client} unavailable`,
                      )
                      .join(", ")}.`
                  : "."}
              </p>
            </motion.div>
          ) : null}
        </AnimatePresence>
      </div>
    </header>
  );
}
