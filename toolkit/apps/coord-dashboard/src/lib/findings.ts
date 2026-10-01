import type { Finding, FindingState } from "@/lib/types";

export const FINDING_PREVIEW_LIMIT = 3;

export type FindingFilter = "open" | "handoff" | "resolved";

export interface FindingCounts {
  pending: number;
  triaging: number;
  handedOff: number;
  terminal: number;
}

export interface FindingGroup {
  repoRoot: string;
  counts: FindingCounts;
  findings: Finding[];
}

const terminalStates: readonly FindingState[] = [
  "fixed",
  "stale",
  "rejected",
  "duplicate",
];

export function isTerminalFinding(finding: Finding): boolean {
  return terminalStates.includes(finding.state);
}

export function countFindings(findings: Finding[]): FindingCounts {
  return findings.reduce<FindingCounts>(
    (counts, finding) => ({
      pending: counts.pending + Number(finding.state === "pending"),
      triaging: counts.triaging + Number(finding.triaging),
      handedOff: counts.handedOff + Number(finding.state === "handed-off"),
      terminal: counts.terminal + Number(isTerminalFinding(finding)),
    }),
    { pending: 0, triaging: 0, handedOff: 0, terminal: 0 },
  );
}

export function orderFindings(findings: Finding[]): Finding[] {
  return [...findings].sort(
    (left, right) =>
      right.updated_at - left.updated_at || left.id.localeCompare(right.id),
  );
}

export function filterFindings(
  findings: Finding[],
  filter: FindingFilter,
): Finding[] {
  return orderFindings(findings).filter((finding) => {
    if (filter === "open") return finding.state === "pending";
    if (filter === "handoff") return finding.state === "handed-off";
    return isTerminalFinding(finding);
  });
}

export function groupFindings(findings: Finding[]): FindingGroup[] {
  const groups = new Map<string, Finding[]>();
  for (const finding of findings) {
    const rows = groups.get(finding.repo_root) ?? [];
    rows.push(finding);
    groups.set(finding.repo_root, rows);
  }

  return [...groups]
    .map(([repoRoot, rows]) => ({
      repoRoot,
      counts: countFindings(rows),
      findings: orderFindings(rows),
    }))
    .sort(
      (left, right) =>
        right.counts.pending - left.counts.pending ||
        right.counts.triaging - left.counts.triaging ||
        left.repoRoot.localeCompare(right.repoRoot),
    );
}
