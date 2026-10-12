# Project notes HTTP editing — bounded AP4 adapter

## Exact ownership

Own `apps/knx-server/src/note_routes.rs`, one module/router merge in
`apps/knx-server/src/lib.rs`, `apps/knx-server/tests/http_project_notes.rs`, this
contract and own handover/log. No shared domain state, global styles, language
catalogues, shell/navigation, protocol or manufacturer data changes.
Foundation/service integration is frozen separately; this adapter uses the
existing application service rather than writing SQL or inventing commands.

## Wire and authority

All five routes sit under the existing authenticated API guard. Disabled-auth
in-process desktop uses the existing explicit local policy; these routes do not
introduce another account or permission system.

- `GET /api/project/notes`: ordered notes with stable `id`, typed `target`, literal
  `title`/`body`, explicit `includeInDocumentation`, `position`, and actual
  `serverIncarnation`/`projectIncarnation`/`snapshotRevision` plus persistence mode.
- `POST /api/project/notes/preview`: those exact three identity fields and `action`.
  Actions are strict tagged `create` (draft, position), `replace` (id, draft),
  `remove` (id) and `move` (id, position). Draft includes all authored fields;
  target is `{kind}` for project or `{kind,id}` for a typed entity. IDs are
  domain IDs, never physical/group addresses or source RefIds.
- Preview returns a random token, exact validated action, previous note (nullable),
  predicted created ID (nullable), no-op and expiry. It does not reserve live IDs,
  mutate notes/history/files, convert source instructions or fetch resources.
- `POST /api/project/notes/apply`: only `token` and `confirmed`; unknown fields
  (including replacement draft/command) are refused. Explicit confirmation is
  mandatory. Failed consent retains the plan; success consumes it, including
  no-op. Apply returns changed, native generation, actual revision and mode.
- `POST /api/project/notes/cancel`: exact token; discards only that plan.
- `GET /api/project/notes/targets` (added 2026-10-12): read-only catalogue of
  entities a note may be attached to, from `knx_app::note_targets`, as
  `{targets:[{target,owner,parts}], serverIncarnation, projectIncarnation,
  snapshotRevision}`. `target` uses the same typed `{kind,id}` grammar (never
  `project`); `parts`/`owner` are locale-neutral label parts (address/number, name;
  `owner` only for communication objects). Only targets that `ProjectNoteTarget`
  validation accepts are listed. Without an open project: 400, like the read.

One router-local pending plan, 300-second monotonic TTL, 24 random token bytes;
newer valid preview replaces an older one even from another authenticated client.
There is no per-user collaboration model. Only token holders can apply/cancel;
no read endpoint exposes the token. A cloned Router shares its cache, a newly
constructed Router does not. Cache is in memory, so restart invalidates it.

Requests are bounded to 1 MiB; domain text/count/target/budget checks remain the
authority. Literal whitespace/Unicode/line breaks are preserved. Unknown actions,
target kinds, properties, malformed IDs and wrong types are refused. Model,
original-source, manifest, incarnation/revision and native compare-and-write
checks precede publication. API errors omit host paths and persistence internals.

## Persistence and exclusions

Session-only without a native file; with a native target, private working-state
and history persistence precedes live model/history publication. This is not an
implicit root Save. Use existing Undo/Redo and explicit Save/Open routes. Failure
must leave live notes, allocators, original context, history and saved file intact.
Typed target deletion policy stays in Core; no silent cascade or retargeting.

Static HelpPanel/HelpTip content is separate and reused by the later isolated UI.
HTTP is not UI/shell adoption, full AP4, ETS export, hardware or Secure support.
The current local adapter passes 11 focused HTTP tests and two module controls;
three registered confirmation/TTL/request-context guard negatives are caught.
Tests include real session-cookie login, native working-state recovery with both
stacks, explicit root Save, concurrent native writer and real SQLite lock/history
budget refusals. This is active-session self-review, not independent approval.
The whole owning server suite passes 811 tests (47 explicitly ignored), strict
Clippy, format and build on frozen adapter inputs. Fresh published-prerequisite
metadata integration was followed by 19 actual source-frozen current-web/
workspace/build/repository stages and 11 selected native-project regressions,
recorded in [the source-bound receipt](../evidence/project-notes-http-2026-10-11.json).
Final publication is a separate exact-ref/readback step; this is not full AP4.
A first guard-control runner failed during an ambiguous restore anchor; canonical
bytes were subsequently restored and verified, then only its unstarted third
control was executed. This orchestration refusal is not a failed product test.
External native conflicts currently use the sanitized `notePersistenceFailed`
422 response, while live snapshot/revision/token conflicts use `noteStale` 409.
Neither error publishes memory or consumes the pending plan.
