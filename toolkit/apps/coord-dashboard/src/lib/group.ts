import type {
  Delegate,
  RepoLaneModel,
  Session,
  Snapshot,
  Work,
  WorkWithQueuePosition,
} from "@/lib/types";

function sessionKey(client: string, sessionId: string): string {
  return `${client}:${sessionId}`;
}

function sessionWorkKey(
  client: string,
  sessionId: string,
  repoRoot: string,
): string {
  return `${sessionKey(client, sessionId)}:${repoRoot}`;
}

function sessionRepo(session: Session): string {
  return session.repo_root ?? session.cwd;
}

function withQueuePositions(work: Work[]): WorkWithQueuePosition[] {
  const positions = new Map<number, number>();
  const queuedByRepo = new Map<string, Work[]>();

  for (const item of work) {
    if (item.state !== "queued") continue;
    const queued = queuedByRepo.get(item.repo_root) ?? [];
    queued.push(item);
    queuedByRepo.set(item.repo_root, queued);
  }

  for (const queued of queuedByRepo.values()) {
    queued
      .sort(
        (left, right) =>
          (left.submitted_at ?? Number.POSITIVE_INFINITY) -
            (right.submitted_at ?? Number.POSITIVE_INFINITY) ||
          left.id - right.id,
      )
      .forEach((item, index) => positions.set(item.id, index + 1));
  }

  return work.map((item) => ({
    ...item,
    ...(positions.has(item.id)
      ? { queuePosition: positions.get(item.id) }
      : {}),
  }));
}

function groupDelegates(delegates: Delegate[]): Map<string, Delegate[]> {
  const grouped = new Map<string, Delegate[]>();
  for (const delegate of delegates) {
    const key = sessionKey(delegate.parent_client, delegate.parent_session_id);
    const rows = grouped.get(key) ?? [];
    rows.push(delegate);
    grouped.set(key, rows);
  }
  for (const rows of grouped.values()) {
    rows.sort(
      (left, right) =>
        right.last_seen - left.last_seen ||
        left.agent_id.localeCompare(right.agent_id),
    );
  }
  return grouped;
}

export function groupSnapshotByRepo(snapshot: Snapshot): RepoLaneModel[] {
  const roots = new Set<string>();
  const work = withQueuePositions(snapshot.work);
  const workBySession = new Map(
    work.map((item) => [
      sessionWorkKey(item.client, item.session_id, item.repo_root),
      item,
    ]),
  );
  const delegatesBySession = groupDelegates(snapshot.delegates);

  snapshot.sessions.forEach((session) => roots.add(sessionRepo(session)));
  work.forEach((item) => roots.add(item.repo_root));
  snapshot.findings.forEach((finding) => roots.add(finding.repo_root));
  snapshot.handoffs.forEach((handoff) => roots.add(handoff.repo_root));
  snapshot.messages.forEach((message) => {
    if (message.repo_root) roots.add(message.repo_root);
  });

  return [...roots]
    .map((repoRoot): RepoLaneModel => {
      const sessions = snapshot.sessions
        .filter(
          (session) =>
            sessionRepo(session) === repoRoot ||
            workBySession.has(
              sessionWorkKey(session.client, session.session_id, repoRoot),
            ),
        )
        .sort(
          (left, right) =>
            right.last_seen - left.last_seen ||
            left.session_id.localeCompare(right.session_id),
        )
        .map((session) => {
          const key = sessionKey(session.client, session.session_id);
          return {
            session,
            work: workBySession.get(
              sessionWorkKey(session.client, session.session_id, repoRoot),
            ),
            delegates:
              sessionRepo(session) === repoRoot
                ? (delegatesBySession.get(key) ?? [])
                : [],
          };
        });
      const liveSessionKeys = new Set(
        snapshot.sessions.map((session) =>
          sessionKey(session.client, session.session_id),
        ),
      );
      const unmatchedWork = work.filter(
        (item) =>
          item.repo_root === repoRoot &&
          !liveSessionKeys.has(sessionKey(item.client, item.session_id)),
      );
      const activity = [
        ...snapshot.sessions
          .filter((session) => sessionRepo(session) === repoRoot)
          .map((session) => session.last_seen),
        ...work
          .filter((item) => item.repo_root === repoRoot)
          .map((item) =>
            Math.max(
              item.draft_created_at ?? Number.NEGATIVE_INFINITY,
              item.submitted_at ?? Number.NEGATIVE_INFINITY,
              item.updated_at,
            ),
          ),
        ...snapshot.findings
          .filter((finding) => finding.repo_root === repoRoot)
          .map((finding) => finding.updated_at),
        ...snapshot.messages
          .filter((message) => message.repo_root === repoRoot)
          .map((message) => message.created_at),
      ];

      return {
        repoRoot,
        sessions,
        unmatchedWork,
        handoffCount:
          snapshot.handoffs.find((handoff) => handoff.repo_root === repoRoot)
            ?.count ?? 0,
        lastActivity: Math.max(...activity),
      };
    })
    .sort(
      (left, right) =>
        right.lastActivity - left.lastActivity ||
        left.repoRoot.localeCompare(right.repoRoot),
    );
}
