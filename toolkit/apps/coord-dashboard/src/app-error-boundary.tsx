import { Button } from "@base-ui/react/button";
import { RotateCcw, TriangleAlert } from "lucide-react";

export function AppErrorBoundary() {
  return (
    <main className="grid min-h-dvh place-items-center bg-canvas px-5 text-ink">
      <section className="w-full max-w-xl border border-line-strong bg-surface p-8">
        <TriangleAlert aria-hidden="true" className="size-8 text-danger" />
        <h1 className="mt-5 text-3xl font-semibold tracking-tight">Something went wrong.</h1>
        <p className="mt-4 text-base/7 text-muted">
          An unexpected error interrupted the dashboard. Reload to try again.
        </p>
        <Button
          className="mt-6 inline-flex items-center gap-2 border border-line-strong bg-canvas px-4 py-2 text-sm font-medium outline-none hover:bg-surface-muted focus-visible:ring-2 focus-visible:ring-accent"
          onClick={() => window.location.reload()}
        >
          <RotateCcw aria-hidden="true" className="size-4" />
          Reload dashboard
        </Button>
      </section>
    </main>
  );
}
