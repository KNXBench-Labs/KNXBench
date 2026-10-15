# ADR-0129: pending programming is the project's per-part belief

- Status: Accepted (owner P3 decisions, 2026-10-14)
- Date: 2026-10-14
- Scope: offline edits and settlement of the server's device-write sessions

## Context and verified facts

The owner-approved [P3 decision record](../goals/ui-feedback-p1-p4.md#p3--changes-not-yet-programmed-point-5)
asks for Address, Application, Parameters and Group addresses, not another bus
comparison feature. P4 remains separate.

`CommissioningState` already holds the imported ETS flags
`individual_address_loaded`, `application_program_loaded`, `parameters_loaded`
and `communication_part_loaded`. `knx-store/src/devices.rs` writes and reads
all four and `medium_config_loaded` already. Native working history persists
model snapshots. No new database field, model version or migration is needed.
The fifth, medium-configuration flag is outside these four editing parts and
is left untouched. Retained original import XML is not rewritten.

All reversible knx-edit/knx-app commands meet at
`CommandStack::do_command`. Server download plans are prepared in
`device_download_routes`, executed by `device_download::run`, and report their
last started step. Individual-address programming has a separate worker and
requires an unambiguous current-project mapping. The ProjectTree and catalog
both project `DeviceNode`; the catalog client admits its fields explicitly.

## Decision

### One model, one undo step

`knx-core::commissioning::pending` names four parts, optional changes to their
loaded flags, and the per-device content relevant to each part. The command
stack compares the affected devices before/after a successful command and
folds each mark into that command's inverse. A refused batch changes neither
flags nor history. Undo/redo returns the edit and marks together.

The candidate-device match is exhaustive: a new command requires an explicit
choice of programming impact. Existing address changes affect Address;
parameter values affect Parameters; links, effective object flags and linked
GA number/deletion affect Group addresses for every affected device. A linked
GA deletion is the existing atomic unlink-then-delete batch. Names, DPTs,
placement, provenance-only equal flags and no-op values do not add marks.
Product/program references form the Application content; there is currently
no application-swap edit command. A newly added such command must join the
exhaustive candidate match, not bypass it through a UI-specific flag setter.

### Device writes are not reversible project edits

The pure application settlement service owns these rules:

- Finished full download: clear all four parts, only where the current project
  still describes what was sent. The reached target address is confirmed too.
- Finished partial download: clear only its written Parameters and/or Group
  addresses. Address and Application are not implicitly cleared.
- Interrupted download: mark the attempted parts pending, including a write
  whose reply failed. The typed plan's load-control machine and last started
  step bound that set; unreached parts stay unchanged. An unattributed write
  or missing progress is conservatively attributed to the selected memory
  parts, never silently discarded.
- Address programming: clear Address only for the same uniquely identified
  device/address captured before the run. An uncertain write makes that part
  pending. A remapped/ambiguous device or another open project gets no claim.
- A run that wrote nothing changes nothing. A verified-memory result with an
  unconfirmed closing restart still clears its parts; the existing independent
  restart warning remains. Restart confirmation is not memory readback.

Settlement is outside undo history, since undo cannot undo a hardware write.
It removes obsolete programmed claims from both undo and redo, including
recovered snapshot history, so undoing an edit after a download cannot claim
that the old value is still on the device. It persists the candidate before
publishing live state/revision, as other native mutations do. Persistence
failure is explicitly logged; no durable-success claim is invented.

The worker settles once before publishing its terminal status. The existing
unexpected-download-task-end fallback also settles once, using its conservative
failed status. A weak state reference avoids a session/callback ownership cycle.
The original project incarnation is captured before preparation. Current
request content is compared under the project lock, so edits made during a
write stay pending rather than acquiring the old run's clean claim.

### Manual marks and presentation

`POST /api/device/{id}/programming` takes `programmed` plus the existing
server/project/revision binding. It marks all four parts through a Core command,
creates one undo step, persists through the normal mutation boundary and writes
a plain session-log sentence. Missing/stale bindings and unknown devices refuse
without state/history changes.

`DeviceNode.pending_programming` derives four booleans from the loaded flags;
it is optional for older clients/hand-written fixtures. Absent is unknown, not
clean. The strict catalog admission helper accepts only four booleans when it
is present. The Devices view shows pending-only Adr/App/Par/GA chips, a pending
filter and sorting by pending count. Explorer device rows have a named status
dot. Properties lists all four states and offers the manual actions.

After a device-write terminal result that may have written, the panels reload
the project once; a failed reload warns that displayed marks may be stale,
without changing the witnessed device result or retrying the write.

## Boundaries and consequences

- These are project beliefs, not a fresh device read, synchronization verdict,
  ETS certification or proof the application is running. P4 comparison is not
  implemented here. Existing download support/safety refusals remain unchanged.
- CLI downloads use their input project as configuration, without owning the
  server's open project or auto-saving changes to that file; CLI address
  programming has no project parameter. CLI/ETS/external writes need explicit
  manual reconciliation in the editor. No hidden cross-process project write.
- A process crash before terminal settlement cannot promise final flags; the
  separate durable activity/recovery evidence remains authoritative. Native
  storage failure is logged, not masked by an invented successful save.
- Unknown/absent presentation data remains distinguishable; original import
  evidence and observed `DeviceLoadStates` remain separate from mutable belief.
- No protocol grammar, vendor-specific product, dependency or storage migration
  is added. The UI does not own any flag-transition rule.

## Large-project presentation boundary

The existing Devices table is not virtualized. P3 keeps at most seventeen
immutable chip presentations per table/locale (sixteen boolean combinations
plus unknown), defers these new nodes until the first visible table visit and
retains them across editor visits. Production attribution and two controlled
counterbalanced twelve-run comparisons used synthetic 5000-device projects;
for the deferral comparison with all parts pending, mean main-thread RunTask
fell from 23204.2 to 16455.1 ms and project open from 18209 to 11449 ms, while
opening the table stayed about 4.55 s. These are a measured envelope on a shared
host, not universal throughput or whole-gate/GPU/native-WebKit certification.

## Verification

Focused regressions live in `commissioning/pending/tests`,
`command/programming_tests`, App `programming_state/tests`, server
`http_pending_programming`, `http_address_programming`, the corpus-gated
`http_device_download`, worker lifecycle tests, web pure/component tests and
`e2e/pending-programming.e2e.ts`. They cover exclusion of DPTs, shared linked
addresses, atomic undo, manual binding/logging, native restart/recovered undo,
full/partial/interrupted writes, project/device changes, restart warnings and
EN/DE wide/narrow presentation. Full delivery-gate results belong in the
implementation-status entry and the P3 evidence receipt, not assumed here.
