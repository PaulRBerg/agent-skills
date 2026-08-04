import { GitBranch } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { tv } from "tailwind-variants";
import {
  formatRelativeTime,
  getLivenessTier,
  sessionDisplayName,
  shortSessionId,
} from "@/lib/format";
import { MOTION_DURATION, MOTION_EASE } from "@/lib/motion";
import type { Delegate, LaneSession } from "@/lib/types";
import { AnimatedValue } from "@/ui/animated-value";
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
      className={`size-2 shrink-0 rounded-full transition-colors ${livenessClasses[tier]}`}
      role="img"
      title={`${tier}; seen ${formatRelativeTime(lastSeen, now)}`}
      aria-label={`${tier} liveness; seen ${formatRelativeTime(lastSeen, now)}`}
    />
  );
}

function DelegateRow({ delegate, now }: { delegate: Delegate; now: number }) {
  return (
    <motion.div
      animate={{ opacity: 1, x: 0 }}
      className="grid gap-2 border-t border-line-muted py-2 pl-8 sm:grid-cols-[minmax(13rem,1.2fr)_6rem_minmax(16rem,2fr)] sm:items-center sm:gap-4"
      data-motion-item
      exit={{ opacity: 0, x: -6 }}
      initial={{ opacity: 0, x: -6 }}
      layout="position"
      transition={{
        duration: MOTION_DURATION.row,
        ease: MOTION_EASE,
        layout: { duration: MOTION_DURATION.layout, ease: MOTION_EASE },
      }}
    >
      <div className="flex min-w-0 items-center gap-2 text-xs text-muted">
        <GitBranch aria-hidden="true" className="size-3.5 shrink-0" />
        <LivenessDot lastSeen={delegate.last_seen} now={now} />
        <span className="truncate font-mono" title={delegate.agent_id}>
          {delegate.agent_id}
        </span>
      </div>
      <AnimatedValue
        className="font-mono text-xs text-muted"
        value={delegate.state}
      >
        {delegate.state}
      </AnimatedValue>
      <span className="text-xs text-muted">
        Delegate · {delegate.agent_type ?? "unknown type"} · seen{" "}
        {formatRelativeTime(delegate.last_seen, now)}
      </span>
    </motion.div>
  );
}

interface SessionRowProps {
  row: LaneSession;
  repoRoot: string;
  now: number;
}

export function SessionRow({ row, repoRoot, now }: SessionRowProps) {
  const { session, claim, delegates } = row;
  const label = sessionDisplayName(session);
  const secondaryNames = [session.label, session.name].filter(
    (value, index, values): value is string =>
      value !== null && value !== label && values.indexOf(value) === index,
  );
  const client =
    session.client === "codex" || session.client === "claude"
      ? session.client
      : "other";

  return (
    <motion.div
      animate={{ opacity: 1, y: 0 }}
      className="border-t border-line-muted first:border-t-0"
      data-motion-item
      exit={{ opacity: 0, y: -6 }}
      initial={{ opacity: 0, y: 8 }}
      layout="position"
      transition={{
        duration: MOTION_DURATION.row,
        ease: MOTION_EASE,
        layout: { duration: MOTION_DURATION.layout, ease: MOTION_EASE },
      }}
    >
      <div className="grid gap-3 py-3 sm:grid-cols-[minmax(13rem,1.2fr)_6rem_minmax(16rem,2fr)] sm:items-center sm:gap-4">
        <div className="min-w-0">
          <div className="flex min-w-0 items-center gap-2">
            <LivenessDot lastSeen={session.last_seen} now={now} />
            <span className={clientBadge({ client })}>{session.client}</span>
            {session.permission_mode === "plan" ? (
              <span className="inline-flex border border-warning bg-warning-subtle px-1.5 py-0.5 font-mono text-[10px]/3 font-semibold uppercase tracking-wide text-warning-ink">
                planning
              </span>
            ) : null}
            <AnimatedValue
              className="min-w-0 flex-1 overflow-hidden text-sm font-semibold"
              value={label}
            >
              <span className="block truncate" title={label}>
                {label}
              </span>
            </AnimatedValue>
          </div>
          <div className="mt-1 flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 pl-4 font-mono text-[11px]/4 text-muted">
            <span>{shortSessionId(session.session_id)}</span>
            {secondaryNames.map((name) => (
              <span className="truncate" key={name} title={name}>
                {name}
              </span>
            ))}
            {session.cwd !== repoRoot ? (
              <span className="truncate" title={session.cwd}>
                cwd {session.cwd}
              </span>
            ) : null}
          </div>
        </div>

        <div className="flex items-center gap-2 pl-4 sm:pl-0">
          <AnimatedValue
            className="font-mono text-xs font-medium"
            value={session.state}
          >
            {session.state}
          </AnimatedValue>
          <span className="text-xs text-muted">
            {formatRelativeTime(session.last_seen, now)}
          </span>
        </div>

        <div className="min-w-0 pl-4 sm:pl-0">
          <AnimatePresence initial={false} mode="wait">
            {claim ? (
              <motion.div
                animate={{ opacity: 1, y: 0 }}
                data-motion-item
                exit={{ opacity: 0, y: -3 }}
                initial={{ opacity: 0, y: 3 }}
                key={`claim-${claim.id}`}
                transition={{
                  duration: MOTION_DURATION.field,
                  ease: MOTION_EASE,
                }}
              >
                <ClaimChips claim={claim} />
              </motion.div>
            ) : (
              <motion.span
                animate={{ opacity: 1 }}
                className="text-xs text-muted"
                data-motion-item
                exit={{ opacity: 0 }}
                initial={{ opacity: 0 }}
                key="no-claim"
                transition={{ duration: MOTION_DURATION.field }}
              >
                No claim
              </motion.span>
            )}
          </AnimatePresence>
        </div>
      </div>

      {delegates.length > 0 ? (
        <div className="border-l border-line-strong">
          <AnimatePresence initial={false} mode="popLayout">
            {delegates.map((delegate) => (
              <DelegateRow
                delegate={delegate}
                key={delegate.agent_id}
                now={now}
              />
            ))}
          </AnimatePresence>
        </div>
      ) : null}
    </motion.div>
  );
}
