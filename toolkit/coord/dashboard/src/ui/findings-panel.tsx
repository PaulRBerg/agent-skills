import { Collapsible } from "@base-ui/react/collapsible";
import { ChevronDown, ClipboardCheck } from "lucide-react";
import { useState } from "react";
import { AnimatePresence, motion } from "motion/react";
import {
  FINDING_PREVIEW_LIMIT,
  filterFindings,
  groupFindings,
  type FindingFilter,
} from "@/lib/findings";
import { formatRelativeTime, shortenPath } from "@/lib/format";
import { MOTION_DURATION, MOTION_EASE } from "@/lib/motion";
import type { Finding } from "@/lib/types";
import { AnimatedValue } from "@/ui/animated-value";

const filters: Array<{ id: FindingFilter; label: string }> = [
  { id: "open", label: "Open" },
  { id: "handoff", label: "Handoff" },
  { id: "resolved", label: "Resolved" },
];

function FindingDetails({ finding, now }: { finding: Finding; now: number }) {
  return (
    <Collapsible.Root className="border-t border-line-muted first:border-t-0">
      <Collapsible.Trigger className="group flex w-full items-start justify-between gap-2 py-2 text-left focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent">
        <span className="min-w-0">
          <span className="block text-xs/5 text-ink-secondary">
            {finding.summary}
          </span>
          <span className="mt-1 flex flex-wrap gap-x-2 font-mono text-[10px]/4 text-muted">
            <span>{finding.state}</span>
            {finding.kind ? <span>{finding.kind}</span> : null}
            {finding.triaging ? <span>triaging</span> : null}
            <span>{finding.sighting_count} sightings</span>
          </span>
        </span>
        <ChevronDown
          aria-hidden="true"
          className="mt-1 size-3 shrink-0 transition-transform group-data-[panel-open]:rotate-180"
        />
      </Collapsible.Trigger>
      <Collapsible.Panel className="pb-2 text-[11px]/4 text-muted">
        <dl className="grid grid-cols-[auto_1fr] gap-x-2 gap-y-1 font-mono">
          <dt>ID</dt>
          <dd className="break-all">{finding.id}</dd>
          <dt>Updated</dt>
          <dd>{formatRelativeTime(finding.updated_at, now)}</dd>
          {finding.paths.length > 0 ? (
            <>
              <dt>Paths</dt>
              <dd className="break-all">{finding.paths.join(", ")}</dd>
            </>
          ) : null}
          {finding.handoff_path ? (
            <>
              <dt>Handoff</dt>
              <dd className="break-all">{finding.handoff_path}</dd>
            </>
          ) : null}
          {finding.commit_oid ? (
            <>
              <dt>Commit</dt>
              <dd className="break-all">{finding.commit_oid}</dd>
            </>
          ) : null}
          {finding.canonical_id ? (
            <>
              <dt>Canonical</dt>
              <dd className="break-all">{finding.canonical_id}</dd>
            </>
          ) : null}
        </dl>
      </Collapsible.Panel>
    </Collapsible.Root>
  );
}

function FindingGroup({
  group,
  filter,
  now,
}: {
  group: ReturnType<typeof groupFindings>[number];
  filter: FindingFilter;
  now: number;
}) {
  const filtered = filterFindings(group.findings, filter);
  const preview = filtered.slice(0, FINDING_PREVIEW_LIMIT);
  const remaining = filtered.slice(FINDING_PREVIEW_LIMIT);

  return (
    <motion.div
      animate={{ opacity: 1, y: 0 }}
      className="border-l border-line pl-3"
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
      <div className="flex items-start justify-between gap-2">
        <h3
          className="truncate font-mono text-[11px]/4 font-semibold"
          title={group.repoRoot}
        >
          {shortenPath(group.repoRoot)}
        </h3>
        <div className="flex shrink-0 flex-wrap justify-end gap-x-2 font-mono text-[10px]/4 text-muted tabular-nums">
          <span>{group.counts.pending} pending</span>
          <span>{group.counts.triaging} triaging</span>
          <span>{group.counts.handedOff} handed off</span>
          <span>{group.counts.terminal} terminal</span>
        </div>
      </div>

      {filtered.length === 0 ? (
        <p className="mt-2 text-xs text-muted">No {filter} findings</p>
      ) : (
        <div className="mt-1">
          <AnimatePresence initial={false} mode="popLayout">
            {preview.map((finding) => (
              <FindingDetails finding={finding} key={finding.id} now={now} />
            ))}
          </AnimatePresence>
          {remaining.length > 0 ? (
            <Collapsible.Root>
              <Collapsible.Trigger className="group flex w-full items-center gap-1.5 py-1 text-left font-mono text-[10px]/4 text-muted hover:text-ink focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent">
                <ChevronDown
                  aria-hidden="true"
                  className="size-3 transition-transform group-data-[panel-open]:rotate-180"
                />
                <AnimatedValue value={remaining.length}>
                  {remaining.length}
                </AnimatedValue>{" "}
                more {filter}
              </Collapsible.Trigger>
              <Collapsible.Panel>
                {remaining.map((finding) => (
                  <FindingDetails
                    finding={finding}
                    key={finding.id}
                    now={now}
                  />
                ))}
              </Collapsible.Panel>
            </Collapsible.Root>
          ) : null}
        </div>
      )}
    </motion.div>
  );
}

interface FindingsPanelProps {
  findings: Finding[];
  now: number;
}

export function FindingsPanel({ findings, now }: FindingsPanelProps) {
  const [filter, setFilter] = useState<FindingFilter>("open");
  const groups = groupFindings(findings);

  return (
    <section
      aria-labelledby="findings-heading"
      className="border-t border-line-strong pt-3"
    >
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h2
          className="flex items-center gap-2 text-sm font-semibold"
          id="findings-heading"
        >
          <ClipboardCheck
            aria-hidden="true"
            className="size-4 text-accent"
            strokeWidth={1.8}
          />
          Findings
        </h2>
        <div aria-label="Finding state" className="flex gap-1" role="group">
          {filters.map(({ id, label }) => (
            <button
              aria-pressed={filter === id}
              className={`rounded-sm px-1.5 py-0.5 font-mono text-[10px]/4 transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent ${filter === id ? "bg-accent-wash text-ink" : "text-muted hover:text-ink"}`}
              key={id}
              onClick={() => setFilter(id)}
              type="button"
            >
              {label}
            </button>
          ))}
        </div>
      </div>

      {groups.length === 0 ? (
        <p className="mt-3 border-l border-line px-3 py-2 text-xs text-muted">
          No durable findings
        </p>
      ) : (
        <div className="mt-3 flex flex-col gap-4">
          <AnimatePresence initial={false} mode="popLayout">
            {groups.map((group) => (
              <FindingGroup
                filter={filter}
                group={group}
                key={group.repoRoot}
                now={now}
              />
            ))}
          </AnimatePresence>
        </div>
      )}
    </section>
  );
}
