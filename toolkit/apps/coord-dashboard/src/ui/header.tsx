import { Activity, FolderGit2, GitMerge, TriangleAlert, Users, Workflow } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { tv } from "tailwind-variants";

import type { ConnectionState } from "@/lib/api.js";
import { formatUpdatedTime } from "@/lib/format.js";
import { MOTION_DURATION, MOTION_EASE } from "@/lib/motion.js";
import type { RepoLaneModel, Snapshot } from "@/lib/types.js";
import { AnimatedValue } from "@/ui/animated-value.js";
import { ConnectionIndicator } from "@/ui/connection-indicator.js";

const metricValue = tv({
  base: "shrink-0 text-[18px]/6 font-semibold tracking-tight tabular-nums",
  variants: { waiting: { true: "text-queued-ink", false: "text-ink" } },
});

type HeaderProps = {
  snapshot: Snapshot | null;
  lanes: RepoLaneModel[];
  connection: ConnectionState;
  lastUpdated: number | null;
  now: number;
  refreshSequence: number;
};

export function Header({
  snapshot,
  lanes,
  connection,
  lastUpdated,
  now,
  refreshSequence,
}: HeaderProps) {
  const blockedCount =
    snapshot?.work.filter(
      (work) =>
        work.state === "queued" ||
        typeof work.blocked_reason === "string" ||
        work.claims.some((claim) => typeof claim.blocked_reason === "string")
    ).length ?? 0;
  const activeWorkCount = snapshot?.work.filter((work) => work.state === "active").length ?? 0;
  const partialProviders =
    snapshot?.providers.filter(
      (provider) => !provider.enabled || !provider.ok || provider.dropped > 0
    ) ?? [];
  const providerSummary = partialProviders
    .map((provider) => {
      if (!provider.enabled) {
        return `${provider.client} disabled`;
      }
      if (provider.dropped > 0) {
        return `${provider.client} dropped ${provider.dropped}`;
      }
      return provider.error
        ? `${provider.client} unavailable (${provider.error})`
        : `${provider.client} unavailable`;
    })
    .join(", ");
  const hasUpdate = lastUpdated !== null;
  const showCoverageWarning =
    snapshot !== null && (!snapshot.complete || partialProviders.length > 0);
  const signals = [
    { label: "Sessions", value: snapshot?.sessions.length ?? 0, icon: Users, waiting: false },
    { label: "Active work", value: activeWorkCount, icon: Activity, waiting: false },
    { label: "Queued / blocked", value: blockedCount, icon: GitMerge, waiting: blockedCount > 0 },
    { label: "Repositories", value: lanes.length, icon: FolderGit2, waiting: false },
  ];

  return (
    <header className="relative isolate overflow-hidden border-b border-line bg-surface">
      <svg
        aria-hidden="true"
        className="pointer-events-none absolute top-0 right-0 -z-1 h-60 w-[min(65rem,85%)] mask-[linear-gradient(to_right,transparent,black_35%)] text-accent opacity-15 sm:opacity-20"
        fill="none"
        focusable="false"
        preserveAspectRatio="xMaxYMid slice"
        viewBox="0 0 1000 240"
      >
        <g stroke="currentColor" strokeWidth="1">
          <path d="M0 54H290C345 54 345 114 400 114H1000" />
          <path d="M120 188H355C410 188 410 114 465 114" />
          <path d="M220 14H510C565 14 565 74 620 74H1000" />
          <path d="M465 114H650C705 114 705 188 760 188H1000" />
          <path d="M600 240V210C600 154 655 154 655 114" />
          <path d="M760 188C815 188 815 74 870 74" />
          <path d="M755 0V24C755 74 810 74 810 114" />
        </g>
        <g fill="var(--color-surface-base)" stroke="currentColor" strokeWidth="2">
          <circle cx="290" cy="54" r="4" />
          <circle cx="400" cy="114" r="5" />
          <circle cx="510" cy="14" r="4" />
          <circle cx="620" cy="74" r="5" />
          <circle cx="655" cy="114" r="5" />
          <circle cx="760" cy="188" r="5" />
          <circle cx="870" cy="74" r="4" />
          <circle cx="950" cy="114" r="4" />
        </g>
      </svg>
      <div className="mx-auto max-w-[1520px] px-4 pt-5 sm:px-6 lg:px-8">
        <div className="flex flex-wrap items-center justify-between gap-x-5 gap-y-3 pb-5">
          <div className="flex items-center gap-3">
            <Workflow aria-hidden="true" className="size-7 text-accent" strokeWidth={1.7} />
            <h1 className="text-[30px]/tight font-semibold tracking-[-0.04em]">ai-coord</h1>
            <span className="ml-2 hidden border-l border-line pl-4 text-[12px] text-muted sm:block">
              Local coordination
            </span>
          </div>
          <div className="relative flex flex-wrap items-center gap-x-4 gap-y-1 text-[12px]">
            {hasUpdate ? (
              <span aria-hidden="true" className="refresh-sweep" key={refreshSequence} />
            ) : null}
            <AnimatedValue value={connection}>
              <ConnectionIndicator state={connection} />
            </AnimatedValue>
            <span className="text-[11px] text-muted tabular-nums">
              updated {lastUpdated === null ? "never" : formatUpdatedTime(lastUpdated, now)}
            </span>
          </div>
        </div>

        <dl className="grid grid-cols-2 border-t border-line sm:grid-cols-4">
          {signals.map(({ label, value, icon: Icon, waiting }) => (
            <div
              className="flex items-center justify-between gap-2 border-l border-line px-4 py-2.5 first:border-l-0 nth-3:border-l-0 sm:gap-3 sm:px-6 sm:nth-3:border-l"
              key={label}
            >
              <dt className="flex min-w-0 items-center gap-2 text-[12px] font-medium text-muted">
                <Icon
                  aria-hidden="true"
                  className="hidden size-3.5 shrink-0 sm:block"
                  strokeWidth={1.7}
                />
                <span className="truncate">{label}</span>
              </dt>
              <dd className={metricValue({ waiting })}>
                {snapshot === null ? (
                  <span className="text-muted">—</span>
                ) : (
                  <AnimatedValue value={value}>{value}</AnimatedValue>
                )}
              </dd>
            </div>
          ))}
        </dl>

        <AnimatePresence initial={false}>
          {showCoverageWarning ? (
            <motion.div
              animate={{ opacity: 1, y: 0 }}
              className="mb-4 flex items-start gap-2 rounded-md border border-warning/30 bg-warning-subtle px-3 py-2 text-[13px]/5 text-warning-ink"
              data-motion-item
              exit={{ opacity: 0, y: -4 }}
              initial={{ opacity: 0, y: 4 }}
              role="status"
              transition={{
                duration: MOTION_DURATION.row,
                ease: MOTION_EASE,
              }}
            >
              <TriangleAlert aria-hidden="true" className="mt-0.5 size-3.5 shrink-0" />
              <p>
                Provider coverage is partial. Session ownership may be incomplete
                {providerSummary ? `: ${providerSummary}.` : "."}
              </p>
            </motion.div>
          ) : null}
        </AnimatePresence>
      </div>
    </header>
  );
}
