# Recorded device state — read-only model inspection

> On main since 2026-10-12 (port of the parked branch; fresh gate in the
> [evidence](../evidence/native-evidence-port-2026-10-12.json)). Local acceptance figures below are historical.

Implementation contract, 2026-10-11. Accepted locally on1944frozen inputs: owningRust1315/0/100ignored,
wholeWeb2737/0/183files, built regressions12/0 and real-native12/0, zero
skips/flakes. Sixteen quality/build/type/repository/doc stages pass. Additional
actual scroll cases4/0 capture32raw regions; four derived sheets inspected.
Final supplemental server hash equals the exact final accepted binary; all four
final sheets byte-equal the inspected sheets. Self-review, not independent
approval or main/release integration.

## P3 amendment: the recorded belief is now maintained

[ADR-0129](../adr/0129-per-part-pending-programming.md) makes four of these same
persisted flags follow project edits, server downloads/address programming and
explicit undoable manual marks. `recordedDeviceModel` stays a read-only view of
the current model: inspecting it still performs no device IO. The flags are no
longer only an untouched import snapshot. `medium_config_loaded` and optional
observations remain separate; no fresh-device/synchronization verdict is added.
See [Programming marks](../manual/user-guide/05-devices-and-products.md#programming-marks-p3-source-build-after-alpha-11).

## Meaning

The independent domain already separates `CommissioningState`'s five persisted
`*Loaded` project-belief flags from optional `DeviceLoadStates` observations.
The pure application view projects those existing values, recorded completion,
broken flag, exact device name and stored UTC last-modified/download dates.
It does not evaluate scripts, product data, hardware, write authority or success.

`scope: recordedDeviceModel`, `view_version: 1` deliberately do **not** promise
fresh hardware or complete original-source commissioning metadata. Report octets
are the existing core's six recognized LoadState values. No new protocol grammar
or load state is introduced. A null report means **no report currently retained**,
not unloaded, never queried, invalid or confirmed. Observations have no retained
timestamps here and may be stale. The native loader resets them, and `forget()`
clears them; this view is not a historical observation journal. The current exact
native snapshot journal refuses SaveAs while model-held reports would not reopen
exactly; the HTTP regression checks unchanged model and absent destination on
refusal. A separately explicit test-only `forget()` then allows intent/date/native
roundtrip. Inspection does not clear reports, weaken that writer or migrate them. No new guarantees
about which runtime disconnect paths call `forget()` are established.

Recorded completion and broken flags are metadata; stored dates are not the
runtime last-successful-Save marker or a new device observation. Opposing project
belief/report values remain separate; no success badge or equality shim.

## Boundaries

- App projection accepts only an existing DeviceInstance reference, without IO,
  parser, bus or product adapter. Name text exceeding 64 KiB UTF-8 refuses the
  complete view before cloning; no truncated-success or model mutation.
- Authenticated `GET /api/devices/{id}/recorded-state` holds the existing project
  lock across model read and server/load/revision capture; no-store response.
  No project, malformed/out-of-range ID, missing device and oversized model text
  are explicit refusals. No access keys, raw paths, product payload or endpoints.
- Client admits only the supported scope/version, completion vocabulary, complete
  five parts in deterministic order, actual booleans, nullable reports0..5,
  safe ID/binding fields and the same exact UTF-8 name ceiling. Unknown values
  do not become defaults. A response must match the exact requested device and
  server/load/revision; selection or source changes suppress late replies. A hung old read cannot block
  the new target, and its late finally cannot clear a newer pending read.
- Real device Properties Inspector mounts an explicit EN/DE read action. No
  automatic read, input, writer, connect/reset/program action or clipboard/share
  submission. Literal names preserve whitespace; UI explains retained-versus-
  newly-observed and imported-versus-runtime meanings before displaying results.

## Verification scope

Actual baseline accepted server returned404 for the missing route in isolated
loopback-only execution. Client and panel named RED controls fail before their
behavior implementation; focused GREEN admission/panel tests include contradictory
intent/report, literal text, foreign identity, late selection, complete vocabulary,
UTF-8 bounds and current Inspector mounting. HTTP controls cover auth/refusals,
all existing report states, unchanged model, and real native SaveAs/reopen where
persisted project belief/dates survive but reports are absent. Final broad gates
and built real-server EN/DE400/1440 cases pass. Full element screenshots clip at
the existing Inspector scroll ancestor and are not complete visual proof. Four
supplemental real scroll cases assert visible metadata and all five parts,
preserving32raw captures. Four clearly derived sheets show all scope/facts/parts
readable without horizontal clipping; no CSS/DOM/data rewriting. Earlier
fixture/header failures remain failed historical evidence. The initial scroll
binary differs from the final build, so scroll proof was independently rerun
against an immutable copy of the final accepted binary, not adopted by claim.

No private corpus, manufacturer execution, hardware observation/write, schema
migration, native13/main integration, publication or full commissioning closure.
Review is source self-review. Stored BusAccess/IP details are a separate read-only
view since Goal B3 ([stored bus access](stored-bus-access.md)); target-specific
planning/writers remain independent scope, not quietly included in this view.
