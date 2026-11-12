# ADR 0173: The project remembers the parameter values a verified download wrote

Date: 2026-11-11
Status: Proposed (server part implemented 11 November 2026, WP-37 step 2; UI pending)
Session: issue intake #40, step 2 ([WP-37](../open-work/WP-37-pending-changes-across-ui.md)); restore finding [WP-40](../open-work/WP-40-version-restore-programming-claims.md)

## Context

Issue #40 asks to highlight changed-but-not-downloaded items wherever they
appear, the parameter editor included, and to clear the highlight once a
successful download covered the change. Step 1 (main `9f3a1e63a`) shows the
existing per-part flags in the device editor heading and on its tabs. The
triage reply promised step 2 as its own package: "this exact value differs
from what is in the device" needs the last downloaded value of each parameter.

Verified state of `main` `e17051626` on 2026-11-11 (source reading, no new
run):

- **The belief is four flags.** `CommissioningState` holds
  `parameters_loaded` and three siblings; `pending_parts()` derives the
  Parameters part from it alone ([ADR-0129](0129-per-part-pending-programming.md)).
  Nothing in the project records *which* value the device holds.
- **What a download sends for parameters.** `ImageRequest.values`
  (`knx-productdb/src/image.rs`) is the device's chosen values by
  `ParameterRef` id, built from its `ParameterInstance` rows
  (`image_request.rs`, `parameter_values`); every absent ref takes its product
  default (`resolve_values`, `dynamic/evaluate.rs`: instance value, else
  `ParameterRef/@Value`, else `Parameter/@Value`).
- **Where settlement already has it.** `settle_download`
  (`knx-app/src/programming_state.rs`) receives the request the run wrote
  (`written`) and the one the project yields when the run ends (`now`).
  `DownloadEnd::Completed` means every step ran and every block was read back.
  Parameters become programmed only if the program and `written.values ==
  now.values` are unchanged and the product source still plans the same plan
  (`SourcePlan::Unchanged`). `apply_settlement` applies the result outside the
  undo history and calls `CommandStack::forget_programmed_claims`, which
  rewrites live inverse commands and recovered snapshots so no undo or redo
  restores an obsolete "programmed" claim.
- **Manual marks are edits.** `POST /api/device/{id}/programming` runs the
  undoable Core command `SetDeviceProgramming` over all four parts.
- **History.** Live undo entries are inverse commands or whole `Project`
  snapshots (`HistoryEntry::Snapshot`); a recovered stack holds snapshots
  only. Persisted stack rows and versions are `NativeSnapshot` images
  (`project_history.rs`), each bounded by `MAX_IMAGE_BYTES` and the history
  budget (`check_limits`). Settlement persists through the working-journal
  write that already carries the flags.
- **Settlers.** Regular and batch downloads ([ADR-0172](0172-batch-programming-stops-at-the-first-doubt.md))
  settle through `settle_download`. The experimental System-2 worker decides
  with its own `decide` (`apps/knx-server/src/device_system2/settlement.rs`:
  no `DownloadEnd`/`SourcePlan`, a recovery case, its written side the
  bundle's original project); only `apply_settlement` is shared.
- **What an image encodes.** Of `ImageRequest.values`, only refs the
  parameters activate and that have a memory or property placement reach the
  device; superseded union members and `Assign` targets carry another value
  (`image.rs`, [ADR-0171](0171-assigned-invisible-parameters.md)). Module
  instances are refused by the image builder.
- **Ids.** Device ids are never issued twice ([ADR-0039](0039-project-mutation-goes-through-commands.md)
  `ReserveIds`); undoing a deletion brings the same device back under its id.
- **Version restore resurrects flags.** `HistoryAction::Restore` writes the
  stored version's project as the new root and working state, and the server
  publishes it unchanged (`apps/knx-server/src/project_history.rs`, `mutate`).
  Its `*_loaded` flags and `last_download` come from the version. A device
  programmed after that version was stored can therefore show "programmed"
  for values it no longer holds. No document lists this. `programming_content`
  with `ProgrammingContent::changed_parts`
  (`knx-core/src/commissioning/pending.rs`) already compares the content of
  each part between two projects.
- **ETS.** This ADR claims nothing about whether or how ETS marks single
  parameters; no ETS observation was made for it.

## Decision

### 1. The baseline records what a completed download built its image from

Per device, the project may hold one **parameter baseline**: the program id,
the **effective** value of every unscoped `ParameterRef` of that program as
the completed download resolved it (`resolve_values` over `written.values`,
in the same catalogue lease that proves `SourcePlan::Unchanged`), and the end
time. Storing resolved values means a later change of product data cannot
move the comparison. The baseline is **not** a per-parameter device read: a
ref that was inactive, has no placement, or whose value an `Assign` replaced
carries no claim about device memory. The UI therefore says "value at last
download", never "value in the device". A device without a baseline has an
*unknown* baseline, never "unchanged".

Module-scoped values are out of scope until module downloads exist.

### 2. It lives outside the model images and outside undo

The baseline is keyed by `DeviceId` and stored in its own native tables
(§7), **outside** `Project`, `NativeSnapshot` history images and versions. So
no undo, redo, snapshot recovery or version restore can bring back an older
baseline, and the history images do not grow. No edit command touches it.
A device deletion leaves its entry until the next native save, which prunes
entries of devices absent from the saved project. A plain save of another
project into the file (another `project_id`, whose device ids name other
devices) drops every entry. A device that an undo,
redo or version restore brings back after that save returns with an unknown
baseline; this conservative loss is accepted.

The same `project_id` does not prove the same devices: `knx import X.knxproj
--store existing.knxdb --replace` writes a fresh import with fresh device ids
under the id it had (amended after review, 12 November 2026). What a save that
brings no baselines of its own keeps therefore depends on the writer:

- A writer **admitted at the journal's generation** (the server's plain save,
  every journal write) continues, command by command, the project the rows
  were written for, and ids are never issued twice: an entry stays while the
  saved project holds its device. Comparing with the saved root instead would
  drop the entry of a device added and downloaded since the last save.
