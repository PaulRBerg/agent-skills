import type { Finding, FindingState } from "@/lib/types.js";

export type FindingFilter = "all" | "unresolved" | "open" | "handoff" | "resolved";

export type FindingCounts = {
  total: number;
  unresolved: number;
  pending: number;
  triaging: number;
  handedOff: number;
  terminal: number;
};

export type FindingGroup = {
  repoRoot: string;
  counts: FindingCounts;
  findings: Finding[];
};

const terminalStates = new Set<FindingState>(["fixed", "stale", "rejected", "duplicate"]);

export function isTerminalFinding(finding: Finding): boolean {
  return terminalStates.has(finding.state);
}

export function countFindings(findings: Finding[]): FindingCounts {
  const counts: FindingCounts = {
    total: findings.length,
    unresolved: 0,
    pending: 0,
    triaging: 0,
    handedOff: 0,
    terminal: 0,
  };
  for (const finding of findings) {
    counts.pending += Number(finding.state === "pending");
    counts.triaging += Number(finding.triaging);
    counts.handedOff += Number(finding.state === "handed-off");
    counts.terminal += Number(isTerminalFinding(finding));
  }
  counts.unresolved = counts.pending + counts.handedOff;
  return counts;
}

export function orderFindings(findings: Finding[]): Finding[] {
  return findings.toSorted(
    (left, right) => right.updated_at - left.updated_at || left.id.localeCompare(right.id)
  );
}

export function filterFindings(findings: Finding[], filter: FindingFilter): Finding[] {
  return orderFindings(findings).filter((finding) => {
    if (filter === "all") {
      return true;
    }
    if (filter === "unresolved") {
      return !isTerminalFinding(finding);
    }
    if (filter === "open") {
      return finding.state === "pending";
    }
    if (filter === "handoff") {
      return finding.state === "handed-off";
    }
    return isTerminalFinding(finding);
  });
}

export function groupFindings(findings: Finding[], filter: FindingFilter = "all"): FindingGroup[] {
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
      findings: filterFindings(rows, filter),
    }))
    .filter((group) => group.findings.length > 0)
    .toSorted(
      (left, right) =>
        right.counts.unresolved - left.counts.unresolved ||
        right.counts.triaging - left.counts.triaging ||
        left.repoRoot.localeCompare(right.repoRoot)
    );
}
