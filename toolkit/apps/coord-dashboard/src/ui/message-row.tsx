import { ArrowRight, Check } from "lucide-react";
import { motion } from "motion/react";
import {
  formatRelativeTime,
  messageEndpointName,
  shortenPath,
} from "@/lib/format";
import { MOTION_DURATION, MOTION_EASE } from "@/lib/motion";
import type { Message } from "@/lib/types";
import { AnimatedValue } from "@/ui/animated-value";

interface MessageRowProps {
  compact?: boolean;
  message: Message;
  now: number;
  showRepository?: boolean;
}

export function MessageRow({
  compact = false,
  message,
  now,
  showRepository = false,
}: MessageRowProps) {
  const acknowledged = message.acknowledged_at !== null;
  const senderLabel = messageEndpointName(
    message.sender_callsign,
    message.sender_session_id,
  );
  const recipientLabel = messageEndpointName(
    message.recipient_callsign,
    message.recipient_session_id,
  );

  return (
    <motion.li
      animate={{ opacity: 1, y: 0 }}
      className="relative border-t border-line-muted px-3 py-3 first:border-t-0"
      data-motion-item
      exit={{ opacity: 0, y: -6 }}
      initial={{ opacity: 0, y: 8 }}
      layout="position"
      transition={{
        duration: MOTION_DURATION.row,
        ease: MOTION_EASE,
        layout: { duration: MOTION_DURATION.layout, ease: MOTION_EASE },
      }}
    >
      <span
        className={`absolute -left-1 top-4 size-2 rounded-full transition-colors ${acknowledged ? "bg-muted" : "bg-accent"}`}
        aria-hidden="true"
      />
      <div className="flex min-w-0 flex-wrap items-center gap-1 font-mono text-[11px]/4">
        <AnimatedValue className="min-w-0" value={senderLabel}>
          <span className="block truncate font-semibold">{senderLabel}</span>
        </AnimatedValue>
        <ArrowRight aria-hidden="true" className="size-3 shrink-0 text-muted" />
        <AnimatedValue className="min-w-0" value={recipientLabel}>
          <span className="block truncate">{recipientLabel}</span>
        </AnimatedValue>
      </div>
      {showRepository && message.repo_root ? (
        <p
          className="mt-1 truncate font-mono text-[10px]/4 text-muted"
          title={message.repo_root}
        >
          {shortenPath(message.repo_root)}
        </p>
      ) : null}
      <p
        className={`mt-1.5 text-xs/5 text-ink-secondary ${compact ? "line-clamp-3" : "whitespace-pre-wrap wrap-break-word"}`}
        title={compact ? message.text : undefined}
      >
        {message.text}
      </p>
      <div className="mt-2 flex items-center justify-between gap-3 font-mono text-[10px]/4 text-muted">
        <span>{formatRelativeTime(message.created_at, now)}</span>
        <AnimatedValue value={acknowledged}>
          <span className="inline-flex items-center gap-1">
            {acknowledged ? (
              <Check aria-hidden="true" className="size-3" />
            ) : null}
            {acknowledged ? "Acknowledged" : "Unread"}
          </span>
        </AnimatedValue>
      </div>
    </motion.li>
  );
}
