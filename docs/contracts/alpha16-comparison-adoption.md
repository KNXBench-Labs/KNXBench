# Alpha 16 device → project comparison adoption

Status: admitted offline implementation contract (WP-19, 23 October 2026).
Mounted API/UI and integrated release acceptance belong to the release integrator.

## Authority and scope

Comparison remains read-only. Adoption is a separate confirmed project edit, not
an additional download implementation. Project → device must hand over to the
existing validated download workflow; this service never sends a device write.

Only a successful application-identity-checked comparison performed by the App
service can create its opaque observation. No constructor accepts response JSON,
`ComparisonRow`, a supplied `DeviceBackup`, or client-provided values as evidence.
The service prepares the image and plan itself from the actual project and
product connection, then uses the existing `compare_with_plan` read-only path.
A transport remains an adapter trusted by the application, not a request payload.

The observation retains server/project incarnation, stable device/address,
monotone P3 comparison revision, original P3 programming content, and source-derived
request/image/plan. Labels and loaded beliefs alone do not invalidate the read;
preview captures the current full project instead of reverting those later edits.
Preview and apply rebuild and compare that source authority.
The final edit additionally binds the complete native snapshot and both command
stack directions, including comparison revisions. Undo/redo cannot resurrect
old read authority. The caller serialises project/stack/context and supplies its
current incarnation; incarnation values must never be copied from a request.

## Rows and refusals

- Named parameters are adoptable only with the decoder's exact typed attribution
  (reference, segment, bit field and admitted value encoding). Labels and row
  order are not identity. Both sides must be known with no decode reason.
- The selection contains row indices into the retained observation, not values.
  Empty/duplicate/out-of-range selections refuse. Every requested row must be
  supported; a mixed selection never silently drops an unsupported row.
- Raw, unknown, masked, unread and unsupported rows remain present in
  the observation/preview and never become parameter values. Unknown unselected
  rows do not prevent a supported named-parameter selection.
- Address counts, individual addresses, GA and association rows are explicitly
  unsupported for adoption in this bounded increment. The existing presentation
  codec has table slots/object numbers but no stable project GA/object identity
  and direction attribution. No identity is inferred from captions, TSAP order,
  object-number order, or a coincidentally equal address. No GA/object is created.
  A future table-adoption increment requires typed unambiguous source mapping.
- Wrong application, malformed identity, failed comparison, changed source,
  stale project/revision/incarnation, duplicate parameter attribution/placement,
  unavailable parameter mapping, invalid final image, and native/history
  preflight failures refuse the complete edit with typed reasons.
- Candidate configuration is rebuilt through the existing image request/builder
  and plan rules. Every selected field must remain attributable at its original
  exact placement and decode equal to the retained device observation. This
  prevents activation/layout changes from turning a decoded value into a write
  to another field. No new binary/text encoding or format admission is added.

## Atomicity and history

The Core prepared command composes existing `ReserveIds` and
`SetParameterValue` commands into one atomic `Batch`, applied through the actual
`CommandStack` on clones. All existing owning-installation, duplicate/id,
validation and programming-pending rules remain in force. Full history replay
is preflighted; the App preflights the full candidate native snapshot and stack
using the existing native history writer in memory, including retained context.

Preview mutates neither project nor history. Apply rechecks the original scope,
source, exact native snapshot and full stack; persistence (when a native target
is supplied) succeeds before publishing the candidate. Refusal leaves model,
allocator, context and undo/redo unchanged. A failed persistence does not consume
the preview. The integrator must supply the saved project's real native target
and history generation, not treat a saved project as unsaved.

One undo restores all prior user content, parameter rows, provenance and loaded
beliefs; redo reapplies the whole edit. As required by ADR-0039, newly reserved
allocator high-water marks never rewind. No-op selections produce no undo step.
Successful adoption invalidates old comparison evidence; it does not set Synced
or claim that unknown/unread device bytes match. Session history should name the
single operation `AdoptDeviceComparison`, never one event per parameter.

## Callable integration API

All entry points are in `knx_app::comparison_adoption`; no server/AppState,
route, token store, UI dependency or physical transport is owned by this module.

1. Under the current project/stack and catalogue-read ownership, call
   `prepare_comparison(conn, project, stack, &current_scope, device_id)`.
   `ComparisonScope { server_incarnation: String, project_incarnation: u64 }`
   comes from the live session. This builds a full supported memory comparison;
   this increment does not adopt partial-comparison or System-2 raw-only rows.
   The connection must not already have an active transaction: the service
   takes its own bounded read snapshot for each source preparation.
2. Immediately before the existing gateway/tunnel admission, check
   `prepared.is_current(project, stack, &current_scope)` under the existing
   locks. Refuse before connecting if false. Release project locks before await;
   call `prepared.observe(&transport, timing).await` in place of the current
   direct `compare_with_plan` call. Existing gateway guards, activity ownership,
   disconnect and stop admission remain the adapter's responsibility. No new
   tunnel or alternate management route is introduced.
3. Retain the resulting `ObservedComparison` in trusted session memory. Use
   `rows()`, `comparison()`, `preparation().image()/plan()/program_id()` for the
   current response projection, and `is_current(...)` for late-result staleness.
   Do not re-create it from the response DTO. Wrong application/read failure
   returns `AdoptionError::Compare` and creates no observation.
4. On preview, resolve the session-owned observation handle and call
   `preview_adoption(conn, &native_snapshot, stack, &current_scope,
   &observation, &selected_indices)`. `PreparedAdoption::rows()` contains every
   original row, `selected()` the exact requested indices, `changed()` the edit
   outcome, and `candidate()` immutable project/stack views for projection.
   Retain the opaque preview server-side; only that retained selection can be
   confirmed. Do not accept a replacement row/value/command/candidate payload.
5. After separate explicit user confirmation, while holding project/context,
   stack and catalogue-read ownership, call
   `apply_adoption(conn, &mut native_snapshot, &mut stack, &current_scope,
   &retained_preview, native_target)`. For a saved project pass
   `Some(AdoptionNativeTarget { path, history_generation })` from authoritative
   session state; `None` is only for an unsaved project. Native persistence uses
   `save_editor_if_unchanged` to compare the full before-snapshot hash and
   generation under the same write transaction, not a generation-only save.
   On success publish the returned project/stack and `native_generation`, bump
   the existing project revision and log the single operation. On error publish
   nothing. No fallible persistence is left for the adapter to perform after
   the service has acknowledged the edit.

The integrator owns handle entropy, browser/project ownership, bounded retention,
confirmation, locks and translating typed errors to its HTTP vocabulary. Never
hold a project lock across the device-read await. The service accepts no consent
boolean that could turn comparison into a device write. The separate project →
device button invokes the existing download review with the current stable ID;
it must not submit comparison bytes as a new write plan.

## Acceptance boundary

Minimal focused offline RED/GREEN tests cover atomic rollback, whole-edit undo
and redo, exact scope/source and stack refusal, unsupported mixed selections,
retained unknown rows and simulator re-comparison. No bus/device operation,
physical support claim, migration or storage-format change. The release plan's
single final integrated acceptance supersedes historical per-increment full
campaign requirements for this increment.
