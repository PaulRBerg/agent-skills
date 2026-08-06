import { FolderGit2 } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { formatRelativeTime, shortSessionId } from "@/lib/format";
import { MOTION_DURATION, MOTION_EASE } from "@/lib/motion";
import type { RepoLaneModel } from "@/lib/types";
import { AnimatedValue } from "@/ui/animated-value";
import { WorkChips } from "@/ui/work-chips";
import { SessionRow } from "@/ui/session-row";

interface RepoLaneProps {
  lane: RepoLaneModel;
  now: number;
}

export function RepoLane({ lane, now }: RepoLaneProps) {
  const workCount =
    lane.sessions.filter((row) => row.work !== undefined).length +
    lane.unmatchedWork.length;

  return (
    <motion.section
      animate={{ opacity: 1, y: 0 }}
      className="border-y border-line-strong bg-surface"
      data-motion-item
      exit={{ opacity: 0, y: -8 }}
      initial={{ opacity: 0, y: 10 }}
      layout="position"
      aria-label={lane.repoRoot}
      transition={{
        duration: MOTION_DURATION.row,
        ease: MOTION_EASE,
        layout: { duration: MOTION_DURATION.layout, ease: MOTION_EASE },
      }}
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
            <AnimatedValue value={lane.sessions.length}>
              {lane.sessions.length}
            </AnimatedValue>{" "}
            session
            {lane.sessions.length === 1 ? "" : "s"}
          </span>
          <span>
            <AnimatedValue value={workCount}>{workCount}</AnimatedValue> work
            {workCount === 1 ? " item" : " items"}
          </span>
          <span>active {formatRelativeTime(lane.lastActivity, now)}</span>
        </div>
      </div>

      <div className="hidden grid-cols-[minmax(13rem,1.2fr)_6rem_minmax(16rem,2fr)] gap-4 border-b border-line-muted px-3 py-1.5 font-mono text-[10px]/4 uppercase tracking-wider text-muted sm:grid">
        <span>Agent</span>
        <span>State</span>
        <span>Work scopes</span>
      </div>

      <div className="px-3">
        <AnimatePresence initial={false} mode="popLayout">
          {lane.sessions.map((row) => (
            <SessionRow
              key={`${row.session.client}:${row.session.session_id}`}
              now={now}
              repoRoot={lane.repoRoot}
              row={row}
            />
          ))}

          {lane.unmatchedWork.map((work) => (
            <motion.div
              animate={{ opacity: 1, y: 0 }}
              className="grid gap-3 border-t border-line-muted py-3 sm:grid-cols-[minmax(13rem,1.2fr)_6rem_minmax(16rem,2fr)] sm:items-center sm:gap-4"
              data-motion-item
              exit={{ opacity: 0, y: -6 }}
              initial={{ opacity: 0, y: 8 }}
              key={work.id}
              layout="position"
              transition={{
                duration: MOTION_DURATION.row,
                ease: MOTION_EASE,
                layout: {
                  duration: MOTION_DURATION.layout,
                  ease: MOTION_EASE,
                },
              }}
            >
              <div className="min-w-0 pl-4">
                <p className="truncate text-xs font-medium">
                  Unreported session
                </p>
                <p className="mt-1 font-mono text-[11px]/4 text-muted">
                  {work.client}:{shortSessionId(work.session_id)}
                </p>
              </div>
              <AnimatedValue
                className="pl-4 font-mono text-xs sm:pl-0"
                value={work.state}
              >
                {work.state}
              </AnimatedValue>
              <div className="min-w-0 pl-4 sm:pl-0">
                <WorkChips work={work} />
              </div>
            </motion.div>
          ))}
        </AnimatePresence>
      </div>

      <AnimatePresence initial={false}>
        {workCount === 0 ? (
          <motion.p
            animate={{ opacity: 1 }}
            className="border-t border-line-muted px-3 py-2 text-xs text-muted"
            data-motion-item
            exit={{ opacity: 0 }}
            initial={{ opacity: 0 }}
            transition={{ duration: MOTION_DURATION.field }}
          >
            No work recorded
          </motion.p>
        ) : null}
      </AnimatePresence>
    </motion.section>
  );
}
