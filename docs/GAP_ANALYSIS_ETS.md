# GAP_ANALYSIS_ETS.md

A systematic comparison of KNXBench against ETS's feature set, as of
**Session 6, Cycle 5 plus the 2026-09-06 topology/group-range/group-link
command layer and editable device/com-object descriptions** (2026-09-06 —
see [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md)).

This document does not duplicate [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md)
(consequences of evidence gaps or recorded decisions on *existing* features)
or `ideas.md` (an informal wish list). It is a feature-coverage audit: for
every major ETS capability area, what exists here, what is missing, and a
task backlog to close the gaps that are worth closing. Some entries restate
a KNOWN_LIMITATIONS item where that item *is* the gap (cross-referenced, not
copied); most are new findings not documented anywhere else yet.

Method: read [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md),
[KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md), [ROADMAP.md](ROADMAP.md), and
`ideas.md`; inspected `knx-core::command` (the full list of editable
operations) and `apps/knx-web/src` (the full list of screens) directly,
rather than trusting prose alone. ETS's feature set is the general knowledge
of ETS 5/6 as a professional tool, not a specific verified version — treat
"ETS does X" below as the general claim it is, not a citation.

---

## A. Domain model gaps

| # | Gap | ETS has | KNXBench has | Notes |
|---|-----|---------|---------------|-------|
| A1 | **`Functions`** | Groups several group addresses under one named function (e.g. "Living room ceiling light" = switch + status + dim). | Not modelled at all. | Flagged since RESEARCH §12/ROADMAP "open questions"; absent from the one reference project, never designed. |
| A2 | **KNX Secure** (Data Secure, IP Secure, `.knxkeys` keyring) | Full support: secure group communication, secure tunnelling/routing, keyring import/export. | Not implemented; `knx-secure` is an empty, deliberately isolated crate. | [KNOWN_LIMITATIONS.md §8](KNOWN_LIMITATIONS.md#8-knx-secure-is-not-implemented), [§26](KNOWN_LIMITATIONS.md#26-busconnection-does-not-yet-support-knx-ip-secure). |
| A3 | **Parameter semantics** (`Dynamic`/`choose`/`when` tree) | Renders a parameter UI per device, with visibility/enable rules. | Parameters preserved as opaque values only; no interpretation. | [KNOWN_LIMITATIONS.md §3](KNOWN_LIMITATIONS.md#3-device-parameters-are-preserved-but-not-interpreted). |
| A4 | **Schema coverage** | Reads any ETS3/4/5/6 project. | Schema 11 (ETS4) fully known; schema 23 (ETS6) detected and refused by name; 12-22 undocumented. | [KNOWN_LIMITATIONS.md §1](KNOWN_LIMITATIONS.md#1-single-sample-bias). |
| A5 | **`.knxprod` scheme ≥ 12** | Reads current manufacturer product files directly. | Only master data scheme 11 is readable. | [KNOWN_LIMITATIONS.md §11](KNOWN_LIMITATIONS.md#11-knxprod-files-for-master-data-scheme--12-cannot-be-imported-directly). |
| A6 | **Password-protected projects** | Opens ZipCrypto (ETS4/5) and AES/PBKDF2 (ETS6) protected projects. | Detected, refused, never decrypted. | [KNOWN_LIMITATIONS.md §13](KNOWN_LIMITATIONS.md#13-password-protected-projects-are-refused-not-decrypted). |

## B. Editing / CRUD gaps

ETS lets you build a project from nothing. KNXBench today can only *edit*
what an ETS import (or a prior `.knxdb` save) already contains.
`knx-core`'s `Command` enum has **sixteen** variants as of the
2026-09-06 topology/group-range/group-link layer:
`SetIndividualAddress`, `SetComObjectDpt`/`RestoreComObjectDpt`,
`SetDeviceDescription`, `SetComObjectDescription`/`RestoreComObjectDescription`,
`CreateGroupAddress`/`DeleteGroupAddress`, `CreateArea`/`DeleteArea`,
`CreateLine`/`DeleteLine`, `MoveDeviceToLine`, `CreateGroupRange`/
`DeleteGroupRange`/`RenameGroupRange`, `LinkComObject`/`UnlinkComObject`.
The last ten have an `apps/knx-server` HTTP route each but **no frontend UI**
yet (see T4-T6 in the task backlog) — a backend command closes the *model*
gap but not the *usability* gap until a screen exists to drive it. Everything
else below has no command and therefore no UI, regardless of whether the
underlying model field exists.

| # | Gap | Impact |
|---|-----|--------|
| B1 | **No device creation.** No "insert product from catalog" workflow exists — a device can only arrive via ETS import. | You cannot start a project from scratch, or add one device to an existing project, without ETS. This is arguably the single biggest parity gap: ETS's core workflow (catalog → drag device onto a line) has no equivalent here at all. |
| B2 | **No device deletion.** | A device, once imported, cannot be removed. |
| B3 | **Topology CRUD has no UI.** `CreateArea`/`DeleteArea`/`CreateLine`/`DeleteLine`/`MoveDeviceToLine` exist as `knx-core` commands with `apps/knx-server` routes (2026-09-06) — no longer import-only at the model/API layer — but no frontend screen calls any of them. | Cannot add/remove/rename an area or line, or move a device between lines, from the UI. |
| B4 | **No building-part CRUD.** Building parts are projected and rendered but explicitly read-only in the Inspector — "no `Command` exists for either yet" (per IMPLEMENTATION_STATUS Session 5 cycle 5). | Cannot create a building/floor/room, rename one, or move a device between rooms. |
| B5 | **Group-range CRUD has no UI.** `CreateGroupRange`/`DeleteGroupRange`/`RenameGroupRange` exist as commands and routes (2026-09-06); only individual group addresses have a UI (create/delete, cycle 9) — the containing `GroupRange` still can't be created, renamed, or deleted from the frontend. | A UI-created group address still has no `GroupRange` to nest in *through the UI* — see [KNOWN_LIMITATIONS.md §21](KNOWN_LIMITATIONS.md#21-a-ui-created-group-address-has-no-ets_id-and-is-dropped-on-export), a direct symptom, only partially resolved. |
| B6 | **No communication-object flag editing.** Read/write/transmit/update/communication flags are projected and shown in the Inspector but are display-only — "no `Command` exists yet to edit a flag" (Session 5 cycle 4 notes, restated in the "Next session" backlog). | Cannot re-flag a comm object (e.g. turn on "read on start") without ETS. |
| B7 | **Group-link editing has no UI.** `LinkComObject`/`UnlinkComObject` exist as a command and route (2026-09-06), finally calling the `check_group_link_target_exists` validation that already existed; no frontend screen drives it yet. | Cannot wire up a new device's comm objects to group addresses in the UI — compounds B1: even if device creation existed, there would be no screen to link it afterward. |
| B8 | **No parameter editing.** (See A3 — no interpretation means no editor is possible yet regardless.) | |
| B9 | **No bulk/multi-select operations.** Every edit in the UI targets exactly one entity (one device's address, one comm object's DPT, one group address create/delete). | No "select 20 devices, change all their addresses' area", no multi-delete, no copy/paste of a device with its parameters — all standard ETS workflows for any project past a handful of devices. |
| B10 | **No drag-and-drop anywhere in the UI.** CLAUDE.md's UI/UX section lists drag & drop as a target capability. | Every structural change that ETS does by dragging (device onto a line, device onto a room, GA onto a comm object) has no equivalent gesture here, and per B1-B7 mostly has no non-drag equivalent either. |
| B11 | **Undo history is session-only**, never persisted to `.knxdb` (explicit design choice, restated across several cycles). | Closing and reopening a project loses all undo history — ETS's own undo is also session-scoped, so this one is closer to parity than most, but worth listing since it's a real behavioral difference from a saved-and-reopened ETS project's expectations. |

## C. Import / export & compatibility gaps

| # | Gap | Notes |
|---|-----|-------|
| C1 | **No project comparison/diff.** ETS can compare two project versions structurally. KNXBench's `compare.rs` exists only as an internal roundtrip-equality oracle for tests, not a user-facing feature. | No way to answer "what changed between these two saves" without external tooling — compounded by [KNOWN_LIMITATIONS.md §9](KNOWN_LIMITATIONS.md#9-project-files-are-not-diffable) (SQLite isn't diffable at the file level either). |
| C2 | **No CSV/Excel group-address import or export.** A common ETS workflow for bulk-authoring group addresses outside the tool. | Not present in any form — not import, not export. |
| C3 | **No partial/selective import.** ETS import here is all-or-nothing per project. | Cannot import "just this one line" or "just this device" from a `.knxproj`. |
| C4 | **Export has no UI caller anywhere.** `export_ets_project` (in `knx-app`) is exercised only by its own crate's tests — no CLI subcommand, no `knx-server` route, no frontend button. | A user cannot export a `.knxproj` from this application today, through any interface, despite the exporter itself being implemented and tested. This is worth stating plainly: **import works end-to-end for a user; export does not.** |
| C5 | **No signed export**, and ETS acceptance of an unsigned one is unverified. | [KNOWN_LIMITATIONS.md §5](KNOWN_LIMITATIONS.md#5-exports-are-unsigned-and-ets-acceptance-is-untested). |
| C6 | **No online device-catalog update.** ETS pulls manufacturer catalog updates from an online service (myKNX / KNX Online Catalog). | `knx-productdb` only ingests what a `.knxproj` already bundles — there is no independent product-database update path at all. |

## D. UI/UX gaps vs. the ETS workbench

| # | Gap | Notes |
|---|-----|-------|
| D1 | **No graphical topology view.** ETS's Topology tab shows areas/lines/couplers/devices as a diagram. | KNXBench's Project Explorer is a tree, not a diagram; there is no visual representation of the bus structure at all. |
| D2 | **No building/floor-plan graphical view.** ETS's Building view can show rooms spatially (and, with the right edition, overlay them on a floor plan image). | Building parts are a tree branch only, per Session 5's own scope; no spatial/graphical representation exists or is planned in DATA_MODEL. |
| D3 | **No device catalog browser.** See B1 — there is no UI screen listing manufacturers/products/hardware variants from `knx-productdb` at all, editing aside. | The data exists (`knx-productdb::query`) but nothing in `apps/knx-web` reads it directly; enrichment is the only consumer today. |
| D4 | **No printing / documentation export.** ETS can print topology, building, device, and group-address reports (to paper or PDF). | No print or PDF/document-export path exists anywhere in the application. |
| D5 | **No live Group Monitor GUI.** `knx bus monitor`/`bus write` exist as CLI subcommands (Session 6) with raw-byte, no-DPT-decoding output; ETS's Group Monitor is a GUI table, DPT-decoded, filterable, with send-from-the-table. | The bus-communication features that exist have no desktop/web front end at all — they're developer/CLI tools today, not end-user features. |
| D6 | **No bus/line diagnostics UI.** ETS can scan a line for connected devices, ping/identify a device, and show its individual info (mask version, order number) read live from the bus. | `knx-net` has no such capability yet (see Session E below) and there is no UI slot reserved for it either. |
| D7 | **No import-report screen.** `ImportReport` (errors/warnings/unsupported list) is real and populated, but the frontend only surfaces it as toast notifications for errors — there is no dedicated screen to review the full report (all warnings, the unsupported-devices list, opaque-passthrough summary) after the initial import moment has passed. | A user who dismisses the import toasts has no way to revisit what was lost or flagged, short of re-importing. |
| D8 | **No settings/preferences beyond theme.** ETS has a Workbench-wide options dialog (default group-address style, backup behavior, language, etc). | `ThemePanel.tsx` is the only settings surface that exists. |
| D9 | **Two near-duplicate modal-overlay implementations** (Search, Command Palette) with an unaddressed accessibility gap. | Already tracked: [KNOWN_LIMITATIONS.md §20](KNOWN_LIMITATIONS.md#20-command-palette-and-search-share-overlay-css-and-an-accessibility-gap-unaddressed). Restated here only because it will get worse, not better, once D1/D5/D6 add more overlay-like screens without a shared shell. |

## E. KNXnet/IP & commissioning gaps

| # | Gap | Notes |
|---|-----|-------|
| E1 | **No commissioning at all.** No individual-address programming (via the device's programming button), no application-program download, no memory read/write. | Explicit, permanent scope decision — [KNOWN_LIMITATIONS.md §7](KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-out-of-scope). Listed here for completeness of the gap picture, not as a proposed task: CLAUDE.md's "only implement protocol behavior that is technically verified" plus the bricking risk on real hardware makes this a durable non-goal, not a backlog item. |
| E2 | **No line-scan / device-discovery-on-the-bus.** `knx bus discover` (Session 6 cycle 3) finds *KNXnet/IP gateways* on the LAN, not KNX devices on a line (that needs an individual-address broadcast scan over the bus itself, a different operation). | Easy to conflate with "Device Discovery" in `ideas.md`, which is about the same gateway-discovery feature already shipped — this is a distinct, unaddressed capability. |
| E3 | **No KNX IP Secure.** | [KNOWN_LIMITATIONS.md §26](KNOWN_LIMITATIONS.md#26-busconnection-does-not-yet-support-knx-ip-secure); explicitly shelved once already (Session 6 cycle 4/5 planning). |
| E4 | **No DPT-aware bus tooling.** `bus monitor`/`bus write` operate on raw `GroupValue::Short`/`Bytes` — no decoding/encoding against a comm object's actual DPT. | A user has to know the raw encoding of the value they're reading or writing; ETS's Group Monitor decodes/encodes per DPT automatically. |
| E5 | **Docker discovery needs `--network host`.** | [ROADMAP.md](ROADMAP.md) "carried in from web/Docker deployment target"; unresolved, tracked but not fixed. |
| E6 | **No custom routing multicast address.** | [KNOWN_LIMITATIONS.md §31](KNOWN_LIMITATIONS.md#31-knxnetip-routing-has-no-custom-multicast-address-override). |

## F. Non-functional / operational gaps

| # | Gap | Notes |
|---|-----|-------|
| F1 | **No authentication on the web/Docker deployment.** | [KNOWN_LIMITATIONS.md §22](KNOWN_LIMITATIONS.md#22-the-webdocker-deployment-target-has-no-authentication); deliberate LAN-only scope, not a bug. |
| F2 | **No multi-user/concurrent-edit support.** `knx-server` holds one project behind one `Mutex` — a second connected client editing the same project has no conflict detection, merge, or locking at all. | Not documented as a limitation anywhere yet — new finding. Only matters once more than one person opens the same web deployment at once, which the current single-project server model doesn't anticipate. |
| F3 | **The project licence is undecided** (`AGPL-3.0-or-later` placeholder). | [KNOWN_LIMITATIONS.md §10](KNOWN_LIMITATIONS.md#10-the-project-licence-is-not-decided). |
| F4 | **No ETS-style "App"/plugin ecosystem.** ETS 6 supports third-party ETS Apps (e.g. manufacturer diagnostic tools) embedded in the workbench. | Nothing analogous exists or is planned; not currently blocking anything, listed for completeness since it's a real ETS differentiator for manufacturer-specific tooling. |

---

## Task backlog

Tasks are grouped by rough priority/dependency, not a fixed session number —
several are large enough to be their own future "session" the way
Sessions 0-7 were originally scoped, and CLAUDE.md's rule against
prematurely starting a later phase before an earlier architectural
dependency is resolved still applies (e.g. T1-T5 below are a prerequisite
chain, not independent picks).

Each task: **what**, **why**, **depends on**.

### Tier 1 — closes the biggest usability gap (cannot author a project from scratch)

- **T1. Device-from-catalog insertion command. Done (2026-09-08, backend
  only — see [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md)).**
  `Command::CreateDevice` lands, seeding a device's communication-object
  instances from `knx-productdb` once at creation time
  (`knx_productdb::enrich::apply`, made `pub` and reused directly rather
  than duplicated) instead of on every load. `apps/knx-server` gains
  `POST /api/devices` plus the `GET /api/catalog/manufacturers`/
  `GET /api/catalog/items` routes T2's future browser will call; no
  frontend caller yet. Closes **B1**.
- **T2. Device catalog browser UI.** A new screen/panel listing
  manufacturers → products → hardware variants from `knx-productdb`,
  with search, feeding T1's create command. Closes **D3**, prerequisite
  UI for **T1**.
- **T3. Device deletion command. Done (2026-09-08, backend only).**
  `Command::DeleteDevice` refuses (`CommandError::DeviceHasLinks`) if any
  of the device's communication objects still links to a group address —
  the explicit choice this task's own text asked for, matching
  `DeleteArea`/`DeleteLine`/`DeleteGroupAddress`'s existing
  refuse-with-dependents convention rather than a silent cascade.
  `apps/knx-server` gains `DELETE /api/devices/{id}`; no frontend caller
  yet. Closes **B2**.
- **T4. Topology CRUD commands.** **Done** (2026-09-06, backend only —
  see [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md)). `CreateArea`/
  `DeleteArea`/`CreateLine`/`DeleteLine`/`MoveDeviceToLine` land as
  `knx-core` commands with `apps/knx-server` routes; no frontend UI yet.
- **T5. Group-range CRUD commands.** **Done** (2026-09-06, backend only).
  `CreateGroupRange`/`DeleteGroupRange`/`RenameGroupRange` land; the
  export-drop bug ([KNOWN_LIMITATIONS.md §21](KNOWN_LIMITATIONS.md#21-a-ui-created-group-address-without-a-range-is-still-dropped-on-export--partially-resolved))
  is only partially closed — see that entry for why `range_id` stays
  optional until a UI exists to pick one.
- **T6. Group-link editing command.** **Done** (2026-09-06, backend
  only). `LinkComObject`/`UnlinkComObject` land, finally calling the
  validation.rs function that already existed for this
  (`check_group_link_target_exists`).
- **T23. Topology/group-range/group-link UI. Done (2026-09-07).**
  T4-T6's ten commands are
  reachable over HTTP but not from any screen: an area/line tree-edit UI
  in the Project Explorer, a "Group Ranges" branch (sibling to cycle 9's
  "Group Addresses" branch) with create/rename/delete, a range picker so
  a UI-created group address gets a real parent (closing
  [KNOWN_LIMITATIONS.md §21](KNOWN_LIMITATIONS.md#21-a-ui-created-group-address-without-a-range-is-still-dropped-on-export--partially-resolved)
  for real), and a link/unlink control on the comm-object Inspector row.
  Closes the remaining UI half of **B3**, **B5**, **B7**. Depends on:
  none architecturally — T4-T6's routes and `knx-projection`'s new
  `GroupRangeNode` already exist; this is frontend-only work, the same
  shape as cycle 9's group-address UI.

  **Slice 1 done (2026-09-07).** The "Group Ranges" branch (nested
  main/middle, create/rename/delete, `treeUtils.ts`'s `nestGroupRanges`
  rebuilding the hierarchy from `GroupRangeNode`'s flat `parent` pointer)
  and the range `<select>` on `NewGroupAddressRow` both ship — closing
  **B5** and the picker half of KNOWN_LIMITATIONS §21 (still not a
  *forced* choice, see that entry).

  **Slice 2 done (2026-09-07).** The link/unlink control on the
  comm-object Inspector row ships, closing **B7**. `knx-projection`
  gains `ComObjectNode.links: Vec<GroupLinkNode>` — the projection had no
  field for a comm object's *existing* `GroupLink`s at all until now,
  found while scoping this slice, not a pre-existing doc gap — resolved
  per-link against every installation's group addresses (a link names its
  target by id alone, with no installation of its own to narrow the
  search), defensively `None`-address/name on a dangling link rather than
  panicking. `Inspector.tsx`'s `NewGroupLinkRow` picker is
  `installations[0]`-only (`Command::LinkComObject`'s own restriction);
  `GroupLinkRow`'s Unlink is not (`Command::UnlinkComObject` carries no
  such check).

  **Slice 3 done (2026-09-07) — T23 complete.** The area/line tree-edit
  UI ships, closing **B3** and the last open piece of T23: `AreaItem`/
  `LineItem` in the Project Explorer become selectable (previously
  expand-only, unlike every other tree row) with `NewAreaRow`/
  `NewLineRow` create affordances (`medium_ref` pre-filled `"MT-0"` — an
  opaque, uninterpreted reference, per `Line`'s own doc comment, so no
  dropdown to pick from); `Inspector.tsx` gains `AreaInspector`/
  `LineInspector` (summary plus Delete, no rename field — no
  `RenameArea`/`RenameLine` command exists, unlike `GroupRangeInspector`)
  and `LineMoveField` on the device Inspector (a `Command::
  MoveDeviceToLine` picker, independent of `AddressField`'s individual
  address per that command's own doc comment), hidden rather than shown
  disabled for a device `treeUtils.ts`'s new
  `findDeviceLineInFirstInstallation` can't place in `installations[0]`'s
  topology at all (a building-only device, or a later installation) —
  the same "nothing sensible to show" case `canDelete` gates handle by
  disabling elsewhere, applied to visibility here since there's no
  current value to show. `treeUtils.ts` also gains `findArea`/`findLine`,
  matching every other selectable kind's own find helper.

### Tier 2 — closes remaining single-field-editor gaps

- **T7. Communication-object flag editing.** `Command::SetComObjectFlags`
  (read/write/transmit/update/communication, and priority if modelled),
  wired to the existing Inspector fields that already display them
  read-only. Closes **B6**.
- **T8. Building-part CRUD commands.** `CreateBuildingPart`/
  `DeleteBuildingPart`/`RenameBuildingPart`/`MoveDeviceToBuildingPart`.
  Closes **B4**.
- **T9. Bulk/multi-select operations.** Multi-select in the Project
  Explorer (devices, group addresses) plus batch variants of existing
  single-entity commands (batch address reassignment, batch delete).
  Design note: needs a `Command::Batch(Vec<Command>)` wrapper or
  equivalent for atomic undo of a multi-entity edit — worth its own
  design spec given `CommandStack`'s current one-command-one-inverse
  shape. Closes **B9**.

### Tier 3 — export & reporting parity

- **T10. Wire up `export_ets_project` to a real interface.** A
  `knx export` CLI subcommand, a `knx-server` route, and a frontend
  "Export to .knxproj" action — the exporter itself is done and tested,
  only its user-facing entry points are missing. Closes **C4**.
- **T11. Import-report review screen.** A dedicated panel (reachable
  after the initial import, not just at import time) listing every
  warning, unsupported item, and opaque-passthrough summary from the
  `ImportReport` that produced the currently open project. Closes **D7**.
- **T12. CSV group-address import/export.** A common bulk-authoring
  workflow independent of a full `.knxproj` round trip. Closes **C2**.
- **T13. Project documentation export (PDF/HTML report).** Topology,
  building, device, and group-address listings rendered to a printable
  document — start with one format (HTML, easiest to generate and to
  test deterministically) before considering PDF. Closes **D4**.
- **T14. Project diff/compare.** Promote `knx-etsproj::compare` (or a
  new `knx-app`-level comparison) from an internal test oracle to a
  user-facing "what changed between these two saves" report. Closes
  **C1**, partially mitigates [KNOWN_LIMITATIONS.md §9].

### Tier 4 — bus-facing UI (builds on Session 6's KNXnet/IP work)

- **T15. Group Monitor GUI.** A live telegram table in the desktop/web
  UI (not just the CLI), DPT-decoding values against the open project's
  comm objects, with send-from-the-table. Closes **D5**, **E4** (DPT
  awareness) so far as display goes.
- **T16. Device-catalog browser used for topology, not just insertion**
  — i.e. the same T2 screen doubling as a way to inspect an existing
  device's product/hardware identity without opening the full Inspector.
  Minor, bundle with T2 rather than schedule separately.
- **T17. Line-scan (bus-side device discovery).** A genuinely new
  `knx-net` capability — broadcasting/probing individual addresses on a
  connected line to enumerate real devices present, distinct from
  gateway discovery already shipped. Closes **E2**. Needs its own
  research spike against the spec (individual-address serial-number
  read services), same rigor as Session 6's existing cycles.

### Tier 5 — larger, multi-cycle efforts (own future "session")

- **T18. Parameter interpretation and editor.** Requires the `when/@test`
  grammar research spike (RESEARCH R3, still not started) before any
  editor UI is possible. Closes **A3**, prerequisite for a large share
  of realistic ETS parity. This is the single largest remaining gap by
  effort, and every parameter-adjacent gap above (T7 aside) is smaller
  in comparison.
- **T19. KNX Secure (Data Secure + IP Secure + keyring).** Needs sample
  key material and a real secured installation to verify against — a
  hard external dependency, not purely an engineering task. Closes
  **A2**, **E3**.
- **T20. `Functions` domain concept.** Needs its own ADR (new domain
  concept, not in DATA_MODEL today) before implementation, same as
  ROADMAP's existing rule for the project-notes idea. Closes **A1**.
- **T21. Graphical topology and building views.** A genuinely new UI
  paradigm (diagram/canvas rendering) alongside the existing tree-based
  Project Explorer, not a replacement for it. Closes **D1**, **D2**.
- **T22. Multi-user/concurrent-edit support for `knx-server`.** Only
  matters once the web deployment is used by more than one person at
  once; needs its own design (locking vs. merge vs. last-writer-wins,
  and what "conflict" even means for a `Command`-based undo model).
  Closes **F2**.

### Not backlog items — durable non-goals, listed for completeness only

- Commissioning/device download (**E1**) — permanent scope exclusion,
  see [KNOWN_LIMITATIONS.md §7](KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-out-of-scope).
  Do not schedule without a deliberate, explicit decision to reverse
  that exclusion, with real hardware to test against.
- Online device-catalog update (**C6**) — depends on an external KNX
  Association service whose terms/availability to a non-ETS tool are
  unknown; needs research before it's even a well-formed task.
- ETS App/plugin ecosystem (**F4**) — no demonstrated need yet; classic
  speculative-abstraction risk (CLAUDE.md "avoid speculative
  abstractions") until a concrete third-party integration motivates it.

---

## Cross-references

- [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md) — decisions and evidence
  gaps for *existing* features; consult before assuming any gap above is
  new or unrecorded.
- `ideas.md` — informal feature wish list, partially overlapping (e.g.
  device discovery, project documentation) with tasks above; reconcile
  before scheduling to avoid duplicate tracking.
- [ROADMAP.md](ROADMAP.md) — the Session 0-7 structure this backlog does
  not replace; Tier 5 items are, in effect, Session 8+ candidates.
