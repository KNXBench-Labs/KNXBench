# User-Reported UX and Workflow Issues Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` when implementing these tasks inline. Use isolated worktrees per task. Do not dispatch subagents merely because this document has several tasks.

**Goal:** Turn the user observations collected in `docs/Issues.md` into independently testable improvements without duplicating features that already exist.

**Architecture:** UI-only behavior stays in `apps/knx-web`; authoritative dirty state, atomic mutations, validation, and manufacturer-data resolution stay in their existing server/domain owners. Existing command, settings, overlay, discovery, DPT, and session-log mechanisms must be extended rather than paralleled. Claims about KNX semantics or ETS data are researched and documented before they change the model.

**Tech Stack:** Rust workspace (`knx-core`, `knx-productdb`, `knx-projection`, `knx-server`), React/TypeScript/Vitest (`apps/knx-web`), Tauri v2 (`apps/knx-desktop`).

**Spec:** The normalized requirements and coverage table in this document preserve the user report that was removed from `docs/Issues.md` after intake.

## Global Constraints

- Correctness and data integrity outrank convenience; saves, autosaves, and bulk creation must be atomic and honestly reported.
- Do not invent ETS/KNX semantics. Verify the individual-address and site/building claims against repository evidence or the KNX Standard before changing domain behavior.
- Preserve keyboard and screen-reader equivalents for resizing, navigation, drag/drop, filters, and contextual help.
- Reuse the versioned settings store in `apps/knx-server/src/settings.rs` and `apps/knx-web/src/settingsStore.ts`; do not add new preference keys directly to browser storage.
- Reuse `Command`/`CommandStack` for undoable project mutations and batch commands for all-or-nothing multi-entity changes.
- Every task adds focused regression tests and updates `docs/IMPLEMENTATION_STATUS.md`; compatibility or known limitations are updated when facts change.
- Implementation runs the smallest focused tests first, then the repository gates required by `AGENTS.md` before completion.

## Review Focus

- Save and autosave failure must never clear dirty state, advance the displayed save time, close a confirmation, or lose edits.
- Zoom, pane resizing, and resizable dialogs must remain usable with keyboard-only input and at narrow Linux desktop sizes.
- Bulk device creation must be atomic; address/name allocation must not leave a half-created batch.
- Product-data fallbacks must remain explicit diagnostics, never guessed names, DPTs, channels, or conditional visibility.
- Pausing or filtering the bus monitor must not discard buffered telegrams or advance/reset the server cursor incorrectly.

## Scheduling and Existing-Task Overlap

- ISSUE-04 is the concrete user-facing completion of the active goal's dirty-state items (§103/§81) and settings work; merge ownership instead of implementing two dirty flags.
- ISSUE-05 extends the active goal's structure editing and drag/drop item (B10). All gestures must call the same validated commands as buttons/forms.
- ISSUE-12 is follow-up validation for the already-shipped KNXnet/IP discovery UI, not a second discovery implementation.
- ISSUE-10 must preserve the bus monitor's existing freetext and service filters. The report that filtering is missing is a reachability/layout regression to reproduce.
- ISSUE-08 must preserve the core's existing support for separate Send and Receive links to the same group address.

---

### ISSUE-01: Application zoom and persistent workbench geometry

**User observations:** `Ctrl`+`+` does not resize UI text; the navigation width is forgotten after hide/show; device icons and communication-object counts disappear on hover.

**Files:**

- Modify: `apps/knx-web/src/App.tsx`
- Modify: `apps/knx-web/src/ResizablePane.tsx`
- Modify: `apps/knx-web/src/settingsStore.ts`
- Modify: `apps/knx-web/src/styles.css`
- Test: `apps/knx-web/src/App.test.tsx`
- Test: `apps/knx-web/src/Workbench.test.tsx`

**Interfaces:** Persist bounded `uiScale`, `navigationPaneWidth`, and `inspectorPaneWidth` preferences through the existing versioned settings document. `ResizablePane` becomes controlled or reports committed width without creating a second persistence mechanism.

