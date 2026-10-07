import { Collapsible } from "@base-ui/react/collapsible";
import { ChevronDown, ClipboardCheck } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { useState } from "react";

import { countFindings, groupFindings } from "@/lib/findings.js";
import type { FindingFilter, FindingGroup as FindingGroupModel } from "@/lib/findings.js";
import { displayPath, formatRelativeTime } from "@/lib/format.js";
import { MOTION_DURATION, MOTION_EASE } from "@/lib/motion.js";
import type { Finding, FindingState } from "@/lib/types.js";
import { AnimatedValue } from "@/ui/animated-value.js";

const filters: { id: FindingFilter; label: string }[] = [
  { id: "unresolved", label: "Unresolved" },
  { id: "open", label: "Pending" },
  { id: "handoff", label: "Handed off" },
  { id: "resolved", label: "Resolved" },
  { id: "all", label: "All" },
];

const stateLabels: Record<FindingState, string> = {
  pending: "Pending",
  "handed-off": "Handed off",
  fixed: "Fixed",
  stale: "Stale",
  rejected: "Rejected",
  duplicate: "Duplicate",
};

const emptyMessages: Record<FindingFilter, string> = {
  all: "No findings recorded",
  unresolved: "No unresolved findings",
  open: "No pending findings",
  handoff: "No handed-off findings",
  resolved: "No resolved findings",
};

function FindingDetails({ finding, now }: { finding: Finding; now: number }) {
  return (
    <Collapsible.Root className="border-t border-line-muted first:border-t-0">
      <Collapsible.Trigger className="group flex w-full items-start justify-between gap-4 p-4 text-left transition-colors hover:bg-surface-muted focus-visible:relative focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent motion-reduce:transition-none sm:px-5">
        <span className="min-w-0 flex-1">
          <span className="block text-sm/6 font-medium wrap-break-word text-ink">
            {finding.summary}
          </span>
          <span className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-2 text-xs/5 text-muted">
            <span
              className="border border-line bg-surface-muted px-2 py-0.5 font-medium text-ink-secondary data-[state=fixed]:border-active-line data-[state=fixed]:bg-active-subtle data-[state=fixed]:text-active-ink data-[state=handed-off]:border-draft-line data-[state=handed-off]:bg-draft-subtle data-[state=handed-off]:text-draft-ink data-[state=pending]:border-warning data-[state=pending]:bg-warning-subtle data-[state=pending]:text-warning-ink"
              data-state={finding.state}
            >
              {stateLabels[finding.state]}
            </span>
            {finding.kind ? <span className="capitalize">{finding.kind}</span> : null}
            {finding.triaging ? <span className="font-medium text-accent">Triaging</span> : null}
            <span>Updated {formatRelativeTime(finding.updated_at, now)}</span>
            <span>
              {finding.sighting_count} {finding.sighting_count === 1 ? "sighting" : "sightings"}
            </span>
          </span>
        </span>
        <span className="mt-1 flex shrink-0 items-center gap-1 text-xs text-muted">
          <span className="hidden sm:inline">Evidence</span>
          <span className="sr-only">Toggle finding evidence</span>
          <ChevronDown
            aria-hidden="true"
            className="size-4 transition-transform group-data-panel-open:rotate-180 motion-reduce:transition-none"
          />
        </span>
      </Collapsible.Trigger>
      <Collapsible.Panel className="border-t border-line-muted bg-canvas p-4 text-xs/5 text-ink-secondary sm:px-5">
        <dl className="grid grid-cols-[auto_minmax(0,1fr)] gap-x-4 gap-y-2">
          <dt className="text-muted">ID</dt>
          <dd className="font-mono break-all">{finding.id}</dd>
          <dt className="text-muted">Created</dt>
          <dd>{formatRelativeTime(finding.created_at, now)}</dd>
          {finding.terminal_at === null ? null : (
            <>
              <dt className="text-muted">Closed</dt>
              <dd>{formatRelativeTime(finding.terminal_at, now)}</dd>
            </>
          )}
          {finding.paths.length > 0 ? (
            <>
              <dt className="text-muted">Paths</dt>
              <dd className="font-mono break-all">
                <ul className="space-y-1">
                  {finding.paths.map((path) => (
                    <li key={path}>{path}</li>
                  ))}
                </ul>
              </dd>
            </>
          ) : null}
          {finding.handoff_path ? (
            <>
              <dt className="text-muted">Handoff</dt>
              <dd className="font-mono break-all">{finding.handoff_path}</dd>
            </>
          ) : null}
          {finding.commit_oid ? (
            <>
              <dt className="text-muted">Commit</dt>
              <dd className="font-mono break-all">{finding.commit_oid}</dd>
            </>
          ) : null}
          {finding.canonical_id ? (
            <>
              <dt className="text-muted">Canonical</dt>
              <dd className="font-mono break-all">{finding.canonical_id}</dd>
            </>
          ) : null}
        </dl>
      </Collapsible.Panel>
    </Collapsible.Root>
  );
}

