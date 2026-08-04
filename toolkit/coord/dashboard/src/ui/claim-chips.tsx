import { Clock3, LockKeyhole } from "lucide-react";
import { tv } from "tailwind-variants";
import type { ClaimWithQueuePosition } from "@/lib/types";

const chip = tv({
  base: "inline-flex max-w-full items-center border px-1.5 py-0.5 font-mono text-xs/4",
  variants: {
    state: {
      active: "border-active-line bg-active-subtle text-active-ink",
      queued: "border-queued-line bg-queued-subtle text-queued-ink",
      intent: "border-intent-line bg-intent-subtle text-intent-ink",
    },
  },
});

interface ClaimChipsProps {
  claim: ClaimWithQueuePosition;
}

export function ClaimChips({ claim }: ClaimChipsProps) {
  return (
    <div className="flex min-w-0 flex-wrap items-center gap-1.5">
      <span className="font-mono text-[10px]/4 font-semibold uppercase tracking-wide text-muted">
        {claim.state}
      </span>
      {claim.paths.length > 0 ? (
        claim.paths.map((path) => (
          <span
            className={chip({ state: claim.state })}
            key={path}
            title={path}
          >
            <span className="min-w-0 truncate">{path}</span>
          </span>
        ))
      ) : (
        <span className="font-mono text-xs text-muted">No paths declared</span>
      )}

      {claim.state === "queued" && claim.queuePosition !== undefined ? (
        <span className="inline-flex items-center gap-1 font-mono text-xs text-queued-ink">
          <Clock3 aria-hidden="true" className="size-3" />#{claim.queuePosition}{" "}
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
    </div>
  );
}