- [x] Add failing tests for `Ctrl`+`+`, `Ctrl`+`-`, and `Ctrl`+`0`, including editable-field handling and bounded scale. (`App.test.tsx::uses Ctrl+Plus` and `::clamps persisted and repeatedly changed zoom`, RED on absent CSS scale.)
- [x] Add failing tests that resize the navigation pane, hide it, show it, remount the app, and recover the same clamped width. (`App.test.tsx::keeps a resized navigation pane`; `::stacks the inspector`; `Workbench.test.tsx::commits the clamped pointer width`; `http_settings.rs::geometry_preferences_round_trip_from_a_v1_document_without_the_new_keys`.)
- [x] Add a CSS regression test proving `.diagram-device:hover` does not hide its icon, address, or `.device-object-count`. (`Workbench.test.tsx::keeps diagram device content visible`; headless Chromium hover checked the three descendants' computed visibility and `transform: none`.)
- [x] Implement zoom and persisted pane geometry using the shared settings store; retain pointer and arrow-key resizing. (`settingsStore.test.ts::adds geometry preferences`; `Workbench.test.tsx::resizes a pane with the keyboard`; four UI mutation controls failed before restoration.)
- [x] Verify at narrow, default, and enlarged scales and run the focused App/Workbench/settings tests. (Chromium 640×700 and 1280×720 at scale 1 and 1.5, including max widths/reload; `App.test.tsx`, `Workbench.test.tsx`, `settingsStore.test.ts`, and `http_settings.rs`.)

### ISSUE-02: Welcome surface and new-project clarity

**User observations:** A fresh window should explain its actions with tiles instead of unexplained buttons; “open .knxdb” is unclear; Project Language should be a dropdown; the dialog should explain how the eventual filename is chosen.

**Files:**

- Modify: `apps/knx-web/src/App.tsx`
- Modify: `apps/knx-web/src/NewProjectDialog.tsx`
- Modify: `apps/knx-web/src/messages/en.ts`
- Modify: `apps/knx-web/src/messages/de.ts`
- Modify: `apps/knx-web/src/styles.css`
- Test: `apps/knx-web/src/App.test.tsx`
- Test: `apps/knx-web/src/NewProjectDialog.test.tsx`

**Interfaces:** The language choices come from the application's supported/project-language source rather than a second hard-coded list. A custom well-formed BCP-47 value remains possible only if current project semantics require it.

- [x] Pin the empty-window actions and explanations in a failing accessibility test: new project, open KNXBench `.knxdb`, and import ETS `.knxproj` are distinct cards with descriptions. (`App.test.tsx::explains the three distinct empty-workspace routes` first RED, then GREEN; verifies descriptions, button types and native/ETS picker filters.)
- [x] Pin a labelled language selector with common supported choices and the current UI language selected by default. (`NewProjectDialog.test.tsx` checks English/German, installed packs, the active UI language and custom tags; removing an active pack retains the chosen project tag. Two pack guards rejected mutations.)
- [x] Add copy explaining that Save/Save As chooses the `.knxdb` filename; project and installation names do not silently become a filesystem path. (`NewProjectDialog.test.tsx::says that Save or Save As chooses` first RED, then GREEN; the hint is ahead of the form, associates with both names and is translated.)
- [x] Implement the card layout and dialog changes without changing load/import behavior. (Existing picker actions remain, now distinct cards; `App.test.tsx` verifies `.knxdb` versus `.knxproj` routing; no server or domain changes.)
- [x] Verify keyboard order, translated copy, and narrow-window wrapping. (`App.test.tsx` checks welcome-first DOM/Tab order and German copy; Chromium on local Vite at 1280×800, 640×700 and 400×700 showed no horizontal page overflow. The 640 px dialog retained readable filename copy and scrollable actions at 150% zoom; at 320×568/150%, actions wrapped within the scrollable panel. Custom `de-DE`, Escape and focus return were checked without a backend. Native WebKitGTK and real screen reader remain unverified.)

### ISSUE-03: Resizable dialogs and readable form layouts

**User observations:** The debug-report dialog is too small; popup windows should be resizable; some inputs are narrower than their text; Settings may be wider/multi-column; the sun icon misleadingly opens Settings and should be a gear.

**Files:**

- Modify: `apps/knx-web/src/Overlay.tsx`
- Modify: `apps/knx-web/src/DebugReportButton.tsx`
- Modify: `apps/knx-web/src/SettingsPanel.tsx`
- Modify: `apps/knx-web/src/WorkbenchIcon.tsx`
- Modify: `apps/knx-web/src/App.tsx`
- Modify: `apps/knx-web/src/styles.css`
- Test: `apps/knx-web/src/Overlay.test.tsx`
- Test: `apps/knx-web/src/DebugReportButton.test.tsx`
- Test: `apps/knx-web/src/SettingsPanel.test.tsx`

**Interfaces:** `Overlay` gains one shared opt-in sizing contract with min/max viewport bounds and scroll containment. Content-specific overlays set useful initial sizes; dialogs do not each invent resize behavior.

- [x] Add tests for resizing without escaping the viewport, preserving focus trap/Escape behavior, and keeping long debug-report text reachable. (`Overlay.test.tsx::clamps keyboard resizing` and `::does not dismiss a resizable dialog`; `DebugReportButton.test.tsx::opens a roomy, scroll-contained shared resizable dialog`; `App.test.tsx::keeps the Debug report readable`.)
- [x] Add layout assertions for full-width inputs and a two-column Settings layout that collapses to one column at narrow widths. (`SettingsPanel.test.tsx::uses a roomy shared resize shell`; Chromium at 1280×800 and 640×700, with document width confined to the viewport.)
- [x] Replace the Settings trigger glyph with an accessible gear icon while preserving its translated accessible name. (`App.test.tsx::replaces the settings dialog rather than stacking help on top of it` checks the gear path and existing localized `aria-label`.)
- [x] Implement shared overlay sizing and content overflow; keep pointer resizing supplemented by usable CSS/browser and keyboard behavior. (`Overlay.test.tsx` covers bounded arrow keys, CSS grip, focus trap, Escape, drag-to-backdrop dismissal and zoomed layout-pixel steps; the pointer-origin and viewport-width guards rejected mutations.)
- [x] Inspect every `Overlay` consumer for regressions and run overlay, debug-report, and settings tests. (Only Settings and Debug report opt in; all other consumers retain their dimensions. Full web suite: 77 files / 1169 passed. Headless Chromium verified the File-menu portal and focus restoration, internal scroll and pointer/keyboard resize; no native WebKitGTK claim.)

### ISSUE-04: Authoritative dirty state, Save-and-continue, last-save status, and autosave

**User observations:** Unsaved-changes prompts need Save; saved projects still trigger the prompt; last save time belongs in the status bar; autosave should default to five minutes, be configurable, and show a five-second warning toast.

**Files:**

- Modify: `apps/knx-server/src/domain.rs`
- Modify: `apps/knx-server/src/routes.rs`
- Modify: `crates/knx-projection/src/lib.rs`
- Modify generated bindings under: `apps/knx-web/src/bindings/`
- Modify: `apps/knx-web/src/App.tsx`
- Modify: `apps/knx-web/src/NewProjectDialog.tsx`
- Modify: `apps/knx-web/src/SettingsPanel.tsx`
- Modify: `apps/knx-web/src/settingsStore.ts`
- Test: `apps/knx-server/tests/http_project_routes.rs`
- Test: `apps/knx-web/src/App.test.tsx`
- Test: `apps/knx-web/src/NewProjectDialog.test.tsx`
- Test: `apps/knx-web/src/SettingsPanel.test.tsx`

**Interfaces:** Replace `can_undo` as a dirty proxy with authoritative saved-baseline state that survives undo/redo correctly. The refreshed `GET /api/project` snapshot carries `is_modified` and `last_saved_at` after Save/Save As; failures leave both unchanged.

- [x] Write server tests for edit→save→prompt (no prompt), edit→save failure (still dirty), save→edit→undo to saved baseline (clean), and edit→undo→branch (correct dirty result). (`http_project_routes.rs::saved_edit_can_be_replaced_without_a_discard_confirmation`, `::modified_state_clears_only_after_successful_save_or_save_as`, `::undo_to_saved_baseline_then_branch_tracks_the_new_unsaved_edit`.)
- [x] Write UI tests for Save-and-create and Save-and-quit; failed or cancelled Save keeps the prompt and project open. (2026-09-27)
- [x] Write status-bar tests proving the timestamp changes only after a successful save and is formatted in the UI locale. (`App.test.tsx::uses the selected UI language and advances only after a successful save`; formats `de` while the test browser is `en`, and rejects an unchanged timestamp after a failed save.)
- [x] Add autosave settings: enabled by default, five-minute interval default, configurable interval, and explicit disabled state. (`autosaveSettings.test.ts`, `SettingsPanel.test.tsx::autosave interval is disabled while autosave is off`.)
- [x] Add fake-timer tests for the five-second countdown, manual-save cancellation, edits during the countdown, missing Save-As path, concurrent save suppression, and autosave failure. (`useAutosave.test.tsx`, nine focused cases.)
- [x] Implement the authoritative state first, then prompts/status, then autosave using the same save operation; never create a parallel persistence path. (`domain.rs::project_is_modified`, `App.tsx::refreshSavedProject`/`saveProject`, `useAutosave.ts`; HTTP and UI regressions above.)

### ISSUE-05: Editable structure workspaces and understandable hierarchy

**User observations:** Buildings, Topology, and Group Addresses should allow editing from their main workspaces; lines/areas should be renameable in Properties; hierarchy creation is unclear; misplaced entities cannot be moved.

**Files:**

- Modify: `crates/knx-core/src/command.rs`
- Modify: `crates/knx-store/src/command_sync.rs`
- Modify: `apps/knx-server/src/domain.rs`
- Modify: `apps/knx-server/src/routes.rs`
- Modify: `apps/knx-web/src/StructureWorkspace.tsx`
- Modify: `apps/knx-web/src/Inspector.tsx`
- Modify: `apps/knx-web/src/ProjectExplorer.tsx`
- Test: command tests in `crates/knx-core/src/command.rs`
- Test: `apps/knx-server/tests/http_edit_routes.rs`
- Test: `apps/knx-web/src/StructureWorkspace.test.tsx`
- Test: `apps/knx-web/src/Inspector.test.tsx`

**Interfaces:** New create/rename/move operations are ordinary undoable `Command` variants. Buttons, forms, keyboard actions, and later drag/drop all call those same server routes.

- [x] Inventory existing commands and add only the missing area/line/building-part/group-range mutations.
- [x] Add failing domain tests for duplicate addresses, invalid parents, non-empty deletion, and move-cycle prevention before routes or UI.
- [x] Add inline creation and context-aware actions to each centre workspace, with Properties-based rename for selected area/line.
- [x] Add move controls with clear source/target labels and keyboard operation; integrate the active B10 drag/drop task through the same commands.
- [x] Verify undo/redo and persistence round trips for every new mutation.

Evidence: `knx-core::command` order/ambiguity/cycle/rollback tests,
`http_edit_routes` deletion/rename/move/undo regressions, `Inspector` and
`StructureWorkspace` UI suites, and native `.knxdb` save/reopen tests for
renamed area/line, reparented building/range and moved line. Device drag/drop
keeps using its existing validated device commands; structure reparenting
uses keyboard-accessible selects, not an unverified drag gesture. Scope and
remaining limits are recorded in
[IMPLEMENTATION_STATUS.md](../../IMPLEMENTATION_STATUS.md#2026-09-30--u12--issue-05-structure-editor-published-on-main)
and [KNOWN_LIMITATIONS.md](../../KNOWN_LIMITATIONS.md#u12-structure-editor-scope-issue-05).

### ISSUE-06: Site/property hierarchy decision

**User observation:** A property/site is missing; multiple buildings using one KNX infrastructure should be groupable beneath it.

**Files:**

- Research: `docs/RESEARCH.md`
- Review: `crates/knx-core/src/building.rs`
- Review: `docs/DATA_MODEL.md`
- If supported, modify storage/model/projection/import files identified by the research and add a migration.
- If not supported as an ETS type, create an ADR before adding a KNXBench-native concept.

**Interfaces:** No `BuildingPartType` or storage value is added until the representation, import mapping, and compatibility behavior are documented. Unknown external `Space/@Type` values remain reported rather than silently becoming Site.

- [x] Check KNX/ETS schema evidence and the installed corpus for a property/site concept and record exact findings. (ADR-0038 E1–E4; `a_ground_root_groups_two_buildings_of_one_installation`)
- [x] Decide between an evidenced external type, a KNXBench-native hierarchy node, or no domain change with a documented explanation. (ADR-0038 accepted 2026-09-28; `an_undocumented_root_type_is_reported_not_read_as_a_site`)
- [x] If a type is added, specify migration, import preservation, export non-goal, tree placement, and multi-building tests before implementation. (Not applicable: accepted decision adds no type or migration; import and native round-trip are covered by `a_ground_root_groups_two_buildings_of_one_installation` and `a_ground_site_over_two_buildings_on_one_line_round_trips`.)
- [x] Add the UI only after the model decision; prove multiple buildings can share the same installation without ownership duplication. (2026-09-30: Buildings workspace **Add site / property** creates a `Ground` root through `createBuildingPart`; the existing Inspector `moveBuildingPart` reparents two buildings. `StructureWorkspace.test.tsx` covers both commands and a single device projection per building; local mocked EN/DE Chromium `site.e2e.ts` covers the flow at 360/1440 px. `a_ground_root_groups_two_buildings_of_one_installation` proves two owned devices are referenced once in one installation, and `a_ground_site_over_two_buildings_on_one_line_round_trips` covers native persistence. No ETS Ground export or live device tested.)

### ISSUE-07: Main-workspace product catalog with atomic multi-device creation

**User observations:** Product Catalog should open in the main window, accept a quantity, and add multiple devices.

**Files:**

- Modify: `apps/knx-web/src/App.tsx`
- Modify: `apps/knx-web/src/CatalogBrowser.tsx`
- Modify: `apps/knx-web/src/StructureWorkspace.tsx`
- Modify: `apps/knx-server/src/domain.rs`
- Modify: `apps/knx-server/src/routes.rs`
- Test: `apps/knx-server/tests/http_catalog_to_device.rs`
- Test: `apps/knx-web/src/CatalogBrowser.test.tsx`
- Test: `apps/knx-web/src/App.test.tsx`

**Interfaces:** Catalog is a workbench view, not an `Overlay`. Multi-create submits one bounded batch request and applies one atomic `Command::Batch`; diagnostics identify the item/index that failed.

- [x] Add navigation tests proving Catalog occupies the centre workspace and retains search/selection while switching ordinary project views. (`App.test.tsx`, including narrow-screen navigation.)
- [x] Define and validate quantity bounds, generated names, target line, and address allocation; do not guess addresses when the line has no free valid slot. (1–32; no automatic physical address; `domain.rs`, HTTP corpus test and guide.)
- [x] Add server tests proving all-or-nothing creation and one undo removes the full batch. (`http_catalog_to_device.rs`; `command.rs` also identifies and rolls back a late failing child.)
- [x] Implement quantity and preview in the catalog before submission, then render per-device diagnostics without hiding successful metadata. (`CatalogBrowser.test.tsx`; old-server partial result is reported without retry.)
- [x] Verify single-device creation remains the quantity-one path, not a separate implementation. (`create_device_impl` delegates to `create_devices_impl(..., 1)`; API omission test preserves the old body.)

### ISSUE-08: Product-data fidelity and communication-object organization

**User observations:** Communication objects and DPTs appear with generic names; Parameters often show missing-data issues; conditionally active communication objects should be hidden/inactive; objects should be grouped under their channel/main function and collapsed by default.

**Files:**

- Investigate: `crates/knx-productdb/src/`
- Investigate: `crates/knx-etsproj/src/`
- Modify as evidence requires: product enrichment/projection DTOs
- Modify: `apps/knx-web/src/Inspector.tsx`
- Modify: `apps/knx-web/src/ParameterPanel.tsx`
- Test: corpus/product tests in `crates/knx-app/tests/` and `crates/knx-productdb/`
- Test: `apps/knx-web/src/Inspector.test.tsx`
- Test: `apps/knx-web/src/ParameterPanel.test.tsx`

**Interfaces:** Product data remains separate from project data. Generic labels are explicit fallbacks only after language-aware name/DPT resolution fails. Conditional visibility and channel grouping come from parsed/evaluated manufacturer data, never UI name heuristics.

- [x] Reproduce each symptom against installed product data and trace source XML/database → normalized product model → enrichment → projection → UI. (Data half, 2026-09-29/30: throwaway corpus probes, aggregates only. The "all inactive" symptom traced to the import boundary — schema ≥21 read an absent `IsActive` as `false` — fixed in P1 `a2ff938`; the rest traced to projection gaps closed in P2/P3. The UI step is the UI half.)
- [x] Count and classify diagnostics; fix resolvable mapping/import defects and document unsupported constructs with preserved source evidence. (Panel `NoBranchMatched` 1016/978/61 counted and classified `info`, everything else `warning` (P2 `dae6c1a`); the `IsActive` defect fixed (P1); untitled channels and objects with no or multi-DPT product data documented, not guessed — KNOWN_LIMITATIONS §146.)
- [x] Add language-aware object and DPT display-name resolution while retaining canonical DPT identifiers alongside names. (Names: T33 overlay, 0 corpus objects without one. DPT: `dpt_text` beside the canonical `dpt`/`program_dpt`; product `function_text` with per-instance module arguments — P3 `9795168`.)
- [x] Carry evaluated active/visible state and evidenced channel ownership into the projection. (`ComObjectNode.activation` four-valued and `channel` from the evaluator's own `ActiveRef` owner — ADR-0050, P2 `dae6c1a`. Server-only `#[ts(skip)]` until the UI half adopts them.)
- [x] Render channel groups collapsed by default, preserve a user-expanded group while the device remains selected, and visibly distinguish inactive from unsupported data. (U12 UI half published on `main` as `9040df0f` + review fix `4f06dfba`: `DeviceWorkspace.test.tsx` verifies opaque keys, evaluated order, refresh/device reset, channel-independent vs unassigned, missing-owner fail-closed, four activation states and stored-claim mismatch; `ParameterPanel.test.tsx` verifies notes vs warnings and unknown/missing severity fail-closed; local-only `e2e/device-editor-layout.e2e.ts` exercises EN/DE keyboard and 360/1440 px DPT readability without KNX hardware. Product channel `name`/`number` presentation is a separate §146 step.)
- [x] Present `Channel/@Name` and `Channel/@Number` without inventing a combined label or numeric meaning. (`DeviceWorkspace.test.tsx::shows source channel name and opaque textual number beside translated text`, `::uses the verbatim channel name without Text, and keeps numbers visible even without a name`; local `device-editor-layout.e2e.ts` checks EN/DE and 360/1440 px.)
- [x] Add corpus regression counts so improved resolution cannot silently reduce another product application's data. (`crates/knx-etsproj/tests/com_object_activity.rs` and `apps/knx-server/tests/com_object_activation_corpus.rs`: activation, channel, DPT and function-text counts for ETS4, ETS 6.3.0 and KV.)

### ISSUE-09: Device editor semantics and compact readable controls

**User observations:** Fields can be too narrow; R/W/T/U/C/I should be written out; one object should be linkable in both directions; when assigned to a line, only the device octet may need editing.

**Files:**

- Modify: `apps/knx-web/src/Inspector.tsx`
- Modify: `apps/knx-web/src/styles.css`
- Modify as verified: `crates/knx-core/src/command.rs`
- Modify as verified: `apps/knx-server/src/domain.rs`
- Test: `apps/knx-web/src/Inspector.test.tsx`
- Test: command tests in `crates/knx-core/src/command.rs`

**Interfaces:** Keep separate Send and Receive `GroupLink` values in the domain. A “both” UI action submits both links atomically. Any line-relative address editor still validates the reconstructed complete `IndividualAddress` in the domain.

- [x] Add layout tests for long names, addresses, and translated labels without clipping. (`e2e/device-editor-layout.e2e.ts`: Chromium with mocked API, EN/DE × 360/640/1440 px, long text and actual Inspector; removing narrow tab wrapping made the 360 px regression fail.)
- [x] Replace letter-only flag controls with readable names (a compact 2×3 layout is acceptable) while retaining standard letters and contextual help. (`Inspector.test.tsx::shows the standard letters and the %s flag names without hiding them in hover tips`; browser layout checks both languages.)
- [x] Add a “Send and receive” choice that creates/removes the two existing directional links atomically; preserve separate editing afterward. (`Inspector.test.tsx::offers Both as one request`, `::reports a refused paired link without refreshing or retrying a partial result`, `::offers one atomic unlink-both action`; `http_edit_routes.rs::linking_then_unlinking_a_com_object_to_a_group_address`; `command.rs::both_direction_links_are_one_undoable_batch_and_roll_back_a_late_duplicate`, `::failed_unlink_both_and_undo_preserve_original_group_link_order`, `::unlink_both_undo_and_redo_keep_interleaved_links_in_order`, `::restore_group_link_rejects_an_out_of_bounds_position_without_mutation`, `::undo_preserves_imported_duplicate_group_links_and_redo`.)
- [x] Verify from KNX documentation that line membership constrains area/line octets before changing the address editor. (`docs/RESEARCH.md` §20.2 cites KNX Association *Project Schema23* §1.2.4–1.2.5 and the coupler-specific offline project check; importer mapping cross-check recorded.)
- [x] If verified, expose only the device octet for line-assigned devices, reconstruct the full address, and reject mismatch/duplicate/reserved values in core tests. (`Inspector.test.tsx` line-bound cases; `command.rs::line_bound_address_rejects_a_different_area_or_line_without_mutation`, `::assigning_coupler_only_zero_to_a_line_bound_device_is_explicitly_unsupported`, `::line_with_two_owning_areas_cannot_pick_an_arbitrary_address_prefix`, `::later_installation_line_owns_the_device_address_prefix`, `::a_device_listed_as_both_line_bound_and_unassigned_cannot_be_readdressed_or_moved`, `::moving_a_device_unassigned_in_two_installations_cannot_leave_a_duplicate_placement`, `::repeated_device_in_one_line_is_not_a_single_valid_placement`; `Inspector.test.tsx` mixed/repeated placement regressions; `knx-store/src/command_sync.rs::undo_restores_an_imported_coupler_address_in_the_device_row`; `http_edit_routes.rs::a_line_bound_address_route_refuses_invalid_values_and_undo_restores_imported_data`; targeted guard mutations all failed before restoration.)

### ISSUE-10: Actionable validation errors and contextual help routing

**User observations:** HTTP 422 should explain the error and suggest syntax; Group Address Range needs explanation; F1 invoked from contextual help should open the exact topic rather than the generic beginning.

**Files:**

- Modify: `apps/knx-server/src/errors.rs`
- Modify route parsers in: `apps/knx-server/src/routes.rs`
- Modify: `apps/knx-web/src/api.ts`
- Modify: `apps/knx-web/src/HelpTip.tsx`
- Modify: `apps/knx-web/src/HelpPanel.tsx`
- Modify: `apps/knx-web/src/help.ts`
- Modify: `apps/knx-web/src/GroupAddressTable.tsx`
- Test: `apps/knx-web/src/api.test.ts`
- Test: `apps/knx-web/src/HelpTip.test.tsx`
- Test: `apps/knx-web/src/HelpPanel.test.tsx`

**Interfaces:** Validation responses carry a stable machine-readable kind plus human detail and syntax/example fields where applicable. Help opening accepts a topic id; global F1 uses the focused control's registered topic, falling back to the normal overview.

- [x] Add 422 tests for malformed individual address, group address, DPT, and BCP-47 language with exact expected syntax examples. (`http_validation_errors.rs::malformed_editor_values_have_stable_kinds_and_exact_syntax_hints`; `api.test.ts` preserves raw detail.)
- [x] Add contextual-help tests: flag tip → communication-object flags, range help → group ranges, ordinary F1 → workbench overview. (`App.test.tsx::routes F1 from a focused flag tip`, `App.test.tsx::opens the group-range topic requested`, `App.test.tsx::opens the help panel on F1`, `HelpTip.test.tsx`.)
- [x] Implement structured errors at the parsing boundary and keep raw server detail available for diagnostics. (`http_validation_errors.rs`; `api.test.ts::preserves structured 422 diagnostics`.)
- [x] Implement topic-targeted HelpPanel focus/scroll and make the active heading the announced dialog context. (`HelpPanel.test.tsx::names the dialog for the active topic`; `App.test.tsx`.)
- [x] Add concise Range help describing hierarchy/containment without implying a datapoint range. (`GroupAddressTable.test.tsx::explains that Range`; `help.test.ts` checks both languages.)

### ISSUE-11: Bus-monitor pause, export, decoding visibility, and statistics

**User observations:** Play/Pause and export are missing; filter input is too small; decoded payloads are not apparent; useful statistics are absent. Existing code already has freetext and service filters, so their reachability is part of this task.

**Files:**

- Modify: `apps/knx-web/src/BusMonitorPanel.tsx`
- Modify: `apps/knx-web/src/styles.css`
- Modify as required: `apps/knx-server/src/bus_routes.rs`
- Modify as required: `apps/knx-server/src/bus.rs`
- Test: `apps/knx-web/src/BusMonitorPanel.test.tsx`
- Test: `apps/knx-server/tests/http_bus_monitor.rs`

**Interfaces:** Pause stops client polling/render advancement but does not disconnect or reset the server cursor; resume fetches the buffered gap with existing dropped-row accounting. Export preserves timestamp, source, destination, service, raw payload, DPT, decoded value/error, and sequence.

- [x] Add tests proving current text/service filters are visible, labelled, wide enough, and filter already-buffered rows without altering the cursor. (`BusMonitorPanel.test.tsx::keeps the existing text and service filters labelled and wide enough to reach`, `::filters rows client-side, without re-fetching`; `diagnosticShell.test.ts::gives a narrow telegram table a horizontal scroll area`; headless Chromium at 640 px measured a 299 px filter, 928 px internally scrollable table and no document overflow.)
- [x] Add fake-timer tests for pause/resume, gateway-close while paused, buffer overflow while paused, and disconnect while paused. (`BusMonitorPanel.test.tsx::stops polling and resumes from the held cursor`, `::ignores an in-flight reply after pausing`, `::disconnects while paused`, `::does not overlap slow polls`; test URLs never reach a production gateway.)
- [x] Add deterministic export tests over raw-only, decoded, decode-error, and dropped-gap rows; use a documented open format and escape spreadsheet-active cells if CSV is chosen. (`busMonitorCapture.test.ts::roundtrips raw, decoded, error and closed-marker rows`, `::preserves formula-looking Unicode`, `::prunes only the oldest client rows`; `busMonitorCaptureDelivery.test.ts` and native `bus_capture_export_tests`. JSON v1, not CSV; manual chapter 07 documents the fields/loss counters and 16 MiB boundary.)
- [x] Trace a reported undecoded payload through group-address DPT resolution and codec support; distinguish “no DPT assigned”, “conflicting DPT”, “unsupported DPT”, and “decode failed” in the UI. (`docs/RESEARCH.md` §21 traces `bus.rs::GroupAddressContext::decode` to `knx_core::decode`/`DptCodecError`; `bus_routes.rs::codec_failures_keep_the_dpt_and_distinguish_unsupported_from_malformed_payloads`, `BusMonitorPanel.test.tsx::labels unresolved, conflicting, unsupported and malformed decoded payloads`, and `::does not guess the cause of a legacy decode error`.)
- [x] Add session statistics for service counts, busiest group addresses, and talkative source devices only when the captured DTO contains the required source data; label buffer-limited results honestly. (`busMonitorStatistics.test.ts::counts actual services` and `::bounds both retained rows`; `BusMonitorPanel.test.tsx::bounds a large captured batch`; Chromium fixture with 5 retained rows and distinct source/destination counts.)
- [x] Verify large buffers do not block polling or render an unbounded statistics table. (`BusMonitorPanel.test.tsx::bounds a large captured batch and its statistics without stalling the next server cursor`; `busMonitorStatistics.test.ts::bounds both retained rows and every displayed ranking`; slow-poll overlap mutation rejected.)

### ISSUE-12: Gateway endpoint UX and AppImage discovery diagnosis

**User observations:** Host and port should be separate with KNXnet/IP port 3671 as default; AppImage discovery found no gateway although manual connection worked.

**Files:**

- Modify: `apps/knx-web/src/BusMonitorPanel.tsx`
- Modify: `apps/knx-web/src/busDiscovery.ts`
- Inspect/modify as evidence requires: `crates/knx-net/src/discovery.rs`
- Inspect/modify as evidence requires: AppImage/Tauri packaging under `apps/knx-desktop/`
- Test: `apps/knx-web/src/BusMonitorPanel.test.tsx`
- Test: `apps/knx-web/src/busDiscovery.test.ts`
- Test: `apps/knx-server/tests/http_bus_discover.rs`
- Document: `docs/KNOWN_LIMITATIONS.md` and `docs/RESEARCH.md`

**Interfaces:** UI keeps host and numeric port separate and composes the existing endpoint string only at the API boundary. Discovered endpoints populate both fields. Discovery remains a read-only multicast operation and never auto-connects.

- [x] Add parsing/validation tests for IPv4, IPv6, hostname, blank/default port, invalid port, and discovered endpoints. (`gatewayEndpoint.test.ts` covers parsing, defaults and rejected unsupported hosts/ports; `BusMonitorPanel.test.tsx` covers the separate fields, preference hydration, discovery selection and the unchanged start payload. IPv6 and hostnames are preserved but explicitly rejected: the server's `SocketAddrV4` tunnel cannot connect to them.)
- [x] Reproduce discovery from the unpackaged dev build and the exact AppImage on the same host/interface, recording bind address, HPAI, multicast interface, timeout, and firewall/sandbox evidence. (2026-09-28 U2 live syscall comparison, RESEARCH §20.1: both send from the host interface and get no response; firewall-rule inspection is denied, explicitly recorded as unknown.)
- [x] Rank causes from evidence before editing; do not add retries, sleeps, or a packaging workaround without a demonstrated mechanism. (U2 AppImage-vs-dev comparison, RESEARCH §20.1 and KNOWN_LIMITATIONS §79: a packaging-only fault is unsupported; network/gateway behavior remains unproven.)
- [ ] Implement the narrow fix at the owning network/packaging layer and retain manual connection as a first-class fallback.
- [ ] Add loopback tests where possible and document the boundary that still requires a real multicast network.

**U10 scope boundary (2026-09-29):** Separate fields and the manually entered
IPv4 endpoint can ship without changing the KNXnet/IP protocol. U2 did not
establish an AppImage-only defect: both packaged and unpackaged servers sent
the same multicast search and neither received a response. A wire capture or
gateway-side evidence is still required before declaring or coding a
discovery fix; the two discovery items above remain open, not silently closed
by the endpoint UX work.

**U13 pre-review correction (2026-10-01):** the preceding paragraph predates
RESEARCH §20.1's later findings. On 2026-09-29 the gateway's unicast response
was seen in the host's UFW drop log; a unicast probe to the same gateway
answered. On 2026-09-30 the user added an incoming UDP source-port-3671 rule,
and unchanged CLI and HTTP discovery both returned the gateway, without a
KNX bus write. No actual wire packet capture was taken, but the host cause
and its environmental fix were verified. The two checkboxes above stay open
until the chosen independent read-only review reconciles the external
firewall fix and existing offline tests (`http_bus_discover.rs`, the local
discovery-HPAI test) against their exact wording. Do not add a speculative
protocol retry, change the firewall here, or claim a loopback multicast
roundtrip that those tests do not provide.

### ISSUE-13: Session-log freetext search and export

**User observation:** Session Log has no freetext filter and no export.

**Files:**

- Modify: `apps/knx-web/src/LogPanel.tsx`
- Modify: `apps/knx-web/src/styles.css`
- Modify as required: `apps/knx-server/src/session_log.rs`
- Modify as required: `apps/knx-server/src/routes.rs`
- Test: `apps/knx-web/src/LogPanel.test.tsx`
- Test: `apps/knx-server/tests/http_log_route.rs`

**Interfaces:** Freetext search combines with existing severity filters over the same in-memory entries. Export scope is explicit (all entries or current filtered view) and preserves timestamp, severity, operation, summary, detail, and report metadata.

- [x] Add tests for case-insensitive search across operation/summary/detail and composition with severity filters. (`LogPanel.test.tsx::searches source, summary and detail case-insensitively`.)
- [x] Add export tests for quotes, newlines, Unicode, empty logs, and entries containing user-controlled spreadsheet-active prefixes. (`sessionLogExport.test.ts` JSON roundtrip/empty/loss tests; JSON strings, not CSV cells; `sessionLogExportDelivery.test.ts` native/browser delivery.)
- [x] Implement a labelled search input with a clear action and result count. (`LogPanel.test.tsx` search and count test.)
- [x] Implement export through the existing file-picker/download boundary; do not expose server filesystem paths to the browser. (Browser Blob URL; Tauri command owns its native save dialog and atomic write, `session_log_export_tests`; no path accepted from JS, no new HTTP route.)
- [x] Verify log capacity/drop behavior is disclosed so an export is not presented as a complete lifetime audit. (`sessionLogExport.test.ts` filtered loss marker, cap, unknown marker; `LogPanel.test.tsx` retention hint; §36/ADR-0047.)

## Source Coverage

| Original `docs/Issues.md` lines | Task |
| --- | --- |
| 3, 22, 28 | ISSUE-01 |
| 4-6 | ISSUE-02 |
| 8, 11, 13, 57, 90 | ISSUE-03 |
| 16, 18, 20, 24 | ISSUE-04 |
| 33, 41-46, 51 | ISSUE-05 |
| 48-50 | ISSUE-06 |
| 35-39 | ISSUE-07 |
| 26, 56, 59, 62 | ISSUE-08 |
| 57-61 | ISSUE-09 |
| 31, 64-70 | ISSUE-10 |
| 72-78 | ISSUE-11 |
| 79-80 | ISSUE-12 |
| 82-85 | ISSUE-13 |

The trailing empty bullet on line 85 carries no requirement. Every substantive observation is assigned at least once; overlapping layout concerns intentionally appear in both the broad layout task and their domain-specific task.

## Plan Self-Review

- **Spec coverage:** All substantive source lines are mapped above; existing filter/discovery/directional-link behavior is preserved rather than duplicated.
- **Placeholder scan:** Every task names its concrete outcome. Evidence-gated branches state the permitted results and required documentation.
- **Ownership:** UI state, server authority, domain validation, product data, and protocol/packaging diagnostics remain in their current layers.
- **Ordering:** ISSUE-04 precedes autosave UX; ISSUE-06 precedes any site UI; ISSUE-08 precedes channel grouping; ISSUE-12 diagnoses before changing discovery.
- **Risk:** No task authorizes real KNX writes. Real multicast discovery validation remains read-only and must follow the project's hardware/network rules at execution time.
