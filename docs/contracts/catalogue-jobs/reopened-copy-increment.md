# WP-04.3 reopened-copy validation increment

Decision accepted for bounded implementation, 23 October 2026. Independent
readiness rereview closes ownership/continuity/settlement findings R1–R3;
implementation is in progress, with no runtime acceptance yet. This is a
bounded implementation supplement to the accepted
[managed-binding contract](managed-binding.md#4-typed-full-current-schema-streaming-witness-v1)
and [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md), not a new public API
or compatibility lane.

## Missing behaviour and scope decision

The existing copy primitive observes native backup completion and fresh cookie
0→1. Orchestration must additionally validate the **closed and read-only
reopened disk destination** before rehearsal or recovery use. Existing helper
tests already exercise a real disk close/reopen and copy-witness agreement;
they do not establish this orchestration gate.

Admit a private, non-clone owned disk-destination lifecycle instead of the
current bare `Connection` result from `CatalogueArtifacts`. The private value
retains its exclusively owned path/file identity and role (D or B), consumes and
explicitly closes its copy writer, then opens that same file existing-only,
read-only and private-cache. A real pinned read transaction produces the full
`CopyContentWitness/1`; explicit rollback/close ends validation. A validated D
may reopen existing-only READ_WRITE for rehearsal after continuity checks.
B remains closed recovery data. No migration, CREATE, guessed connection path,
`query_only` toggle or in-memory substitute can confer validated status.
Construction and corruption boundaries may be supplied by synthetic tests;
production `ArtifactRoot` integration stays excluded. No generic registry or
new dependency is needed.

Preview must retain W IMMEDIATE through original stamp acquisition, pinning S,
exact W/S prestate agreement, copy validation and final W baseline/identity
recheck. Explicitly end S and roll back unchanged W with inspected settlement
before rehearsal. No generation increment for this unchanged reservation;
retain the original acquisition stamp in the binding. Apply retains the already
corrected W reservation and rollback/recovery accounting throughout the same
validation of both D and B. Do not compare connection-local S/W data_version.

## Closed ownership and transition choices (review R1–R3)

### Construction and retained inventory

The artifact owner has exactly two allocation slots, D and B, not a generic
registry. A private constructor receives the owner-selected destination path and
role and creates the regular file itself with create-new semantics; it never
accepts an arbitrary connection/path pair. Immediately after creation, before
identity capture, SQLite open or configuration, register the file in its slot.
Later initialization failure therefore still counts as allocated B. Only that
constructor produces Fresh; only full validation produces Validated. Neither
state exposes an unrestricted connection accessor before consumer admission.

Allocation inventory outlives connection lifecycle values and consuming errors.
Record allocated/incomplete and validated separately. Drop never deletes B;
retention/disposal is a separate explicit ownership-checked action after W
settlement. An incomplete or corrupt retained file is not usable recovery.
Lifecycle failures retain any returned live connection in that same owner slot,
not solely in an ephemeral error that could silently lose ownership on drop.

### Stable identity and strict reopening

Keep a file anchor from create-new and the parent identity. Stable comparisons
require a regular, nonsymlink main, unchanged device/inode, link count one and
unchanged parent identity before/after each open and after copy. Expected
length/time changes are allowed only during the copy writer's work. After its
successful explicit close, freeze length and nanosecond mtime/ctime; require
that complete post-copy stamp unchanged across RO validation and RO→RW handoff.
Reject missing/replaced/linked/uncertain targets without deletion. These checks
inherit the existing exclusion of hostile same-user filesystem/timestamp
attacks; they are not a pathname reservation or comprehensive VFS guarantee.

S and destination validation use strict existing-only READ_ONLY/private-cache
openers; D rehearsal uses existing-only READ_WRITE/private-cache. No CREATE,
URI, immutable/nolock, ordinary migration opener or in-memory fallback. Verify
native main access mode independently of query_only. Retain from reserved W/
pinned S the actual application_id, current schema/user_version, UTF-8,
positive admitted page size/count and checked extent at most 8 GiB; require
exact agreement at validation, including cookie1 and full copy witness.

Finalize validation statements, explicitly end the RO transaction and explicitly
close its connection before reopening D RW. Check Stop, stable identity, frozen
content stamp, native access and header/page facts again before consumer access.
In-place modification during RO→RW invalidates the capability, even if inode is
unchanged; refuse rather than revalidating a different file. B has no RW reopen.

RW rehearsal establishes and reads back foreign_keys=ON, query_only=OFF,
ignore_check_constraints=OFF, writable_schema=OFF and defer_foreign_keys=OFF.
Record and preserve the copy-writer's admitted trusted_schema/recursive_triggers
values on reopen; failure to restore/read back refuses. No configuration step
creates schema. A previous connection's max_page_count is not a cap on the new
handle: growth acceptance remains a separately required increment and is not
claimed by this package.

### One inspected reservation and failure path

Preview arms the health guard before beginning W IMMEDIATE and sets Ignore drop
behaviour on the retained transaction. Do not invoke the releasing capture_stamp
helper for this acquisition. Every post-BEGIN success, refusal and Stop converges
on one cleanup path: end and close S, recheck original W baseline/identity,
explicitly roll back W and verify autocommit before disarming the guard. No
preview binding or D rehearsal is possible after an uninspected end/close.
Generation remains unchanged. Cleanup SQL is not skipped by a latched Stop.

Preserve the primary cause independently from typed cleanup/rollback/close
failures in a private failure payload. Clean unchanged rollback allows Healthy
with no binding; unresolved W rollback or identity uncertainty quarantines and
blocks reuse. No live COMMIT was issued, so rollback failure alone never invents
Unknown COMMIT. Apply retains its accepted independent settlement accounting.

Explicit SQLite close failure returns a live connection: store it with its
receipt in owner-held unresolved state. Failed writer/RO/S close or failed RO
transaction end withholds Validated and prevents rehearsal/live DML. Do not
fall through to ordinary Drop and call that inspected success; W settlement
still runs and both primary and cleanup failures survive. The retained owner
may later explicitly close/quarantine known handles; no uncertain target is
deleted by this lifecycle. Add native close-failure, S-end, preview Stop/failed-W-
rollback, post-create initialization and both reopen-seam corruption controls.

## Required admission facts

- Reuse the existing complete copy codec unchanged: omit **only** schema_version,
  retain all counts and encoded length as well as digest. It never substitutes
  for the exact live stamp or live-witness cookie.
- Independently establish W/S full live-witness agreement and admitted page/header
  facts under W reservation; retain S's original complete copy witness.
- Each destination starts fresh/empty/autocommit with cookie0; exactly one bounded
  backup observes Done, drops Backup and explicitly closes its writer.
- Read-only reopened destination must have cookie1, admitted current schema,
  UTF-8, matching source application_id and page size/count/checked extent,
  and exact complete copy-witness equality. No invented application_id=0 rule.
- Refuse a missing/non-disk/replaced/unopenable destination, unsupported header,
  failed close/reopen, wrong cookie/page facts, schema/content mismatch or Stop
  before any consumer. Return a typed error, never a partial valid capability.
- Keep copy-validity separate from artifact existence and retention. Failed live
  rollback still quarantines W and retains allocated recovery as in the accepted
  correction; a clean no-COMMIT refusal is not automatically unknown.

## Smallest allowed write surface

ProductDB `catalogue_copy`, `orchestration`, `managed_catalogue`, crate-private
copy-witness exports in `catalogue_schema`/`witness`, and focused subordinate tests.
Mechanically adapt existing artifact fixtures without weakening their assertions.
Documentation/status may record actual decisions/results. No server, DTO, UI,
protocol schema, migration, quiet API or network adapter changes.

## Finite executable acceptance

Tests must use actual synthetic schema-28 disk files and native close/reopen.
First observe semantic missing-behaviour failures on original production;
interface-absence compilation failures are separate setup evidence.

1. Source cookie !=1, destinations cookie0→1, full copy identity equal after RO
   reopen, exact page/header agreement; live cookie identity remains distinct.
2. Independently corrupt D/B ordinary rows, hidden rowids, actual source/package
   bytes beyond a chunk boundary, and supported schema metadata where feasible.
   Refuse before rehearsal/live DML; unsupported schema is separately refused.
3. Wrong copied cookie with otherwise equal copy identity; nonfresh/active
   destination refusal before Backup; page/header/extent/reopen/memory refusals.
4. Deterministic contender proves preview W remains reserved until validation;
   mismatched W/S live prestate refuses. Original stamp/generation/live data
   preserved after checked reservation rollback.
5. Actual preview D, apply D and apply B validation failures reach no consumer
   or live commit claim; cancellation/failed rollback retain their typed outcomes.
6. Original reservation, policy/version and recovery regressions remain green;
   owning ProductDB, dependent scoped gates and independent review pass.

Keep leaf/helper-only arithmetic evidence distinct from actual disk workflows.
Retain exact test/source/binary provenance; do not manufacture old RED identities.

## Explicit remaining boundaries

This does **not** close apply's separately required second rehearsal/result
agreement, comprehensive filesystem/VFS admission, production artifact durability/
sidecar inventory, live growth reassertion, concurrent B10 commit controls,
route/worker, embedded-server, desktop or UI integration. No whole §6 compliance,
WP-04.3 completion, hardware or ETS compatibility claim follows. WP-19 remains
subsequent in the authorised queue.
