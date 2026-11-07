# Editable object priority with a programming guard — 7 November 2026

Scope: [priority-edit contract](../contracts/object-priority-edit.md), WP-06
under [ADR-0158](../adr/0158-object-priority-provenance.md). Owner decisions of
7 November 2026: a separate user layer with native migration 17 → 18, and a
programming refusal (not a new encoding) when a stated priority differs from
the product literal. New boundary [KL-188](../KNOWN_LIMITATIONS.md#catalogue-kl-188).

## Delivered

- Core: `ObjectPriority` (`Low`/`High`/`Alert`, exact schema literals),
  `ComObjectInstance.priority_edit`, `Command::SetComObjectPriority` (set/clear,
  refuses unknown objects and no-ops, self-inverse undo). `ProgrammingContent`
  carries the edit, so the command marks the device's group-object part.
- Store: native schema 18 adds `com_object_priority_edit` (additive, no
  backfill); orphan and non-token rows are refused on load; history images of
  12–18 decode; a store without edits keeps its native17 semantic identity.
- Authentic predecessor `native17-before-priority-edit.sqlite`, produced by the
  unchanged v17 writer (`c240cdbdf`, producer source and provenance beside it):
  read-only open leaves it byte-identical; baseline, working, undo, redo and
  named images reproduce the v17 writer's semantic hashes after migration.
- Image: `ImageRequest.priority_statements` (user edit, else a bound captured
  Known declaration; Empty/Unknown refused as `UnreadablePriority`); the
  builder refuses an active ref whose statement is not exactly the product
  literal (`PriorityDiffers`, message names both values). Equal statements
  change no octet; inactive refs are ignored; the existing High refusal stays.
- Effective priority: new source `userEdit`, winning over every saved and
  product state. Project diff: new field `priority_edit`.
- Server: `POST /api/com-object-priority` (`priority` required, exact token or
  `null`, unknown fields refused). Web: *Your priority* select in the object
  editor (en/de), selective import model 18.

## Executed checks (author, before review)

- Rust (offline, locked, scratch XDG): knx-core lib 799 and `it` 139,
  knx-store 206 (4 ignored), knx-productdb lib 865, knx-diff 73,
  knx-projection 87 (bindings regenerated), knx-app lib 518 and filtered
  `selective_import` 37, knx-server `it` filtered device/diff/history/dispatch
  51. knx-server lib 865 passed, 7 failed: all seven are the known
  `caption_mounted::recovery_authority` worker timeouts (`Elapsed`) under
  parallel load; the same filter run alone passes 8/8.
- Clippy `-D warnings` for the workspace except `knx-desktop` (needs the web
  bundle), `cargo fmt --check`.
- Web: `npm ci`, `tsc -b`, Vitest 330 files. The first run failed one file:
  the new German strings used the formal address; corrected to the du-form
  (`i18n.duForm.test.ts` and the new `PriorityEdit.test.tsx` then 7/7).
- Author mutants, each file restored by SHA-256, 10/10 killed: builder
  equality, Empty/Unknown refusal, binding check, edit precedence in the
  request, edit precedence in the effective view, store load, empty-table
  identity skip, programming content, no-op refusal, diff field.

## Independent review and corrections

Review of `947e342af` in a separate source copy: **APPROVE**, no Critical or
Important findings, 7 Minor. 20 reviewer mutants: 13 killed, 6 survived, one
did not compile. The reviewer also confirmed that the unchanged v17 code reads
the fixture with exactly the provenance hashes, that the guard has no bypass
(the two other `ImageRequest` constructions are program-default only) and, on
three private real projects (115 devices, read-only), that no device is newly
refused and all 42 admitted images are byte-identical with and without
statements. Browser: a temporary real-server spec 4/4 (en/de, 1440/400 px),
`device-editor-layout` 14/14 twice, `communication-objects` 52/52 twice.

Corrections in the follow-up commit:

- HTTP refusals now target an object holding `High`, so token and
  unknown-field checks are not hidden by the no-op refusal (survivors R05/R18).
- New store test removes an edited object and saves again; it also reads the
  edit through the single-object loader (survivors R13/R21).
- Two instances sharing a `ComObjectRef` key must state the same priority or
  the request is refused (`ConflictingPriority`); previously the last one won
  (survivors R07/R08 describe an unreachable path today, since active module
  refs are refused by the builder).
- Contract: 422 for a missing `priority` or unknown field; the request-time
  refusal of Empty/Unknown declarations on inactive objects and its effect on
  read-only previews are stated (also in KL-188). Manual: the refusal names
  the object reference, not the number.

## Limits

- No changed priority is ever programmed; a differing statement refuses the
  device. Uncaptured instance priorities are not checked (KL-188).
- No end-to-end download run with a real product: the refusal is proven at
  request-mapping and builder level, and the server route at HTTP level.
- No web object-table column (owner UX decision), no documentation-report or
  CSV consumer, no batch edit, no bus or device action.
