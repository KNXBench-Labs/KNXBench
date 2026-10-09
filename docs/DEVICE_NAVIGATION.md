# Device navigation and the project-wide device list

## Delivery scope

Owner-approved Devices package, 8 October 2026. Implemented locally on top of
`115b19f65eeebf7df21598681b34e829d2055099`; acceptance and execution scope are
recorded in [the delivery receipt](evidence/devices-navigation-2026-10-08.json).
Local implementation does not imply a main-branch merge, deployment or a new
AppImage. [ADR-0096](adr/0096-devices-navigation-and-catalogue-batch.md) records
the architecture decision.

## Navigation contract

- **Devices / Geräte** is a main navigation item and a command-palette action.
  It is disabled until a project is open. The Overview device count also opens it.
- A device selection opens the existing device editor **instead of** the current
  central view. The right-hand properties inspector remains the existing editor.
  The navigation marks Devices while either its list or the device editor is open.
- **Back** returns to the originating app view, including its building/group-range
  scope and originating selection. **All devices** opens the project-wide table.
  This is app-local navigation, not URL routing or browser history.
- The device list stays mounted, hidden while its editor or another view is active.
  Filter, sort, table scroll and checkbox selection therefore survive an editor visit.
  Project replacement/reset starts a new list lifetime; this is not persistent state.
- Ordinary device-row/name/address clicks open the editor. Checkbox toggles and
  Ctrl/Meta/Shift clicks use the existing shared multi-selection controller and
  BulkActionToolbar. Shift ranges use the filtered/sorted table order, never hidden
  filter matches. A new project snapshot prunes removed bulk targets.
- Detail-request identity guards remain in place. Pending device switches clear the
  old editable panel, delayed responses cannot replace a newer selection, and a
  failed selected-device refresh no longer leaves old editable fields on screen.

## Device list and identity

The table has address, name, manufacturer, product, order number, installation/KNX
line, installation/building/room, communication-object count and description columns.
Every column can be sorted. Text filtering covers names, addresses, descriptions,
product references, manufacturer/product/order text and both placement paths.
Address sorting is numeric; equal sort keys have the stable device ID as a tie-breaker.

One canonical device may occur in several placement branches. Its row is deduplicated
by **device ID**, while all topology/building placement occurrences are kept separately.
Paths retain installation names and complete building ancestry. An absent placement is
shown as Unassigned, not invented. The all-devices batch also supplies canonical devices
not reachable through any installation placement; the list must not omit them just
because the explorer is a placement projection.

## Read-only catalogue batch

`GET /api/devices?language=<optional language>` is additive: the existing
`POST /api/devices` create contract is unchanged. The authenticated project router
owns both methods; this GET has no command, save, import, download, tunnel or bus call.

Response version 1 contains:

- `schemaVersion: 1`, `serverIncarnation`, `snapshotRevision`;
- `devices`, sorted by canonical ID, each with `id`, the existing canonical `device`
  projection, `productRef`, product-only `resolution`, nullable manufacturer ID/name,
  product text, order number and answering/source product-text languages;
- `problem: null | "lookupFailed"` for a product-query failure.

The server copies canonical identity/reference inputs and the revision while holding
only the project lock, releases it, then holds only the product-database lock. It
resolves each distinct product reference once per batch through the existing
`knx_productdb::query::device_product` query. It does not evaluate device parameters,
communication-object activation or application compatibility. No core/store/product
schema or importer/exporter changes are involved.

Catalogue resolution is explicitly **product identity**, not commissioning evidence:

| State | Meaning |
| --- | --- |
| Resolved | The product reference answered from the installed product database |
| NoReference | No product reference is stated |
| NoDatabase | There is no open product database |
| NotInDatabase | The database does not contain the product reference |
| Unavailable | The product query failed; canonical device identity is still returned |

A product lookup failure retains canonical devices, nulls unavailable product fields
and reports `lookupFailed`; it must not turn unplaced devices into an empty table.
A transport/malformed-batch failure is shown explicitly and retains the current
project-tree rows. Canonical devices absent from that tree require a successful batch
read. The client refuses malformed/duplicate metadata IDs, unsupported response
versions and batches bound to a different server/revision. Delayed project/language
responses are ignored. Returning to the list or **Refresh product data** refetches;
there is no cross-request product cache or new version-selection policy.

Product text uses the existing requested-language/fallback rule and badge. Identifiers,
order numbers and project-owned names are not translated or rewritten.

## Links and bus boundary

A context-scoped UI index is built once per project snapshot. Known device IDs navigate
by ID. Observed individual addresses navigate only when **exactly one distinct current
project device** has that address; zero or multiple matches stay passive with an
explanation. A dangling explicit ID never falls back to a plausible address match.

Covered main-workbench entry points:

- explorer, topology/building diagrams/tables, search, wizard and existing Flow navigation;
- linked devices in the group-address table and the group-address inspector;
- monitor source addresses and retained line-scan result addresses;
- explicit Open in device editor links in download, address-programming and inspection
  panels, plus named readiness rows;
- the Overview device count.

These links are `type="button"` and cannot submit hardware-operation forms. Monitor
links stop row/key activation bubbling so opening the editor does not seed the compose
form. The existing monitor stays mounted and continues polling while hidden; Back
restores it without reconnecting or stopping capture. Opening a device is not an
instruction to scan, compare, download, program an address or write a telegram.

The independent diagnostic companion mounts no editing-navigation provider: these
new generic source links remain passive there, and its existing exact API/mutation
inventory stays unchanged. Existing guarded Flow/main-window navigation remains intact.

## Deliberately outside this package

URL deep links, browser Back/Forward integration, persistent view state, switchable
list grouping, arbitrary free-text log parsing and project-diff device links are not
implemented. Address links are current-project lookups, not physical-device identity or
historical installation evidence. There is no new ETS/device compatibility claim,
application version selector, deployment, hardware acceptance, native WebKitGTK/Orca
acceptance or complete accessibility certification.

A 5000-device synthetic production-browser scenario is measured in the receipt; it is
not an unlimited-size or real-installation performance guarantee. The table uses local
scrolling and no new virtualization/framework dependency.

## Verification surfaces

- `apps/knx-server/tests/http_device_product.rs`: the real HTTP router, read-only state,
  canonical/unplaced identities, missing/refused product data, query-failure disclosure,
  deterministic shared-reference rows and snapshot binding.
- `apps/knx-web/src/deviceList.test.ts`, `DevicesWorkspace.test.tsx`, `DeviceLink.test.tsx`,
  `App.test.tsx`: identity, placements, sorting/filtering, transient/preserved selection,
  malformed/delayed responses, translation fallback and editor invalidation.
- `apps/knx-web/src/DiagnosticsCompanion.test.tsx`: exact transitive capability inventory;
  new helpers are pure/UI-only and do not add mutating API calls.
- `apps/knx-web/e2e/devices-navigation.e2e.ts`: the real built frontend with strictly
  intercepted fictional API data, EN/DE, wide/narrow layouts, list/editor return paths,
  monitor polling and each diagnostics-panel link. Production runs use
  `playwright.devices.config.ts` in a loopback-only network namespace.
