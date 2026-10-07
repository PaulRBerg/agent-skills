import type { HandoffCategory, HandoffRecord, HandoffsResponse } from "@/lib/handoff-types.js";

export type HandoffState = HandoffRecord["state"];

export type HandoffGroup = {
  root: string;
  repository: string;
  handoffs: HandoffRecord[];
};

const categoryNames: Record<HandoffCategory, string> = {
  implementation: "Implementation",
  investigation: "Investigation",
  research: "Research",
  audit: "Audit",
  operations: "Operations",
};

const monthNames = [
  "Jan",
  "Feb",
  "Mar",
  "Apr",
  "May",
  "Jun",
  "Jul",
  "Aug",
  "Sep",
  "Oct",
  "Nov",
  "Dec",
];

export async function fetchHandoffs(signal?: AbortSignal): Promise<HandoffRecord[]> {
  const response = await fetch("/api/handoffs", { cache: "no-store", signal });
  if (!response.ok) {
    throw new Error(`Request failed (${response.status})`);
  }
  const payload = (await response.json()) as HandoffsResponse;
  return payload.handoffs;
}

/** Groups records by origin, preserving the server's repository and newest-first order. */
export function groupByProvenance(handoffs: HandoffRecord[]): HandoffGroup[] {
  const groups = new Map<string, HandoffGroup>();
  for (const handoff of handoffs) {
    const key = `${handoff.root}\u0000${handoff.repository}`;
    const group = groups.get(key);
    if (group) {
      group.handoffs.push(handoff);
    } else {
      groups.set(key, { handoffs: [handoff], repository: handoff.repository, root: handoff.root });
    }
  }
  return [...groups.values()];
}

export function categoryLabel(category: HandoffCategory | null): string {
  return category ? categoryNames[category] : "Uncategorized";
}

/** Formats a timestamp as `21 Sep`, adding the year when it differs from the current one or when forced. */
export function formatHandoffDate(value: string, withYear = false, now = new Date()): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  const dayMonth = `${date.getDate()} ${monthNames[date.getMonth()]}`;
  return withYear || date.getFullYear() !== now.getFullYear()
    ? `${dayMonth} ${date.getFullYear()}`
    : dayMonth;
}

/** Drops a leading `# Title` line that repeats the record title already shown in the reader header. */
export function stripDuplicateTitle(markdown: string, title: string): string {
  const lines = markdown.split("\n");
  const index = lines.findIndex((line) => line.trim() !== "");
  const heading = /^#\s+(?<heading>.+)$/u.exec(lines[index]?.trimEnd() ?? "")?.groups?.heading;
  if (heading?.trim() !== title.trim()) {
    return markdown;
  }
  const next = lines[index + 1]?.trim() === "" ? index + 2 : index + 1;
  return lines.slice(next).join("\n");
}
