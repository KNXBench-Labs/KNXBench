# ADR 0032: Order project snapshots at the application boundary

Date: 2026-09-23
Status: Accepted
Session: Goal-completion, T13 review corrections

## Context

T13 exposes server-authoritative modification state. Browser request completion
order is not server snapshot order: an edit response may arrive after the Save
GET that already includes that edit, and an Open response may arrive after an
edit accepted against the replacement project. A local request-generation guard
covers some permutations, but cannot establish the order in which independently
issued server operations actually observed the project.

The review also found that choosing a save path before taking the project lock,
or publishing a replacement's opaque/manufacturer collections after releasing
that lock, can mix two projects. A dirty check performed before a separate
replacement transaction can discard an intervening accepted edit. These are
application-state coherence failures, not KNX import-format or UI styling issues.

Evidence: `apps/knx-server/src/domain.rs` save/replacement paths,
`apps/knx-web/src/App.tsx` publication paths, and the
[T13 review record](../../.ai/logs/2026-09-23_codex__t13_ui_residue_batch_b.md).
These are repository implementation findings, not external format claims.

## Decision

Keep the existing project mutex as the leading coherence boundary. Replacement
must hold it through the dirty predicate, project/history/baseline/save-path
publication and opaque/manufacturer collection replacement. Saving chooses its
destination under the same boundary and establishes a clean baseline only after
the write succeeds. Readers follow the documented lock order. Do not add a second
mutation mutex that every call site must remember independently.

Attach application-owned ordering metadata to public project snapshots while the
project lock is held. A response carries the order of the snapshot it actually
contains, not the time at which its HTTP handler finally sends it. All client
publication paths must reject superseded snapshots before altering the tree,
save metadata, selection or session-context records. Pure domain projection does
not manufacture live application ordering information.

This metadata is transient. It is neither a KNX project version nor a persisted
field in `.knxdb`; native project schema version 9 remains unchanged. It does not
implement optimistic write preconditions, collaborative editing or server push.
Implementation must define its process-lifetime and legacy/unversioned boundaries
explicitly rather than treating an absent ordering token as fresh evidence.

The lifetime boundary is `ProjectTree.server_incarnation` together with
`snapshot_revision`. Each `AppState` creates a non-secret 16-byte identity from
the existing OS randomness dependency and encodes it as Base64url. Failure to
obtain randomness refuses initialization rather than inventing an identity.
Pure/offline projections omit both fields. The browser accepts a new
incarnation even at revision 1 after the old incarnation's revision 100;
within an incarnation, older revisions are still refused.

App state and browser context records remember retired incarnation identities
so a late old-process response cannot switch the active ordering space back.
The browser record preserves that retirement information across reloads and
same-profile windows. Legacy records without an incarnation remain ordered
among themselves, but cannot supersede an accepted modern record. Retirement
history is browser-local metadata, grows with observed server incarnations,
and is not a durable multi-server consensus mechanism.

Bus Start/Poll/Stop metadata carries `serverIncarnation`; browser session
identity is the pair `(serverIncarnation, sessionId)`, obtained from those
responses rather than inferred from a stale project record. Missing or
mismatched incarnation is `unverified`, never evidence of synchronization.
Controller execution of the formerly failing restart probe now accepts B/1,
rejects retired A/101 and refuses to verify A's reused numeric session ID in B.

For the bus context, a confirmed application-level style publication may identify
the particular active session whose complete context was replaced. The UI may
rebase only a matching recorded session using an accepted, current response.
An arbitrary name/DPT edit, another session or a stale response is not evidence
that a context refresh occurred. The project mutex must not span an asynchronous
bus-session lock wait; the existing style-publication serialization remains.

## Alternatives considered

- **More browser request counters:** rejected as the sole ordering mechanism.
  Request initiation/completion order cannot prove server mutation order.
- **Serialize every browser request:** rejected. This does not coordinate other
  clients or server-side replacement and needlessly serializes independent reads.
- **Persist revisions in the KNX domain/store:** rejected. Transport snapshot
  order has no meaning in imported project data and needs no schema migration.
- **Rebase every bus fingerprint on every edit:** rejected. It would turn an
  observed stale context into a false `synced` state without server evidence.

## Consequences

- Dirty state remains derived from the saved baseline, not inferred in React.
- Save and replacement cannot combine one project's path or passthrough with
  another project's normalized data.
- Reverse-response-order regressions become deterministic API/UI tests.
- Transport metadata and its defaults require coverage alongside generated
  TypeScript bindings; no core/UI dependency is introduced.
- Cross-client notification, durable session identity and hardware verification
  remain separate concerns. Passing these tests is not a claim of multi-user
  conflict resolution or full ETS compatibility.
