import { GitBranch } from "lucide-react";
import { tv } from "tailwind-variants";
import {
  formatRelativeTime,
  getLivenessTier,
  shortSessionId,
} from "@/lib/format";
import type { Delegate, LaneSession } from "@/lib/types";
import { ClaimChips } from "@/ui/claim-chips";

const clientBadge = tv({
  base: "inline-flex border px-1.5 py-0.5 font-mono text-[11px]/4 font-semibold uppercase tracking-wide",
  variants: {
    client: {
      codex: "border-codex-line bg-codex-subtle text-codex",
      claude: "border-claude-line bg-claude-subtle text-claude",
      other: "border-line bg-surface-muted text-muted",
    },
  },
});

const livenessClasses = {
  fresh: "liveness-fresh bg-positive",
  aging: "bg-warning opacity-70",
  stale: "bg-muted opacity-35",
} as const;

function LivenessDot({ lastSeen, now }: { lastSeen: number; now: number }) {
  const tier = getLivenessTier(lastSeen, now);
  return (
    <span
      className={`size-2 shrink-0 rounded-full ${livenessClasses[tier]}`}
      title={`${tier}; seen ${formatRelativeTime(lastSeen, now)}`}
      aria-label={`${tier} liveness; seen ${formatRelativeTime(lastSeen, now)}`}
    />
  );
}

function DelegateRow({ delegate, now }: { delegate: Delegate; now: number }) {
  return (
    <div className="grid gap-2 border-t border-line-muted py-2 pl-8 sm:grid-cols-[minmax(13rem,1.2fr)_6rem_minmax(16rem,2fr)] sm:items-center sm:gap-4">
      <div className="flex min-w-0 items-center gap-2 text-xs text-muted">
        <GitBranch aria-hidden="true" className="size-3.5 shrink-0" />
        <LivenessDot lastSeen={delegate.last_seen} now={now} />
        <span className="truncate font-mono" title={delegate.agent_id}>
          {delegate.agent_id}
        </span>
      </div>
      <span className="font-mono text-xs text-muted">{delegate.state}</span>
      <span className="text-xs text-muted">
        Delegate · {delegate.agent_type ?? "unknown type"} · seen{" "}
        {formatRelativeTime(delegate.last_seen, now)}
      </span>
    </div>
  );
}

interface SessionRowProps {
  row: LaneSession;
  repoRoot: string;
  now: number;
}

export function SessionRow({ row, repoRoot, now }: SessionRowProps) {
  const { session, claim, delegates } = row;
  const label =
    session.label ?? session.name ?? shortSessionId(session.session_id);
  const secondaryName = session.label && session.name ? session.name : null;
  const client =
    session.client === "codex" || session.client === "claude"
      ? session.client
      : "other";

  return (
    <div className="border-t border-line-muted first:border-t-0">
      <div className="grid gap-3 py-3 sm:grid-cols-[minmax(13rem,1.2fr)_6rem_minmax(16rem,2fr)] sm:items-center sm:gap-4">
        <div className="min-w-0">
          <div className="flex min-w-0 items-center gap-2">
            <LivenessDot lastSeen={session.last_seen} now={now} />
            <span className={clientBadge({ client })}>{session.client}</span>
            <span className="truncate text-sm font-semibold" title={label}>
              {label}
            </span>
          </div>
          <div className="mt-1 flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 pl-4 font-mono text-[11px]/4 text-muted">
            <span>{shortSessionId(session.session_id)}</span>
            {secondaryName ? (
              <span className="truncate">{secondaryName}</span>
            ) : null}
            {session.cwd !== repoRoot ? (
              <span className="truncate" title={session.cwd}>
                cwd {session.cwd}
              </span>
            ) : null}
          </div>
        </div>

        <div className="flex items-center gap-2 pl-4 sm:pl-0">
          <span className="font-mono text-xs font-medium">{session.state}</span>
          <span className="text-xs text-muted">
            {formatRelativeTime(session.last_seen, now)}
          </span>
        </div>

        <div className="min-w-0 pl-4 sm:pl-0">
          {claim ? (
            <ClaimChips claim={claim} />
          ) : (
            <span className="text-xs text-muted">No claim</span>
          )}
        </div>
      </div>

      {delegates.length > 0 ? (
        <div className="border-l border-line-strong">
          {delegates.map((delegate) => (
            <DelegateRow
              delegate={delegate}
              key={delegate.agent_id}
              now={now}
            />
          ))}
        </div>
      ) : null}
    </div>
  );
}
