import { shortSessionId } from "@/lib/format";
import type { Message, Session } from "@/lib/types";

export const MESSAGE_PREVIEW_LIMIT = 5;
export const MESSAGE_PAGE_SIZE = 20;

export type MessageStatusFilter = "all" | "unread" | "acknowledged";

export interface MessageFilters {
  query: string;
  repoRoot: string | null;
  status: MessageStatusFilter;
}

export interface MessagePage {
  items: Message[];
  page: number;
  pageCount: number;
  start: number;
  end: number;
  total: number;
}

export function messageParticipantLabel(
  sessions: Session[],
  client: string,
  sessionId: string,
): string {
  const session = sessions.find(
    (candidate) =>
      candidate.client === client && candidate.session_id === sessionId,
  );
  return session?.label ?? session?.name ?? shortSessionId(sessionId);
}

export function orderMessages(messages: Message[]): Message[] {
  return [...messages].sort(
    (left, right) =>
      right.created_at - left.created_at || left.id.localeCompare(right.id),
  );
}

export function previewMessages(
  messages: Message[],
  limit = MESSAGE_PREVIEW_LIMIT,
): Message[] {
  return orderMessages(messages).slice(0, limit);
}

export function messageRepositories(messages: Message[]): string[] {
  return [...new Set(messages.flatMap((message) => message.repo_root ?? []))]
    .filter((repoRoot) => repoRoot.length > 0)
    .sort((left, right) => left.localeCompare(right));
}

export function filterMessages(
  messages: Message[],
  sessions: Session[],
  filters: MessageFilters,
): Message[] {
  const query = filters.query.trim().toLowerCase();

  return orderMessages(messages).filter((message) => {
    const acknowledged = message.acknowledged_at !== null;
    if (filters.status === "unread" && acknowledged) return false;
    if (filters.status === "acknowledged" && !acknowledged) return false;
    if (filters.repoRoot !== null && message.repo_root !== filters.repoRoot) {
      return false;
    }
    if (query.length === 0) return true;

    const senderLabel = messageParticipantLabel(
      sessions,
      message.sender_client,
      message.sender_session_id,
    );
    const recipientLabel = messageParticipantLabel(
      sessions,
      message.recipient_client,
      message.recipient_session_id,
    );
    const searchable = [
      message.text,
      message.repo_root ?? "",
      message.sender_client,
      message.sender_session_id,
      senderLabel,
      message.recipient_client,
      message.recipient_session_id,
      recipientLabel,
    ]
      .join("\n")
      .toLowerCase();

    return searchable.includes(query);
  });
}

export function paginateMessages(
  messages: Message[],
  requestedPage: number,
  pageSize = MESSAGE_PAGE_SIZE,
): MessagePage {
  const pageCount = Math.max(1, Math.ceil(messages.length / pageSize));
  const page = Math.min(Math.max(1, requestedPage), pageCount);
  const offset = (page - 1) * pageSize;
  const items = messages.slice(offset, offset + pageSize);

  return {
    items,
    page,
    pageCount,
    start: items.length === 0 ? 0 : offset + 1,
    end: offset + items.length,
    total: messages.length,
  };
}
