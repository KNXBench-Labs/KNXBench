# WP-51 — Wizard for empty project-object names

Owner request: 24 November 2026, approved naming-wizard interview decisions
Q1, Q2 and Q4–Q17; Q18 requests this independent work package.
Status: Partial in progress — slice 51.1a published; slice 51.1b structural name commands implemented in candidate, pending coordinator review/gate/delivery; remaining 51.1, 51.2–51.4 and Wizard not complete.
Slice 51.1a covers validated, reversible name-only domain commands for Project (`RenameProject` / `RestoreProjectName`) and Project Function (`RenameFunction` / `RestoreFunctionName`), unambiguous Function ID resolution, native working-history snapshot replay/roundtrip coverage, and diagnostic allowlist wiring. Slice 51.1b adds validated structural renames and exact Restore*Name inverses for Installation, all 11 supported BuildingPart kinds, Area, Line and GroupRange. Installation name operations reject duplicate IDs without changing legacy create resolution. Name no-ops, programming-state preservation, atomic rollback and native working-history replay use the existing command stack. No wizard UI, recipes, detection or provenance work is included.
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
| 51.1b | Validated structural name-only commands and exact restore inverses; unique installation name resolution; core identity/admission/no-op/batch regressions and native history replay across every supported building kind | Implemented in candidate; focused diagnostics recorded below; coordinator review, scoped gate and delivery pending |
| 51.1 (remaining) | Audit Q9 source/model/name constraints beyond the implemented command paths; define provenance and deterministic missing-name detection without mutating import data | Planned (not started) |
| 51.2 | Implement all recipes, ambiguity/manual cases, parent dependency invalidation, bounded scan/count/page semantics and preview binding; reuse candidate stack, `Command::Batch`, stale-snapshot guards and durable native write-before-publish | Planned (not started) |
| 51.3 | Implement entry timing, Later/manual reopen, grouped editable selection, source/context/errors, duplicate warnings, 256 cap and explicit confirmation; reuse Overlay; keyboard/focus, narrow displays and all supported translations; update all Q2 device consumers | Planned (not started) |
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

## Acceptance matrix (planned, unchecked)

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

None at owner level: the naming interview frontier is closed. Slice 51.1a is
published. The owner authorized exactly successor 51.1b; implementation stops
at that boundary. Coordinator review, scoped gate and delivery are pending;
remaining 51.1, 51.2–51.4 and the Wizard remain incomplete. Source-derived details still to establish within
this plan:
exact descriptive-text/product-resolution access for every type, validated command
coverage and history serialization, address/span formatting, scan/page budgets
and completeness signals, preview invalidation/API plumbing, manual-draft
retention and the exact surviving focus target. Resolve these from current code
and tests without weakening the approved rules; report a genuine scope blocker
rather than inventing source evidence or silently narrowing the entity set.
