import { Radio } from "lucide-react";
import { tv } from "tailwind-variants";
import type { ConnectionState } from "@/lib/api";

const indicator = tv({
  slots: {
    root: "inline-flex items-center gap-2 text-xs font-medium",
    dot: "size-2 rounded-full",
  },
  variants: {
    state: {
      connecting: {
        root: "text-muted",
        dot: "bg-muted",
      },
      live: {
        root: "text-positive",
        dot: "liveness-fresh bg-positive",
      },
      polling: {
        root: "text-warning",
        dot: "bg-warning",
      },
      disconnected: {
        root: "text-danger",
        dot: "bg-danger",
      },
    },
  },
});

const labels: Record<ConnectionState, string> = {
  connecting: "Connecting",
  live: "SSE live",
  polling: "Polling",
  disconnected: "Disconnected",
};

interface ConnectionIndicatorProps {
  state: ConnectionState;
}

export function ConnectionIndicator({ state }: ConnectionIndicatorProps) {
  const styles = indicator({ state });
  return (
    <span className={styles.root()} aria-label={`Connection: ${labels[state]}`}>
      <Radio aria-hidden="true" className="size-3.5" strokeWidth={1.8} />
      <span className={styles.dot()} aria-hidden="true" />
      {labels[state]}
    </span>
  );
}
