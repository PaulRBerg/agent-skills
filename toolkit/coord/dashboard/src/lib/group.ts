import type {
  Claim,
  ClaimWithQueuePosition,
  Delegate,
  RepoLaneModel,
  Session,
  Snapshot,
} from "@/lib/types";

function sessionKey(client: string, sessionId: string): string {
  return `${client}:${sessionId}`;
}

function sessionRepo(session: Session): string {
  return session.repo_root ?? session.cwd;
}

function withQueuePositions(claims: Claim[]): ClaimWithQueuePosition[] {
  const positions = new Map<number, number>();
  const queuedByRepo = new Map<string, Claim[]>();

  for (const claim of claims) {
    if (claim.state !== "queued") continue;
    const queued = queuedByRepo.get(claim.repo_root) ?? [];
    queued.push(claim);
    queuedByRepo.set(claim.repo_root, queued);
  }

  for (const queued of queuedByRepo.values()) {
    queued
      .sort(
        (left, right) =>
          left.created_at - right.created_at || left.id - right.id,
      )
      .forEach((claim, index) => positions.set(claim.id, index + 1));
  }

  return claims.map((claim) => ({
    ...claim,
    ...(positions.has(claim.id)
      ? { queuePosition: positions.get(claim.id) }
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
  const claims = withQueuePositions(snapshot.claims);
  const claimBySession = new Map(
    claims.map((claim) => [sessionKey(claim.client, claim.session_id), claim]),
  );
  const delegatesBySession = groupDelegates(snapshot.delegates);

  snapshot.sessions.forEach((session) => roots.add(sessionRepo(session)));
  claims.forEach((claim) => roots.add(claim.repo_root));
  snapshot.notes.forEach((note) => roots.add(note.repo_root));
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
            claim: claimBySession.get(key),
            delegates: delegatesBySession.get(key) ?? [],
          };
        });
      const sessionKeys = new Set(
        sessions.map(({ session }) =>
          sessionKey(session.client, session.session_id),
        ),
      );
      const unmatchedClaims = claims.filter(
        (claim) =>
          claim.repo_root === repoRoot &&
          !sessionKeys.has(sessionKey(claim.client, claim.session_id)),
      );
      const activity = [
        ...sessions.map(({ session }) => session.last_seen),
        ...claims
          .filter((claim) => claim.repo_root === repoRoot)
          .map((claim) => Math.max(claim.created_at, claim.updated_at)),
        ...snapshot.notes
          .filter((note) => note.repo_root === repoRoot)
          .map((note) => note.resolved_at ?? note.created_at),
        ...snapshot.messages
          .filter((message) => message.repo_root === repoRoot)
          .map((message) => message.created_at),
      ];

      return {
        repoRoot,
        sessions,
        unmatchedClaims,
        lastActivity: Math.max(...activity),
      };
    })
    .sort(
      (left, right) =>
        right.lastActivity - left.lastActivity ||
        left.repoRoot.localeCompare(right.repoRoot),
    );
}
