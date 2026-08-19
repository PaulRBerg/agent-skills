import { Clock3, LockKeyhole } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { tv } from "tailwind-variants";
import { shortenPath } from "@/lib/format";
import { MOTION_DURATION, MOTION_EASE } from "@/lib/motion";
import type {
  WorkClaimWithQueuePosition,
  WorkWithQueuePosition,
} from "@/lib/types";
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

function ClaimDetail({
  claim,
  state,
}: {
  claim: WorkClaimWithQueuePosition;
  state: WorkWithQueuePosition["state"];
}) {
  return (
    <motion.div
      animate={{ opacity: 1, y: 0 }}
      className="flex min-w-0 flex-wrap items-center gap-1.5"
      data-motion-item
      exit={{ opacity: 0, y: -3 }}
      initial={{ opacity: 0, y: 3 }}
      layout="position"
      transition={{ duration: MOTION_DURATION.field, ease: MOTION_EASE }}
    >
      <span
        className="max-w-full truncate font-mono text-[10px]/4 font-semibold uppercase tracking-wide text-muted"
        title={claim.repo_root}
      >
        {shortenPath(claim.repo_root)}
      </span>
      {state === "draft" ? (
        <span className={chip({ state })}>
          draft · {claim.scope_count} scope{claim.scope_count === 1 ? "" : "s"}
        </span>
      ) : (
        claim.scopes?.map((scope) => (
          <span
            className={chip({ state })}
            key={`${scope.kind}:${scope.path}`}
            title={`${scope.path} (${scope.kind})`}
          >
            <span className="min-w-0 truncate">{scope.path}</span>
          </span>
        ))
      )}
      {state === "queued" && claim.queuePosition !== undefined ? (
        <span className="inline-flex items-center gap-1 font-mono text-xs text-queued-ink">
          <Clock3 aria-hidden="true" className="size-3" />#
          <AnimatedValue value={claim.queuePosition}>
            {claim.queuePosition}
          </AnimatedValue>{" "}
          in queue
        </span>
      ) : null}
      {claim.blocked_reason ? (
        <span className="inline-flex min-w-0 items-center gap-1 text-xs text-danger">
          <LockKeyhole aria-hidden="true" className="size-3 shrink-0" />
          <span className="truncate" title={claim.blocked_reason}>
            {claim.blocked_reason}
          </span>
        </span>
      ) : null}
    </motion.div>
  );
}

interface WorkChipsProps {
  work: WorkWithQueuePosition;
}

export function WorkChips({ work }: WorkChipsProps) {
  const parentBlocker = work.blocked_reason ?? undefined;
  const showParentBlocker =
    parentBlocker !== undefined &&
    !work.claims.some((claim) => claim.blocked_reason === parentBlocker);

  return (
    <motion.div
      className="flex min-w-0 flex-col gap-1.5"
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
      {showParentBlocker ? (
        <span className="inline-flex min-w-0 items-center gap-1 text-xs text-danger">
          <LockKeyhole aria-hidden="true" className="size-3 shrink-0" />
          <span className="truncate" title={parentBlocker}>
            {parentBlocker}
          </span>
        </span>
      ) : null}
      <AnimatePresence initial={false} mode="popLayout">
        {work.claims.map((claim) => (
          <ClaimDetail claim={claim} key={claim.repo_root} state={work.state} />
        ))}
      </AnimatePresence>
    </motion.div>
  );
}
