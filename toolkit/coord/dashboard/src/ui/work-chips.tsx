import { Clock3, LockKeyhole } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { tv } from "tailwind-variants";
import { MOTION_DURATION, MOTION_EASE } from "@/lib/motion";
import type { WorkWithQueuePosition } from "@/lib/types";
import { AnimatedValue } from "@/ui/animated-value";

const chip = tv({
  base: "inline-flex max-w-full items-center border px-1.5 py-0.5 font-mono text-xs/4",
  variants: {
    state: {
      active: "border-active-line bg-active-subtle text-active-ink",
      draft: "border-draft-line bg-draft-subtle text-draft-ink",
      queued: "border-queued-line bg-queued-subtle text-queued-ink",
    },
  },
});

interface WorkChipsProps {
  work: WorkWithQueuePosition;
}

export function WorkChips({ work }: WorkChipsProps) {
  return (
    <motion.div
      className="flex min-w-0 flex-wrap items-center gap-1.5"
      data-motion-item
      layout
      transition={{ duration: MOTION_DURATION.layout, ease: MOTION_EASE }}
    >
      <AnimatedValue
        className="font-mono text-[10px]/4 font-semibold uppercase tracking-wide text-muted"
        value={work.state}
      >
        {work.state}
      </AnimatedValue>
      <AnimatePresence initial={false} mode="popLayout">
        {work.state === "draft" ? (
          <motion.span
            animate={{ opacity: 1 }}
            className={chip({ state: work.state })}
            data-motion-item
            exit={{ opacity: 0 }}
            initial={{ opacity: 0 }}
            key={`draft:${work.scope_count}`}
            transition={{ duration: MOTION_DURATION.field }}
          >
            draft · {work.scope_count ?? 0} scopes
          </motion.span>
        ) : (
          work.scopes?.map((scope) => (
            <motion.span
              animate={{ opacity: 1, scale: 1 }}
              className={chip({ state: work.state })}
              data-motion-item
              exit={{ opacity: 0, scale: 0.96 }}
              initial={{ opacity: 0, scale: 0.96 }}
              key={`${work.state}:${scope.kind}:${scope.path}`}
              layout="position"
              title={`${scope.path} (${scope.kind})`}
              transition={{
                duration: MOTION_DURATION.field,
                ease: MOTION_EASE,
                layout: {
                  duration: MOTION_DURATION.layout,
                  ease: MOTION_EASE,
                },
              }}
            >
              <span className="min-w-0 truncate">{scope.path}</span>
            </motion.span>
          ))
        )}
      </AnimatePresence>

      <AnimatePresence initial={false}>
        {work.state === "queued" && work.queuePosition !== undefined ? (
          <motion.span
            animate={{ opacity: 1, x: 0 }}
            className="inline-flex items-center gap-1 font-mono text-xs text-queued-ink"
            data-motion-item
            exit={{ opacity: 0, x: -4 }}
            initial={{ opacity: 0, x: -4 }}
            transition={{
              duration: MOTION_DURATION.field,
              ease: MOTION_EASE,
            }}
          >
            <Clock3 aria-hidden="true" className="size-3" />#
            <AnimatedValue value={work.queuePosition}>
              {work.queuePosition}
            </AnimatedValue>{" "}
            in queue
          </motion.span>
        ) : null}
      </AnimatePresence>

      <AnimatePresence initial={false} mode="wait">
        {work.blocked_reason ? (
          <motion.span
            animate={{ opacity: 1, x: 0 }}
            className="inline-flex min-w-0 items-center gap-1 text-xs text-danger"
            data-motion-item
            exit={{ opacity: 0, x: -4 }}
            initial={{ opacity: 0, x: -4 }}
            key={work.blocked_reason}
            transition={{
              duration: MOTION_DURATION.field,
              ease: MOTION_EASE,
            }}
          >
            <LockKeyhole aria-hidden="true" className="size-3 shrink-0" />
            <span className="truncate" title={work.blocked_reason}>
              {work.blocked_reason}
            </span>
          </motion.span>
        ) : null}
      </AnimatePresence>
    </motion.div>
  );
}
