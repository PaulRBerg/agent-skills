import type { Session } from "@/lib/types";

export type LivenessTier = "fresh" | "aging" | "stale";

export function formatRelativeTime(
  timestamp: number,
  now = Date.now() / 1000,
): string {
  const age = Math.max(0, Math.floor(now - timestamp));

  if (age < 5) return "just now";
  if (age < 60) return `${age}s ago`;
  if (age < 3_600) return `${Math.floor(age / 60)}m ago`;
  if (age < 86_400) return `${Math.floor(age / 3_600)}h ago`;
  return `${Math.floor(age / 86_400)}d ago`;
}

export function formatUpdatedTime(
  timestamp: number,
  now = Date.now() / 1000,
): string {
  const age = Math.max(0, Math.floor(now - timestamp));
  if (age < 60) return `${age}s ago`;
  return formatRelativeTime(timestamp, now);
}

export function getLivenessTier(
  lastSeen: number,
  now = Date.now() / 1000,
): LivenessTier {
  const age = Math.max(0, now - lastSeen);
  if (age < 30) return "fresh";
  if (age < 300) return "aging";
  return "stale";
}

export function shortenPath(value: string): string {
  const segments = value.split("/").filter(Boolean);
  if (segments.length <= 3) return value;
  return `…/${segments.slice(-3).join("/")}`;
}

export function shortSessionId(sessionId: string): string {
  return sessionId.slice(0, 8);
}

export function sessionDisplayName(
  session: Pick<Session, "callsign" | "name" | "session_id">,
  workLabel?: string,
): string {
  return (
    session.callsign ??
    workLabel ??
    session.name ??
    shortSessionId(session.session_id)
  );
}

export function messageEndpointName(
  callsign: string | null | undefined,
  sessionId: string,
): string {
  return callsign ?? shortSessionId(sessionId);
}
