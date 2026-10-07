import { FolderGit2 } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";

import { displayPath, formatRelativeTime, shortSessionId } from "@/lib/format.js";
import { MOTION_DURATION, MOTION_EASE } from "@/lib/motion.js";
import type { RepoLaneModel } from "@/lib/types.js";
import { AnimatedValue } from "@/ui/animated-value.js";
import { SessionRow } from "@/ui/session-row.js";
import { DraftChip, WorkChips } from "@/ui/work-chips.js";

type RepoLaneProps = {
  lane: RepoLaneModel;
  now: number;
};

export function RepoLane({ lane, now }: RepoLaneProps) {
  const workCount =
    lane.sessions.filter((row) => row.work !== undefined).length + lane.unmatchedWork.length;

  return (
    <motion.section
      animate={{ opacity: 1, y: 0 }}
      className="min-w-0 overflow-hidden rounded-2xl border border-line bg-surface shadow-panel"
      data-motion-item
      exit={{ opacity: 0, y: -8 }}
      initial={{ opacity: 0, y: 10 }}
      layout="position"
      aria-label={displayPath(lane.repoRoot)}
      transition={{
        duration: MOTION_DURATION.row,
        ease: MOTION_EASE,
        layout: { duration: MOTION_DURATION.layout, ease: MOTION_EASE },
      }}
    >
      <div className="flex flex-col gap-3 border-b border-line bg-surface-muted/60 p-4 sm:px-5">
        <div className="flex min-w-0 items-center gap-3">
          <span className="flex size-10 shrink-0 items-center justify-center rounded-xl border border-accent/15 bg-accent-wash text-accent">
            <FolderGit2 aria-hidden="true" className="size-5" strokeWidth={1.7} />
          </span>
          <div className="min-w-0">
            <h2 className="truncate text-base/6 font-semibold tracking-tight" title={lane.repoRoot}>
              {lane.repoRoot.split("/").findLast(Boolean) ?? lane.repoRoot}
            </h2>
            <p className="mt-0.5 truncate font-mono text-[11px]/4 text-muted" title={lane.repoRoot}>
              {displayPath(lane.repoRoot)}
            </p>
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-[11px]/4 text-muted tabular-nums">
          <span>
            <AnimatedValue value={lane.sessions.length}>{lane.sessions.length}</AnimatedValue>{" "}
            session
            {lane.sessions.length === 1 ? "" : "s"}
          </span>
          {lane.handoffCount > 0 ? (
            <span>
              {lane.handoffCount} handoff{lane.handoffCount === 1 ? "" : "s"}
            </span>
          ) : null}
          <span>
            <AnimatedValue value={workCount}>{workCount}</AnimatedValue> work
            {workCount === 1 ? " item" : " items"}
          </span>
          <span className="sm:ml-auto">
            activity{" "}
            {lane.lastActivity === null ? "unknown" : formatRelativeTime(lane.lastActivity, now)}
          </span>
        </div>
      </div>

      <div className="hidden grid-cols-[minmax(0,1.1fr)_7rem_minmax(0,1.6fr)] gap-4 border-b border-line-muted px-5 py-2 text-[10px]/4 font-medium tracking-wider text-muted uppercase md:grid">
        <span>Agent</span>
        <span>State</span>
        <span>Work scopes</span>
      </div>

      <div className="px-4 sm:px-5">
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
              className="grid gap-3 border-t border-line-muted py-4 first:border-t-0 md:grid-cols-[minmax(0,1.1fr)_7rem_minmax(0,1.6fr)] md:items-start md:gap-4"
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
              <div className="min-w-0">
                <p className="text-[13px]/5 font-medium wrap-anywhere" title={work.label}>
                  {work.label}
                </p>
                <p className="mt-1 font-mono text-[11px]/4 text-muted">
                  Unreported session · {work.client}:{shortSessionId(work.session_id)}
                </p>
              </div>
              <AnimatedValue className="text-xs font-medium text-muted" value={work.state}>
                {work.state}
              </AnimatedValue>
              <div className="min-w-0">
                <WorkChips work={work} />
              </div>
            </motion.div>
          ))}
        </AnimatePresence>
      </div>

      {lane.drafts.length > 0 ? (
        <div className="flex flex-wrap items-center gap-2 border-t border-line-muted bg-surface-muted/40 px-4 py-3 sm:px-5">
          <AnimatePresence initial={false} mode="popLayout">
            {lane.drafts.map((laneDraft) => (
              <motion.div
                animate={{ opacity: 1, y: 0 }}
                data-motion-item
                exit={{ opacity: 0, y: -3 }}
                initial={{ opacity: 0, y: 3 }}
                key={laneDraft.draft.id}
                layout="position"
                transition={{ duration: MOTION_DURATION.field, ease: MOTION_EASE }}
              >
                <DraftChip laneDraft={laneDraft} />
              </motion.div>
            ))}
          </AnimatePresence>
        </div>
      ) : null}

      <AnimatePresence initial={false}>
        {workCount === 0 && lane.drafts.length === 0 ? (
          <motion.p
            animate={{ opacity: 1 }}
            className="border-t border-line-muted bg-surface-muted/30 px-4 py-3 text-xs text-muted sm:px-5"
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
