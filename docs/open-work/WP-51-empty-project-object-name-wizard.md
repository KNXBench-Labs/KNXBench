# WP-51 — Wizard for empty project-object names

Owner request: 24 November 2026, approved naming-wizard interview decisions
Q1, Q2 and Q4–Q17; Q18 requests this independent work package.
Status: Partial in progress — slices 51.1a–51.1c and 51.2a–51.2c published (`c6c6cfbcf` after independent re-review APPROVE and a green change-scoped package gate); 51.3a (web wizard dialog and manual entry) is published (`a3af339e1`); 51.3b (automatic opening and Later) is published on `main` (`13e5cb946`..`688eb5aeb`); 51.3c (Q17 read-only rule) is published on `main` (`1510e8883`, `e210ed533`); 51.3d (Q2 device-name display in the web consumers) is a work-branch candidate, pending review; the rest of 51.3, 51.4 and the Wizard as a whole are not complete.
Slice 51.1a covers validated, reversible name-only domain commands for Project (`RenameProject` / `RestoreProjectName`) and Project Function (`RenameFunction` / `RestoreFunctionName`), unambiguous Function ID resolution, native working-history snapshot replay/roundtrip coverage, and diagnostic allowlist wiring. Slice 51.1b adds validated structural renames and exact Restore*Name inverses for Installation, all 11 supported BuildingPart kinds, Area, Line and GroupRange. Slice 51.1c records the verified Q9 source/model/name constraints audit and adds the structured provenance data model and deterministic, strictly read-only missing-name detection across all Q9 entity types in `knx-edit::missing_names` (backed by shared `knx_core::names::is_blank_name`). Slice 51.1c did not include recipes or wizard/Apply paths; the bounded 51.2a successor below adds recipes only.
Blocked by: None (WP-51 and WP-52 are independent).

## Source report and evidence limits

The owner requested a wizard that proposes and, after explicit confirmation,
really renames unnamed project objects. This record translates the finished
Q1–Q18 interview into public requirements; private interview material and project
data are not reproduced. The reporter's ETS project and screenshots have not
been reproduced or validated here. This is not a proven fix for that project.
Source observations were read-only and bounded; the anchors below were checked
against current source at `8ab381831afffc42c25326747bb6775c5ba9e90f`, 24 November
2026. No build, test, import or runtime experiment was performed for this intake.

## Requested outcome

An end-to-end, context-aware wizard for genuinely missing project-object names:
detect, explain, preview, select/edit, explicitly apply validated real names,
undo, save and reopen. It covers every Q9 entity below, not just devices or GAs.
Device names must appear uniformly in topology, device list/editor and GA
assignment/selection, with physical addresses still identifiable (Q2). IDs and
addresses remain stable. An address or display fallback alone is not a rename.

## Existing verified base

All links are repository-relative source anchors at the recorded base, not
claims that the requested feature exists:

- [Literal GA recipe intent and limits](../../crates/knx-edit/src/name_recipe.rs#L16)
  and [prefix/suffix construction](../../crates/knx-edit/src/name_recipe.rs#L152):
  the existing engine concatenates literals around GA names. It is **not** the
  context-aware suggestion engine requested here.
- [Candidate stack, stale snapshot and write-before-publish](../../crates/knx-edit/src/name_recipe.rs#L195):
  rejects snapshot/revision conflicts, applies to clones, persists native working
  history before publishing. [Batch construction](../../crates/knx-edit/src/name_recipe.rs#L176)
  and [core batch rollback](../../crates/knx-core/src/command.rs#L3914) are reuse
  foundations, not a second undo implementation to replace.
- [Server preview](../../apps/knx-server/src/name_recipe_routes.rs#L159),
  [confirmation and apply](../../apps/knx-server/src/name_recipe_routes.rs#L220)
  and [native/session-only result](../../apps/knx-server/src/name_recipe_routes.rs#L273)
  provide preview identity, expiry, revision and persistence patterns.
  [GA toolbar entry](../../apps/knx-web/src/BulkActionToolbar.tsx#L84) is currently
  GA-only, not the requested project-wide entry.
- [Device/GA validated rename and old-name restore](../../crates/knx-core/src/command.rs)
  distinguish edited names from undo of imported text. Published slice 51.1a
  adds Project and Function name-only commands. Slice 51.1b extends this pattern
  to installations, all supported building kinds, areas, lines and group ranges:
  edited names pass `validate_edited_name`; dedicated restore commands retain
  exact imported names, including invalid historical text. Missing or duplicate
  target identities refuse mutation and name lookup. Labels may repeat. These
  are backend foundations; discovery, preview and Apply wiring remain open.
- Entity bases: [project name](../../crates/knx-core/src/project.rs#L231),
  [installation](../../crates/knx-core/src/installation.rs#L15),
  [device name/description](../../crates/knx-core/src/device.rs#L20),
  [building type/number/parent](../../crates/knx-core/src/building.rs#L13),
  [topology names/addresses](../../crates/knx-core/src/topology.rs#L12),
  [group ranges and GA descriptions](../../crates/knx-core/src/group.rs#L12),
  [function details versus member labels](../../crates/knx-core/src/function.rs#L30).
  Empty function-member labels can be normal; they are not wizard targets.
- Reuse [WP-49](WP-49-ui-sdk-behavior-pilot.md) and the existing
  [Overlay shell](../../apps/knx-web/src/Overlay.tsx#L18) for dialog focus,
  background isolation and dismissal; [focus restoration](../../apps/knx-web/src/Overlay.tsx#L107)
  needs an explicit surviving return target when the opener disappears. Recheck
  WP-49's pending policy decisions at implementation time; this package does not
  decide them or claim its review/acceptance.

## Authoritative owner decisions

These Q numbers belong to the 24 November naming interview, not WP-49's questions.

| Decision | Binding requirement |
| --- | --- |
| Q1 | Wizard for empty names across devices, rooms, GAs and all Q9 objects; proposals become real project names only after an explicit preview confirmation, not display-only fallbacks. |
| Q2 | Use the confirmed device name consistently in topology, list/editor and GA assignment/selection; retain the recognizable physical address. |
| Q4 | Check after import and on project open. If eligible candidates exist, automatically open the wizard at most once per project session; provide Later and a manual “Fill missing names” entry. Q17's read-only exception suppresses automatic opening. |
| Q5 | Only missing, empty or whitespace-only names qualify. Preserve every nonempty name, including defaults, placeholders and address-based names; no unsolicited cleanup. |
| Q6 | Suggest only from existing project/product evidence, with visible provenance. Never invent a location or function; require manual input without a basis. |
| Q7 | Editable preview grouped by object type with context and stable identifier, individual/group selection; only explicit Apply changes the project. |
| Q8 | Selected names apply atomically as one undoable batch. Never overwrite changes since preview; addresses, assignments and parameters are unchanged. |
| Q9 | Include project/installations, building parts, devices, topology areas/lines, group ranges/GAs and project functions. Exclude product/manufacturer data, communication-object text and labels of individual function memberships. |
| Q10 | Device recipe: location path – product designation; product alone if location is absent; existing device description if product is absent. Show missing evidence; without a usable basis require manual input. |
| Q11 | GA recipe: unambiguous function/role/location context, alternatively an existing GA description. Conflicting context requires manual input. DPT alone is insufficient. |
| Q12 | Building-part recipe: actual number plus type. Without a basis require manual input; never invent room purpose. |
| Q13 | Duplicate names are allowed and visibly warned, not blocked or silently numbered. Distinguish by stable IDs/context; no automatic suffixes. |
| Q14 | Use existing native durable editing history for a `.knxdb` target; an import without a native target remains session-only with a clear save notice. Never overwrite `.knxproj`; read-only has no Apply. |
| Q15 | Topology names derive from addresses; group ranges from the actual span in the project's address format. Project, installation and function names use only clearly associated existing descriptive text, otherwise manual input; never infer semantics from IDs/numbers. |
| Q16 | Resolve parents before children. Use existing parent names or parents selected in the same batch only. Parent edit/deselection invalidates dependents; preserve manual child edits, recheck and then apply atomically. |
| Q17 | Count and paginate; at most 256 selected changes per explicit Apply. Reject 257 or more without partial application. Further batches require manual confirmation and have separate Undo entries. Label limited detection incomplete; full totals only after a full scan. Read-only gets no automatic change prompt; manual inspection is available without Apply. |
| Q18 | Record the complete wizard independently from WP-52 and only link WP-50. A backend layer alone is not a finished wizard. |

### Per-type suggestion and ambiguity contract

| Type | Evidence-based recipe | Manual / ambiguity handling |
| --- | --- | --- |
| Project | Clearly associated existing descriptive text only | No such text or unclear association: manual; IDs/project numbers do not imply a purpose. |
| Installation | Clearly associated existing descriptive text only | No such text or ambiguous association: manual; no inferred semantics. |
| Building part, including room and every supported building-space kind | Actual number + actual type | Missing basis: manual; no fabricated room function. |
| Device | Location path – product designation; product alone without location; existing device description without product | Show missing location/product; no usable basis or conflicting evidence: manual. Do not manufacture product data. |
| Topology area / line | Existing topology address in its correct context | Missing/ambiguous address ownership or context: manual; never alter the address. |
| Group range | Actual start–end span in the project's group-address format | Missing/ambiguous span: manual; no inferred function from address numbers. |
| GA | Unambiguous function/role/location context; alternatively existing GA description | Conflicting context: manual, not silent fallback. No usable context/description: manual; DPT alone never suffices. |
| Project function | Clearly associated existing descriptive text only | Absent/ambiguous text: manual; member labels are evidence only where unambiguous, never rename targets. |

Every automatic suggestion shows its contributing source and missing/ambiguous
inputs. Manual edits remain identifiable as manual. Source descriptions,
communication-object text, member labels and product data are read-only inputs,
not fields to “repair”. Parent names from outside the selection may be used only
if already nonempty; a proposed but unselected parent is not an accepted name.

## Bounded internal stages and actual status

WP-51 is explicitly partial. The stages below are internal steps of one feature;
none is an individually complete product slice, and the Wizard is not complete.

| Stage | Planned work | Actual status |
| --- | --- | --- |
| 51.1a | Validated, reversible name-only domain commands for Project (`RenameProject`/`RestoreProjectName`) and Project Function (`RenameFunction`/`RestoreFunctionName`), unambiguous Function identity checks, native working-history replay and focused regression tests | Published before 51.1b (publication record at `746778942`; backend foundation only) |
| 51.1b | Validated structural name-only commands and exact restore inverses; unique installation name resolution; core identity/admission/no-op/batch regressions and native history replay across every supported building kind | Published (`3d335ab7e` / `b57106993`; backend foundation only) |
| 51.1c (remaining 51.1) | Audit Q9 source/model/name constraints beyond the implemented command paths; define provenance and deterministic missing-name detection without mutating import data | Published as `9c8c4e039`; independent review and 16/16 scoped gate accepted; publication recorded in snapshot archive |
| 51.2 | Implement all recipes, ambiguity/manual cases, parent dependency invalidation, bounded scan/count/page semantics and preview binding; reuse candidate stack, `Command::Batch`, stale-snapshot guards and durable native write-before-publish | Partial: 51.2a read-only recipes (`2867d28da`) and 51.2b offline preview, parent dependency invalidation, confirmation token and atomic `Command::Batch` apply (`0aa842ee1`, independent APPROVE, 14/14 scoped gate) published in `knx-edit`; server/API plumbing and durable native persistence wiring remain open |
| 51.3 | Implement entry timing, Later/manual reopen, grouped editable selection, source/context/errors, duplicate warnings, 256 cap and explicit confirmation; reuse Overlay; keyboard/focus, narrow displays and all supported translations; update all Q2 device consumers | Partial: 51.3a (published `a3af339e1`) adds the web wizard dialog (scan, editable selection, preview, explicit Apply, 256 cap, en/de) and a manual “Fill missing names” entry in the File menu and command palette. 51.3b (published on `main`, `13e5cb946`..`688eb5aeb`) adds Q4 automatic opening after import/open and a Later button, vitest only. 51.3c (published on `main`, `1510e8883`, `e210ed533`) adds the Q17 read-only rule (scan/preview `readOnly`, 409 `missingNameReadOnly` before any state change, no auto-open, review without Apply). 51.3d (work-branch candidate, vitest only) shows a blank device name as “Unnamed device (ID n)”, display-only (nonblank names unchanged), in these web consumers: device links, Project Explorer, Inspector, Devices and Structure workspaces, bulk toolbar, group-address links/matrix/member add/object inventory, global search, bus-monitor flow view, Device Download panel and batch, device checks (readiness table and compare target), pending-programming list, address-programming suggestions and identity-read device choice. Remaining 51.3 Q2 sites: `useDeviceComparisons.ts`, `App.tsx` (around line 811), `System2DownloadPanel` (server DTO `deviceName`), `PluginDataDialog.tsx` (around line 183) and the `downloadHandoff` no-address/selected names. Known limitation: global search matches the fallback label, but the group-address matrix column filter and the `deviceList.ts` device search still match the raw name only, so they do not find a blank-named device by its fallback. Narrow-display/real-browser checks and 51.4 remain open |
| 51.4 | Integrate native/session-only persistence, save/reopen and Undo, failure/conflict paths and end-to-end acceptance across every supported entity; update affected user/status/limitations docs only with verified results | Planned (not started) |

### 51.1b focused implementation diagnostics (26 November 2026)

Two offline, locked Cargo selections passed with dedicated scratch data/config
homes: `knx-core --test it structural_names::` (7 passed) and
`knx-store --test it project_history::structural_names_replay_inverses_save_open_undo_reopen_and_redo_exact_snapshots`
(1 passed, four historical-name fixtures). The [core matrix](../../crates/knx-core/tests/it/structural_names.rs)
checks all 11 building kinds, both group-range levels and the other structural
types for Unicode admission/limits, exact invalid-name restoration, complete
state equality, identity refusals, duplicate labels, no-ops and failed mixed-batch
history preservation. The [native history regression](../../crates/knx-store/tests/it/project_history.rs)
compares full snapshots including opaque data and manufacturer references through
inverse replay, save/open, undo, reopen and redo. No bus or hardware operations.
These are focused implementation diagnostics, not package acceptance: independent
review, the scoped package gate and delivery remain the coordinator's next steps.

### 51.1c Q9 audit conclusions and focused diagnostics (26 November 2026)

Verified source/model/name constraints across all 9 Q9 categories without mutating import data:

- **Command and admission coverage**: All 9 Q9 entity categories have validated `Rename*` and exact `Restore*Name` inverses in `crates/knx-core/src/command.rs`. Eight categories (`Project`, `Installation`, all 11 `BuildingPartType` kinds, `Device`, `Area`, `Line`, `GroupRange` at all nesting levels, `GroupAddress`) enforce `names::validate_edited_name` (non-blank via `is_blank_name`, <= 1024 Unicode scalars, no control chars or U+2028/U+2029). `Function` additionally enforces `FunctionDetails::validate_name` (<= 255 code points, `knx:String255_t`).
- **Import lowering and Q5 blank rule**: `crates/knx-etsproj/src/map.rs` and `map/functions.rs` lower absent XML `@Name` attributes to `""` via `.unwrap_or_default()` and keep present empty/whitespace strings verbatim. Shared `knx_core::names::is_blank_name` (`trim().is_empty()`) matches `validate_edited_name` exactly and preserves defaults, placeholders, address-based names and `\u{feff}`.
- **Identity and parent-child constraints (Q13, Q16)**: `InstallationId(0)` is valid, while `FunctionId(0)` is reserved (`FunctionInvariantError::ZeroId`). Vector-backed collections (`installations`, `buildings`, `areas`, `lines`, `group_ranges`, `group_addresses`, `functions`) can represent duplicate IDs, dangling or cyclic parent links, and multi-parent placements (line in multiple areas; device in multiple lines or building parts). `knx_edit::missing_names::detect_missing_names` surfaces every occurrence and parent state explicitly in deterministic parent-before-child order.
- **Explicit Q9 exclusions**: Product/manufacturer references (`product_ref`, `program_ref`, `medium_ref`, `ProgramDefaults`), `ComObjectInstance.{text,description}`, `FunctionMember.name`, `BinaryDataRef.name`, `DeviceInstance.description`, `GroupAddressEntry.description` and `FunctionDetails.{comment,description,function_type,number}` are read-only evidence only and never rename targets.
- **Focused diagnostics**: Offline locked Cargo checks with scratch XDG dirs passed: `knx-core --lib names::` (5 passed), `knx-edit --test missing_names --test offline_boundary` (7 passed: all 9 Q9 types + 11 building kinds, Q5 preservation + Q9 exclusions, duplicate/zero identities + cyclic/missing/ambiguous parents, Q17 incomplete-scan total suppression + pagination, determinism + immutability, Q6 provenance JSON roundtrip), `xtask check-layering`, `xtask check-headers`, and `tools/check_local_info.py`. Two mutant proofs (Q5 `\u{feff}` blank filter and Q17 incomplete-scan total leakage) failed the focused tests and were restored byte-identical. These are focused candidate diagnostics, not full package acceptance.

## Acceptance matrix (planned, unchecked)

Slice 51.2a implements the read-only recipe foundation in
`knx-edit::missing_name_recipes`: immutable proposal pages for all nine Q9 target
categories, including all 11 building kinds, with explicit contributing-source
provenance and manual gaps/refusals. Existing named ancestors may contribute;
proposed or edited parent names are not consumed. Dangling/cyclic ancestry,
ambiguous identity/installation/line ownership, contradictory evidence and DPT-only
GA semantics fail closed instead of producing invented names. Description fallbacks
must retain only sources that actually supplied the returned name.
The focused candidate suite and independent review cover exact recipes, all address
formats, structural refusal causes, fallback provenance and unchanged inputs.
Independent final review APPROVE; published as `2867d28da` after 14/14 scoped
package-gate stages passed (4,203 Rust passes, 146 ignored; corpus 92/92),
frozen inputs and no touched/unchecked user data. Live remote readback matched.
No UI, selection/draft
lifecycle, cross-page collision validation, revision-bound preview, confirmation,
Apply, save/reopen or native undo workflow is claimed for 51.2a. An Automatic
proposal is not a write authorization or a revision-bound apply plan.

Slice 51.2b implements the offline preview, parent-dependency invalidation,
confirmation-token and atomic `Command::Batch` apply layer in
`knx-edit::missing_name_apply` (`crates/knx-edit/src/missing_name_apply.rs`) with
17 focused integration tests (`crates/knx-edit/tests/it/missing_name_apply.rs`):
parent-before-child evaluation via the shared dependency graph (preventing
unselected proposed parents from leaking into child recipes while preserving
manual child edits), non-blocking duplicate-name warnings without automatic
suffixing, bounded preview payload accounting (`PreviewBudget` including full
warning payloads and JSON escaping), revision-bound confirmation tokens, the
256-item batch cap (rejecting 257+ with zero writes), stale-snapshot conflict
rejection (`HistoryError::Stale`), and single-entry atomic undoable apply on
`CommandStack`. Independent cross-vendor final review returned APPROVE (zero
findings); two targeted runtime mutants (warning payload budget charging and
`BuildingPart.children` dependency-graph edge) were killed (exit 101) and
restored in an isolated archive. Published as `0aa842ee1` after 14/14 scoped
package-gate stages passed (4,220 Rust passes, 146 ignored; corpus 92/92),
frozen inputs and no touched/unchecked user data; live remote readback matched.
No server/API routes, durable native write-before-publish wiring, wizard UI or
end-to-end save/reopen workflow is claimed for 51.2b.

Slice 51.2c adds the `knx-app` facades (`missing_names`, `missing_name_recipes`,
`missing_name_apply`), the server routes in
`apps/knx-server/src/missing_name_routes.rs` and the HTTP integration tests
(`apps/knx-server/tests/it/http_missing_names.rs`). Product-hardware conflicts
stay manual in scan and child re-evaluation (re-review finding F1), and the
cancel lock, token check and drop run inside `off_runtime` (F2). Independent
re-review: Spec and Standards APPROVE. Change-scoped package gate green
(4,234 Rust passes, 146 ignored; corpus 92/92), frozen inputs. Published as
`c6c6cfbcf`. Non-blocking follow-ups: resolve product evidence only for blank
candidates (S1) and strengthen history/reopen assertions (S2). No wizard UI
(51.3), acceptance matrix (51.4) or WP-52 work is claimed.

**51.3a status: PARTIAL (28 November 2026, published as `a3af339e1`).** The web UI
has the wizard dialog (`apps/knx-web/src/features/missingNames/`: typed client,
pure draft logic, Overlay dialog with scan pages, editable selection, preview
and explicit Apply) and a manual “Fill missing names” entry in the File menu
(History submenu) and the command palette, both enabled only with an open
project. English and German texts are added. Verification is vitest and
TypeScript only; no real-browser or end-to-end check has run. Not implemented in 51.3a:
automatic opening and Later (see 51.3b), the Q2 device-name consumers, and 51.4
persistence acceptance. **Open gap:** the web UI has no
read-only signal and the server has no read-only flag, so the Q17 read-only rule
(no automatic prompt, inspection without Apply) is not implemented in the UI.
The 51.3 row stays partial and no acceptance-matrix row is checked.

**51.3b status: PARTIAL (28 November 2026, published on `main` as
`13e5cb946`..`688eb5aeb`).** After a project open or import completes, the web UI opens the
wizard automatically at most once per project session (key
`server_incarnation|project_incarnation`). The check uses a cheap scan
(limit 1) and opens only when a candidate exists. It is silent on failure, and a
failed scan does not consume the once. It is suppressed while any overlay is
open or a save/load is running. The Later button appears only for the automatic
opening; manual reopen from the File menu or command palette is unchanged
(`features/missingNames/autoOpen.ts`, `useMissingNamesAutoOpen`, wired in
`finishLoadedProject`). Verification is vitest only (hook and App tests); no
real-browser run. Known gap: the auto-open gate sees browser-rendered modals
(`aria-modal`), the report dialog and App's own file pickers, but not native
file dialogs opened by other components (group-address CSV, project diff,
unresolved-program notice, plugin data) in the desktop app, so a late scan could
open the wizard behind one; the fix needs a shared counter in `filePicker.ts`.
The Q17 read-only rule followed in 51.3c.

**51.3c status: PARTIAL (28 November 2026, published on `main` as `1510e8883` and `e210ed533`).**
Owner decision for Q17: a project is read-only when its native `.knxdb` file (or
the directory holding it) is not writable for the server process; a session-only
project is never read-only. The server checks this with `access(2)` on every
request (`missing_name_routes.rs::native_read_only`), so a file made read-only
after a scan or preview is still caught. Scan and preview responses carry
`readOnly`. Preview and Apply refuse a read-only project up front with HTTP 409
`missingNameReadOnly`, before a plan, token, command-stack entry, working-history
write or publication exists; the earlier late 422 `missingNamePersistenceFailed`
path no longer applies to this case. The web client admits the flag (absent means
writable, a non-boolean is a contract error); the auto-open hook feeds the
one-item scan's `readOnly` into `shouldAutoOpen`, so a read-only project never
opens the wizard by itself; the manual wizard still scans and lets the user
review and edit the selection, shows an English/German read-only note and keeps
Preview and Apply disabled. Verification: HTTP integration tests (scan reports
`readOnly`, preview and a forged apply return 409, project, `history_lengths()`
`(0, 0)`, file bytes and on-disk history generation unchanged; a file made
read-only after the preview refuses the apply the same way; session-only stays
writable) and vitest (client, hook, wizard EN/DE). No real-browser run. Not
covered: write permission granted or revoked through ACLs or a read-only mount is
detected by `access(2)` but untested; the desktop app's native picker gap from
51.3b is unchanged. Open: Q2 device consumers (now 51.3d), narrow-display browser
check and 51.4. The 51.3 row stays partial and no acceptance-matrix row is
checked: the read-only row also needs the real-browser and import-timing evidence.

**51.3d status: PARTIAL (28 November 2026, work-branch candidate, pending review).**
Q2 in the web UI: `devicePresentation.ts::deviceLabel` returns a nonblank device
name unchanged and shows a blank one (same rule as the wizard, now shared from
`blankName.ts`) as “Unnamed device (ID n)” / “Unbenanntes Gerät (ID n)”. It is
display-only: no stored name, ID, address, assignment, parameter or programming
state changes, and the server DTOs are unchanged. Wired consumers: explorer
topology rows and drag-and-drop summaries, device list (name cell and selection
label), topology and building views, Inspector device heading and tab-list label,
GA links, GA matrix columns (the `addressTable.unknownDevice` fallback still
covers a missing device), GA member add, object inventory and `DeviceLink` (text
and accessible name). The physical address stays next to the label wherever it
was shown. The rename fields keep the raw stored name. After the wizard Apply
(`onTreeUpdate`) and after Undo/Redo (`App.tsx` passes `api.undo()`/`api.redo()`
to `handleTreeUpdate`), the consumers re-render from the republished tree; the
Undo path is confirmed by source reading, not by a test through `App`.
Verification: vitest `devicePresentation.test.ts` (blank variants incl. NBSP and
EM SPACE, exact nonblank names, German, English fallback, shared blank rule),
`DeviceLink.test.tsx` (blank label and accessible name), `StructureWorkspace.test.tsx`
(topology and building views: blank → named → blank re-render keeps ID and
address) and the `DiagnosticsCompanion` import-graph guard extended for the two
pure modules; full `vitest run` and `tsc` for every web tsconfig pass. No
real-browser run. The Q2 acceptance row stays unchecked: it also needs an
end-to-end Apply/Undo run against a native project.

Each row requires recorded evidence at implementation time, using fictional
fixtures and simulated/offline workflows. No row has been executed for intake.

| Accepted | Case | Observable result |
| --- | --- | --- |
| [ ] | All Q9 types, including project, installation, every building kind, topology, range, GA and function | Every supported object is discoverable, editable in preview, applied, undoable and persistent where a native target exists; excluded fields are byte-for-byte unchanged. |
| [ ] | Missing/empty/whitespace versus nonempty default, placeholder and address-based names | Only the former are candidates; preview/scan/Later/cancel never rename anything. |
| [ ] | Each recipe, absent inputs, contradictory evidence and DPT-only GA | Exact Q10–Q12/Q15 behavior above; visible provenance/gaps, manual input where required; no invented semantics. |
| [ ] | Parent selected, edited, deselected; child automatic versus manually edited | Invalidated dependencies require revalidation; no use of unselected proposed parents and no silent loss of manual child edits. |
| [ ] | Duplicate labels and duplicate/missing/ambiguous target identity | Labels warn and remain allowed without suffixes; invalid target identity is refused without partial mutation. |
| [ ] | Import/open/session timing, Later and manual reopen; read-only project | Automatically open the wizard after completed import or project opening when eligible candidates exist, at most once per project session; Later dismisses and manual reopen works. Read-only suppresses automatic opening and manual review cannot Apply, including via API. |
| [ ] | Individual/group selection, inline edits and explicit confirmation | Only selected validated names from the shown preview apply; no background auto-apply; stale or expired confirmations cannot overwrite newer project state. |
| [ ] | 0, 1, 256 and 257 selected changes; large/paginated/limited scan | 256 succeeds atomically with one Undo; 257 explicitly rejects with zero changes, no automatic splitting. Zero/no-op adds no write or history entry. Each later manual batch has its own Undo. Incomplete scan says incomplete; no full-project total without complete detection. |
| [ ] | Invalid edited names, stale snapshot, concurrent native edit or failed history write | Entire batch refused; in-memory project, command stack and durable history remain unchanged; retry requires an appropriate fresh preview, not silent overwrite. |
| [ ] | `.knxdb` apply, save/close/reopen, Undo/Redo and reopen after Undo | Exact names and existing native working history survive; Undo restores the exact original empty/whitespace names and no unrelated values change. |
| [ ] | Imported session without native target, then Save As and reopen | Clear session-only/save notice; names and Undo work in-session, become native on explicit save; original `.knxproj` untouched. |
| [ ] | Topology, device list/editor, GA assignment/selection after Apply and Undo | All Q2 consumers display the same device name and identifiable physical address; IDs, addresses, assignments, parameters and programming state unchanged. |
| [ ] | Keyboard-only and real-browser focus, narrow viewport, translations | Named dialog; usable selection/edit/confirm controls and error focus; Tab stays in dialog, Escape closes the appropriate level without applying, focus returns to a surviving trigger; content/actions remain reachable without clipped context, no untranslated new messages. |

## Exclusions and coordination

No bus operations, programming-state changes, GA assignments, parameter edits,
manufacturer/product editing, communication-object text repair, function-member
label renaming or changes to nonempty names. No second undo stack, persistence
store or speculative product-data inference. No `.knxproj` writer or promise to
fix the reporter's unreproduced import/screenshot.

Coordinate with [WP-49](WP-49-ui-sdk-behavior-pilot.md) for shared Overlay policy
and with existing domain/history owners for command compatibility. These are
reuse seams, not claims or changes to foreign plans. [WP-52](WP-52-windows-docker-storage-guidance.md)
is independently deliverable. [WP-50](WP-50-windows-desktop.md) is a separate
native Windows effort; linking it claims no Windows support and creates no
blocking dependency. Feature completion requires the wizard and **all** supported
objects to pass end-to-end acceptance; backend completion alone is insufficient.

## Open decisions

None at owner level: the naming interview frontier is closed. Slices 51.1a–51.1c
and 51.2a–51.2c (offline recipes, preview, invalidation, atomic batch apply,
server API and native write-before-publish apply) are published. 51.3a (web
dialog and manual entry) and 51.3b (automatic opening, Later) are published and
51.3c (Q17 read-only rule) is published; 51.3d (Q2 device-name display) is a
branch candidate; the rest of 51.3, 51.4 and
the Wizard as a whole remain incomplete. Source-derived details still
to establish within this plan:
exact descriptive-text/product-resolution access for every type, validated command
coverage and history serialization, address/span formatting, scan/page budgets
and completeness signals, preview invalidation/API plumbing, manual-draft
retention and the exact surviving focus target. Resolve these from current code
and tests without weakening the approved rules; report a genuine scope blocker
rather than inventing source evidence or silently narrowing the entity set.