- **Any other writer** (a plain store save such as the CLI's, an unadmitted
  root save) keeps an entry only if the saved device has the same identity as
  the device under that id in the project the rows were written for (the
  journal's working state, else the saved root): the same `source` (path and
  ETS id), the same `program_ref` and the same `product_ref`. Otherwise the
  entry goes.

A **version restore** brings back a project whose ids may likewise name other
devices (a version stored before such a re-import). It keeps an entry only if
the restored device under that id has the same identity as the live one, in
the file and in the server's held set alike; other entries go.

Apart from a restore, the server's in-memory set needs no such check: it is
replaced together with the project, and its save is admitted only at the
generation it holds.

### 3. Only settlement writes it

`Settlement` gains an explicit instruction, `parameter_baseline: Keep |
Remove | Record { program_id, values, at }`, so `apply_settlement` can tell
"completed, project edited during the run" (record) from "changed source" or
"interrupted" (remove), which all give `programmed.parameters = false`.
Only settlement records it, in `apply_settlement`, in the same step as the
flags. It is removed by settlement, by a manual mark (below) and by pruning
at save (§2); nothing else changes it.

| Settler and end | Baseline |
| --- | --- |
| download `Completed`, source `Unchanged`, Parameters in scope (complete or partial) | `Record` the written request's resolved values, even when the project changed during the run: exactly those edits then differ |
| download `Completed`, source `Unchanged`, Parameters in scope, but the written values cannot be resolved in that lease | `Remove` (added by the implementation: nothing to record, and an older baseline would no longer describe the device) |
| download `Completed`, source `ChangedOrUnknown`, Parameters in scope | `Remove` |
| download `Interrupted`, Parameters attempted | `Remove` |
| download `Interrupted`, Parameters in scope but not attempted | `Keep` |
| download with Parameters not in scope (partial group addresses) | `Keep` |
| download `NothingWritten` | `Keep` |
| System-2 download or recovery whose settlement touches Parameters, **including a verified run** that marks them programmed | `Remove`: its written side is not an `ImageRequest`; Parameters may be programmed with the baseline unknown ([System-2 settlement](../contracts/system2-download-result.md#settlement)) |
| System-2 without possible mutation | `Keep` |
| individual-address programming | `Keep` |

A manual **Mark as programmed / not programmed** removes the baseline outside
its undoable command: a claim KNXBench did not witness. Undoing the mark
restores the flags, not the baseline. The removal is written in the same
transaction as the mark's journal entry, which is therefore written at once
even on a server that otherwise journals in the background (ADR-0166); a mark
of a device without a baseline has nothing to remove and keeps the ordinary
(background) journal path; a mark that is refused or stale removes nothing. Adopting values read from a device
(project comparison adoption) never records a baseline; adopted values then
show as differing from the last download until the next one.

### 4. The comparison

A parameter **differs from the last download** when the baseline is usable
and its stored value differs from the current effective value
(`resolve_values` over the current chosen values). Setting a value explicitly
to its default is therefore not a difference. A ref with a value on exactly
one side (no value at all, or a key the program does not declare) differs;
a ref absent on both sides is not compared. Values compare as exact raw
strings, the strings the image builder encodes.

The baseline is **not usable** (state reported, no per-parameter verdict) when
it is absent, when its program id differs from the device's current resolved
program, or when the product data needed for the current side is unavailable.
The implementation found a fourth case: the project states two different
values for one ref of the device, so the current side is unknown (a download
refuses the device for the same reason); it is reported as its own state,
`currentUnreadable`, rather than as missing product data. A stored key the
current program does not declare is a value on the current side, as above.

The per-part flag stays the authority for "pending". The comparison can find
no difference while Parameters is pending (a value set explicitly to its
default; a System-2 or manual path that removed the baseline); the UI then
says that no value differs from the last download, or that it is unknown.

### 5. Presentation

The device parameter response (`GET /api/device/{id}/parameters`, optional
field `lastDownload`) reports the state (`usable`, `absent`, `otherProgram`,
`productUnavailable`, `currentUnreadable`) and, when usable, the end time and
the differing refs with stored and current values. The response of a parameter
write does not carry it; the client reads it with the next `GET`. The parameter editor marks a
differing visible row with the step-1 amber mark and a tooltip/accessible
description "Value at last download: …" / "Wert beim letzten Programmieren:
…", using enum labels where declared. The panel names the number of differing
values and how many of them the current settings do not show (inactive in the
current settings, which carry no device claim either). An unusable baseline
gives one plain sentence and no row marks. Links and object flags get no
per-item marks.

### 6. Version restore settles the four flags against the live project

Independent of the baseline, restoring a version must not resurrect programmed
claims. Inside `change_history`, before the restored state becomes root and
working state, each device of the restored project is compared with the live
project using `programming_content` (the same comparison edit commands use):

- present live under the same id: parts whose content differs, plus their
  dependants (parameters on the application; group addresses on parameters
  and address, as in `still_current`), become pending; the other parts take
  the **live** flag; `last_download` takes the live value;
- absent live: all four parts become pending; `last_download` stays as stored.

The fifth flag, `medium_config_loaded`, is left alone (ADR-0129). The result
is durable with the restore and no undo step (the restore clears the undo
stack). `HistoryChange` returns, per device, the parts that became pending although
the version claimed them programmed; the server writes one session-log line
per such device (separate wording for a device the live project lacks).
Parts that move the other way (the version said pending, the live project
programmed with the same content) are applied but not logged.

Restore is therefore no longer an exact round trip for these flags: restoring
version V and then its **Before restore** version P leaves pending every part
where P and V differ, although the device still holds P's content. This is
the conservative direction; a download or a manual mark clears it. The stored version itself is not rewritten.
Delivered first and separately as [WP-40](../open-work/WP-40-version-restore-programming-claims.md).

### 7. Storage

Native schema 20 adds `device_parameter_baseline` (device id primary key,
program id, end time) and `device_parameter_baseline_value` (device id,
`ParameterRef` id, raw value). They are not part of history images: the
snapshot codec and its schema check must know them as excluded tables. The
migration from 19 is additive and leaves every baseline unknown; nothing is
derived from imported ETS flags, the activity history or a settlement
receipt. Older builds refuse schema 20 files, as with every schema step.

Baseline rows are written in the same SQLite transaction as the working
journal write that persists the settled flags (`persist_native_working_reported`),
so a crash cannot pair newer flags with an older baseline. If that write
fails, the existing `PersistFailed` handling applies to both: the session
log says so and no durable claim is made. A project without a native file
keeps baselines in server memory, bound to the project incarnation, and the
first native save writes them; **Save As** (a new file, an overwritten other
file, the first native save) copies them like the project. A plain save to
the same file writes no baseline row: settlement and marks already wrote each
change there, so it only prunes (§2). Row count is one per unscoped ref per programmed device; a
large-project size and time test is part of the implementation.
Restoring a version stored by an older schema is tested with the authentic
native-18 fixture's named version; a schema-19 history image is tested to
decode with its identity (snapshot codec unit test). `.knxproj` import, project
comparison, exports, reports and selective import ignore the baseline in this
step; the selective-import reference closure is audited for the new tables.

## Alternatives considered

- **Store the written memory image and compare bytes per parameter.**
  Closest to the device, but needs the memory layout of every parameter,
  marks hidden and union parameters that share bytes, and fails for
  property-placed parameters. Values are what the user edits and what the
  flag already compares.
- **Store only the chosen values, compare without defaults.** Marks a value
  set explicitly to its default as different although the device holds the
  same value.
- **Make the baseline part of the undoable model or of `Project`.** Undo,
  snapshot recovery and version restore would bring back an older baseline
  after a later download, and every history image would carry a second copy
  of all parameter values; the same reason settlement is outside undo
  (ADR-0129).
- **Store chosen values only and resolve defaults at display time.** A product
  data change after the download would move the stored side; a real
  difference could disappear.
- **Derive the Parameters flag from the baseline.** Would change ADR-0129's
  belief for imported projects, which have no baseline. The flag remains the
  authority; the comparison only explains it.
- **Keep the baseline on a manual mark.** A mark is a claim KNXBench did not
  witness; keeping an older baseline could show stale differences, and
  setting one from the project would invent a device read.
- **Read the parameters from the device.** A bus read is a separate feature
  with its own authority (P4); this ADR adds no bus traffic.

## Consequences

- The user sees which parameter values changed since the last download that
  KNXBench completed, for devices programmed by KNXBench since schema 20.
  Imported projects, older projects, System-2 runs and manually marked devices
  show the per-part flag only until their next completed regular download.
- New schema step with migration and refusal tests; the snapshot codec
  excludes the new tables; history images keep their size.
- Settlement tests cover every row of §3, including a project edited during
  the run and a changed product source; restore, undo and redo tests pin that
  no older baseline returns.
- The comparison needs the product database at display time; without it the
  state is reported, never guessed.
- Not covered: module parameters, per-link and per-flag marks, a device read,
  ETS behaviour, differential download (LS-03).
