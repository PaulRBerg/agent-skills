import { describe, expect, test } from "vitest";
import {
  filterMessages,
  messageRepositories,
  paginateMessages,
  previewMessages,
} from "@/lib/messages";
import type { Message, Session } from "@/lib/types";

function message(
  id: string,
  createdAt: number,
  overrides: Partial<Message> = {},
): Message {
  return {
    id,
    sender_client: "codex",
    sender_session_id: "sender-session",
    recipient_client: "claude",
    recipient_session_id: "recipient-session",
    repo_root: "/repo/alpha",
    text: `message ${id}`,
    created_at: createdAt,
    acknowledged_at: null,
    ...overrides,
  };
}

const sessions: Session[] = [
  {
    client: "codex",
    session_id: "sender-session",
    cwd: "/repo/alpha",
    repo_root: "/repo/alpha",
    state: "working",
    name: null,
    label: "dashboard-agent",
    waiting_for: null,
    pid: 1,
    source: "hook",
    started_at: 1,
    last_seen: 2,
  },
];

describe("previewMessages", () => {
  test("returns the five newest messages without mutating the snapshot order", () => {
    const messages = Array.from({ length: 7 }, (_, index) =>
      message(String(index), index),
    );

    expect(previewMessages(messages).map(({ id }) => id)).toEqual([
      "6",
      "5",
      "4",
      "3",
      "2",
    ]);
    expect(messages.map(({ id }) => id)).toEqual([
      "0",
      "1",
      "2",
      "3",
      "4",
      "5",
      "6",
    ]);
  });
});

describe("filterMessages", () => {
  test("combines text, repository, and acknowledgement filters", () => {
    const messages = [
      message("unread-alpha", 3, { text: "Deploy dashboard motion" }),
      message("read-alpha", 2, {
        text: "Dashboard checks passed",
        acknowledged_at: 4,
      }),
      message("unread-beta", 1, {
        repo_root: "/repo/beta",
        text: "Deploy API",
      }),
    ];

    expect(
      filterMessages(messages, sessions, {
        query: "deploy",
        repoRoot: "/repo/alpha",
        status: "unread",
      }).map(({ id }) => id),
    ).toEqual(["unread-alpha"]);
  });

  test("matches resolved labels and full session identifiers", () => {
    const messages = [message("target", 1)];

    expect(
      filterMessages(messages, sessions, {
        query: "dashboard-agent",
        repoRoot: null,
        status: "all",
      }),
    ).toHaveLength(1);
    expect(
      filterMessages(messages, sessions, {
        query: "recipient-session",
        repoRoot: null,
        status: "all",
      }),
    ).toHaveLength(1);
  });
});

describe("messageRepositories", () => {
  test("returns sorted, unique non-null repositories", () => {
    expect(
      messageRepositories([
        message("beta", 1, { repo_root: "/repo/beta" }),
        message("global", 2, { repo_root: null }),
        message("alpha", 3),
        message("alpha-again", 4),
      ]),
    ).toEqual(["/repo/alpha", "/repo/beta"]);
  });
});

describe("paginateMessages", () => {
  test("uses 20-item pages and clamps a page invalidated by live updates", () => {
    const messages = Array.from({ length: 45 }, (_, index) =>
      message(String(index), index),
    );

    expect(paginateMessages(messages, 2)).toMatchObject({
      page: 2,
      pageCount: 3,
      start: 21,
      end: 40,
      total: 45,
    });
    expect(paginateMessages(messages.slice(0, 3), 3)).toMatchObject({
      page: 1,
      pageCount: 1,
      start: 1,
      end: 3,
      total: 3,
    });
    expect(paginateMessages([], 1)).toMatchObject({ start: 0, end: 0 });
  });
});