function FindingGroup({ group, now }: { group: FindingGroupModel; now: number }) {
  return (
    <motion.section
      animate={{ opacity: 1, y: 0 }}
      className="min-w-0 border border-line bg-surface"
      data-motion-item
      exit={{ opacity: 0, y: -6 }}
      initial={{ opacity: 0, y: 6 }}
      layout="position"
      transition={{
        duration: MOTION_DURATION.row,
        ease: MOTION_EASE,
        layout: { duration: MOTION_DURATION.layout, ease: MOTION_EASE },
      }}
    >
      <div className="flex flex-col gap-2 border-b border-line bg-surface-muted px-4 py-3 sm:flex-row sm:items-baseline sm:justify-between sm:gap-4 sm:px-5">
        <h3 className="min-w-0 font-mono text-xs/5 font-semibold break-all text-ink">
          {displayPath(group.repoRoot)}
        </h3>
        <p className="shrink-0 text-xs/5 text-muted tabular-nums">
          {group.findings.length} shown · {group.counts.unresolved} unresolved
        </p>
      </div>
      <div>
        {group.findings.map((finding) => (
          <FindingDetails finding={finding} key={finding.id} now={now} />
        ))}
      </div>
    </motion.section>
  );
}

type FindingsPanelProps = {
  findings: Finding[];
  now: number;
};

export function FindingsPanel({ findings, now }: FindingsPanelProps) {
  const [filter, setFilter] = useState<FindingFilter>("unresolved");
  const counts = countFindings(findings);
  const groups = groupFindings(findings, filter);
  const filterCounts: Record<FindingFilter, number> = {
    all: counts.total,
    unresolved: counts.unresolved,
    open: counts.pending,
    handoff: counts.handedOff,
    resolved: counts.terminal,
  };

  return (
    <section aria-labelledby="findings-heading" className="min-w-0">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h2 className="flex items-center gap-2 text-base font-semibold" id="findings-heading">
            <ClipboardCheck aria-hidden="true" className="size-4 text-accent" strokeWidth={1.8} />
            Findings
          </h2>
          <p className="mt-1 text-xs/5 text-muted">
            {counts.unresolved} unresolved · {counts.triaging} triaging
          </p>
        </div>
        <div aria-label="Finding state" className="flex flex-wrap gap-1" role="group">
          {filters.map(({ id, label }) => (
            <button
              aria-pressed={filter === id}
              className="flex min-h-9 items-center gap-2 border border-transparent px-2.5 py-1.5 text-xs font-medium text-muted transition-colors hover:bg-surface-muted hover:text-ink focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent aria-pressed:border-accent aria-pressed:bg-accent-wash aria-pressed:text-ink motion-reduce:transition-none"
              key={id}
              onClick={() => setFilter(id)}
              type="button"
            >
              {label}
              <AnimatedValue
                className="font-mono text-[11px] tabular-nums"
                value={filterCounts[id]}
              >
                {filterCounts[id]}
              </AnimatedValue>
            </button>
          ))}
        </div>
      </div>

      <p aria-live="polite" className="mt-5 text-xs/5 text-muted tabular-nums">
        {filterCounts[filter]} {filterCounts[filter] === 1 ? "finding" : "findings"} in{" "}
        {groups.length} {groups.length === 1 ? "repository" : "repositories"}
      </p>
      {groups.length === 0 ? (
        <div className="mt-3 border-y border-line bg-surface px-4 py-12 text-center">
          <p className="text-sm font-medium">
            {counts.total === 0 ? emptyMessages.all : emptyMessages[filter]}
          </p>
          {counts.total > 0 ? (
            <button
              className="mt-3 min-h-9 px-2 text-xs font-medium text-accent hover:underline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
              onClick={() => setFilter("all")}
              type="button"
            >
              View all {counts.total} findings
            </button>
          ) : (
            <p className="mt-2 text-xs text-muted">
              Findings will appear here when agents record them.
            </p>
          )}
        </div>
      ) : (
        <div className="mt-3 flex flex-col gap-5">
          <AnimatePresence initial={false} mode="popLayout">
            {groups.map((group) => (
              <FindingGroup group={group} key={group.repoRoot} now={now} />
            ))}
          </AnimatePresence>
        </div>
      )}
    </section>
  );
}
