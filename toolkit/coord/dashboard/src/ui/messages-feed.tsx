import { ArrowRight, Check, MessageSquareText } from "lucide-react";
import { formatRelativeTime, shortSessionId } from "@/lib/format";
import type { Message, Session } from "@/lib/types";

function sessionLabel(
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

interface MessagesFeedProps {
  messages: Message[];
  sessions: Session[];
  now: number;
}

export function MessagesFeed({ messages, sessions, now }: MessagesFeedProps) {
  const ordered = [...messages].sort(
    (left, right) =>
      right.created_at - left.created_at || left.id.localeCompare(right.id),
  );

  return (
    <section
      className="border-t border-line-strong pt-3"
      aria-labelledby="messages-heading"
    >
      <div className="flex items-center justify-between gap-3">
        <h2
          id="messages-heading"
          className="flex items-center gap-2 text-sm font-semibold"
        >
          <MessageSquareText
            aria-hidden="true"
            className="size-4 text-accent"
            strokeWidth={1.8}
          />
          Messages
        </h2>
        <span className="font-mono text-xs text-muted tabular-nums">
          {messages.length}
        </span>
      </div>

      {ordered.length === 0 ? (
        <p className="mt-3 border-l border-line px-3 py-2 text-xs text-muted">
          No coordination messages
        </p>
      ) : (
        <ol className="mt-3 border-l border-line">
          {ordered.map((message) => {
            const acknowledged = message.acknowledged_at !== null;
            return (
              <li
                className={`relative border-t border-line-muted px-3 py-3 first:border-t-0 ${acknowledged ? "opacity-55" : ""}`}
                key={message.id}
              >
                <span
                  className={`absolute -left-1 top-4 size-2 rounded-full ${acknowledged ? "bg-muted" : "bg-accent"}`}
                  aria-hidden="true"
                />
                <div className="flex min-w-0 flex-wrap items-center gap-1 font-mono text-[11px]/4">
                  <span className="truncate font-semibold">
                    {sessionLabel(
                      sessions,
                      message.sender_client,
                      message.sender_session_id,
                    )}
                  </span>
                  <ArrowRight
                    aria-hidden="true"
                    className="size-3 shrink-0 text-muted"
                  />
                  <span className="truncate">
                    {sessionLabel(
                      sessions,
                      message.recipient_client,
                      message.recipient_session_id,
                    )}
                  </span>
                </div>
                <p className="mt-1.5 text-xs/5 text-ink-secondary">
                  {message.text}
                </p>
                <div className="mt-2 flex items-center justify-between gap-3 font-mono text-[10px]/4 text-muted">
                  <span>{formatRelativeTime(message.created_at, now)}</span>
                  <span className="inline-flex items-center gap-1">
                    {acknowledged ? (
                      <Check aria-hidden="true" className="size-3" />
                    ) : null}
                    {acknowledged ? "Acknowledged" : "Unread"}
                  </span>
                </div>
              </li>
            );
          })}
        </ol>
      )}
    </section>
  );
}
