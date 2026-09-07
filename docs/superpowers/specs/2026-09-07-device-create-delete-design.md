# Device create/delete commands — design

**Date.** 2026-09-07.

**Closes (backend only).** `GAP_ANALYSIS_ETS.md` T1 (device-from-catalog
insertion command) and, as a mechanical side effect of giving T1 a working
undo, T3 (device deletion command). B1/B2.

**Not in this slice.** T2, the device catalog browser UI, and any frontend
caller for the routes this slice adds — same "backend now, UI later" shape
as the 2026-09-06 topology/group-range/group-link command layer (T4-T6),
which got its frontend in T23 a day later. This slice's UI companion is
tracked as its own future cycle, not written here.

---

## Why bundle device deletion in

`CreateDevice` needs an inverse for undo/redo, the same way every other
`Create*` command in `knx-core::command` has one. That inverse is
necessarily "remove the device this just created" — there is no way to
give `CreateDevice` undo without also writing `DeleteDevice`. T3's own
text asked for the cascade-vs-refuse choice to be made explicitly rather
than defaulted silently; this slice makes it: refuse, matching
`DeleteArea`/`DeleteLine`/`DeleteGroupAddress`, the only convention this
codebase has ever used for a delete with dependents.

## 1. `knx-core` command layer

Two new `Command` variants:

```rust
CreateDevice {
    device: DeviceInstance,
    com_objects: Vec<ComObjectInstance>,
    line: Option<LineId>,
}
DeleteDevice {
    id: DeviceId,
}
```

`device.com_objects` already lists the ids of the entries in
`com_objects` — no separate id list is threaded through twice.

