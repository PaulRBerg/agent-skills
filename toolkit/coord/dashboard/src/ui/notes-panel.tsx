import { Collapsible } from "@base-ui/react/collapsible";
import { ChevronDown, NotebookTabs } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { formatRelativeTime, shortenPath } from "@/lib/format";
import { MOTION_DURATION, MOTION_EASE } from "@/lib/motion";
import type { Note } from "@/lib/types";
import { AnimatedValue } from "@/ui/animated-value";

interface NoteGroup {
  repoRoot: string;
  unresolved: Note[];
  resolved: Note[];
}

function groupNotes(notes: Note[]): NoteGroup[] {
  const groups = new Map<string, Note[]>();
  for (const note of notes) {
    const rows = groups.get(note.repo_root) ?? [];
    rows.push(note);
    groups.set(note.repo_root, rows);
  }
  return [...groups]
    .map(([repoRoot, rows]) => ({
      repoRoot,
      unresolved: rows
        .filter((note) => note.resolved_at === null)
        .sort((left, right) => right.created_at - left.created_at),
      resolved: rows
        .filter((note) => note.resolved_at !== null)
        .sort(
          (left, right) => (right.resolved_at ?? 0) - (left.resolved_at ?? 0),
        ),
    }))
    .sort(
      (left, right) =>
        right.unresolved.length - left.unresolved.length ||
        left.repoRoot.localeCompare(right.repoRoot),
    );
}

function NoteRow({
  note,
  now,
  resolved = false,
}: {
  note: Note;
  now: number;
  resolved?: boolean;
}) {
  return (
    <motion.li
      animate={{ opacity: resolved ? 0.5 : 1, y: 0 }}
      className={`border-t border-line-muted py-2 first:border-t-0 ${resolved ? "opacity-50" : ""}`}
      data-motion-item
      exit={{ opacity: 0, y: -4 }}
      initial={{ opacity: 0, y: 5 }}
      layout="position"
      transition={{
        duration: MOTION_DURATION.row,
        ease: MOTION_EASE,
        layout: { duration: MOTION_DURATION.layout, ease: MOTION_EASE },
      }}
    >
      <p className="text-xs/5 text-ink-secondary">{note.text}</p>
      <div className="mt-1 flex items-center justify-between gap-2 font-mono text-[10px]/4 text-muted">
        <span>{note.id}</span>
        <span>
          {formatRelativeTime(note.resolved_at ?? note.created_at, now)}
        </span>
      </div>
    </motion.li>
  );
}

interface NotesPanelProps {
  notes: Note[];
  now: number;
}

export function NotesPanel({ notes, now }: NotesPanelProps) {
  const groups = groupNotes(notes);
  const unresolvedCount = notes.filter(
    (note) => note.resolved_at === null,
  ).length;

  return (
    <section
      className="border-t border-line-strong pt-3"
      aria-labelledby="notes-heading"
    >
      <div className="flex items-center justify-between gap-3">
        <h2
          id="notes-heading"
          className="flex items-center gap-2 text-sm font-semibold"
        >
          <NotebookTabs
            aria-hidden="true"
            className="size-4 text-accent"
            strokeWidth={1.8}
          />
          Notes
        </h2>
        <span className="font-mono text-xs text-muted tabular-nums">
          <AnimatedValue value={unresolvedCount}>
            {unresolvedCount}
          </AnimatedValue>{" "}
          unresolved
        </span>
      </div>

      {groups.length === 0 ? (
        <p className="mt-3 border-l border-line px-3 py-2 text-xs text-muted">
          No repository notes
        </p>
      ) : (
        <div className="mt-3 flex flex-col gap-4">
          <AnimatePresence initial={false} mode="popLayout">
            {groups.map((group) => (
              <motion.div
                animate={{ opacity: 1, y: 0 }}
                className="border-l border-line pl-3"
                data-motion-item
                exit={{ opacity: 0, y: -6 }}
                initial={{ opacity: 0, y: 6 }}
                key={group.repoRoot}
                layout="position"
                transition={{
                  duration: MOTION_DURATION.row,
                  ease: MOTION_EASE,
                  layout: {
                    duration: MOTION_DURATION.layout,
                    ease: MOTION_EASE,
                  },
                }}
              >
                <h3
                  className="truncate font-mono text-[11px]/4 font-semibold"
                  title={group.repoRoot}
                >
                  {shortenPath(group.repoRoot)}
                </h3>
                {group.unresolved.length > 0 ? (
                  <ul className="mt-1">
                    <AnimatePresence initial={false} mode="popLayout">
                      {group.unresolved.map((note) => (
                        <NoteRow key={note.id} note={note} now={now} />
                      ))}
                    </AnimatePresence>
                  </ul>
                ) : (
                  <p className="mt-2 text-xs text-muted">No unresolved notes</p>
                )}

                {group.resolved.length > 0 ? (
                  <Collapsible.Root className="mt-2">
                    <Collapsible.Trigger className="group flex w-full items-center gap-1.5 py-1 text-left font-mono text-[10px]/4 text-muted hover:text-ink focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent">
                      <ChevronDown
                        aria-hidden="true"
                        className="size-3 transition-transform group-data-[panel-open]:rotate-180"
                      />
                      <AnimatedValue value={group.resolved.length}>
                        {group.resolved.length}
                      </AnimatedValue>{" "}
                      resolved
                    </Collapsible.Trigger>
                    <Collapsible.Panel>
                      <ul>
                        <AnimatePresence initial={false} mode="popLayout">
                          {group.resolved.map((note) => (
                            <NoteRow
                              key={note.id}
                              note={note}
                              now={now}
                              resolved
                            />
                          ))}
                        </AnimatePresence>
                      </ul>
                    </Collapsible.Panel>
                  </Collapsible.Root>
                ) : null}
              </motion.div>
            ))}
          </AnimatePresence>
        </div>
      )}
    </section>
  );
}
