import { FolderGit2 } from "lucide-react";
import { formatRelativeTime, shortSessionId } from "@/lib/format";
import type { RepoLaneModel } from "@/lib/types";
import { ClaimChips } from "@/ui/claim-chips";
import { SessionRow } from "@/ui/session-row";

interface RepoLaneProps {
  lane: RepoLaneModel;
  now: number;
}

export function RepoLane({ lane, now }: RepoLaneProps) {
  const claimCount =
    lane.sessions.filter((row) => row.claim !== undefined).length +
    lane.unmatchedClaims.length;

  return (
    <section
      className="snapshot-flash border-y border-line-strong bg-surface"
      aria-label={lane.repoRoot}
    >
      <div className="flex flex-col gap-2 border-b border-line bg-surface-muted px-3 py-2 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex min-w-0 items-center gap-2">
          <FolderGit2
            aria-hidden="true"
            className="size-4 shrink-0 text-accent"
            strokeWidth={1.8}
          />
          <h2
            className="min-w-0 break-all font-mono text-xs font-semibold sm:truncate"
            title={lane.repoRoot}
          >
            {lane.repoRoot}
          </h2>
        </div>
        <div className="flex shrink-0 items-center gap-3 font-mono text-[11px]/4 text-muted tabular-nums">
          <span>
            {lane.sessions.length} session
            {lane.sessions.length === 1 ? "" : "s"}
          </span>
          <span>
            {claimCount} claim{claimCount === 1 ? "" : "s"}
          </span>
          <span>active {formatRelativeTime(lane.lastActivity, now)}</span>
        </div>
      </div>

      <div className="hidden grid-cols-[minmax(13rem,1.2fr)_6rem_minmax(16rem,2fr)] gap-4 border-b border-line-muted px-3 py-1.5 font-mono text-[10px]/4 uppercase tracking-wider text-muted sm:grid">
        <span>Agent</span>
        <span>State</span>
        <span>Path ownership</span>
      </div>

      <div className="px-3">
        {lane.sessions.map((row) => (
          <SessionRow
            key={`${row.session.client}:${row.session.session_id}`}
            now={now}
            repoRoot={lane.repoRoot}
            row={row}
          />
        ))}

        {lane.unmatchedClaims.map((claim) => (
          <div
            className="grid gap-3 border-t border-line-muted py-3 sm:grid-cols-[minmax(13rem,1.2fr)_6rem_minmax(16rem,2fr)] sm:items-center sm:gap-4"
            key={claim.id}
          >
            <div className="min-w-0 pl-4">
              <p className="truncate text-xs font-medium">Unreported session</p>
              <p className="mt-1 font-mono text-[11px]/4 text-muted">
                {claim.client}:{shortSessionId(claim.session_id)}
              </p>
            </div>
            <span className="pl-4 font-mono text-xs sm:pl-0">
              {claim.state}
            </span>
            <div className="min-w-0 pl-4 sm:pl-0">
              <ClaimChips claim={claim} />
            </div>
          </div>
        ))}
      </div>

      {claimCount === 0 ? (
        <p className="border-t border-line-muted px-3 py-2 text-xs text-muted">
          No claims held
        </p>
      ) : null}
    </section>
  );
}