**`CreateDevice::apply`.** Targets `installations[0]`, same as every
other `Command` (`Command::apply`'s own doc comment; not revisited here).
If `line` is `Some`, the line must exist (`CommandError::LineNotFound`
otherwise). Each entry in `com_objects` is inserted via
`Devices::insert_com_object`, then `device` via `Devices::insert`, then
the device's id is pushed onto the target line's `devices` or, if `line`
is `None`, onto `installation.topology.unassigned` — mirroring
`MoveDeviceToLine`'s own placement logic exactly, since a device is
placed in a line xor left unassigned the same way in both commands.
Inverse: `DeleteDevice { id: device.id }`.

**`DeleteDevice::apply`.** Refuses with a new `CommandError::
DeviceHasLinks(DeviceId)` if any of the device's com-object instances has
a non-empty `links` — checked before anything is removed, same shape as
`DeleteGroupAddress`'s `GroupAddressInUse` check. Otherwise: finds and
removes the device's id from wherever it currently sits (line or
unassigned — same search `MoveDeviceToLine` already performs, factored
into a small shared helper if that reads better once written), removes
every one of its com-object instances from `Devices`, removes the device
itself. Inverse: `CreateDevice` rebuilt from the exact `DeviceInstance`/
`Vec<ComObjectInstance>`/`line` just removed — so redoing a delete (i.e.
re-running the `CreateDevice` a delete's own undo produced) restores
whatever values were present at delete time, including anything
enrichment filled in after the original creation (Section 3) — no
special-casing needed, since delete captures current state, not
creation-time state.

New `CommandError` variant: `DeviceHasLinks(DeviceId)`.

## 2. `knx-productdb` catalog queries

Three additions to `query.rs`:

```rust
pub struct CatalogItemRow {
    pub id: String,
    pub manufacturer_id: String,
    pub name: Option<String>,
    pub number: Option<String>,
    pub visible_description: Option<String>,
    pub product_ref_id: Option<String>,
    pub hardware2program_ref_id: Option<String>,
}

pub fn catalog_items(
    conn: &Connection,
    manufacturer: Option<&str>,
    search: Option<&str>,
) -> Result<Vec<CatalogItemRow>, ProductDbError>;

pub fn catalog_item(conn: &Connection, id: &str) -> Result<Option<CatalogItemRow>, ProductDbError>;

pub fn com_object_ref_ids(conn: &Connection, program_id: &str) -> Result<Vec<String>, ProductDbError>;
```

`catalog_items` narrows to one manufacturer when given, and does a
case-insensitive substring match (`LOWER(name) LIKE '%' || LOWER(?) ||
'%'`, likewise for `number`) when `search` is given; ordered by
`manufacturer_id, name`. Backs the future catalog browser (T2) — added
now because it is the natural place for this query to live, not because
this slice uses it (it doesn't: device creation goes straight to
`catalog_item` by id, chosen by whatever picks the id, which for now is
nothing — see "Not in this slice").

`catalog_item` is the single-row lookup `create_device_impl` (Section 3)
uses.

`com_object_ref_ids` returns every `com_object_ref.id` for a program,
`ORDER BY rowid` — the ids themselves (`A-1_O-1_R-1`, `A-1_O-1_R-10`,
`A-1_O-1_R-2`, …) don't sort into document order lexically, and `rowid`
(present on this table; it is not declared `WITHOUT ROWID`) preserves
insertion order, which is ingest/document order.

**`enrich.rs`'s `apply` becomes `pub`.** It already takes exactly
`(project, com_id, ref_id, view, issues)` and only touches the one
com-object instance named by `com_id` — nothing about its current shape
assumes it's being called from `enrich()`'s own per-project loop. Device
creation (Section 3) calls it directly, once per newly created
com-object, instead of duplicating its DPT-list/text/flags/size mapping.

## 3. `knx-server` wiring

**`AppState` gains `product_db: Option<Mutex<knx_productdb::Connection>>`.**
`AppState::new` tries `knx_productdb::default_path()` then
`open_and_migrate`; any failure (no path derivable, file missing, open
error) leaves it `None` — never a startup error, same graceful-degradation
stance every other product-db consumer already takes (ADR-0012's "missing
product database is ordinary, not an error").

**Bonus fix, same root cause, bundled here rather than split into its own
task.** `import_and_project` (backing both `open_project_impl` and
`open_project`) calls `import_ets_project_with(path, &conn,
ImportOptions::default())` — `ImportOptions::default()` has always had
`product_db: None`, which means every `.knxproj` opened through
`knx-server` (desktop and web deployment both) has never been enriched
from the product database, unlike `knx import --product-db` on the CLI.
Nothing previously wired a connection in for it to use. This slice adds
one (`state.product_db`), so `import_and_project` passes `ImportOptions {
product_db: state.product_db.as_ref().map(|m| &*m.lock()...) }` — exact
locking shape decided during implementation, whatever avoids holding the
lock longer than the import call needs.

**New `domain.rs` function**, `create_device_impl(state, line_id:
Option<u32>, catalog_item_id: String, name: String) -> Result<ProjectTree,
String>`:

1. Lock `product_db` (`"no product database configured"` if `None`).
   Look up `catalog_item(id)` (`"catalog item not found"` if absent).
   If `hardware2program_ref_id` is `Some`, `resolve_program` it; if that
   also resolves, `com_object_ref_ids` for it, then `com_object_view` per
   ref id, collecting `(ref_id, ComObjectView)` pairs. A missing hardware
   program, or a hardware2program with no application program, both fall
   through to zero com-objects — not an error; some catalog items are
   passive hardware with no program to seed from. Unlock `product_db`
   before the next step (no nested lock ordering with `project`).
2. Lock `project`. For each collected `(ref_id, view)`: allocate a
   `ComObjectInstanceId`, build a shell (`number: view.number.unwrap_or(0)
   as u16` — a plain field, not part of the Override fill-pass in step 3;
   `source: SourceRef { path: ref_id.clone(), ets_id: ref_id.clone() }` —
   the real productdb ref id, required so step 3 can look the com-object
   back up the same way `enrich()` does; everything else `Override::
   Absent`/`ResolvedFlags::none()`/`size: None`/`links: vec![]`/
   `module_instance: None`). Allocate a `DeviceId`; build the
   `DeviceInstance` (`source`: synthetic `KB-DEV-<id>`, matching
   `create_group_address_impl`'s own `KB-GA-<id>` convention;
   `address: None`; `product_ref`/`program_ref` from the catalog row,
   empty string if either was absent; `commissioning: CommissioningState::
   default()`; `visibility_calculated: true`). Apply `Command::
   CreateDevice` through `state.command_stack` (undoable, same as every
   other edit).
3. Still under the same `project` lock: for each `(com_id, ref_id, view)`
   from step 1, call `knx_productdb::enrich::apply(project, com_id,
   ref_id, &view, &mut issues)` — the "seeded once at creation, not
   re-resolved on every load" pass. Not pushed onto the undo stack, same
   as import's own enrichment isn't undoable: undoing a `CreateDevice`
   removes the device regardless of which slots got filled, and redo
   restores it exactly as `DeleteDevice`'s inverse captured it (Section
   1) — enrichment never needs to run twice for the same device.
   `issues` (ambiguous DPT lists, missing com-object-ref rows) are
   collected but not surfaced anywhere this slice — see Known
   Limitations below.
4. Return the projected tree.

**New routes** (`routes.rs`, mirroring the existing `create_group_address`/
`delete_group_address` pair's shape):

- `GET /api/catalog/manufacturers`
- `GET /api/catalog/items?manufacturer=&search=`
- `POST /api/devices` — body `{ line_id: Option<u32>, catalog_item_id: String, name: String }`
- `DELETE /api/devices/:id`

No frontend caller for any of these four routes yet.

## 4. Testing

- `knx-core`: `CreateDevice`/`DeleteDevice` apply+inverse round trip
  (create then undo restores the prior project exactly); delete refuses
  with `DeviceHasLinks` when a com-object still links to a group address;
  redo after a delete restores the device with whatever values were
  present at delete time (proving redo doesn't re-run enrichment or lose
  it).
- `knx-productdb`: `catalog_items`/`catalog_item`/`com_object_ref_ids`
  against the existing fixture XML in `query.rs`'s own test module; one
  test calling `enrich::apply` directly (not through `enrich()`) to prove
  it's safe to reuse standalone, not just from the per-project loop.
- `knx-server`: route tests for create (with a product db configured, and
  the "no product database" error path with none configured) and delete
  (success; refusal when linked); one test asserting a server-opened
  `.knxproj` is now enriched when `KNX_DATA_DIR`'s sibling product-db path
  has one available (the bonus fix).

## 5. Documentation

- `GAP_ANALYSIS_ETS.md`: T1 and T3 marked done (backend only, same
  phrasing T4-T6 used); T2 stays open.
- `IMPLEMENTATION_STATUS.md`: new cycle entry.
- `KNOWN_LIMITATIONS.md`: new entry — `EnrichmentIssue`s from device
  creation (Section 3, step 3) are silently dropped, unlike import's
  `ImportReport`; a device created against an application program with an
  ambiguous DPT list on one of its com-objects gets that com-object with
  no DPT set and no visible warning, recoverable by hand via the existing
  `SetComObjectDpt` UI but not reported.
