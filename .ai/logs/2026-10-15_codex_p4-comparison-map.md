# P4 implementation map (2026-10-15)

Owner decisions: `docs/goals/ui-feedback-p1-p4.md` P4. Base: `8c42e4f5`;
claim: `5f065bed`. This is implementation planning, not delivery evidence.

## Existing paths and selected reuse

- `apps/knx-server/src/device_compare_routes.rs`: POST comparison prepares the
  same `knx_app::device_download::PreparedDownload` as downloading; management
  session is read-only, no access key or authorisation. Server holds address
  programming → monitor → line-scan guards; active monitor returns structured
  `gatewayBusy`, and download admission is checked. Reuse these guards.
- `crates/knx-net/src/commissioning/memory_download.rs::compare_with_plan`:
  reads `DeviceBackup`, computes byte differences, but previously checked only
  mask/manufacturer. New RED reproduces false ordinary differences for a wrong
  application. Reuse partial download's verified five-byte identity instead.
- `knx-core::commissioning::partial_memory_download::application_identity`:
  derives MM/MM/application/version from the Application Program task record;
  existing partial plan checks ObjectIndex::APPLICATION_PROGRAM, PID 13.
  MP v02.01.02 §3.31.2 defines the task record; Identifier Tables v01.04.01
  property table defines PID_PROGRAM_VERSION as PDT_GENERIC_05. Existing
  source documents a real 0701 read of `00 83 00 27 15` on 2026-09-29.
  No new on-wire service, object discovery assumption, or hardware permission.
- `knx-productdb::image`: actual active parameters are written through
  `ParameterImage`, with field offset/bit/width available exactly at write time;
  group tables use `ProgramCode` placements and Core's verified 0701 codec.
  Package 1 decodes these placements; package 2 records parameter attribution
  at the existing writer, not by reverse guessing from changed bytes.
- `knx-core::commissioning::pending::ProgrammingContent` and
  `command::programming_commands`: P3 already defines relevant content and
  candidates. Reuse for monotone session-only comparison revisions; undo/redo
  changes also invalidate results. Loaded flags are beliefs, not revisions.
- `DevicesWorkspace`: its `rows` are the sorted/filtered visible list. Freeze
  that exact list at review; one gateway confirmation, sequential reads,
  per-device errors continue; cancellation drains the current bounded read
  before stopping so tunnel ownership is not abandoned mid-operation.
- `DeviceInspectionPanel`: keep existing readiness/single read route compatible;
  share the decoded result renderer rather than a second byte interpreter.

## Planned boundaries

Core owns content/revisions, product adapter owns memory attribution/decoding,
server owns bound admission/results and shared gateway exclusion, React owns
session display and the explicit queue controls. No project-store migration.
No network/bus test: simulator and intercepted browser traffic only. Unknown
identity/shape is Not comparable with a reason, never Wrong application based
on malformed/truncated data. Raw comparison bytes remain visible.

## Acceptance (pending)

RED/GREEN and named mutations; separate self-review; EN/DE 1440/400 browser
images; full template gates with RUST=1 and fifteen EXTRA_CONFIGS on branch and
fresh-main integration for each package; source-bound evidence, push/readback,
private archive and exact owned cleanup. No acceptance or completion yet.
