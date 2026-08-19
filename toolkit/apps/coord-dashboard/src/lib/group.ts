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

function sessionRepo(session: Session): string {
  return session.repo_root ?? session.cwd;
}

function withQueuePositions(work: Work[]): WorkWithQueuePosition[] {
  const positions = new Map<string, number>();
  const queuedByRepo = new Map<string, Work[]>();

  for (const item of work) {
    if (item.state !== "queued") continue;
    for (const claim of item.claims) {
      const queued = queuedByRepo.get(claim.repo_root) ?? [];
      queued.push(item);
      queuedByRepo.set(claim.repo_root, queued);
    }
  }

  for (const [repoRoot, queued] of queuedByRepo) {
    queued
      .sort(
        (left, right) =>
          (left.submitted_at ?? Number.POSITIVE_INFINITY) -
            (right.submitted_at ?? Number.POSITIVE_INFINITY) ||
          left.id - right.id,
      )
      .forEach((item, index) =>
        positions.set(`${item.id}:${repoRoot}`, index + 1),
      );
  }

  return work.map((item) => ({
    ...item,
    claims: item.claims.map((claim) => ({
      ...claim,
      ...(positions.has(`${item.id}:${claim.repo_root}`)
        ? { queuePosition: positions.get(`${item.id}:${claim.repo_root}`) }
        : {}),
    })),
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
  const sessionsByKey = new Map(
    snapshot.sessions.map((session) => [
      sessionKey(session.client, session.session_id),
      session,
    ]),
  );
  const workHome = new Map(
    work.map((item) => {
      const session = sessionsByKey.get(sessionKey(item.client, item.session_id));
      const sessionRoot = session ? sessionRepo(session) : undefined;
      const home =
        sessionRoot && item.claims.some((claim) => claim.repo_root === sessionRoot)
          ? sessionRoot
          : [...item.claims]
              .map((claim) => claim.repo_root)
              .sort((left, right) => (left < right ? -1 : left > right ? 1 : 0))[0]!;
      return [item.id, home];
    }),
  );
  const workBySession = new Map(
    work.flatMap((item) => {
      const home = workHome.get(item.id)!;
      const session = sessionsByKey.get(sessionKey(item.client, item.session_id));
      return session && sessionRepo(session) === home
        ? [[sessionKey(item.client, item.session_id), item] as const]
        : [];
    }),
  );
  const delegatesBySession = groupDelegates(snapshot.delegates);

  snapshot.sessions.forEach((session) => roots.add(sessionRepo(session)));
  work.forEach((item) => item.claims.forEach((claim) => roots.add(claim.repo_root)));
  snapshot.findings.forEach((finding) => roots.add(finding.repo_root));
  snapshot.handoffs.forEach((handoff) => roots.add(handoff.repo_root));
  snapshot.messages.forEach((message) => {
    if (message.repo_root) roots.add(message.repo_root);
  });

  return [...roots]
    .map((repoRoot): RepoLaneModel => {
      const sessions = snapshot.sessions
        .filter((session) => sessionRepo(session) === repoRoot)
        .sort(
          (left, right) =>
            right.last_seen - left.last_seen ||
            left.session_id.localeCompare(right.session_id),
        )
        .map((session) => {
          const key = sessionKey(session.client, session.session_id);
          return {
            session,
            work: workBySession.get(sessionKey(session.client, session.session_id)),
            delegates:
              sessionRepo(session) === repoRoot
                ? (delegatesBySession.get(key) ?? [])
                : [],
          };
        });
      const unmatchedWork = work.filter(
        (item) =>
          workHome.get(item.id) === repoRoot &&
          !workBySession.has(sessionKey(item.client, item.session_id)),
      );
      const activity = [
        ...snapshot.sessions
          .filter((session) => sessionRepo(session) === repoRoot)
          .map((session) => session.last_seen),
        ...work
          .filter((item) => item.claims.some((claim) => claim.repo_root === repoRoot))
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
