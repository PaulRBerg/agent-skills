import { TriangleAlert } from "lucide-react";
import type { ConnectionState } from "@/lib/api";
import { formatUpdatedTime } from "@/lib/format";
import type { RepoLaneModel, Snapshot } from "@/lib/types";
import { ConnectionIndicator } from "@/ui/connection-indicator";

interface HeaderProps {
  snapshot: Snapshot | null;
  lanes: RepoLaneModel[];
  connection: ConnectionState;
  lastUpdated: number | null;
  now: number;
}

export function Header({
  snapshot,
  lanes,
  connection,
  lastUpdated,
  now,
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
                  {snapshot?.sessions.length ?? 0}
                </dd>
                <dt className="text-muted">live sessions</dt>
              </div>
              <div className="flex items-baseline gap-1.5">
                <dd className="font-mono text-sm font-semibold tabular-nums">
                  {lanes.length}
                </dd>
                <dt className="text-muted">active repos</dt>
              </div>
              <div className="flex items-baseline gap-1.5">
                <dd className="font-mono text-sm font-semibold tabular-nums">
                  {blockedCount}
                </dd>
                <dt className="text-muted">queued / blocked</dt>
              </div>
            </dl>

            <div className="flex items-center gap-3 border-l border-line pl-4">
              <ConnectionIndicator state={connection} />
              <span className="font-mono text-xs text-muted tabular-nums">
                updated{" "}
                {lastUpdated === null
                  ? "never"
                  : formatUpdatedTime(lastUpdated, now)}
              </span>
            </div>
          </div>
        </div>

        {showCoverageWarning ? (
          <div
            className="mt-4 flex items-start gap-2 border-l-2 border-warning bg-warning-subtle px-3 py-2 text-xs text-warning-ink"
            role="status"
          >
            <TriangleAlert
              aria-hidden="true"
              className="mt-0.5 size-3.5 shrink-0"
            />
            <p>
              Provider coverage is partial. Session ownership may be incomplete
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
          </div>
        ) : null}
      </div>
    </header>
  );
}
