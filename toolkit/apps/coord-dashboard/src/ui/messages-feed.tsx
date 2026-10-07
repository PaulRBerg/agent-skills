import { MessageSquareText } from "lucide-react";
import { AnimatePresence } from "motion/react";

import { previewMessages } from "@/lib/messages.js";
import type { Message } from "@/lib/types.js";
import { AnimatedValue } from "@/ui/animated-value.js";
import { MessageHistoryDialog } from "@/ui/message-history-dialog.js";
import { MessageRow } from "@/ui/message-row.js";

type MessagesFeedProps = {
  messages: Message[];
  now: number;
};

export function MessagesFeed({ messages, now }: MessagesFeedProps) {
  const preview = previewMessages(messages, 3);

  return (
    <section
      className="overflow-hidden rounded-2xl border border-line bg-surface shadow-panel"
      aria-labelledby="messages-heading"
    >
      <div className="flex items-center justify-between gap-3 px-4 pt-4">
        <h2 id="messages-heading" className="flex items-center gap-2 text-sm font-semibold">
          <MessageSquareText aria-hidden="true" className="size-4 text-accent" strokeWidth={1.8} />
          Messages
        </h2>
        <AnimatedValue
          className="rounded-md bg-surface-muted px-2 py-0.5 text-[11px] text-muted tabular-nums"
          value={messages.length}
        >
          {messages.length}
        </AnimatedValue>
      </div>
      <p className="mt-1 px-4 text-[11px]/5 text-muted">Latest from the coordination ledger</p>

      {preview.length === 0 ? (
        <p className="m-4 rounded-lg bg-surface-muted px-3 py-5 text-xs text-muted">
          No coordination messages
        </p>
      ) : (
        <div className="mt-3">
          <ol className="px-4">
            <AnimatePresence initial={false} mode="popLayout">
              {preview.map((message) => (
                <MessageRow compact key={message.id} message={message} now={now} />
              ))}
            </AnimatePresence>
          </ol>
          <div className="border-t border-line-muted bg-surface-muted/40">
            <MessageHistoryDialog messages={messages} now={now} />
          </div>
        </div>
      )}
    </section>
  );
}
