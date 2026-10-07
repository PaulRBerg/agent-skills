import { ChevronDown, Clock3, LockKeyhole } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { tv } from "tailwind-variants";

import { shortenPath } from "@/lib/format.js";
import { MOTION_DURATION, MOTION_EASE } from "@/lib/motion.js";
import type {
  RepoLaneDraft,
  WorkClaimWithQueuePosition,
  WorkWithQueuePosition,
} from "@/lib/types.js";
import { AnimatedValue } from "@/ui/animated-value.js";

const chip = tv({
  base: "inline-flex max-w-full items-center rounded-md border px-2 py-0.5 font-mono text-[11px]/5",
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
  const scopes = claim.scopes ?? [];
  return (
    <motion.div
      animate={{ opacity: 1, y: 0 }}
      className="min-w-0 rounded-lg border border-line-muted bg-canvas/40 px-2.5 py-2"
      data-motion-item
      exit={{ opacity: 0, y: -3 }}
      initial={{ opacity: 0, y: 3 }}
      layout="position"
      transition={{ duration: MOTION_DURATION.field, ease: MOTION_EASE }}
    >
      <div className="flex min-w-0 flex-wrap items-center justify-between gap-x-3 gap-y-1">
        <span
          className="min-w-0 truncate font-mono text-[10px]/4 text-muted"
          title={claim.repo_root}
        >
          {shortenPath(claim.repo_root)}
        </span>
        <span className="shrink-0 text-[10px]/4 text-muted tabular-nums">
          {claim.scope_count} scope{claim.scope_count === 1 ? "" : "s"}
        </span>
      </div>
      <div className="mt-1.5 flex min-w-0 flex-wrap gap-1.5">
        {scopes.slice(0, 2).map((scope) => (
          <span
            className={chip({ state })}
            key={`${scope.kind}:${scope.path}`}
            title={`${scope.path} (${scope.kind})`}
          >
            <span className="min-w-0 wrap-anywhere">{scope.path}</span>
          </span>
        ))}
      </div>
      {scopes.length > 2 ? (
        <details className="group mt-1.5 min-w-0">
          <summary className="flex min-h-7 w-fit cursor-pointer list-none items-center gap-1 rounded-sm text-[11px] font-medium text-accent focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent [&::-webkit-details-marker]:hidden">
            <ChevronDown
              aria-hidden="true"
              className="size-3.5 transition-transform group-open:rotate-180 motion-reduce:transition-none"
            />
            {scopes.length - 2} more · show all paths
          </summary>
          <ul className="mt-2 space-y-1.5 border-t border-line-muted pt-2">
            {scopes.map((scope) => (
              <li className="text-[11px]/5" key={`${scope.kind}:${scope.path}`}>
                <span className="font-mono wrap-anywhere text-ink-secondary">{scope.path}</span>
                <span className="ml-1.5 text-[10px] text-muted">{scope.kind}</span>
              </li>
            ))}
          </ul>
        </details>
      ) : null}
      {state === "queued" && claim.queuePosition !== undefined ? (
        <span className="mt-1.5 inline-flex items-center gap-1 text-xs font-medium text-queued-ink">
          <Clock3 aria-hidden="true" className="size-3" />#
          <AnimatedValue value={claim.queuePosition}>{claim.queuePosition}</AnimatedValue> in queue
        </span>
      ) : null}
      {claim.blocked_reason ? (
        <span className="mt-1.5 flex min-w-0 items-start gap-1.5 text-xs/5 text-danger">
          <LockKeyhole aria-hidden="true" className="mt-1 size-3 shrink-0" />
          <span className="wrap-anywhere">{claim.blocked_reason}</span>
        </span>
      ) : null}
    </motion.div>
  );
}

type WorkChipsProps = {
  work: WorkWithQueuePosition;
};

export function WorkChips({ work }: WorkChipsProps) {
  const parentBlocker = work.blocked_reason ?? undefined;
  const showParentBlocker =
    parentBlocker !== undefined &&
    !work.claims.some((claim) => claim.blocked_reason === parentBlocker);

  return (
    <motion.div
      className="flex min-w-0 flex-col gap-2"
      data-motion-item
      layout
      transition={{ duration: MOTION_DURATION.layout, ease: MOTION_EASE }}
    >
      <AnimatedValue
        className={
          work.state === "active"
            ? "text-[10px]/4 font-semibold tracking-wide text-active-ink uppercase"
            : "text-[10px]/4 font-semibold tracking-wide text-queued-ink uppercase"
        }
        value={work.state}
      >
        {work.state}
      </AnimatedValue>
      {showParentBlocker ? (
        <span className="flex min-w-0 items-start gap-1.5 text-xs/5 text-danger">
          <LockKeyhole aria-hidden="true" className="mt-1 size-3 shrink-0" />
          <span className="wrap-anywhere">{parentBlocker}</span>
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

type DraftChipProps = {
  laneDraft: RepoLaneDraft;
};

export function DraftChip({ laneDraft }: DraftChipProps) {
  const { who, scopeCount } = laneDraft;
  return (
    <span className={chip({ state: "draft" })} title={laneDraft.draft.label}>
      draft {who} · {scopeCount} scope{scopeCount === 1 ? "" : "s"}
    </span>
  );
}
