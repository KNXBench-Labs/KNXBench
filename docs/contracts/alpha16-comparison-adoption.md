# Alpha 16 device → project comparison adoption

Status, 24 October 2026: App primitives and mounted production API/UI implemented
(WP-19); actual mounted comparison/adoption/native persistence/undo has bounded
software acceptance in the [current release matrix](../goals/alpha16-release-plan-2026-10-23.md#current-acceptance-disposition--24-october-2026).
Earlier slice evidence below retains its source identity, not a full gate at the
final successor. Local amd64 software/package acceptance is complete, including
both formerly blocked pinned-input passes; extra local ARM64 preacceptance is
explicitly deferred and unverified, without changing the normal amd64+arm64
publication workflow. Publication Go is conditional on final readiness; parent closeout and
public readback remain pending. No physical-device or broader adoption authority follows.

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
  *Amended 24 October 2026 for association entries only:* see
  [Association link adoption](#association-link-adoption-wp-19-amendment-24-october-2026).
  Address counts, individual-address, group-address and association-count rows
  remain unsupported.
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

## Mounted API and UI

The existing `device_compare_routes` router mounts the following endpoints;
the standalone and embedded compositions already share this router:

- `POST /api/device-compare` keeps the existing address/gateway/optional expected
  binding request. Full comparisons now prepare and observe through the opaque
  App service under the existing stop, gateway, activity and disconnect guards.
  No project lock crosses the read await. `observationToken` is returned only
  for a genuine full read still current at publication. Late results are outdated
  and have no adoption handle. The response's `decoded` rows add original `index`
  and an `adoptable` presentation hint, not mutation authority. Existing partial
  read-only comparisons remain supported, but have no adoption token; their
  legacy transport result is never promoted into App observation evidence.
- `POST /api/device-compare/preview` accepts only
  `{ observationToken, selected: [originalRowIndex, ...] }`.
  It resolves retained opaque evidence and revalidates source and selection in
  App; it returns `{ previewToken, selected, changed, rows, expiresInSeconds }`.
  All original rows remain in the response, including unknown/raw/table rows.
- `POST /api/device-compare/apply` accepts only
  `{ previewToken, confirmation: "AdoptDeviceComparison" }`.
  The selected values, project/native target, scope, revision and command stack
  are never client inputs. A separate explicit confirmation is mandatory. Under
  the project/context/stack and catalogue-read ownership, App checks the exact
  native snapshot and both history directions, persists saved-native working
  state transactionally before publication, and returns `{ changed, tree }`.
  The server publishes the returned native generation and logs exactly one
  `AdoptDeviceComparison` action for a changed edit. No second save runs. No-op
  previews produce no edit/history action. Native refusal publishes nothing and
  leaves the retained preview available until expiry/eviction for explicit retry.

Both handles use independent OS-random 256-bit entropy. Router-owned transient
storage binds them to the authoritative server/project scope and an HttpOnly,
SameSite=Strict, server-issued browser-session cookie scoped to these endpoints.
That cookie is not project/native authority or a substitute for deployment
authentication; existing authentication/same-origin policy still protects the
routes. All responses are `no-store`; strict DTOs reject extra fields, values,
paths, candidates and authority payloads. Request bodies are capped at 16 KiB.

Observation handles are valid at most ten minutes (32 retained globally);
preview handles at most two minutes (16 globally). Admission samples monotonic expiry while holding the
retention lock, not an eventual timer. Oldest entries can be evicted earlier;
these are upper limits on handle authority, not guaranteed replay windows. Expired
entries are pruned at the next comparison/preview/apply admission or router
disposal; retained memory is additionally bounded by the global counts. Project replacement
invalidates by scope; an edit invalidates currency, and successful adoption
clears retained handles. Server restart/router disposal forgets all handles.
Unknown/expired/foreign handles fail closed with `deviceComparisonExpired`;
stale scope/history/source uses `deviceCompareStale`, selection refusal uses
`deviceAdoptionRefused`, native failure uses `deviceAdoptionPersistenceFailed`.
A lost apply response never proves rollback and must not trigger automatic
reapply. The UI instructs a project/history refresh before a new attempt.

The existing Devices comparison dialog shows selection controls only for
server-hinted named rows. Filtering preserves original row indices. It requests
a preview, displays exactly its retained selection and old/device values, then
requires a separate checkbox and apply gesture. Project snapshot changes
discard local previews; the server remains the final stale/history authority.
Acknowledged edits flow through the workbench's normal tree update, including
undo/redo/programming revision projections. Copy is provided in EN and DE.
Project → device passes only the current stable device ID through
`OpenDownloadButton` into the existing download review. It never passes
comparison bytes, values or plans and never starts a download itself.

## Association link adoption (WP-19 amendment, 24 October 2026)

The owner lifted the association-row block where it can be done honestly. This
section adds object ↔ group-address **link** adoption from mask-`0701h`
association entries. Everything above still applies: the same opaque
observation, scope, P3 revision, complete history, rebuilt source identity,
native preflight/persistence and one-undo rules. Adoption still never writes to a
device, never creates a group address or communication object, and never
changes the individual address. Address counts, individual-address,
group-address and association-count rows stay unsupported: the address table
follows from the links, so it is checked, not adopted.

**Typed attribution.** The decoder already resolves each association entry
through *its own side's* verified address table (§4.16.3.3.2 TSAP, §4.17.9
ASAP). Each association row now also carries a typed attribution per side: the
entry `(TSAP, object number, group address)`, or *beyond* when the slot lies past
that side's verified Current Size, or unknown. Captions are display only and are
never parsed. Unknown sides stay unknown; the decoder adds no new codec.

**Admission.** Every rule must hold, otherwise the **whole** selection, including
selected parameters, is refused with a typed reason (`deviceAdoptionRefused`):

1. Each selected association row is known on both sides.
2. The complete device association table is decoded: the association count and
   every association row's device side are known. A removal would otherwise rest
   on an unknown device link set.
3. Granularity is the object. The touched objects are the object numbers named by
   either side of a selected row. Every differing association row that names a
   touched object must be selected too; the refusal names the missing row. A
   touched object's project links become exactly the device's links for that
   number; there is no partial merge inside one object.
4. Each device group address of a touched object resolves to exactly one existing
   project group address with that raw address across all installations. None
   refuses (adoption creates none); several refuse as ambiguous.
5. Each touched object number with device links resolves to exactly one project
   communication object of the compared device whose `ComObjectRef` the
   program's `Dynamic` tree activates outside modules. Activation is evaluated
   with the candidate parameter values (current values plus parameters selected
   in the same adoption). Zero active instances, several, any module-based
   instance with that number, or links currently held by another instance with
   that number refuse. A touched object whose device side has no links resolves
   to the single project instance currently holding its links.
6. **Sending links.** Retained links keep their direction; removed links are
   removed with theirs. An added link receives, except on an object with no
   retained link, where the device's first listed entry for that object becomes
   the sending link. This is the order the encoder writes (§4.17.9.5, sending
   association first) and the established first-listed-sends reading of link
   lists without directions. The rebuilt order must then match the device; if it
   could only match by flipping a retained flag, the adoption refuses
   (`sending link undetermined`) instead of guessing. Where the device's first
   entry is also its lowest address, the device table cannot say whether that
   link sends; the project's retained flag decides and is not changed.
7. **Candidate recheck.** The candidate is rebuilt through the normal image
   request, `Dynamic` evaluation, image builder and plan. For every touched
   object the rebuilt association entries, in table order, must equal the
   device's entries. Any other difference, capacity refusal or activation
   refusal refuses the whole edit.

**Edit and history.** Unlinks, then links in device order, then any parameter
edits form the existing `ReserveIds`/`Batch` command through the real
`CommandStack`: one undo step restores links (at their original list positions)
and parameters; redo reapplies the whole edit. Association rows that already
match produce no change; an all-no-op selection is a no-op preview without an
undo step.

**Preview and UI.** The App preview exposes `link_changes()`: add/remove, object
number, project communication object, group address and whether the link sends.
The preview response adds `linkChanges` in that shape. The comparison DTO marks
an association row `adoptable` only as a presentation hint (typed attribution,
both sides known, differing). The web client admits selection only for
`parameter` and `association` rows the server marked adoptable, and shows the
link changes in the existing review in English and German. The server remains
the only admission authority.

**Boundaries.** No System 1/2/B tables, partial comparisons or module-based
objects; a group address that exists in several installations is ambiguous. No
live-device acceptance; tests use the simulator only.

## Integrated acceptance boundary

Minimal focused offline RED/GREEN tests cover atomic rollback, whole-edit undo
and redo, exact scope/source and stack refusal, unsupported mixed selections,
retained unknown rows and simulator re-comparison. No bus/device operation,
physical support claim, migration or storage-format change. The release plan's
single final integrated acceptance supersedes historical per-increment full
campaign requirements for this increment.
