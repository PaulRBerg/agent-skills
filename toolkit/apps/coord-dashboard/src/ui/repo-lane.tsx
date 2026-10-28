import { Check, Copy, FolderGit2 } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { useEffect, useRef, useState } from "react";

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

const COPIED_FEEDBACK_MS = 1500;

export function RepoLane({ lane, now }: RepoLaneProps) {
  const [copied, setCopied] = useState(false);
  const copiedTimer = useRef<number | undefined>(undefined);
  const path = displayPath(lane.repoRoot);
  const workCount =
    lane.sessions.filter((row) => row.work !== undefined).length + lane.unmatchedWork.length;

  useEffect(() => () => window.clearTimeout(copiedTimer.current), []);

  const copyPath = async () => {
    try {
      await navigator.clipboard.writeText(path);
    } catch {
      // Clipboard access was denied or is unavailable; keep the idle icon.
      return;
    }
    setCopied(true);
    window.clearTimeout(copiedTimer.current);
    copiedTimer.current = window.setTimeout(() => setCopied(false), COPIED_FEEDBACK_MS);
  };

  return (
    <motion.section
      animate={{ opacity: 1, y: 0 }}
      className="min-w-0 overflow-hidden rounded-lg border border-line-strong bg-surface"
      data-motion-item
      exit={{ opacity: 0, y: -8 }}
      initial={{ opacity: 0, y: 10 }}
      layout="position"
      aria-label={path}
      transition={{
        duration: MOTION_DURATION.row,
        ease: MOTION_EASE,
        layout: { duration: MOTION_DURATION.layout, ease: MOTION_EASE },
      }}
    >
      <div className="flex flex-col gap-2.5 border-b border-line bg-surface-muted px-4 py-2.5 sm:flex-row sm:items-center sm:justify-between sm:gap-4 sm:px-5">
        <div className="flex min-w-0 items-center gap-4 sm:gap-5">
          <span className="flex size-7 shrink-0 items-center justify-center rounded-md border border-accent/15 bg-accent-wash text-accent">
            <FolderGit2 aria-hidden="true" className="size-4" strokeWidth={1.7} />
          </span>
          <h3 className="shrink-0 text-[17px]/6 font-semibold tracking-tight" title={lane.repoRoot}>
            {lane.repoRoot.split("/").findLast(Boolean) ?? lane.repoRoot}
          </h3>
          <div className="flex min-w-0 items-center gap-1 font-mono text-[12px]/4 text-muted">
            <span
              className="max-w-44 truncate sm:max-w-[16rem] md:max-w-[20rem] lg:max-w-[24rem]"
              title={lane.repoRoot}
            >
              {path}
            </span>
            <button
              type="button"
              onClick={copyPath}
              className="inline-flex size-5 shrink-0 items-center justify-center rounded-sm text-muted transition-colors hover:bg-surface hover:text-ink focus-visible:outline-2 focus-visible:outline-accent motion-reduce:transition-none"
              title={copied ? "Copied!" : `Copy path: ${path}`}
              aria-label={copied ? "Path copied to clipboard" : "Copy project path to clipboard"}
            >
              {copied ? (
                <Check aria-hidden="true" className="size-3.5 text-accent" strokeWidth={1.7} />
              ) : (
                <Copy aria-hidden="true" className="size-3.5" strokeWidth={1.7} />
              )}
            </button>
          </div>
        </div>
        <div className="flex shrink-0 flex-wrap items-center gap-x-3.5 gap-y-1 text-[12px]/5 text-muted tabular-nums sm:justify-end">
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
          <span className="text-[13px]/5 font-medium text-ink">
            activity{" "}
            {lane.lastActivity === null ? "unknown" : formatRelativeTime(lane.lastActivity, now)}
          </span>
        </div>
      </div>

      <div className="hidden grid-cols-[minmax(0,1.1fr)_7rem_minmax(0,1.6fr)] gap-4 border-b border-line-muted py-2 pr-4 pl-7 text-[11px]/4 font-medium tracking-wide text-muted uppercase sm:pr-5 sm:pl-8 md:grid">
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
              className="grid gap-3 border-t border-line-muted py-4 pl-3 first:border-t-0 md:grid-cols-[minmax(0,1.1fr)_7rem_minmax(0,1.6fr)] md:items-start md:gap-4"
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
                <p className="text-[14px]/5 font-medium wrap-anywhere" title={work.label}>
                  {work.label}
                </p>
                <p className="mt-1 text-[12px]/5 wrap-anywhere text-muted">
                  Unreported session ·{" "}
                  <span className="font-mono">
                    {work.client}:{shortSessionId(work.session_id)}
                  </span>
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
        <div className="flex flex-wrap items-center gap-3 border-t border-line-muted px-4 py-3 sm:px-5">
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
            className="border-t border-line-muted px-4 py-3 text-[13px] text-muted sm:px-5"
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
