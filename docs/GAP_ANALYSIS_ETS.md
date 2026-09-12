# GAP_ANALYSIS_ETS.md

A systematic comparison of KNXBench against ETS's feature set, as of
**Session 6, Cycle 5 plus the 2026-09-06 topology/group-range/group-link
command layer, editable device/com-object descriptions, catalog device
creation/deletion/flag editing (T1-T3, T7, T23), and the 2026-09-10
standalone `.knxprod` product-package installer** (2026-09-10 — see
[IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md)).

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
| A3 | **Parameter semantics** (`Dynamic`/`choose`/`when` tree) | Renders a parameter UI per device, with visibility/enable rules. | **Partially closed (2026-09-11, T18 slice 3).** `knx-productdb` parses, stores (schema v3, `dynamic_node`) and evaluates the tree headlessly, including expanding a `Module` node into its `ModuleDef`'s own stored tree (slice 2). **Slice 3 adds a real UI**: `GET`/`POST /api/device/{id}/parameters` plus `apps/knx-web`'s parameter panel read every field, write a top-level one, and show the recomputed activation set in the same response — the "No UI reads it and no value is ever written" claim this row used to make is no longer accurate. What stays open: module-scoped (per-channel) fields are read and displayed correctly but not *editable* (design D25 — the evaluator's flat `ValueMap` has no scope in its key); D16 (all instantiations of one `ModuleDef` still evaluate against identical parameter values) stays true in the narrower sense that the *activation* is identical even though D21/D22 now make the *displayed value* per channel correct; argument values (`NumericArg`/`TextArg`, stored but uninterpreted) and `AllocatorRef` (unattested) remain open; deep format validation for `Float`/`Text`/`IPAddress`/`Picture`/`Raw` is a non-empty-string check only. | [KNOWN_LIMITATIONS.md §3](KNOWN_LIMITATIONS.md#3-device-parameters-are-preserved-but-not-interpreted). |
| A4 | **Schema coverage** | Reads any ETS3/4/5/6 project. | Schema 11 (ETS4) fully known; schema 23 (ETS6) detected and refused by name; 12-22 undocumented. | [KNOWN_LIMITATIONS.md §1](KNOWN_LIMITATIONS.md#1-single-sample-bias). |
| A5 | **`.knxprod` scheme ≥ 12** | Reads current manufacturer product files directly. | **Partially closed (2026-09-10).** Standalone `.knxprod` product-package install (`knx_productdb::install_package`) reads scheme 11 and scheme 20 packages — verified against 3 real scheme-11 and 2 real scheme-20 files (`installs_the_readable_corpus`). Schemes 12-19, 21, 22 remain unread; `.vd2` (a pre-2013 legacy container, not the same ZIP/XML family at all) is explicitly rejected. **Both accepted out of scope, user decision 2026-09-11** — see [KNOWN_LIMITATIONS.md §11](KNOWN_LIMITATIONS.md#11-knxprod-files-for-master-data-scheme--12-cannot-be-imported-directly) for the dated notes. Full `.knxproj` *project* import is still schema-11/21/23 only — this row is about standalone `.knxprod` *product packages*, a narrower claim. | [KNOWN_LIMITATIONS.md §11](KNOWN_LIMITATIONS.md#11-knxprod-files-for-master-data-scheme--12-cannot-be-imported-directly). |
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
| B1 | **Closed (2026-09-08, T1+T2).** No "insert product from catalog" workflow existed — a device could only arrive via ETS import. | `Command::CreateDevice` (backend, T1) plus `CatalogBrowser.tsx`'s install/pick/name flow (UI, T2) now let a user add a device from the product catalog without ETS. *(Historical impact before closure: could not start a project from scratch or add one device without ETS — arguably the single biggest parity gap.)* |
| B2 | **Closed (2026-09-08, T3).** A device, once imported, could not be removed. | `Command::DeleteDevice` (refuses with `CommandError::DeviceHasLinks` if any comm object still links a group address) plus `DeviceInspector`'s Delete button close this. |
| B3 | **Closed (2026-09-07, T23 slice 3).** `CreateArea`/`DeleteArea`/`CreateLine`/`DeleteLine`/`MoveDeviceToLine` existed as commands/routes (2026-09-06) but no frontend screen called any of them. | The Project Explorer's area/line tree-edit UI (`NewAreaRow`/`NewLineRow`, `AreaInspector`/`LineInspector`, `LineMoveField`) now drives all five. |
| B4 | **Closed (2026-09-08, T8).** Building parts were projected and rendered but explicitly read-only in the Inspector — "no `Command` exists for either yet" (per IMPLEMENTATION_STATUS Session 5 cycle 5). | `CreateBuildingPart`/`DeleteBuildingPart`/`RenameBuildingPart`/`MoveDeviceToBuildingPart` plus a building-part create row in the Project Explorer and a rename/delete/move UI in the Inspector close this. |
| B5 | **Closed (2026-09-07, T23 slice 1).** `CreateGroupRange`/`DeleteGroupRange`/`RenameGroupRange` existed as commands/routes but only individual group addresses had a UI. | The "Group Ranges" branch in the Project Explorer (create/rename/delete, `nestGroupRanges`) plus the range `<select>` on `NewGroupAddressRow` close this. |
| B6 | **Closed (2026-09-08, T7).** Read/write/transmit/update/communication flags were projected and shown but display-only. | `Command::SetComObjectFlag`/`RestoreComObjectFlag` (one flag at a time, no bulk "clear to inherited" gesture) plus `ComObjectFlagsRow`'s five checkboxes on the comm-object Inspector row close this. |
| B7 | **Closed (2026-09-07, T23 slice 2).** `LinkComObject`/`UnlinkComObject` existed as a command/route but no frontend screen drove it. | The link/unlink control on the comm-object Inspector row (`NewGroupLinkRow`/`GroupLinkRow`) closes this. |
| B8 | **Closed for top-level fields (2026-09-11, T18 slice 3).** (See A3.) `Command::SetParameterValue`/`RestoreParameterValue` plus `ParameterPanel.tsx` let a user write a top-level parameter value with undo/redo. Module-scoped (per-channel) fields stay read-only (design D25); see A3 for why. | |
| B9 | **Closed (2026-09-10, T9).** Every edit in the UI targeted exactly one entity (one device's address, one comm object's DPT, one group address create/delete). | `Command::Batch(Vec<Command>)` composes existing single-entity commands with all-or-nothing apply/rollback; ctrl/shift-click multi-select of devices and group addresses in the Project Explorer plus a bulk-action toolbar (batch delete, batch move-to-line, batch move-to-building-part) drive it, undoable as one `Ctrl+Z`. Copy/paste of a device with its parameters remains out of scope. |
| B10 | **No drag-and-drop anywhere in the UI.** CLAUDE.md's UI/UX section lists drag & drop as a target capability. | Every structural change that ETS does by dragging (device onto a line, device onto a room, GA onto a comm object) has no equivalent gesture here, and per B1-B7 mostly has no non-drag equivalent either. |
| B11 | **Undo history is session-only**, never persisted to `.knxdb` (explicit design choice, restated across several cycles). | Closing and reopening a project loses all undo history — ETS's own undo is also session-scoped, so this one is closer to parity than most, but worth listing since it's a real behavioral difference from a saved-and-reopened ETS project's expectations. |

## C. Import / export & compatibility gaps

| # | Gap | Notes |
|---|-----|-------|
| C1 | **Closed for `.knxdb`-to-`.knxdb` comparison only (2026-09-10, T14).** A new `crates/knx-diff` crate (`diff_projects(&Project, &Project) -> ProjectDiff`, depending on `knx-core` only) computes a "KNXBench project diff" — never described as, or claiming parity with, ETS's own compare feature (no ETS-produced comparison sample exists anywhere in this repository to check against). Reachable from `knx diff <a.knxdb> <b.knxdb>` (`apps/knx-cli`), `POST /api/project/diff {path}` comparing the open project against a `.knxdb` file (`apps/knx-server`), and a "Compare with…" button (`ProjectDiffPanel.tsx`, `apps/knx-web`). | Answers "what changed between these two saves" for `.knxdb`-to-`.knxdb` comparisons. Does **not**: accept a raw `.knxproj` on either side; merge or apply a diff back onto a project; do a three-way (common-ancestor) comparison; do version history/time travel; or detect an ETS re-import's regenerated `RefId`s as "the same entity" (design spec §9 — full list in `KNOWN_LIMITATIONS.md` §51-§58). [KNOWN_LIMITATIONS.md §9](KNOWN_LIMITATIONS.md#9-project-files-are-not-diffable) is partially mitigated, not lifted: the `.knxdb` SQLite file itself is still not diffable at the file/version-control level — this closes the gap by giving the *application* a diff instead, per that entry's own "Lifted when" note. |
| C2 | **Closed (2026-09-10, T12).** A new `knx-csv` crate reads and writes "KNXBench group-address CSV v1" — a format KNXBench defines and documents itself, **not** a claim of ETS CSV compatibility (no verified ETS sample exists anywhere in this repository or the KNX Standard v3.0.0 corpus). Reachable from `knx ga-export`/`knx ga-import [--dry-run]` on the CLI, `POST /api/group-addresses/csv-export`/`csv-import` on the server, and two toolbar buttons in the web group-address view. | Bulk-authoring group addresses in a spreadsheet is now possible without a full `.knxproj` round trip. See `IMPORT_EXPORT.md §11` for the format and `KNOWN_LIMITATIONS.md` for what it deliberately does not do (re-address, delete, touch group ranges, or apply `DatapointType`/`MainGroup`/`MiddleGroup`). |
| C3 | **No partial/selective import.** ETS import here is all-or-nothing per project. | Cannot import "just this one line" or "just this device" from a `.knxproj`. |
| C4 | **Closed (2026-09-10, T10).** `export_ets_project` now has three real callers: `knx export` on the CLI, `POST /api/project/export` on the server, and an "Export to .knxproj…" button in the web Project Explorer. | Closing this also surfaced and fixed two real data-integrity bugs (see `IMPLEMENTATION_STATUS.md`'s T10 entry for the full account): (1) server-side ETS import used a throwaway store, so Save As never persisted opaque passthrough / manufacturer manifest data, and an export taken after it silently lost that data — fixed by carrying that data through `AppState` into every save; (2) the two functions that write those tables (`knx_store::insert_opaque`/`insert_manufacturer_refs`) were plain `INSERT`s with no clear-first step, so once (1)'s fix made every save call them, a plain repeated Save duplicated every row without bound — fixed by clearing the tables before insert, matching `save_project`'s own convention. `export_project` was also changed to read opaque/manifest from live `AppState` instead of re-opening `store_path` off disk, closing off the staleness risk described in [KNOWN_LIMITATIONS.md #18](KNOWN_LIMITATIONS.md#18-open_project-does-not-clear-the-previous-knxdb-store_path) for this specific data (the broader gap in #18 itself is unchanged and out of scope here). |
| C5 | **No signed export**, and ETS acceptance of an unsigned one is unverified. | [KNOWN_LIMITATIONS.md §5](KNOWN_LIMITATIONS.md#5-exports-are-unsigned-and-ets-acceptance-is-untested). |
| C6 | **No online device-catalog update.** ETS pulls manufacturer catalog updates from an online service (myKNX / KNX Online Catalog). | `knx-productdb` only ingests what a `.knxproj` already bundles — there is no independent product-database update path at all. |

## D. UI/UX gaps vs. the ETS workbench

| # | Gap | Notes |
|---|-----|-------|
| D1 | **No graphical topology view.** ETS's Topology tab shows areas/lines/couplers/devices as a diagram. | KNXBench's Project Explorer is a tree, not a diagram; there is no visual representation of the bus structure at all. |
| D2 | **No building/floor-plan graphical view.** ETS's Building view can show rooms spatially (and, with the right edition, overlay them on a floor plan image). | Building parts are a tree branch only, per Session 5's own scope; no spatial/graphical representation exists or is planned in DATA_MODEL. |
| D3 | **Closed (2026-09-08, T2; extended 2026-09-10 with package install).** There was no UI screen listing manufacturers/products/hardware variants from `knx-productdb` at all. | `CatalogBrowser.tsx` (opened from a `+ Add device` row) lists catalog items via `GET /api/catalog/manufacturers`/`GET /api/catalog/items`, with a manufacturer filter and search; it also gained an install file-picker for standalone `.knxprod` packages (`installProductPackage`, install-report/error display, post-install catalog refresh, in-modal creation-diagnostics rendering) on 2026-09-10. |
| D4 | **Closed for HTML only (2026-09-10, T13).** A new `crates/knx-report` crate (`render_html`) renders topology, buildings, group addresses, and devices into one self-contained HTML document — reachable via `POST /api/project/documentation-export` (`apps/knx-server`), `knx doc-export <store.knxdb> <out.html>` (`apps/knx-cli`), and an "Export documentation…" button (`DocumentationExportButton.tsx`, `apps/knx-web`). | Printing from inside the application and PDF generation without a browser both remain open — the document ships `@media print` rules and relies on the browser's own print-to-PDF dialog, which is not the same thing as native PDF generation or an in-app print preview. No claim of ETS report parity is made anywhere: no ETS-produced report sample exists in this repository to compare against, the same evidence gap [KNOWN_LIMITATIONS.md §38](KNOWN_LIMITATIONS.md#38-group-address-csv-exportimport-t12-has-no-verified-ets-interoperability) records for T12's CSV format. See `IMPORT_EXPORT.md §12` and the new `KNOWN_LIMITATIONS.md` entries this task adds. |
| D5 | **Closed for tunnelling, 2026-09-11 (T15).** `apps/knx-server` gains a `knx-net` dependency (ADR-0017) and a bus-monitor session (`apps/knx-server/src/bus.rs`); four routes (`POST /api/bus/monitor/start`, `POST /api/bus/monitor/stop`, `GET /api/bus/monitor/telegrams`, `POST /api/bus/write`); and `apps/knx-web` gets a live telegram table (`BusMonitorPanel.tsx`) with a client-side text/service-type filter and a compose/send form (`BusComposeForm.tsx`), DPT-decoded against the open project exactly as `bus monitor --project`/`bus write` already were. What did **not** close: routing (tunnelling only, D7 in the design spec); ETS-depth filtering (client-side text/service-type only, nothing like ETS's multi-criteria/saved filter sets); more than one session at a time (`409` on a second `start`); individual-addressed frames (not rendered as rows at all); and — the one that matters most — **none of it has been run against a physical KNX installation**. Full accounting: [KNOWN_LIMITATIONS.md §62](KNOWN_LIMITATIONS.md#62-the-group-monitor-gui-t15-is-tunnelling-only-single-session-client-filtered-and-has-never-talked-to-a-real-gateway). Design spec: `docs/superpowers/specs/2026-09-11-group-monitor-design.md`. | A user with an open project and a reachable gateway can now watch decoded group telegrams and send one, from the web/desktop UI, without a terminal — for one tunnelled gateway at a time. `knx bus monitor`/`bus write` remain the CLI-only path to routing and to a second concurrent connection. |
| D6 | **No bus/line diagnostics UI.** ETS can scan a line for connected devices, ping/identify a device, and show its individual info (mask version, order number) read live from the bus. | `knx-net` has no such capability yet (see Session E below) and there is no UI slot reserved for it either. |
| D7 | **Closed (2026-09-10, T11).** `ImportReport` (errors/warnings/unsupported list) is real and populated, but the frontend only surfaced it as toast notifications for errors — there was no dedicated screen to review the full report after the initial import moment had passed. | A new server-side `SessionLog` (`apps/knx-server`, in-memory, never persisted to `.knxdb`) plus a "Log" tab in the web UI (`LogPanel.tsx`) close this — see the T11 backlog entry below for the full shape. |
| D8 | **Partially addressed. No settings/preferences beyond theme and motion.** ETS has a Workbench-wide options dialog (default group-address style, backup behavior, language, etc). | A settings surface now exists — `SettingsPanel.tsx`, reached via a gear button in the toolbar — but it holds exactly three `<select>`s: Theme (over `THEMES`), Motion style, and Motion level (the last two new in T27, 2026-09-12, closing **D11**; see the rewritten [KNOWN_LIMITATIONS.md §43](KNOWN_LIMITATIONS.md#43-animations-have-no-in-app-switch-only-the-os-reduced-motion-preference)). It replaced `ThemeSwitcher.tsx`'s single theme `<select>`, which T27 deleted outright once its one consumer moved into the panel. None of ETS's actual options — default group-address style, backup behaviour, language (**D10**) — live in it or anywhere else. Row stays open: the surface got wider, not more ETS-equivalent. |
| D9 | **Closed (2026-09-12, T31).** Four consumers of the same overlay CSS with no shared component behind it — three of them (Search, Command Palette, Catalog Browser) near-duplicate modal *implementations*, plus Settings Panel, which reused the shape only — with an accessibility gap on top. | `apps/knx-web/src/Overlay.tsx` (new) is now the one component behind `.search-overlay`/`.search-panel`, rendered by all four former hand-rolled consumers: `role="dialog"`/`aria-modal="true"`, backdrop and panel-level `Escape` dismissal, initial focus, a `Tab` focus trap, and focus restoration on close, all in one place instead of four. `overlayShell.test.ts` fails the suite, naming the offender, if a fifth `.tsx` file ever contains the literal `search-overlay` outside `Overlay.tsx` — the same lift-trigger-missed failure mode that let this row reach four consumers unnoticed cannot repeat silently. The listbox half of the accessibility gap is also closed uniformly (`role="combobox"`/`listbox`/`option`, `aria-activedescendant`), and `CatalogBrowser.tsx`'s result list — previously `<li onClick>` with no keyboard path at all — gained `ArrowDown`/`ArrowUp`/`Enter`-to-pick, matching `Search.tsx`. What stays open, tracked in [KNOWN_LIMITATIONS.md §20](KNOWN_LIMITATIONS.md#20-command-palette-and-search-share-overlay-css-and-an-accessibility-gap--partially-resolved), which itself stays open for exactly these residues: no scroll-into-view for an off-panel highlight, no `inert`/`aria-hidden` on background content, no focus-visible styling pass, and no verification against a real screen reader — jsdom asserts wiring, not assistive-technology behaviour. Design: `docs/superpowers/specs/2026-09-12-modal-overlay-shell-design.md`. |
| D10 | **Open.** The UI is English-only, and the translated data already in the model is displayed in only one place. ETS ships a localized workbench and renders manufacturer/product/parameter text in the language the user picked. | Two distinct halves, still open on the chrome side. (a) *Chrome:* untouched — every user-facing string in `apps/knx-web` is a hard-coded English literal; `package.json` has no i18n dependency of any kind and there is no message catalogue, locale detection, or language setting (D8's missing options dialog is where one would live). (b) *Data:* T26's first slice (2026-09-12) gave this half its first reader. `knx-productdb`'s `parameter_views`/`parameter_type_enum_options` overlay the `translation (program_id, language, ref_id, attribute_name, text)` table (`migration.rs:248`) onto parameter text, parameter-ref text and enum option labels at exactly one surface — the device parameter panel (`GET`/`POST /api/device/{id}/parameters?language=`) — selected by a persisted Settings-panel setting. T32 (2026-09-12) added a second reader: the catalog browser overlays item `Name`/`VisibleDescription` in the selected language, and the ingestion gap underneath it is closed, so every `Languages` block in a package now reaches the database. T33 (2026-09-12) added a third: `com_object_view` overlays a communication object's `Text`/`FunctionText`/`VisibleDescription` the same way, and `GET /api/device/{id}?language=` applies it to `ComObjectNode::name`/`description` — but only where the stored override's layer is `Layer::Program` or `Layer::ProgramRef`; project-authored layers (`Instance`/`Inferred`/`UserEdit`) are never translated, and device creation/`enrich()` still bake untranslated text into the project file, on purpose. `knx_core::string_table`'s `Language`/`LocalizedString`/`StringTable` (with its `default_language` fallback, `Project::strings`, `project.rs:182`) still has no resolver against a *user-selected* language anywhere: `build_device_detail` only ever calls it with the fixed `default_language()`, and T33's overlay works around it rather than through it, substituting `knx-productdb` text into `ComObjectNode` after that call has already run — none of the three readers above teach the string table a new language. The project's own `Language` field remains an unread placeholder. See [KNOWN_LIMITATIONS.md §37](KNOWN_LIMITATIONS.md#37-imported-translations-are-stored-but-never-read-and-the-ui-is-english-only--partially-resolved-2026-09-12). Row stays open: the chrome half (T25) is untouched, and on the data side hardware- and master-scope translations are ingested but read by no surface at all — see [KNOWN_LIMITATIONS.md §64](KNOWN_LIMITATIONS.md#64-languages-blocks-outside-an-application-program-are-discarded-on-import). |
| D11 | **Closed (2026-09-12, T27).** Not an ETS parity gap — ETS has no comparable animation — but the user-facing control gap recorded here alongside D8 is resolved. | `apps/knx-web/src/motion.ts` adds two independent, persisted axes — level (`off`/`subtle`/`standard`) and style (`apple`/`glitch`) — surfaced in the new `SettingsPanel.tsx` and enforced structurally: every `transition:`/`animation:` declaration in `styles.css` sits inside a `@media (prefers-reduced-motion: no-preference)` block, and a new guard test (`motionGuard.test.ts`) fails the suite if a future declaration doesn't. `prefers-reduced-motion: reduce` still always wins — no `.ts`/`.tsx` file calls `window.matchMedia`, so nothing in-app can override it. T15's Group Monitor table, the one animated feature that had shipped ahead of this control, has been retrofitted with a guarded new-row highlight (`BusMonitorPanel.tsx`). What the switch and its guard still cannot do — no per-category control, guard blind spots for longhands/other stylesheets — is recorded rather than hidden: [KNOWN_LIMITATIONS.md §43](KNOWN_LIMITATIONS.md#43-animations-have-no-in-app-switch-only-the-os-reduced-motion-preference). |
| D12 | **No in-application help of any kind, and no end-user documentation.** ETS ships context help, tooltips throughout the workbench, and a user manual. | Measured, not remembered: the entire frontend contains **one** `title` attribute (`Inspector.tsx:218`, a communication-object flag's raw name), four `aria-label`s, zero `aria-describedby`, no tooltip component, no help panel, and no `F1` handler. All nine files in `docs/` are architecture/format documentation written for developers; none is reachable from inside the application. `commandRegistry.ts`'s `shortcutHint` is the only user-facing explanatory text, and it appears only inside the Command Palette. Distinct from **D8** (settings): this is explanation, not configuration. Tracked as **T28**, deliberately scheduled last — see [ROADMAP.md](ROADMAP.md)'s "In-application help and user documentation". |

## E. KNXnet/IP & commissioning gaps

| # | Gap | Notes |
|---|-----|-------|
| E1 | **No commissioning at all.** No individual-address programming (via the device's programming button), no application-program download, no memory read/write. | **Not excluded — required, blocked (ruling 2026-09-11).** Asked whether this is permanently out, the user said no: commissioning must work too, but the work waits until the KNX specification database is finished. The R5 research spike (2026-09-11, [RESEARCH.md §8.4](RESEARCH.md)) has since queried that database and documented the generic download/unload/reset/memory procedures from the Standard — that closes the *research* question, not this row: no commissioning code exists, nothing has been verified on hardware, and the product-specific `Legacy*` matrix and vendor-DLL download involvement remain undocumented. [KNOWN_LIMITATIONS.md §7](KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked) has the full account; CLAUDE.md's "only implement protocol behavior that is technically verified" plus the bricking risk on real hardware are why it hasn't started, not why it never will. Tracked as backlog task **T30** (Tier 5); this row stays open. |
| E2 | **No line-scan / device-discovery-on-the-bus.** `knx bus discover` (Session 6 cycle 3) finds *KNXnet/IP gateways* on the LAN, not KNX devices on a line (that needs an individual-address broadcast scan over the bus itself, a different operation). | Easy to conflate with "Device Discovery" in `ideas.md`, which is about the same gateway-discovery feature already shipped — this is a distinct, unaddressed capability. |
| E3 | **No KNX IP Secure.** | [KNOWN_LIMITATIONS.md §26](KNOWN_LIMITATIONS.md#26-busconnection-does-not-yet-support-knx-ip-secure); explicitly shelved once already (Session 6 cycle 4/5 planning). Folded into **T19**'s scope; deferred 2026-09-11 by user ruling, documented as a limitation, not rejected. |
| E4 | **Partially closed 2026-09-11 (T29), display side finished for tunnelling 2026-09-11 (T15).** A Standard-cited codec lives in `knx-core` (`crates/knx-core/src/dpt/codec.rs`), covering main types 1, 2, 3, 5, 6 (except `6.020`), 7, 8, 9, 12, 13, 14, 16, 17, 18. `apps/knx-cli bus monitor --project <path>` decodes; `bus write --dpt <DPST-m-s>` (or resolved from `--project`) encodes; **T15** wires the same `decode`/`encode`/`resolve_project_group_address_dpts` calls into `apps/knx-server`'s bus session and `apps/knx-web`'s telegram table, so a web/desktop user gets the same decoding the CLI already had, without a terminal — for tunnelling only, one session at a time. What is still open: the main types this codec never implemented (4, 10, 11, 15, 19, 20, 21-30 and the rest of the 46 `knx_master.xml` main types beyond the fourteen listed); no `knx_master.xml` DPT catalogue is consulted, so there are no enumeration names and no units beyond what the scaled subtypes' own arithmetic already implies; and T15's own display-side gaps (routing, filtering depth, multi-session, individual-address frames — see **D5** above). See `docs/KNOWN_LIMITATIONS.md` §§61-62 for the full accounting. | A user still has to know the raw encoding for anything outside the implemented main types; for the main types that are implemented, both the CLI and now the web/desktop GUI decode/encode per DPT through the same `knx-core` codec — see [KNOWN_LIMITATIONS.md §61](KNOWN_LIMITATIONS.md#61-the-dpt-codec-covers-fourteen-main-types-infers-rather-than-reads-its-input-and-leaves-several-encoding-questions-to-a-stated-ruling-rather-than-the-standard) for exactly what that codec's output does and does not match, including that it deliberately differs from a published ETS/AN188 reference figure by one step in a known case. |
| E5 | **Docker discovery needs `--network host`.** | [ROADMAP.md](ROADMAP.md) "carried in from web/Docker deployment target"; unresolved, tracked but not fixed. |
| E6 | **No custom routing multicast address.** | [KNOWN_LIMITATIONS.md §31](KNOWN_LIMITATIONS.md#31-knxnetip-routing-has-no-custom-multicast-address-override). |

## F. Non-functional / operational gaps

| # | Gap | Notes |
|---|-----|-------|
| F1 | **No authentication on the web/Docker deployment.** | [KNOWN_LIMITATIONS.md §22](KNOWN_LIMITATIONS.md#22-the-webdocker-deployment-target-has-no-authentication); deliberate LAN-only scope, not a bug. |
| F2 | **No multi-user/concurrent-edit support.** `knx-server` holds one project behind one `Mutex` — a second connected client editing the same project has no conflict detection, merge, or locking at all. | [KNOWN_LIMITATIONS.md §63](KNOWN_LIMITATIONS.md#63-knx-server-has-no-multi-userconcurrent-edit-support--one-shared-project-one-shared-undo-stack-no-conflict-detection-at-all). Only matters once more than one person opens the same web deployment at once, which the current single-project server model doesn't anticipate; still open, still unimplemented — tracked as **T22** below. |
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
  `GET /api/catalog/items` routes T2's `CatalogBrowser.tsx` now calls; no
  frontend caller until T2 shipped, same day. Closes **B1**.
- **T2. Device catalog browser UI. Done (2026-09-08).** `CatalogBrowser.tsx`
  is a new modal (opened from a `+ Add device` row on any line, or the
  Unassigned bucket, in the first installation's Project Explorer —
  `Command::CreateDevice`'s own `installations[0]` restriction) listing
  catalog items from `knx-productdb` via the already-existing
  `GET /api/catalog/manufacturers`/`GET /api/catalog/items` routes, with a
  manufacturer filter and a debounced search box; picking an item and
  naming the device calls the already-existing `POST /api/devices`. No
  backend change needed — see [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md).
  Closes **D3**, prerequisite UI for **T1**, which is now reachable end to
  end.
- **T3. Device deletion command. Done (2026-09-08).**
  `Command::DeleteDevice` refuses (`CommandError::DeviceHasLinks`) if any
  of the device's communication objects still links to a group address —
  the explicit choice this task's own text asked for, matching
  `DeleteArea`/`DeleteLine`/`DeleteGroupAddress`'s existing
  refuse-with-dependents convention rather than a silent cascade.
  `apps/knx-server` gains `DELETE /api/devices/{id}` (backend,
  2026-09-08); `DeviceInspector`'s own Delete button, gated the same
  `installations[0]`-reachable way as `AreaInspector`/`LineInspector`'s,
  landed the same day. Closes **B2**.
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

- **T7. Communication-object flag editing. Done (2026-09-08).**
  `Command::SetComObjectFlag`/`RestoreComObjectFlag` land, one flag at a
  time (`ComFlagKind`) rather than all five at once — a checkbox has only
  two states, so there is no "clear to inherited" gesture the way
  `SetComObjectDpt`/`SetComObjectDescription`'s empty-string convention
  gives text fields. Priority stays out of scope: `ResolvedFlags`/
  `ComFlags` model no priority field today, so there was nothing to wire.
  `apps/knx-web` gains a `ComObjectFlagsRow` of five checkboxes on the
  comm-object Inspector row — the flags were exported to `ComObjectNode`
  read-only in an earlier cycle but, contrary to this task's original
  text, were never actually rendered anywhere in the UI until now. Closes
  **B6**.
- **T24. Standalone `.knxprod` product-package install + honest creation
  diagnostics. Done (2026-09-10).** `knx_productdb::install_package` adds
  an atomic, transactional installer for a single `.knxprod` ZIP archive
  (no accompanying `.knxproj`): validates the whole archive before
  publishing any row, stores raw member bytes by SHA-256, rejects
  encrypted members/path traversal/duplicate names/oversized
  members/missing `knx_master.xml`/unsupported namespace/a full
  `.knxproj` project archive as a typed `PackageError`, is idempotent
  (`skipped: true` on a byte-identical re-install), and preserves
  first-winner provenance on catalog-item id conflicts. Verified against
  5 real-world files: 3 at master data scheme 11
  (`646704-04_ETS4_2012_47_DE_EN.knxprod`,
  `Weinzierl_730_KNX_IP_Interface_ETS4.knxprod`,
  `Weinzierl_730_KNX_IP_Interface_ETS4_v1.knxprod`) and 2 at scheme 20
  (`MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod`,
  `Dummy_Applikation_Secure.knxprod`) —
  `installs_the_readable_corpus` (`knx-productdb/tests/standalone_packages.rs`).
  The 6th corpus file, `Weinzierl_730_KNX_IP_Interface_ETS2-3.vd2`, is a
  pre-2013 ETS2-era SFX/`.vd_` archive (no `knx_master.xml`, not the same
  ZIP/XML container family at all) and is rejected by filename suffix
  alone — still never opened as a ZIP, never decrypted or parsed — with
  `PackageError::LegacyVd2 { sha256, len }` → `"legacy .vd2 product data
  is unsupported (sha256 <64 hex chars>, <len> bytes)"`
  (`crates/knx-productdb/src/package.rs`). **Closed 2026-09-11:** the
  design spec's acceptance criterion "the caller receives the archive
  hash/size in the error report where available" was unmet as of
  2026-09-10 (the filename check ran before any hash/size was computed);
  the whole-archive `MAX_PACKAGE_SIZE` guard now runs first so hashing
  stays bounded, then the `.vd2` check hashes the bytes and returns the
  evidence, verified against the real corpus file
  (`rejects_the_real_legacy_vd2_corpus_file_with_its_hash_and_size`) and
  a 3-byte synthetic fixture
  (`a_small_vd2_still_reports_hash_and_length`,
  `crates/knx-productdb/tests/standalone_packages.rs`), and against the
  HTTP 400 body
  (`malformed_and_legacy_product_uploads_are_typed_bad_requests`,
  `apps/knx-server/tests/http_product_install.rs`). Both real-corpus
  tests skip silently when the gitignored `OriginalData/` directory is
  absent, so a CI run without the local-only corpus checked out gets
  zero real-`.vd2`-file coverage of this path — a deliberate repository
  convention (commit `10df2a8`), not new to this change, but worth
  saying plainly. `knx products ingest` (CLI) and
  `POST /api/catalog/install` (HTTP) both surface the same typed errors
  as plain strings, so the evidence reaches both surfaces for free.
  `POST /api/devices` also stopped silently dropping
  `EnrichmentIssue`s: `CreateDeviceResponse { tree, diagnostics }` now
  returns typed `CreationDiagnostic`s (`ProgramlessProduct`/
  `AmbiguousDpt`/`ComObjectRefMissing`/`ProgramRefMissing`/
  `DynamicOrModuleNotEvaluated`, each with a server-computed `.detail()`
  string) and `CatalogBrowser.tsx` renders them in-modal instead of
  auto-closing. Partially closes **A5** (standalone `.knxprod` package
  install only, scheme 11/20 only — full `.knxproj` project import at
  scheme 20 is still unverified, see COMPATIBILITY.md §3); resolves
  [KNOWN_LIMITATIONS.md §35](KNOWN_LIMITATIONS.md#35-device-creation-enrichmentissues-are-silently-dropped);
  extends **D3**.
- **T8. Building-part CRUD commands. Done (2026-09-08).**
  `CreateBuildingPart`/`DeleteBuildingPart`/`RenameBuildingPart`/
  `MoveDeviceToBuildingPart` land in `knx-core`, mirroring
  `CreateGroupRange`/`DeleteGroupRange`/`RenameGroupRange`'s flat-list
  tree-CRUD shape — `installation.buildings` was already a flat
  `Vec<BuildingPart>` linked by `parent`/`children` ids, so no new
  nesting logic was needed, only the commands to mutate it. Building
  placement, unlike topology placement, isn't exhaustive: a device can
  have zero or one building part, so `MoveDeviceToBuildingPart`
  targeting `None` means "not placed" rather than a tracked
  "unassigned" bucket the way `MoveDeviceToLine` has one. `apps/knx-web`
  gains a building-part create row in the Project Explorer (unbounded
  nesting depth, unlike group ranges' observed two levels) and a
  rename/delete/move UI in the Inspector, mirroring `GroupRangeInspector`/
  `LineMoveField`. Closes **B4**.
- ~~**T9. Bulk/multi-select operations.**~~ **Closed (2026-09-10).**
  `Command::Batch(Vec<Command>)` composes existing single-entity commands
  (`DeleteDevice`/`DeleteGroupAddress`/`MoveDeviceToLine`/
  `MoveDeviceToBuildingPart`) with all-or-nothing apply/rollback —
  `CommandStack` needed no change, since it was already generic over
  `Command`. `knx-server` exposes four batch routes following the
  existing one-`*_impl`-per-command convention; `apps/knx-web` adds
  ctrl/shift-click multi-select to the Project Explorer and a bulk-action
  toolbar. No batch "address reassignment" for individual addresses —
  B9's own wording described topology reassignment, already covered by
  the two `MoveDeviceTo*` commands. See
  `docs/superpowers/specs/2026-09-10-bulk-operations-design.md` for the
  rollback rationale and out-of-scope items (copy/paste with parameters,
  mixed-kind batch edit). Closes **B9**.

### Tier 3 — export & reporting parity

- ~~**T10. Wire up `export_ets_project` to a real interface.**~~ **Closed
  (2026-09-10).** `knx export` CLI subcommand, `POST /api/project/export`
  on `knx-server`, and an "Export to .knxproj…" action in the web
  Project Explorer all ship. Closes **C4**. Fixing it surfaced a
  pre-existing data-integrity bug in the server's Save path (opaque
  passthrough + manufacturer manifest data never persisted after an ETS
  import) — fixed in the same branch, see `IMPLEMENTATION_STATUS.md`.
- ~~**T11. Import-report review screen.**~~ **Closed (2026-09-10).** A new
  `SessionLog` (`apps/knx-server/src/session_log.rs`) accumulates
  `LogEntry { timestamp, severity, source, message, location, detail }`
  in memory for the current server process — never written to `.knxdb`,
  reset only on a successful import/native-open (a failed one appends an
  error entry without touching what's already there). Every
  import/open/save/export/undo/redo/edit funnels through it: import
  populates it from the same `ImportReport` used for `ProjectTree`'s
  counts (`from_import_report` maps `ImportError`/`UnknownConstruct`/
  `OpaqueSummary`/`Conflict`/`UnsupportedFeature` to warning/info/error
  entries, in that order — `report.opaque` entries are mapped to
  info-level log entries the same as the other report categories;
  `report.inferred` and `SourceInfo.namespace_disagreement` are
  deliberately not mapped in this cycle, a residual rather than a silent
  drop, see `KNOWN_LIMITATIONS.md`), and every other operation that
  reaches a `Command` dispatch or a project-level operation's own
  top-level `Result` logs one info entry on success or one error entry
  (the existing user-facing error string) on failure. A failure that
  never reaches that point — a bad address parse, an empty id list, no
  project open, no product database configured, a catalog item not found
  — produces a toast but no log entry. Export was folded in using the
  same info/error shape as save, even though the design doc's own
  operation list didn't name it — the one other fallible project-level
  operation would otherwise have been an arbitrary, undocumented gap.
  `GET /api/log` returns every entry for the session, oldest-first —
  filtering and ordering are frontend-only. `apps/knx-web` adds a
  `LogPanel.tsx` component and a "Log" toolbar button (disabled until a
  project is open) that swaps into the same slot as the
  Inspector/Dashboard; it renders newest-first, with Error/Warning/Info
  toggle filters (client-side only, never re-fetch), distinct empty
  states for "no entries at all" vs. "entries exist but every filter is
  off," and a rendered fetch-error message if `GET /api/log` itself
  fails. `knx-server`: 6 new unit tests (3 in `session_log.rs`, 3 in
  `domain.rs`; the crate's `--lib` total is 20, the other 14 predate T11
  or cover unrelated modules) plus a dedicated `tests/http_log_route.rs`
  integration test suite (now 2 tests: a corpus-free
  fresh-state-returns-empty-array check that always runs in CI, and the
  gated import → failed edit → successful edit, correct append order
  test). `knx-web`: `LogPanel.test.tsx`, 8 tests (newest-first,
  per-severity filter show/hide with no re-fetch, both empty states,
  fetch-error rendering and clearing, refetch-on-tree-change,
  refetch-on-refreshKey-change). Full gate (`cargo fmt --check`, `cargo test
  --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `npx tsc --noEmit`, `npm test`, `npm run build`) green on the merged
  branch. See `docs/superpowers/specs/2026-09-08-session-log-design.md`
  for the full design and its ownership/reset rationale. Closes **D7**.
- ~~**T12. CSV group-address import/export.**~~ **Closed (2026-09-10).**
  A new `crates/knx-csv` — depending on nothing but `knx-core`, `csv`, and
  `serde`, enforced by a new `xtask check-layering` rule — reads and writes
  "KNXBench group-address CSV v1": `export_group_addresses`,
  `parse_group_addresses`, `plan_import` (`read.rs`, `write.rs`, `plan.rs`).
  **This is a format KNXBench defines and documents itself.** No sample of
  ETS's own group-address CSV export exists anywhere in this repository or
  in the KNX Standard v3.0.0 corpus, so no ETS-compatibility claim is made
  anywhere — not in code, not in docs, not in the UI — and the format is
  never called "ETS CSV." `crates/knx-core/src/command.rs` gains the one
  command the feature needed, `Command::UpdateGroupAddress { id, name,
  central, unfiltered }` (its own inverse), since nothing before this could
  rename a group address at all. Import matches rows to existing entries by
  address, never re-addresses (an address change reads as a new row), never
  deletes an address absent from the file, and is all-or-nothing per file —
  any row-level error blocks the whole import, applied atomically as one
  `Command::Batch` so the result is a single undo step. `DatapointType`,
  `MainGroup`, and `MiddleGroup` are export-only: read back and reported as
  recognized-but-ignored, never applied, since a group address itself
  carries no DPT and no range-creation happens from a CSV. Surfaces:
  `POST /api/group-addresses/csv-export`/`csv-import`
  (`apps/knx-server/src/routes.rs`, feeding the T11 session log via
  `session_log::from_csv_import_report`); `knx ga-export <store.knxdb>
  <out.csv>` and `knx ga-import <store.knxdb> <in.csv> [--dry-run]`
  (`apps/knx-cli`, `--dry-run` printing a report body byte-identical to a
  real import, with a trailing `store written: yes`/`no (…)` line);
  two toolbar buttons in the web group-address view
  (`GroupAddressCsvButtons.tsx`). Tests: `knx-csv`'s own suite (separator
  detection, BOM, CRLF, quoting, every row-level error, every boolean
  spelling, all three address styles, plan outcomes, range placement,
  all-or-nothing) — 50 tests via `cargo test -p knx-csv -- --list`; a
  corpus-gated round trip in `knx-app/tests/csv_roundtrip.rs`
  (`exporting_and_replanning_the_reference_project_is_entirely_unchanged`),
  living in `knx-app` rather than `knx-csv` because it needs
  `knx-etsproj` to build a real `Project`, and `knx-csv` must reach neither
  `knx-etsproj` nor `knx-store` — `check-layering` walks dev-dependency
  edges too, so even a dev-only edge there would have failed the rule;
  4 new `UpdateGroupAddress` tests in `knx-core::command::tests`; 5 new
  server integration tests (`apps/knx-server/tests/http_group_address_csv.rs`)
  and 5 new CLI integration tests
  (`apps/knx-cli/tests/cli_group_address_csv.rs`); 11 new frontend tests
  in `GroupAddressCsvButtons.test.tsx` plus 4 more in `api.test.ts` for the
  two new client functions. Closes **C2**. Design spec:
  `docs/superpowers/specs/2026-09-10-csv-group-address-exchange-design.md`.
- ~~**T13. Project documentation export (PDF/HTML report).**~~ **Closed for
  HTML (2026-09-10).** A new `crates/knx-report` crate — depending only on
  `knx-core`, `knx-projection`, and `chrono`, with a matching `xtask
  check-layering` rule keeping it away from `knx-store`/`knx-etsproj`/
  `knx-productdb` and every `CORE_FORBIDDEN` dependency — renders a
  `&knx_core::Project` into one self-contained HTML "project documentation"
  document via its one public entry point, `render_html(&Project,
  &ReportOptions) -> HtmlReport`. `ReportOptions::generated_at` is the only
  source of "now," so the same project and timestamp render to
  byte-identical HTML on every call. Eight sections: Header, Contents,
  Summary, Topology, Buildings, Group addresses, Devices, and "What this
  report does not contain" — the last one names, inside the document
  itself, exactly what it does not resolve (manufacturer/product/program
  identifiers, parameter values, module arguments) rather than only in
  `docs/`. **This is KNXBench's own document, not an ETS report:** no
  ETS-produced report sample exists anywhere in this repository, so no
  parity claim is made, the same evidence gap [KNOWN_LIMITATIONS.md
  §38](KNOWN_LIMITATIONS.md#38-group-address-csv-exportimport-t12-has-no-verified-ets-interoperability)
  already records for T12's CSV format. Surfaces: `POST
  /api/project/documentation-export` (`apps/knx-server/src/routes.rs`,
  logging one T11 session-log entry per warning under `source:
  "doc-export"`); `knx doc-export <store.knxdb> <out.html>`
  (`apps/knx-cli`); an "Export documentation…" button
  (`apps/knx-web/src/DocumentationExportButton.tsx`) in the same toolbar
  row as the `.knxproj`/CSV export controls. Tested: `knx-report`'s own
  suite (`html.rs`/`model.rs`/`render.rs` unit tests, including
  determinism, escaping of `&`/`<`/`>`/`"`/`'` in names, self-containment,
  and every orphan/dangling-reference finding) is 43 tests via `cargo test
  -p knx-report -- --list`; `apps/knx-server/tests/http_documentation_export.rs`
  (4 tests: a successful export, a warning-generating one with session-log
  entries, a path-outside-data-directory rejection, and the
  no-project-open 400 case); `apps/knx-cli/tests/cli_documentation_export.rs`
  (4 tests, including per-warning printing and the missing-store exit-1
  case); `DocumentationExportButton.test.tsx` (6 tests). A corpus-gated
  integration test,
  `rendering_the_reference_project_produces_a_complete_self_contained_document`
  (`crates/knx-app/tests/documentation_export.rs`, living in `knx-app`
  rather than `knx-report` for the same dev-dependency-layering reason
  `csv_roundtrip.rs` does), imports the reference `.knxproj`
  (36 devices, 907 communication objects, 514 group addresses) and asserts:
  every group address's formatted string appears in the output; every
  device name appears (escaped); `<table>`/`</table>` and `<tr>`/`</tr>`
  counts balance; the Summary section's nine stated counts equal counts
  computed independently from the `Project`; the output contains no
  `<script`, `http://`, or `https://`; and a second render with the same
  timestamp is byte-identical. **Closed for HTML only** — printing from
  inside the application and native PDF generation (without a browser)
  remain open, tracked in `KNOWN_LIMITATIONS.md`. Closes **D4**. Design
  spec:
  `docs/superpowers/specs/2026-09-10-project-documentation-export-design.md`.
- ~~**T14. Project diff/compare.**~~ **Closed for `.knxdb`-to-`.knxdb`
  comparison only (2026-09-10).** A new `crates/knx-diff` crate —
  depending on `knx-core` only, with a matching `xtask check-layering`
  rule keeping it away from `knx-store`/`knx-etsproj`/`knx-productdb` and
  every `CORE_FORBIDDEN` dependency, and carrying no `serde` (matching
  `knx-report`'s own no-`serde` precedent) — computes what changed
  between two `&knx_core::Project`s via its one public entry point,
  `diff_projects(&Project, &Project) -> ProjectDiff`. It is an
  independent reimplementation of `knx-etsproj::compare`'s matching
  technique, not a promotion of `compare.rs` itself: `compare.rs` answers
  "did a roundtrip preserve everything," a byte-identical-fields
  question, while `knx-diff` answers "what did a human change," a
  looser, identity-first question — folding the two together would make
  every future change to either risk silently breaking the other (design
  spec §2). Matching follows design spec §3.3: an `ets_id` match wins
  regardless of field agreement; failing that, a natural key (per entity
  type, §3.4) is tried only among each side's leftovers; a natural key
  with more than one leftover candidate on either side is never guessed
  at — both candidates land in `added`/`removed` and one `AmbiguityNote`
  records the collision. A matched pair with zero differing fields
  produces no output at all (`git diff` convention, not "unchanged").
  `diff_projects` is pure — no clock, filesystem, or RNG — and every
  output list is sorted by the entity's own display key, never `HashMap`
  order (design spec §4). **This is KNXBench's own diff, never described
  as an ETS comparison or a replacement for one** — no ETS-produced
  comparison sample of any kind exists in this repository, so no parity
  claim is made anywhere, the same evidence gap T12's CSV format
  ([KNOWN_LIMITATIONS.md §38]) and T13's HTML report
  ([KNOWN_LIMITATIONS.md §44]) already record. Surfaces: `POST
  /api/project/diff {path}` (`apps/knx-server/src/routes.rs`), comparing
  the open, possibly-edited, in-memory project against a `.knxdb` file at
  `path` — deliberately "what would Save change," not "diff two files" —
  rejecting a missing comparison path or no open project with `400`,
  never silently creating an empty `.knxdb` at `path`; `knx diff
  <a.knxdb> <b.knxdb>` (`apps/knx-cli`), loading both independently and
  printing plain text, `+`/`-`/`~` prefixed lines, `"no differences
  found"` when nothing differs anywhere; a "Compare with…" button
  (`ProjectDiffPanel.tsx`, `apps/knx-web`) that renders one grouped-count
  summary line per non-empty table (e.g. `Devices: 1 added, 2 changed`) —
  no tree view, no inline before/after highlighting (design spec §9).
  Tests, every count re-verified via `cargo test -p <crate> -- --list` at
  documentation time: `knx-diff`'s own suite (`key.rs`, `semantic.rs`,
  `diff.rs`) is 48 tests, covering the three-pass matching algorithm,
  every entity's field extraction, `Override` resolution at every layer,
  and `diff_projects(&p, &p)`'s own anchor property (design spec §3.7)
  against hand-built fixtures; `apps/knx-server/tests/http_project_diff.rs`
  (4 tests: an identical-project empty diff, a changed-device-description
  diff, a missing-comparison-path 400, and the no-project-open 400);
  `apps/knx-cli/tests/cli_project_diff.rs` (4 tests: identical stores, one
  changed group-address name, a missing first store, and wrong argument
  count); `ProjectDiffPanel.test.tsx` (8 tests). A corpus-gated
  integration test,
  `rendering_diff_projects_between_two_independent_imports_of_the_reference_project_is_empty`
  (`crates/knx-app/tests/project_diff.rs`, living in `knx-app` rather than
  `knx-diff` for the same dev-dependency-layering reason
  `csv_roundtrip.rs`/`documentation_export.rs` do — `check-layering` walks
  dev-dependency edges too, and `knx-app` is deliberately the one crate
  already permitted to see both `knx-etsproj` and `knx-diff`), imports the
  reference `.knxproj` twice, independently (36 devices, 907
  communication objects, 514 group addresses), and asserts `diff_projects`
  between the two imports is empty at every level, for every installation
  and every entity table — the strongest form of design spec §3.7's
  property, run against real, large, ETS-shaped data instead of a
  hand-built fixture. **Closed for `.knxdb`-to-`.knxdb` comparison only**:
  comparing against a raw `.knxproj` is not supported on either side; there
  is no merge/apply of a diff back onto a project; no three-way
  comparison; no detection of an ETS re-import's regenerated `RefId`s as
  "the same entity"; no CI-friendly "exit nonzero on any difference" CLI
  flag; a device with no individual address and no matching `ets_id`
  cannot be correlated across two projects; two same-named sibling
  building parts under the same matched parent collide under the
  path-based key — all recorded in new `KNOWN_LIMITATIONS.md` entries
  §51-§58, plus two rendering-scope entries (§59, §60) found during
  implementation and review: the text/web renderers show which fields
  changed, not their before/after values, for every entity table except
  project/installation info (which do show both); and the web panel shows
  grouped counts only, no tree view, no inline highlighting. Closes
  **C1**. `ROADMAP.md` was checked and names neither T14 nor C1, so it was
  left untouched by this task. Design spec:
  `docs/superpowers/specs/2026-09-10-project-diff-design.md`.

### Tier 4 — bus-facing UI (builds on Session 6's KNXnet/IP work)

- **T15. Group Monitor GUI. Closed 2026-09-11.** A live telegram table in
  `apps/knx-web` (not just the CLI), DPT-decoding values against the open
  project's comm objects via the same `knx_core::decode`/
  `resolve_project_group_address_dpts` **T29** already wired into
  `apps/knx-cli`, plus a compose/send form (`POST /api/bus/write`).
  Closes **D5** for tunnelling; finishes what **E4** left open on the
  display side, for tunnelling. Ships a new `apps/knx-server → knx-net`
  dependency edge (ADR-0017) and a `GatewayConnector`/`BusTunnel`
  testability seam so the session, buffer and HTTP layer are tested
  without a real gateway anywhere. Does **not** close: routing (CLI-only,
  by design); ETS-depth filtering; more than one session at a time;
  individual-address frames as rows; or hardware verification of any of
  it — see [KNOWN_LIMITATIONS.md §62](KNOWN_LIMITATIONS.md#62-the-group-monitor-gui-t15-is-tunnelling-only-single-session-client-filtered-and-has-never-talked-to-a-real-gateway).
  Design spec: `docs/superpowers/specs/2026-09-11-group-monitor-design.md`.
  ADR: `docs/adr/0017-knx-server-depends-on-knx-net.md`.
- **T29. DPT codec (`knx-core`) and CLI wiring.** Shipped 2026-09-11.
  Closes **E4** partially — see the row above and
  `docs/KNOWN_LIMITATIONS.md` for exactly which main types and subtypes
  are covered and which are not. Design spec:
  `docs/superpowers/specs/2026-09-11-dpt-codec-design.md`. ADR:
  `docs/adr/0016-dpt-codec-in-knx-core.md`.
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

- **T18. Parameter interpretation and editor.** The `when/@test` research
  spike (RESEARCH R3) ran 2026-09-11 (RESEARCH §4.3): the Standard
  normatively specifies the `@test` value grammar and the
  `choose`→`ParameterRef`→`ParameterType` resolution chain resolves
  100% of the time in a 34-application-program corpus. **Slice 1 shipped
  the same day**: `knx-productdb` parses and stores the `Dynamic` tree
  losslessly (schema v3, `dynamic_node`, backfilled into existing
  databases from stored blobs) and evaluates it headlessly into active
  `ParameterRef`/`ComObjectRef` sets, with the no-match-branch policy
  decided (nothing under an unmatched `choose` activates, an inference —
  RESEARCH §4.3 — not a documented rule), the `TypeNone`-controlled "dummy
  wrapper" `choose` idiom given its own code path, and a defensive parser
  that stores an unrecognized `Dynamic`/when-child construct under its own
  name and reports it rather than dropping it (the spike itself turned up
  one undocumented one, `ChannelIndependentBlock`, mid-research).
  **Slice 2, module expansion, shipped the same day (2026-09-11):** the
  evaluator now follows `Module/@RefId` into the referenced `ModuleDef`'s
  own stored `Dynamic` tree, with every activation and diagnostic
  qualified by the instantiating `Module` (`ModuleScope`), so N sibling
  `Module`s instantiating one `ModuleDef` produce N results, not one.
  Nesting is rejected by policy, not followed
  (`Diagnostic::NestedModuleNotExpanded`) — the corpus has zero nested
  modules and the Standard extraction defines no application-program-side
  `ModuleDef` complexType to recurse against. Corpus-regression-proven for
  `prod3`'s three module-bearing programs (activation totals 22/18/14 →
  382/258/134; RESEARCH §4.4 Q7's other four module-bearing programs live
  in the `kv25` demo `.knxproj`, which these tests do not install, so
  nothing here claims `kv25`). Not closed by slice 2, and not softened by
  it either — see [KNOWN_LIMITATIONS.md §3](KNOWN_LIMITATIONS.md#3-device-parameters-are-preserved-but-not-interpreted):
  all instantiations of one `ModuleDef` still evaluate against identical
  parameter values (per-instantiation values are a project-side construct
  this crate does not model), argument values stay stored-but-uninterpreted,
  and `AllocatorRef` stays unattested. **Slice 3, the editor (shipped
  2026-09-11):** `GET`/`POST /api/device/{id}/parameters`
  (`apps/knx-server`, decisions D20-D26,
  [design](superpowers/specs/2026-09-11-parameter-editor-design.md)) plus
  `apps/knx-web/src/ParameterPanel.tsx` let a user see every declared
  parameter (grouped into one section per `Module` instantiation, D23),
  write a top-level value through `Command::SetParameterValue` (undo/redo
  via `RestoreParameterValue`), and see the evaluator's recomputed
  activation set and diagnostics in the same response — no second `GET`
  needed. It also *reads and displays* a module-scoped (per-channel) value
  correctly where the project stores one: `ParameterInstance` already
  stores per-channel values in our own corpus (the KV v2.5 demo project's
  shape, 5 distinct values for one declared `ParameterRef` across 5
  `Module` instantiations), and slice 3's decomposition (D21) surfaces
  each in its own section (D22/D23). **Module-scoped editing is
  explicitly out of scope for this slice (D25)** — not because storage
  cannot hold a per-channel value (it already does), but because the
  evaluator's flat `ValueMap` has no scope in its key
  (`evaluate.rs:797`/`:348`), so a module-scoped write could never affect
  the same response's recomputed activation set the way a top-level write
  does; D16 (all instantiations of one `ModuleDef` evaluate against
  identical values) stays true in that narrower sense. This is the last of
  the three slices this entry named; T18 as a whole is now feature-complete
  for the scope this row describes, with module-scoped editing named as
  its own follow-on, not silently dropped.
  Partially closes **A3** — a UI now exists and can write a top-level
  value, which is real progress toward realistic ETS parity, but
  module-scoped editing (the harder half) is not done.
- **T19. KNX Secure (Data Secure + IP Secure + keyring).** Needs sample
  key material and a real secured installation to verify against — a
  hard external dependency, not purely an engineering task. **Deferred
  2026-09-11 by user ruling** — not rejected, documented as a limitation
  ([KNOWN_LIMITATIONS.md §8](KNOWN_LIMITATIONS.md#8-knx-secure-is-not-implemented),
  [§26](KNOWN_LIMITATIONS.md#26-busconnection-does-not-yet-support-knx-ip-secure)).
  Closes **A2**, **E3**.
- **T20. `Functions` domain concept.** Needs its own ADR (new domain
  concept, not in DATA_MODEL today) before implementation, same as
  ROADMAP's existing rule for the project-notes idea. **Deferred
  2026-09-11 by user ruling until the new KNX specification documentation
  is available** — not rejected, stays on the roadmap. Closes **A1**.
- **T21. Graphical topology and building views.** A genuinely new UI
  paradigm (diagram/canvas rendering) alongside the existing tree-based
  Project Explorer, not a replacement for it. Closes **D1**, **D2**.
- **T22. Multi-user/concurrent-edit support for `knx-server`.** Only
  matters once the web deployment is used by more than one person at
  once; needs its own design (locking vs. merge vs. last-writer-wins,
  and what "conflict" even means for a `Command`-based undo model).
  Closes **F2**.
- **T30. Commissioning and device download.** Individual-address
  programming (via the device's programming button), application-program
  download, memory read/write over the bus. **Not a durable non-goal** —
  the user ruled 2026-09-11 that this must work, blocked until the KNX
  specification database is finished. That database now exists and, per
  the R5 research spike ([RESEARCH.md §8.4](RESEARCH.md)), the *generic*
  load/unload/reset/memory-write procedures and the Load State Machine
  are documented in the Standard — a spike is documentation, not
  implementation or verification, so this task remains unstarted. What
  the spike could not find anywhere in either KNX specification database,
  and what still blocks a safe start: the product-specific `Legacy*`
  compatibility-flag matrix and vendor-DLL-driven download sequences,
  plus hardware to verify against without real bricking risk.
  Architecturally unblocked already: load procedures, memory layout and
  mask data already live in the product database
  ([KNOWN_LIMITATIONS.md §7](KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked)).
  Closes **E1**.

### Tier 6 — internationalization

Added 2026-09-10 by explicit request ("multilang support für das UI").
Two separate tasks on purpose: T25 is a self-contained frontend effort
that could ship in a single cycle, T26 reaches into the domain model and
the product database and is the larger of the two. T25 does not depend on
T26, and T26 is useful even if T25 never ships (a German catalog rendered
inside an English chrome is still strictly better than an untranslated
one). T25 has no design spec yet; T26's first slice does —
`docs/superpowers/specs/2026-09-12-product-data-language-design.md`.

- **T25. Multi-language UI chrome.** Extract every hard-coded English
  literal in `apps/knx-web` into a message catalogue, add locale
  detection plus an explicit language setting, and render the UI in the
  selected language. German is the obvious second locale — it is the
  language of the KNX Association's own documentation, of the sample
  projects in `OriginalData/`, and of this project's users. Two decisions
  belong in the design spec rather than here: which library (or whether a
  ~200-string catalogue needs one at all), and how the language setting
  is stored, since D8's options dialog does not exist yet and
  `theme.ts`'s `loadThemeId`/`saveThemeId` `localStorage` convention is
  the only precedent (cycle 11's `ThemePanel.tsx`, the earlier precedent,
  no longer exists).
  Partially addresses **D8**, closes half of **D10**.
- **T26. Language-aware display of imported KNX data.** **First slice
  shipped 2026-09-12** (design spec
  `docs/superpowers/specs/2026-09-12-product-data-language-design.md`,
  plan `docs/superpowers/plans/2026-09-12-product-data-language.md`).
  `knx-productdb`'s `parameter_views`/`parameter_type_enum_options` gained
  an `Option<&str>` language that overlays the `translation` table's
  `Text`/`FunctionText`/`SuffixText`/`VisibleDescription`/`Name` rows
  (never `Value` — a value is a key written into the project file, not
  display text) over the package's own untranslated attribute;
  `translation_languages`/`program_translation_languages` list what a
  database or a program actually has. `apps/knx-server` exposes `GET
  /api/product-languages` and an optional `?language=` on the device
  parameter panel's GET and POST. `apps/knx-web` persists the choice
  (`productLanguage.ts`, a "Product data language" Settings-panel select)
  and `ParameterPanel` sends it on load and on write. That is the entire
  surface this slice reads translations at: parameter text, parameter-ref
  text, and enum option labels, nothing else. Still missing, for a later
  T26 slice: an *active language* concept distinct from the UI's,
  `LocalizedString`/`StringTable` resolution at any display site (neither
  was touched — the overlay is entirely `knx-productdb`-side), reading the
  translations of communication-object text (baked into the project at
  device creation, which is why translating it there would make the
  *stored project* depend on a display setting), and a decision on what
  the project's own `Language` means once a user can pick a different one
  — today `Project::new` is still handed a placeholder `"en"` by both
  importers (`map.rs:150`; the `.knxproj` carries no project-wide language
  tag at all). Also unresolved by this slice: the ingestion gap in
  **T32** below (since closed by T32 itself, 2026-09-12), and no
  locale-prefix matching or `navigator.language` detection anywhere. Closes half of **D10** (the half T25 does not
  cover); a prerequisite for T18's parameter editor being usable in
  practice, since parameter text is exactly the data that arrives
  translated. Full accounting:
  [KNOWN_LIMITATIONS.md §37](KNOWN_LIMITATIONS.md#37-imported-translations-are-stored-but-never-read-and-the-ui-is-english-only--partially-resolved-2026-09-12).
- **T32. Done (2026-09-12).** `Languages` blocks outside an application
  program are ingested. Schema v4 widened `translation` to `(scope,
  scope_id, language, ref_id, attribute_name)` with `''` as the
  master-scope sentinel, one `ingest_translations` pass serves
  `Catalog.xml`, `Hardware.xml` and `knx_master.xml`, and a v3→v4
  backfill replays blobs already stored (a blob that fails to parse
  records itself into `ingest_unknown` and does not abort the
  migration). Measured after the change on the package this gap was
  opened against: 40 catalog rows (5 languages), 30 hardware (5), 1635
  master (**18**, not the 24 first published here — 24 is the count of
  `<ProductLanguages>` catalogue entries, which carry no translations),
  18546 program, 20251 in total. The golden corpus assertion moved from
  48,057 to 48,190 rows. The catalog browser reads the catalog-scope
  half: `query::catalog_items(conn, …, language)` behind `GET
  /api/catalog/items?language=`, with the search filter and `ORDER BY`
  following the overlaid name. Hardware- and master-scope rows are
  stored and read by nothing, and the import report still does not state
  how many translations were captured — the residue stays under **D10**
  and in
  [KNOWN_LIMITATIONS.md §64](KNOWN_LIMITATIONS.md#64-languages-blocks-outside-an-application-program-are-discarded-on-import).
  Plan: `docs/superpowers/plans/2026-09-12-shared-translations.md`.
- **T33. Done (2026-09-12).** Communication-object text learns the same
  display-time overlay parameter text got from T26. `knx-productdb`'s
  `com_object_view(conn, program_id, com_object_ref_id, language)` gained
  the fourth `Option<&str>` parameter, applied before `pick()` exactly as
  `parameter_views` already does: a `ComObject`-scope translation is
  keyed by the `ComObject`'s own id, a `ComObjectRef`-scope one by the
  `ComObjectRef`'s id, and only `Text`/`FunctionText`/`VisibleDescription`
  are ever overlaid. `apps/knx-server`'s `GET /api/device/{id}?language=`
  calls it and overwrites `ComObjectNode::name`/`description` — but only
  where the stored `Override<Text>`'s layer is `Layer::Program` or
  `Layer::ProgramRef`; `Layer::Instance`, `Layer::Inferred` and
  `Layer::UserEdit` are project-authored and are always shown verbatim,
  a load-bearing invariant with its own regression test. `language: None`
  or no open product database issues no translation query at all, byte-
  identical to before. `apps/knx-web`'s Inspector sends the persisted
  product-language setting on every device-detail fetch and refetches on
  a language change, guarded by a monotonic request id against an older
  language's response landing after a newer one's. Measured, not
  assumed, on `M-0083_A-0317-31-7DC6`
  (`MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod`): 53 `ComObject`-scope
  `Text` and 50 `ComObject`-scope `FunctionText` translations per
  language, across all five declared languages — rows already ingested
  by T26/T32, now finally read. Device creation and `enrich()` are
  untouched and still bake untranslated text into the project file, on
  purpose — translation stays display-only, never stored. Closes more of
  **D10** (still open for the chrome half, T25, and for the hardware-/
  master-scope residue [KNOWN_LIMITATIONS.md
  §64](KNOWN_LIMITATIONS.md#64-languages-blocks-outside-an-application-program-are-discarded-on-import)
  still names). Full accounting: [KNOWN_LIMITATIONS.md
  §37](KNOWN_LIMITATIONS.md#37-imported-translations-are-stored-but-never-read-and-the-ui-is-english-only--partially-resolved-2026-09-12).
  Plan: `docs/superpowers/plans/2026-09-12-com-object-language.md`.
- **T34. Open.** Non-blocking findings from T33's whole-branch review
  (2026-09-12), none of them Critical. `com_object_view` loads the
  *entire* program/language translation overlay on every call instead of
  accepting a batch of ref ids, so one device fetch costs O(com objects ×
  overlay rows) — measured at 3,876 rows / ~1.26ms of SQL for the
  largest program actually attached to a device, times up to 66 com
  objects per device in the reference project, repeated on every device
  click, every edit-triggered refetch, and every language change. Fix: a
  batched `com_object_views(conn, program_id, &[ref_id], language)`, or
  an overlay-accepting variant, mirroring `parameter_views`'s own
  bulk-query shape. Three smaller items ride along: `apps/knx-web/src/App.tsx`'s
  language-change refetch effect guards against out-of-order replies with
  a monotonic request id, but the two older `deviceDetail` call sites
  (device selection, edit-triggered refetch) do not share that guard, so
  an in-flight language reply can still land after — and overwrite — a
  later edit-triggered refetch; the new `Query<ParameterLanguageQuery>`
  extractor on `GET /api/device/{id}` turns a malformed query string into
  a 400 where it was previously ignored outright (no `Query` extractor
  existed on that route before T33); and on an overlay miss (no
  translation row for the requested language) `com_object_view`'s `pick()`
  still falls back to the product database's *current* untranslated
  column, so `apps/knx-server` overwrites `ComObjectNode::name` with
  whatever that column holds today rather than leaving the value already
  resolved from the project.

### Tier 7 — motion and animation

Added 2026-09-10 by explicit request ("die Animationen sollen togglebar
sein, wenn sie implementiert werden"). One task plus one standing
constraint that binds every *other* task in this backlog.

- **T27. Done (2026-09-12).** An in-app motion control, and the rule that
  every animation obeys it. Restored the user-facing motion setting cycle
  13 removed by accident, and answered the open question below: not a
  global duration multiplier alone and not a per-category switch, but
  **two orthogonal axes** — level (`off`/`subtle`/`standard`, intensity)
  and style (`apple`/`glitch`, feel) — because the 2026-09-10 style memo
  named two independent visual directions rather than asking for finer
  targeting.
  1. **The control itself** — `motion.ts`'s two registries and
     `useMotion()`, surfaced in the gear-button `SettingsPanel.tsx`,
     persisted to `localStorage`, applied as `data-motion-level`/
     `data-motion-style` on `<html>` both before React mounts
     (`index.html`'s bootstrap script) and by `useMotion()` afterward. The
     rule that `prefers-reduced-motion: reduce` always wins is structural,
     not conventional: no `.ts`/`.tsx` file calls `window.matchMedia`,
     so there is nothing in-app to override it with.
  2. **The standing constraint** — no task on this backlog may ship an
     animation that is not switchable off through the control — is now
     enforced by a test, `motionGuard.test.ts`, rather than by prose: it
     fails the suite if a `transition:`/`animation:` declaration in
     `styles.css` sits outside a `@media (prefers-reduced-motion:
     no-preference)` block or uses a literal duration. It still binds
     **T17**'s line-scan progress UI, **T21**'s graphical topology and
     building views (**D1**/**D2**), and the deferred "who talks to whom"
     telegram animation under Session 7 in [ROADMAP.md](ROADMAP.md). And
     the retrofit the constraint's own text warned about was actually
     performed: **T15**'s Group Monitor table, which had shipped
     2026-09-11 ahead of this control, now has a guarded new-row
     highlight (`BusMonitorPanel.tsx`'s `bus-monitor-row-new`).

  Closes **D11**. Partially addresses **D8** — the control has somewhere
  proper to live now (`SettingsPanel.tsx`, three `<select>`s), but none
  of ETS's own options dialog contents exist, so D8 stays open. Design
  spec: `docs/superpowers/specs/2026-09-12-motion-control-design.md`.
  Five specific, deliberate remaining limits (no per-category control,
  guard coverage gaps, no visual verification, the `node:fs`/`?raw`
  guard trap) are recorded in full at
  [KNOWN_LIMITATIONS.md §43](KNOWN_LIMITATIONS.md#43-animations-have-no-in-app-switch-only-the-os-reduced-motion-preference)
  rather than claimed away.

### Tier 8 — in-application help

Added 2026-09-10 by explicit request, and placed last on purpose: help
text describes a specific UI, so writing it before the UI stops changing
means writing it twice. This tier is entered after Session 7's hardening
and after the UI backlog above (T15, T17, T18, T21) has shipped or been
dropped.

- **T28. In-application help: hover explanations, contextual help, and a
  user manual.** Today the application explains nothing about itself:
  one `title` attribute frontend-wide, no tooltip component, no help
  panel, no `F1`, and nine `docs/` files all written for developers and
  none reachable from the running application. Three plausible layers,
  and the design spec has to decide which of them are in the first cut:
  1. **Hover/tooltip text** on controls whose meaning is not obvious from
     their label — the communication-object flags (R/W/T/U/C) being the
     clearest case, since `Inspector.tsx:218`'s lone `title` already
     admits the need and answers it with the raw field name.
  2. **Contextual help** for a selected object or panel — what a group
     range is for, what "unfiltered" does, why a device can legitimately
     have no individual address.
  3. **A user manual**, which is the open question with the largest
     consequences: whether any of `docs/` is shipped to users, or whether
     end-user documentation is written separately from the start. The
     existing files are not candidates as they stand — they are written
     for whoever is building this, not for whoever is using it.

  Two constraints already exist for it. Help strings are user-facing
  chrome, so they fall under **T25**'s extraction of hard-coded English
  into a message catalogue; writing them as literals first means
  extracting them again later, and it is cheaper to decide the ordering
  now than to discover it then. And any hover or disclosure that animates
  is bound by **T27**'s rule — switchable off from inside the
  application.

  Closes **D12**. No design spec yet. Explicitly *not* the same thing as
  the in-app project documentation/notes feature deferred beyond
  Session 7 in [ROADMAP.md](ROADMAP.md) (that stores notes *about a
  project* and needs a `DATA_MODEL.md` addition plus an ADR), and not the
  same thing as **T13**'s documentation export (which prints a project
  rather than explaining the application).

None of Tiers 1-8 fits **T31**: it closes an existing gap (**D9**)
rather than opening new ETS-comparison surface, so it is listed here
undecorated rather than under an invented tier heading.

- **T31. Done (2026-09-12).** A shared modal overlay shell
  (`apps/knx-web/src/Overlay.tsx`), and the accessibility pass its four
  consumers — `Search.tsx`, `CommandPalette.tsx`, `CatalogBrowser.tsx`,
  `SettingsPanel.tsx` — never got. `role="dialog"`/`aria-modal`, backdrop
  and panel-level `Escape`, initial focus, a `Tab` focus trap and focus
  restoration now live in one place; listbox semantics
  (`combobox`/`listbox`/`option`/`aria-activedescendant`) are applied
  uniformly across the three list-bearing overlays; and
  `CatalogBrowser.tsx`'s result list, previously mouse-only, gained
  `ArrowDown`/`ArrowUp`/`Enter`-to-pick. A regression guard
  (`overlayShell.test.ts`) fails the suite if a fifth overlay
  reintroduces the hand-rolled shape by copy-paste. Closes **D9**; the
  residue — no scroll-into-view, no `inert` background, no
  focus-visible pass, no screen-reader verification — is tracked in
  [KNOWN_LIMITATIONS.md §20](KNOWN_LIMITATIONS.md#20-command-palette-and-search-share-overlay-css-and-an-accessibility-gap--partially-resolved),
  which stays open for it. Design spec:
  `docs/superpowers/specs/2026-09-12-modal-overlay-shell-design.md`.

### Not backlog items — durable non-goals, listed for completeness only

Commissioning/device download (**E1**) **used to be listed here and no
longer is.** The user ruled 2026-09-11 that it is required, not excluded —
merely blocked until the KNX specification database is finished. It is now
a real backlog item, **T30** in Tier 5 above; see
[KNOWN_LIMITATIONS.md §7](KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked)
for the full ruling.

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
