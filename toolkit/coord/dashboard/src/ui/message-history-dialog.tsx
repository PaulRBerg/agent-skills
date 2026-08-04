import { useEffect, useMemo, useRef, useState } from "react";
import { Dialog } from "@base-ui/react/dialog";
import { ChevronLeft, ChevronRight, ListFilter, Search, X } from "lucide-react";
import { AnimatePresence } from "motion/react";
import { shortenPath } from "@/lib/format";
import {
  filterMessages,
  messageRepositories,
  paginateMessages,
  type MessageStatusFilter,
} from "@/lib/messages";
import type { Message, Session } from "@/lib/types";
import { MessageRow } from "@/ui/message-row";

const statusFilters: Array<{
  label: string;
  value: MessageStatusFilter;
}> = [
  { label: "All", value: "all" },
  { label: "Unread", value: "unread" },
  { label: "Acknowledged", value: "acknowledged" },
];

interface MessageHistoryDialogProps {
  messages: Message[];
  now: number;
  sessions: Session[];
}

export function MessageHistoryDialog({
  messages,
  now,
  sessions,
}: MessageHistoryDialogProps) {
  const [query, setQuery] = useState("");
  const [repoRoot, setRepoRoot] = useState<string | null>(null);
  const [status, setStatus] = useState<MessageStatusFilter>("all");
  const [page, setPage] = useState(1);
  const resultsRef = useRef<HTMLDivElement>(null);
  const repositories = useMemo(() => messageRepositories(messages), [messages]);
  const filteredMessages = useMemo(
    () => filterMessages(messages, sessions, { query, repoRoot, status }),
    [messages, query, repoRoot, sessions, status],
  );
  const messagePage = useMemo(
    () => paginateMessages(filteredMessages, page),
    [filteredMessages, page],
  );

  useEffect(() => {
    if (messagePage.page !== page) {
      setPage(messagePage.page);
      resultsRef.current?.scrollTo({ top: 0 });
    }
  }, [messagePage.page, page]);

  const showPage = (nextPage: number) => {
    setPage(nextPage);
    resultsRef.current?.scrollTo({ top: 0 });
  };

  const resetFilters = () => {
    setQuery("");
    setRepoRoot(null);
    setStatus("all");
    showPage(1);
  };

  return (
    <Dialog.Root
      onOpenChange={(open) => {
        if (!open) resetFilters();
      }}
    >
      <Dialog.Trigger className="group flex w-full items-center justify-between gap-3 px-3 py-2 text-left text-xs font-medium text-accent transition-colors hover:bg-surface-muted focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent">
        <span>View all {messages.length} messages</span>
        <ChevronRight
          aria-hidden="true"
          className="size-3.5 transition-transform group-hover:translate-x-0.5"
        />
      </Dialog.Trigger>

      <Dialog.Portal>
        <Dialog.Backdrop className="fixed inset-0 z-(--z-modal) min-h-dvh bg-black/35 transition-opacity duration-200 data-ending-style:opacity-0 data-starting-style:opacity-0 motion-reduce:transition-none dark:bg-black/60 supports-[-webkit-touch-callout:none]:absolute" />
        <Dialog.Viewport className="fixed inset-0 z-(--z-modal) flex min-h-dvh items-end justify-center sm:items-center sm:p-4">
          <Dialog.Popup className="flex h-dvh w-full flex-col border-line-strong bg-surface text-ink shadow-2xl transition-[transform,opacity] duration-200 ease-out data-ending-style:translate-y-2 data-ending-style:opacity-0 data-starting-style:translate-y-2 data-starting-style:opacity-0 motion-reduce:transform-none motion-reduce:transition-none sm:h-[min(48rem,calc(100dvh-2rem))] sm:max-w-5xl sm:border sm:data-ending-style:translate-y-0 sm:data-ending-style:scale-[0.985] sm:data-starting-style:translate-y-0 sm:data-starting-style:scale-[0.985]">
            <div className="shrink-0 border-b border-line-strong bg-surface">
              <div className="flex items-start justify-between gap-4 px-4 py-4 sm:px-5">
                <div className="min-w-0">
                  <Dialog.Title className="text-base font-semibold">
                    Message history
                  </Dialog.Title>
                  <Dialog.Description className="mt-1 text-xs/5 text-muted">
                    Search the messages currently retained by the local
                    coordination ledger.
                  </Dialog.Description>
                </div>
                <Dialog.Close
                  aria-label="Close message history"
                  className="inline-flex size-9 shrink-0 items-center justify-center border border-line text-muted transition-colors hover:bg-surface-muted hover:text-ink focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
                  title="Close"
                >
                  <X aria-hidden="true" className="size-4" />
                </Dialog.Close>
              </div>

              <div className="grid gap-3 border-t border-line-muted px-4 py-3 sm:px-5 lg:grid-cols-[minmax(15rem,1fr)_auto_minmax(12rem,16rem)] lg:items-center">
                <label className="relative block min-w-0">
                  <span className="sr-only">Search messages</span>
                  <Search
                    aria-hidden="true"
                    className="pointer-events-none absolute left-3 top-1/2 size-3.5 -translate-y-1/2 text-muted"
                  />
                  <input
                    className="h-9 w-full border border-line bg-canvas pl-9 pr-3 text-xs text-ink outline-hidden placeholder:text-muted focus:border-accent focus:outline-2 focus:outline-offset-1 focus:outline-accent"
                    onChange={(event) => {
                      setQuery(event.target.value);
                      showPage(1);
                    }}
                    placeholder="Search messages, agents, or paths"
                    type="search"
                    value={query}
                  />
                </label>

                <div
                  aria-label="Message status"
                  className="flex min-w-0 overflow-x-auto border border-line p-0.5"
                  role="group"
                >
                  {statusFilters.map((filter) => {
                    const selected = status === filter.value;
                    return (
                      <button
                        aria-pressed={selected}
                        className={`h-8 shrink-0 px-2.5 font-mono text-[11px]/4 transition-colors focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent ${selected ? "bg-accent text-surface" : "text-muted hover:bg-surface-muted hover:text-ink"}`}
                        key={filter.value}
                        onClick={() => {
                          setStatus(filter.value);
                          showPage(1);
                        }}
                        type="button"
                      >
                        {filter.label}
                      </button>
                    );
                  })}
                </div>

                <label className="flex min-w-0 items-center gap-2">
                  <ListFilter
                    aria-hidden="true"
                    className="size-3.5 shrink-0 text-muted"
                  />
                  <span className="sr-only">Repository</span>
                  <select
                    className="h-9 min-w-0 flex-1 border border-line bg-canvas px-2 font-mono text-[11px]/4 text-ink outline-hidden focus:border-accent focus:outline-2 focus:outline-offset-1 focus:outline-accent"
                    onChange={(event) => {
                      setRepoRoot(event.target.value || null);
                      showPage(1);
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
            </div>

            <div
              className="min-h-0 flex-1 overflow-y-auto overscroll-contain px-4 py-2 sm:px-5"
              ref={resultsRef}
            >
              {messagePage.items.length === 0 ? (
                <div className="flex min-h-56 items-center justify-center border-y border-line-muted px-4 text-center">
                  <div>
                    <Search
                      aria-hidden="true"
                      className="mx-auto size-5 text-muted"
                    />
                    <p className="mt-3 text-sm font-medium">
                      No messages match these filters
                    </p>
                    <button
                      className="mt-2 text-xs font-medium text-accent hover:underline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
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
                      <MessageRow
                        key={message.id}
                        message={message}
                        now={now}
                        sessions={sessions}
                        showRepository
                      />
                    ))}
                  </AnimatePresence>
                </ol>
              )}
            </div>

            <div className="flex shrink-0 items-center justify-between gap-3 border-t border-line-strong bg-surface-muted px-4 py-3 sm:px-5">
              <p
                aria-live="polite"
                className="font-mono text-[11px]/4 text-muted tabular-nums"
              >
                {messagePage.start}–{messagePage.end} of {messagePage.total}
              </p>
              <div className="flex items-center gap-2">
                <button
                  className="inline-flex h-8 items-center gap-1 border border-line bg-surface px-2.5 text-xs font-medium disabled:cursor-not-allowed disabled:opacity-40 enabled:hover:bg-canvas focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
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
                  className="inline-flex h-8 items-center gap-1 border border-line bg-surface px-2.5 text-xs font-medium disabled:cursor-not-allowed disabled:opacity-40 enabled:hover:bg-canvas focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
                  disabled={messagePage.page === messagePage.pageCount}
                  onClick={() => showPage(messagePage.page + 1)}
                  type="button"
                >
                  Next
                  <ChevronRight aria-hidden="true" className="size-3.5" />
                </button>
              </div>
            </div>
          </Dialog.Popup>
        </Dialog.Viewport>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
