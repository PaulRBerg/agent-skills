import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

import type { HandoffRecord, HandoffsResponse } from "../shared/handoff";
import "./styles.css";

function App() {
  const [handoffs, setHandoffs] = useState<HandoffRecord[]>([]);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    fetch("/api/handoffs", { signal: controller.signal, cache: "no-store" })
      .then(async (response) => {
        if (!response.ok) throw new Error(`Request failed (${response.status})`);
        return (await response.json()) as HandoffsResponse;
      })
      .then(({ handoffs: nextHandoffs }) => {
        setHandoffs(nextHandoffs);
        setSelectedPath((current) => current ?? nextHandoffs[0]?.path ?? null);
      })
      .catch((reason: unknown) => {
        if (reason instanceof DOMException && reason.name === "AbortError") return;
        setError(reason instanceof Error ? reason.message : "Unable to load handoffs");
      });
    return () => controller.abort();
  }, []);

  const selected = handoffs.find((handoff) => handoff.path === selectedPath) ?? null;

  return (
    <main className="min-h-dvh bg-slate-950 p-4 text-slate-100 md:p-6">
      <div className="mx-auto grid max-w-7xl gap-4 lg:grid-cols-[20rem_1fr]">
        <aside className="rounded-md border border-slate-800 bg-slate-900 p-4">
          <h1 className="text-lg/7 font-semibold">AI Handoffs</h1>
          <p className="mt-1 text-sm/6 text-slate-400">Local read-only task viewer</p>
          {error ? <p className="mt-4 text-sm/6 text-red-300">{error}</p> : null}
          <nav className="mt-4 flex flex-col gap-2" aria-label="Handoffs">
            {handoffs.map((handoff) => (
              <button
                className="rounded-sm border border-slate-800 px-3 py-2 text-left text-sm/5 hover:bg-slate-800"
                key={handoff.id}
                onClick={() => setSelectedPath(handoff.path)}
                type="button"
              >
                <span className="block font-medium">{handoff.title}</span>
                <span className="mt-1 block text-xs/5 text-slate-400">{handoff.repository}</span>
              </button>
            ))}
            {!error && handoffs.length === 0 ? (
              <p className="text-sm/6 text-slate-400">No handoffs found.</p>
            ) : null}
          </nav>
        </aside>
        <article className="min-w-0 rounded-md border border-slate-800 bg-slate-900 p-5">
          {selected ? (
            <>
              <header className="border-b border-slate-800 pb-4">
                <h2 className="text-xl/8 font-semibold">{selected.title}</h2>
                <p className="mt-1 text-sm/6 text-slate-400">
                  {selected.repository} · {selected.state} · {selected.category ?? "uncategorized"}
                </p>
              </header>
              <div className="prose prose-invert mt-5 max-w-none text-sm/7">
                <ReactMarkdown remarkPlugins={[remarkGfm]}>{selected.markdown}</ReactMarkdown>
              </div>
            </>
          ) : (
            <p className="text-sm/6 text-slate-400">Select a handoff to read it.</p>
          )}
        </article>
      </div>
    </main>
  );
}

const root = document.getElementById("root");
if (!root) throw new Error("Missing #root element");
createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);

