import { GitBranch } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { tv } from "tailwind-variants";

import {
  displayPath,
  formatRelativeTime,
  getLivenessTier,
  sessionDisplayName,
  shortSessionId,
} from "@/lib/format.js";
import { MOTION_DURATION, MOTION_EASE } from "@/lib/motion.js";
import type { Delegate, LaneSession } from "@/lib/types.js";
import { AnimatedValue } from "@/ui/animated-value.js";
import { WorkChips } from "@/ui/work-chips.js";

const clientBadge = tv({
  base: "inline-flex shrink-0 rounded-md border px-1.5 py-0.5 text-[10px]/3 font-semibold tracking-wide uppercase",
  variants: {
    client: {
      codex: "border-codex-line bg-codex-subtle text-codex",
      claude: "border-claude-line bg-claude-subtle text-claude",
      other: "border-line bg-surface-muted text-muted",
    },
  },
});
const sessionState = tv({
  base: "inline-flex w-fit items-center rounded-md px-2 py-1 text-[11px]/4 font-medium",
  variants: {
    state: {
      working: "bg-active-subtle text-active-ink",
      in_flight: "bg-active-subtle text-active-ink",
      waiting: "bg-queued-subtle text-queued-ink",
      idle: "bg-surface-muted text-muted",
      unknown: "bg-surface-muted text-muted",
    },
  },
});

const livenessDot = tv({
  base: "size-2 shrink-0 rounded-full transition-colors",
  variants: {
    tier: {
      fresh: "liveness-fresh bg-positive",
      aging: "bg-warning opacity-70",
      stale: "bg-muted opacity-35",
    },
  },
});

function LivenessDot({ lastSeen, now }: { lastSeen: number; now: number }) {
  const tier = getLivenessTier(lastSeen, now);
  return (
    <span
      className={livenessDot({ tier })}
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
      className="grid gap-2 border-t border-line-muted py-2.5 pl-4 md:grid-cols-[minmax(0,1.1fr)_7rem_minmax(0,1.6fr)] md:items-center md:gap-4"
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
      <AnimatedValue className="text-xs text-muted" value={delegate.state}>
        {delegate.state}
      </AnimatedValue>
      <span className="text-xs text-muted">
        Delegate · {delegate.agent_type ?? "unknown type"} · seen{" "}
        {formatRelativeTime(delegate.last_seen, now)}
      </span>
    </motion.div>
  );
}

type SessionRowProps = {
  row: LaneSession;
  repoRoot: string;
  now: number;
};

export function SessionRow({ row, repoRoot, now }: SessionRowProps) {
  const { session, work, delegates } = row;
  const showCwd = session.cwd !== repoRoot;
  const label = sessionDisplayName(session, work?.label);
  const secondaryNames = [work?.label, session.name].filter(
    (value, index, values): value is string =>
      value !== null && value !== undefined && value !== label && values.indexOf(value) === index
  );
  const client =
    session.client === "codex" || session.client === "claude" ? session.client : "other";

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
      <div className="grid gap-3 py-4 md:grid-cols-[minmax(0,1.1fr)_7rem_minmax(0,1.6fr)] md:items-start md:gap-4">
        <div className="min-w-0">
          <div className="flex min-w-0 items-center gap-2">
            <LivenessDot lastSeen={session.last_seen} now={now} />
            <AnimatedValue className="min-w-0 flex-1 text-[13px]/5 font-semibold" value={label}>
              <span className="block wrap-anywhere" title={label}>
                {label}
              </span>
            </AnimatedValue>
            <span className={clientBadge({ client })}>{session.client}</span>
          </div>
          <div className="mt-1.5 flex flex-wrap items-center gap-1.5 pl-4">
            {session.permission_mode === "plan" ? (
              <span className="inline-flex rounded-md border border-warning/40 bg-warning-subtle px-1.5 py-0.5 text-[10px]/3 font-semibold text-warning-ink">
                planning
              </span>
            ) : null}
            {session.coordination_waived ? (
              <span className="inline-flex rounded-md border border-accent/40 bg-accent-wash px-1.5 py-0.5 text-[10px]/3 font-semibold text-accent">
                waived
              </span>
            ) : null}
            <span className="font-mono text-[10px]/4 text-muted">
              {shortSessionId(session.session_id)}
            </span>
          </div>
          <div className="mt-1 flex min-w-0 flex-col gap-y-1 pl-4 text-xs/5 text-muted">
            {secondaryNames.map((name) => (
              <span className="wrap-anywhere" key={name} title={name}>
                {name}
              </span>
            ))}
            {showCwd ? (
              <span className="truncate font-mono text-[10px]/4" title={displayPath(session.cwd)}>
                cwd {displayPath(session.cwd)}
              </span>
            ) : null}
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-x-2 gap-y-1 pl-4 md:flex-col md:items-start md:pl-0">
          <AnimatedValue className={sessionState({ state: session.state })} value={session.state}>
            {session.state}
          </AnimatedValue>
          <span className="text-[10px]/4 text-muted">
            seen {formatRelativeTime(session.last_seen, now)}
          </span>
        </div>

        <div className="min-w-0 md:pl-0">
          <AnimatePresence initial={false} mode="wait">
            {work ? (
              <motion.div
                animate={{ opacity: 1, y: 0 }}
                data-motion-item
                exit={{ opacity: 0, y: -3 }}
                initial={{ opacity: 0, y: 3 }}
                key={`work-${work.id}`}
                transition={{
                  duration: MOTION_DURATION.field,
                  ease: MOTION_EASE,
                }}
              >
                <WorkChips work={work} />
              </motion.div>
            ) : (
              <motion.span
                animate={{ opacity: 1 }}
                className="text-xs text-muted"
                data-motion-item
                exit={{ opacity: 0 }}
                initial={{ opacity: 0 }}
                key="no-work"
                transition={{ duration: MOTION_DURATION.field }}
              >
                No work
              </motion.span>
            )}
          </AnimatePresence>
        </div>
      </div>

      {delegates.length > 0 ? (
        <div className="mb-2 ml-4 border-l border-line">
          <AnimatePresence initial={false} mode="popLayout">
            {delegates.map((delegate) => (
              <DelegateRow delegate={delegate} key={delegate.agent_id} now={now} />
            ))}
          </AnimatePresence>
        </div>
      ) : null}
    </motion.div>
  );
}
