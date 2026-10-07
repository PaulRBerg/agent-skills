import { Activity, FolderGit2, GitMerge, TriangleAlert, Users, Workflow } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";

import type { ConnectionState } from "@/lib/api.js";
import { formatUpdatedTime } from "@/lib/format.js";
import { MOTION_DURATION, MOTION_EASE } from "@/lib/motion.js";
import type { RepoLaneModel, Snapshot } from "@/lib/types.js";
import { AnimatedValue } from "@/ui/animated-value.js";
import { ConnectionIndicator } from "@/ui/connection-indicator.js";

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
      return `${provider.client} unavailable`;
    })
    .join(", ");
  const hasUpdate = lastUpdated !== null;
  const showCoverageWarning =
    snapshot !== null && (!snapshot.complete || partialProviders.length > 0);
  const signals = [
    { label: "Sessions", value: snapshot?.sessions.length ?? 0, icon: Users },
    { label: "Active work", value: activeWorkCount, icon: Activity },
    { label: "Queued / blocked", value: blockedCount, icon: GitMerge },
    { label: "Repositories", value: lanes.length, icon: FolderGit2 },
  ];

  return (
    <header className="relative isolate overflow-hidden border-b border-line bg-surface">
      <svg
        aria-hidden="true"
        className="pointer-events-none absolute top-0 right-0 -z-1 h-60 w-[min(65rem,85%)] mask-[linear-gradient(to_right,transparent,black_35%)] text-accent opacity-20"
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
      <div className="mx-auto max-w-[1520px] px-4 py-3 sm:px-6 sm:pt-6 sm:pb-5 lg:px-8">
        <div className="flex items-center justify-between gap-2 sm:items-start sm:gap-5">
          <div>
            <div className="mb-2 hidden items-center gap-2 text-[11px] font-semibold tracking-[0.12em] text-accent uppercase sm:flex">
              <Workflow aria-hidden="true" className="size-3.5" strokeWidth={1.8} />
              Local coordination
            </div>
            <h1 className="text-[30px]/tight font-semibold tracking-[-0.04em] sm:text-[34px]">
              ai-coord
            </h1>
            <p className="mt-2 hidden text-[13px]/5 text-muted sm:block">
              Agents, ownership, and the work between them.
            </p>
          </div>
          <div className="relative flex shrink-0 flex-wrap items-center gap-x-3 gap-y-1 rounded-full border border-line bg-surface/90 px-2 py-1.5 sm:mt-1 sm:self-start sm:px-3 sm:py-2 [&_svg]:hidden sm:[&_svg]:block">
            {hasUpdate ? (
              <span aria-hidden="true" className="refresh-sweep" key={refreshSequence} />
            ) : null}
            <AnimatedValue value={connection}>
              <ConnectionIndicator state={connection} />
            </AnimatedValue>
            <span className="hidden text-[11px] text-muted tabular-nums sm:inline">
              updated {lastUpdated === null ? "never" : formatUpdatedTime(lastUpdated, now)}
            </span>
          </div>
        </div>

        <dl className="mt-4 grid grid-cols-2 gap-x-4 gap-y-2 sm:mt-6 sm:max-w-3xl sm:grid-cols-4 sm:gap-8">
          {signals.map(({ label, value, icon: Icon }) => (
            <div
              className="flex items-center justify-between gap-2 border-l border-line pl-2 first:border-accent/60 sm:block sm:pl-3"
              key={label}
            >
              <dt className="flex items-center gap-1.5 text-[11px] font-medium text-muted">
                <Icon aria-hidden="true" className="hidden size-3.5 sm:block" strokeWidth={1.7} />
                {label}
              </dt>
              <dd className="text-[20px]/[26px] font-semibold tracking-tight tabular-nums sm:mt-1 sm:text-2xl/8">
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
              className="mt-3 flex items-start gap-2 rounded-lg border border-warning/30 bg-warning-subtle px-3 py-2 text-xs/5 text-warning-ink sm:mt-5"
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
