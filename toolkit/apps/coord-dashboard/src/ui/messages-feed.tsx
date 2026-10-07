import { ChevronLeft, ChevronRight, ListFilter, MessageSquareText, Search } from "lucide-react";
import { AnimatePresence } from "motion/react";
import { useEffect, useRef, useState } from "react";
import { tv } from "tailwind-variants";

import { shortenPath } from "@/lib/format.js";
import { filterMessages, messageRepositories, paginateMessages } from "@/lib/messages.js";
import type { MessageStatusFilter } from "@/lib/messages.js";
import type { Message } from "@/lib/types.js";
import { AnimatedValue } from "@/ui/animated-value.js";
import { MessageRow } from "@/ui/message-row.js";

const statusButton = tv({
  base: "h-8 shrink-0 cursor-pointer rounded-md px-2.5 text-[11px]/4 transition-colors focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent",
  variants: {
    selected: {
      true: "bg-accent text-surface",
      false: "text-muted hover:bg-surface-muted hover:text-ink",
    },
  },
});

const statusFilters: {
  label: string;
  value: MessageStatusFilter;
}[] = [
  { label: "All", value: "all" },
  { label: "Unread", value: "unread" },
  { label: "Acknowledged", value: "acknowledged" },
];

type MessagesFeedProps = {
  messages: Message[];
  now: number;
};

export function MessagesFeed({ messages, now }: MessagesFeedProps) {
  const [query, setQuery] = useState("");
  const [repoRoot, setRepoRoot] = useState<string | null>(null);
  const [status, setStatus] = useState<MessageStatusFilter>("all");
  const [page, setPage] = useState(1);
  const resultsRef = useRef<HTMLElement>(null);
  const repositories = messageRepositories(messages);
  const filteredMessages = filterMessages(messages, { query, repoRoot, status });
  const messagePage = paginateMessages(filteredMessages, page);

  useEffect(() => {
    if (messagePage.page !== page) {
      setPage(messagePage.page);
      resultsRef.current?.scrollIntoView({ block: "start" });
    }
  }, [messagePage.page, page]);

  const showPage = (nextPage: number) => {
    setPage(nextPage);
    resultsRef.current?.scrollIntoView({ block: "start" });
  };

  const resetFilters = () => {
    setQuery("");
    setRepoRoot(null);
    setStatus("all");
    showPage(1);
  };

  return (
    <section
      aria-labelledby="messages-heading"
      className="scroll-mt-4 overflow-hidden rounded-2xl border border-line bg-surface shadow-panel"
      ref={resultsRef}
    >
      <div className="flex items-center justify-between gap-3 px-4 pt-4 sm:px-5">
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
      <p className="mt-1 mb-4 px-4 text-xs/5 text-muted sm:px-5">
        Search the messages currently retained by the local coordination ledger.
      </p>
      <div className="grid gap-3 border-t border-line-muted px-4 py-3 sm:px-5 lg:grid-cols-[minmax(15rem,1fr)_auto_minmax(12rem,16rem)] lg:items-center">
        <label className="relative block min-w-0">
          <span className="sr-only">Search messages</span>
          <Search
            aria-hidden="true"
            className="pointer-events-none absolute top-1/2 left-3 size-3.5 -translate-y-1/2 text-muted"
          />
          <input
            className="h-9 w-full rounded-lg border border-line bg-canvas pr-3 pl-9 text-xs text-ink outline-hidden placeholder:text-muted focus:border-accent focus:outline-2 focus:outline-offset-1 focus:outline-accent"
            onChange={(event) => {
              setQuery(event.target.value);
              setPage(1);
            }}
            placeholder="Search messages, agents, or paths"
            type="search"
            value={query}
          />
        </label>

        <div
          aria-label="Message status"
          className="flex min-w-0 overflow-x-auto rounded-lg border border-line p-0.5"
          role="group"
        >
          {statusFilters.map((filter) => {
            const selected = status === filter.value;
            return (
              <button
                aria-pressed={selected}
                className={statusButton({ selected })}
                key={filter.value}
                onClick={() => {
                  setStatus(filter.value);
                  setPage(1);
                }}
                type="button"
              >
                {filter.label}
              </button>
            );
          })}
        </div>

        <label className="flex min-w-0 items-center gap-2">
          <ListFilter aria-hidden="true" className="size-3.5 shrink-0 text-muted" />
          <span className="sr-only">Repository</span>
          <select
            className="h-9 min-w-0 flex-1 rounded-lg border border-line bg-canvas px-2 font-mono text-[11px]/4 text-ink outline-hidden focus:border-accent focus:outline-2 focus:outline-offset-1 focus:outline-accent"
            onChange={(event) => {
              setRepoRoot(event.target.value || null);
              setPage(1);
            }}
            value={repoRoot ?? ""}
          >
            <option value="">All repositories</option>
            {repositories.map((repository) => (
              <option key={repository} value={repository}>
                {shortenPath(repository)}
              </option>
            ))}
          </select>
        </label>
      </div>
      <div className="px-4 py-2 sm:px-5">
        {messagePage.items.length === 0 ? (
          <div className="flex min-h-56 items-center justify-center border-y border-line-muted px-4 text-center">
            <div>
              <Search aria-hidden="true" className="mx-auto size-5 text-muted" />
              <p className="mt-3 text-sm font-medium">
                {messages.length === 0
                  ? "No coordination messages"
                  : "No messages match these filters"}
              </p>
              <button
                className="mt-2 cursor-pointer text-xs font-medium text-accent hover:underline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
                onClick={resetFilters}
                type="button"
              >
                Clear filters
              </button>
            </div>
          </div>
        ) : (
          <ol className="border-l border-line">
            <AnimatePresence initial={false} mode="popLayout">
              {messagePage.items.map((message) => (
                <MessageRow key={message.id} message={message} now={now} showRepository />
              ))}
            </AnimatePresence>
          </ol>
        )}
      </div>
      <div className="flex flex-wrap items-center justify-between gap-3 border-t border-line-strong bg-surface-muted px-4 py-3 sm:px-5">
        <p aria-live="polite" className="font-mono text-[11px]/4 text-muted tabular-nums">
          {messagePage.start}–{messagePage.end} of {messagePage.total}
        </p>
        <div className="flex items-center gap-2">
          <button
            className="inline-flex h-8 cursor-pointer items-center gap-1 rounded-lg border border-line bg-surface px-2.5 text-xs font-medium focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent enabled:hover:bg-canvas disabled:cursor-not-allowed disabled:opacity-40"
            disabled={messagePage.page === 1}
            onClick={() => showPage(messagePage.page - 1)}
            type="button"
          >
            <ChevronLeft aria-hidden="true" className="size-3.5" />
            Previous
          </button>
          <span className="font-mono text-[11px]/4 text-muted tabular-nums">
            {messagePage.page}/{messagePage.pageCount}
          </span>
          <button
            className="inline-flex h-8 cursor-pointer items-center gap-1 rounded-lg border border-line bg-surface px-2.5 text-xs font-medium focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent enabled:hover:bg-canvas disabled:cursor-not-allowed disabled:opacity-40"
            disabled={messagePage.page === messagePage.pageCount}
            onClick={() => showPage(messagePage.page + 1)}
            type="button"
          >
            Next
            <ChevronRight aria-hidden="true" className="size-3.5" />
          </button>
        </div>
      </div>
    </section>
  );
}
