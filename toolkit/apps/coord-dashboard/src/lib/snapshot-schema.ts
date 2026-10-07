import { Schema } from "effect";

import type { Snapshot } from "./types.js";

const Integer = Schema.Finite.check(
  Schema.makeFilter(Number.isInteger, { expected: "an integer" })
);
const UnsignedInteger = Integer.check(Schema.isGreaterThanOrEqualTo(0));
const PositiveInteger = Integer.check(Schema.isGreaterThan(0));
const NullableString = Schema.NullOr(Schema.String);
const Client = Schema.Literals(["claude", "codex"]);
const SessionIdentity = Schema.Struct({
  client: Client,
  session_id: Schema.String,
});
const WorkScope = Schema.Struct({
  path: Schema.String,
  kind: Schema.Literals(["exact", "recursive"]),
});
const WorkClaim = Schema.Struct({
  repo_root: Schema.String,
  blocked_reason: Schema.optional(NullableString),
  scope_count: PositiveInteger,
  scopes: Schema.mutable(Schema.Array(WorkScope)).check(Schema.isMinLength(1)),
}).check(
  Schema.makeFilter((claim) =>
    claim.scope_count === claim.scopes.length
      ? undefined
      : { path: ["scope_count"], issue: "must match scopes length" }
  )
);
const Work = Schema.Struct({
  ...SessionIdentity.fields,
  id: Integer,
  label: Schema.String,
  state: Schema.Literals(["active", "queued"]),
  blocked_reason: Schema.optional(NullableString),
  scope_count: PositiveInteger,
  submitted_at: Schema.Finite,
  updated_at: Schema.Finite,
  scopes: Schema.optionalKey(Schema.Undefined.annotate({ message: "belongs to a work claim" })),
  claims: Schema.mutable(Schema.Array(WorkClaim)).check(Schema.isMinLength(1)),
}).check(
  Schema.makeFilter((work) => {
    if (work.state === "queued" && typeof work.blocked_reason !== "string") {
      return {
        path: ["blocked_reason"],
        issue: "must describe queued work",
      };
    }
    return work.scope_count === work.claims.reduce((total, claim) => total + claim.scope_count, 0)
      ? undefined
      : { path: ["scope_count"], issue: "must equal the claim scope total" };
  })
);
const DraftClaim = Schema.Struct({
  repo_root: Schema.String,
  scope_count: PositiveInteger,
  scopes: Schema.optionalKey(
    Schema.Undefined.annotate({ message: "must be omitted for a draft claim" })
  ),
});
const Draft = Schema.Struct({
  id: Schema.String,
  name: NullableString,
  owner: Schema.NullOr(SessionIdentity),
  label: Schema.String,
  created_at: Schema.Finite,
  updated_at: Schema.Finite,
  claims: Schema.mutable(Schema.Array(DraftClaim)).check(Schema.isMinLength(1)),
}).check(
  Schema.makeFilter((draft) =>
    (draft.name !== null) === (draft.owner !== null)
      ? "must have exactly one of name or owner"
      : undefined
  )
);
const Session = Schema.Struct({
  ...SessionIdentity.fields,
  cwd: Schema.String,
  repo_root: NullableString,
  state: Schema.Literals(["idle", "in_flight", "unknown", "waiting", "working"]),
  callsign: Schema.optional(NullableString),
  name: NullableString,
  waiting_for: NullableString,
  permission_mode: Schema.optional(NullableString),
  coordination_waived: Schema.Boolean,
  delegate_count: Schema.optional(UnsignedInteger),
  pid: Schema.NullOr(UnsignedInteger),
  source: Schema.String,
  started_at: Schema.Finite,
  last_seen: Schema.Finite,
});
const Provider = Schema.Struct({
  client: Client,
  ok: Schema.Boolean,
  source: Schema.String,
  enabled: Schema.Boolean,
  dropped: UnsignedInteger,
  error: NullableString,
});
const Finding = Schema.Struct({
  id: Schema.String,
  repo_root: Schema.String,
  summary: Schema.String,
  kind: Schema.NullOr(Schema.Literals(["bug", "docs", "improvement"])),
  state: Schema.Literals(["pending", "handed-off", "fixed", "stale", "rejected", "duplicate"]),
  paths: Schema.mutable(Schema.Array(Schema.String)),
  created_at: Schema.Finite,
  updated_at: Schema.Finite,
  terminal_at: Schema.NullOr(Schema.Finite),
  handoff_path: NullableString,
  commit_oid: NullableString,
  canonical_id: NullableString,
  sighting_count: PositiveInteger,
  triaging: Schema.Boolean,
});
const Delegate = Schema.Struct({
  parent_client: Client,
  parent_session_id: Schema.String,
  agent_id: Schema.String,
  agent_type: NullableString,
  state: Schema.String,
  last_seen: Schema.Finite,
});
const Message = Schema.Struct({
  id: Schema.String,
  sender_client: Client,
  sender_session_id: Schema.String,
  sender_callsign: Schema.optional(NullableString),
  recipient_client: Client,
  recipient_session_id: Schema.String,
  recipient_callsign: Schema.optional(NullableString),
  repo_root: NullableString,
  text: Schema.String,
  created_at: Schema.Finite,
  acknowledged_at: Schema.NullOr(Schema.Finite),
});
const SnapshotScope = Schema.Union([
  Schema.Struct({
    kind: Schema.Literal("machine"),
    repo_root: Schema.optionalKey(
      Schema.Undefined.annotate({ message: "must be omitted for machine scope" })
    ),
  }),
  Schema.Struct({
    kind: Schema.Literals(["cwd", "repo"]),
    repo_root: Schema.String,
  }),
]);

export const SnapshotSchema = Schema.Struct({
  schema_version: Schema.Literal(8),
  complete: Schema.Boolean,
  scope: SnapshotScope,
  self: Schema.NullOr(SessionIdentity),
  providers: Schema.mutable(Schema.Array(Provider)),
  sessions: Schema.mutable(Schema.Array(Session)),
  work: Schema.mutable(Schema.Array(Work)),
  drafts: Schema.mutable(Schema.Array(Draft)),
  findings: Schema.mutable(Schema.Array(Finding)),
  handoffs: Schema.mutable(
    Schema.Array(
      Schema.Struct({
        repo_root: Schema.String,
        count: PositiveInteger,
      })
    )
  ),
  delegates: Schema.mutable(Schema.Array(Delegate)),
  outside_scope: Schema.Struct({
    sessions: UnsignedInteger,
    directories: UnsignedInteger,
  }),
  messages: Schema.mutable(Schema.Array(Message)),
  generated_at: Schema.String.check(
    Schema.makeFilter((value) => !Number.isNaN(Date.parse(value)), { expected: "an ISO timestamp" })
  ),
  generation: UnsignedInteger,
}) satisfies Schema.Schema<Snapshot>;
