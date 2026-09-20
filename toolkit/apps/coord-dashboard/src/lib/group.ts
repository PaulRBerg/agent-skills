import { shortSessionId } from "@/lib/format";
import type {
  Delegate,
  RepoLaneDraft,
  RepoLaneModel,
  Session,
  Snapshot,
  SnapshotDraft,
  Work,
  WorkWithQueuePosition,
} from "@/lib/types";

function sessionKey(client: string, sessionId: string): string {
  return `${client}:${sessionId}`;
}

function sessionRepo(session: Session): string {
  return session.repo_root ?? session.cwd;
}

function sortedFirstRoot(repoRoots: string[]): string {
  return [...repoRoots].sort((left, right) =>
    left < right ? -1 : left > right ? 1 : 0,
  )[0]!;
}

function draftHome(
  draft: SnapshotDraft,
  sessionsByKey: Map<string, Session>,
): string {
  const ownerSession = draft.owner
    ? sessionsByKey.get(sessionKey(draft.owner.client, draft.owner.session_id))
    : undefined;
  const ownerRoot = ownerSession ? sessionRepo(ownerSession) : undefined;
  return ownerRoot &&
    draft.claims.some((claim) => claim.repo_root === ownerRoot)
    ? ownerRoot
    : sortedFirstRoot(draft.claims.map((claim) => claim.repo_root));
}

function draftWho(
  draft: SnapshotDraft,
  sessionsByKey: Map<string, Session>,
): string {
  if (draft.name !== null) return draft.name;
  const owner = draft.owner!;
  const session = sessionsByKey.get(sessionKey(owner.client, owner.session_id));
  if (session?.callsign) return session.callsign;
  return `${owner.client}/${shortSessionId(owner.session_id)}`;
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
          : sortedFirstRoot(item.claims.map((claim) => claim.repo_root));
      return [item.id, home];
    }),
  );
  const draftHomeById = new Map(
    snapshot.drafts.map((draft) => [draft.id, draftHome(draft, sessionsByKey)]),
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
  snapshot.drafts.forEach((draft) =>
    draft.claims.forEach((claim) => roots.add(claim.repo_root)),
  );
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
      const drafts: RepoLaneDraft[] = snapshot.drafts
        .filter((draft) => draftHomeById.get(draft.id) === repoRoot)
        .map((draft) => ({
          draft,
          who: draftWho(draft, sessionsByKey),
          scopeCount: draft.claims.reduce(
            (total, claim) => total + claim.scope_count,
            0,
          ),
        }));
      const activity = [
        ...snapshot.sessions
          .filter((session) => sessionRepo(session) === repoRoot)
          .map((session) => session.last_seen),
        ...work
          .filter((item) => item.claims.some((claim) => claim.repo_root === repoRoot))
          .map((item) =>
            Math.max(item.submitted_at ?? Number.NEGATIVE_INFINITY, item.updated_at),
          ),
        ...snapshot.drafts
          .filter((draft) =>
            draft.claims.some((claim) => claim.repo_root === repoRoot),
          )
          .map((draft) => draft.updated_at),
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
        drafts,
        handoffCount:
          snapshot.handoffs.find((handoff) => handoff.repo_root === repoRoot)
            ?.count ?? 0,
        lastActivity: activity.length > 0 ? Math.max(...activity) : null,
      };
    })
    .sort(
      (left, right) =>
        (right.lastActivity ?? Number.NEGATIVE_INFINITY) -
          (left.lastActivity ?? Number.NEGATIVE_INFINITY) ||
        left.repoRoot.localeCompare(right.repoRoot),
    );
}
