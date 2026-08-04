import { RadioTower, TerminalSquare } from "lucide-react";

export function EmptySessions() {
  return (
    <div className="border-y border-line-strong bg-surface px-4 py-12 text-center">
      <RadioTower
        aria-hidden="true"
        className="mx-auto size-6 text-muted"
        strokeWidth={1.5}
      />
      <h2 className="mt-3 text-sm font-semibold">No live agent sessions</h2>
      <p className="mx-auto mt-1 max-w-md text-xs/5 text-muted">
        New Codex and Claude Code sessions will appear here after their first
        coordination heartbeat.
      </p>
    </div>
  );
}

export function ApiErrorState({
  detail,
  compact = false,
}: {
  detail: string;
  compact?: boolean;
}) {
  return (
    <div
      className={`border-l-2 border-danger bg-danger-subtle px-4 ${compact ? "py-3" : "py-8"}`}
      role="alert"
    >
      <div
        className={`flex gap-3 ${compact ? "items-start" : "flex-col items-start"}`}
      >
        <TerminalSquare
          aria-hidden="true"
          className="size-5 shrink-0 text-danger"
          strokeWidth={1.7}
        />
        <div>
          <h2 className="text-sm font-semibold">Dashboard API unreachable</h2>
          <p className="mt-1 text-xs/5 text-ink-secondary">
            Start the local server with{" "}
            <code className="font-mono text-danger">ai-coord serve</code> or{" "}
            <code className="font-mono text-danger">just dev</code>, then leave
            this page open.
          </p>
          <p className="mt-2 font-mono text-[10px]/4 text-muted">{detail}</p>
        </div>
      </div>
    </div>
  );
}
