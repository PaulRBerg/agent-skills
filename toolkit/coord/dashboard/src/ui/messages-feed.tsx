import { MessageSquareText } from "lucide-react";
import { AnimatePresence } from "motion/react";
import { previewMessages } from "@/lib/messages";
import type { Message } from "@/lib/types";
import { AnimatedValue } from "@/ui/animated-value";
import { MessageHistoryDialog } from "@/ui/message-history-dialog";
import { MessageRow } from "@/ui/message-row";

interface MessagesFeedProps {
  messages: Message[];
  now: number;
}

export function MessagesFeed({ messages, now }: MessagesFeedProps) {
  const preview = previewMessages(messages);

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
        <AnimatedValue
          className="font-mono text-xs text-muted tabular-nums"
          value={messages.length}
        >
          {messages.length}
        </AnimatedValue>
      </div>

      {preview.length === 0 ? (
        <p className="mt-3 border-l border-line px-3 py-2 text-xs text-muted">
          No coordination messages
        </p>
      ) : (
        <div className="mt-3">
          <ol className="border-l border-line">
            <AnimatePresence initial={false} mode="popLayout">
              {preview.map((message) => (
                <MessageRow
                  compact
                  key={message.id}
                  message={message}
                  now={now}
                />
              ))}
            </AnimatePresence>
          </ol>
          <div className="border-l border-t border-line-muted">
            <MessageHistoryDialog messages={messages} now={now} />
          </div>
        </div>
      )}
    </section>
  );
}
