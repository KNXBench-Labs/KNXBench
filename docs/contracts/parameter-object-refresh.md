# Parameter-driven communication-object refresh

Scope: offline engineering edits, source builds after Alpha 12.
Decision: [ADR-0133](../adr/0133-parameter-edits-reconcile-object-inventory.md).

## User contract

- Left navigation has independently foldable Workspaces, Project explorer and
  Diagnostics sections. Folding hides the complete content without unmounting
  it, changing selection or editing the project. A deliberate search reveal
  reopens the explorer. Fold state is local to the mounted navigation pane and
  resets when that pane is rebuilt; it is not a persisted project preference.
  The explorer remains its own bounded scroller, so visible-row effects keep
  using the actual visible tree rather than the full hidden overflow.
- File keeps the existing actions, grouped as Project, Import & export,
  History & comparison, and Help & diagnostics. Browser/desktop-specific
  export, logout and Quit rules, keyboard activation and Escape are unchanged.
- **Device functions** in the Communication objects tab exposes the active
  supported manufacturer enum parameters structurally controlling object
  branches, including parameters gating those controls. It is not a free-text
  FunctionText override, DPT editor or arbitrary device-function generator.
  Expand it even if the object table is empty, then select the operating mode
  and, where the manufacturer requires it, the actual button function.
- Enum selections save immediately, with pending controls disabled and inline
  refusal feedback. A stale write keeps its draft and requires explicit retry
  against refreshed state. Numeric/text fields still commit on blur/Enter.
- Alpha 13 groups Parameters fields by declared blocks; source builds after Alpha
  13 use [workspace area navigation](parameter-workspace.md) instead of ordinary
  block folding, reusing the same bound rows. Missing
  group metadata does not hide fields. Folding in the released accordion view
  and area navigation/search in the newer workspace keep row/editor identity and
  drafts; neither reorders fields or adds a command of its own. Existing numeric/text
  blur-save can still run when focus moves to a navigation control. Ambiguous block owners
  remain ungrouped. Optional caption source admission is bounded to 8 MiB,
  100,000 XML events and depth 64; unavailable captions use numbered labels.

## Domain and persistence contract

A successful parameter action uses the existing validation, snapshot binding,
command stack and durable-before-live commit. On a copy, the existing evaluator
identifies positively activated product references. Missing unscoped references
are instantiated through the existing catalogue field/provenance mapper.
Existing object IDs, order, overrides and links are unchanged; additions append.
Existing references are matched against actual catalogue declarations: a literal
catalogue-local ID is not blindly treated as a project-local alias. When both
literal and expanded candidates are declared, reads report the ambiguity while
keeping readable parameters visible, and the whole edit is refused. This is a
conservative application policy for unresolved identity, not a KNX format ban.
Original instance reference strings are never rewritten.
Inactive instances are retained for inspection and later reactivation.

Ambiguous/malformed declared DPTs are never selected automatically. The same
ProductDB declaration resolver is shared by enrichment and read-only diagnostics.
Warnings are reconstructed on panel reads while the configured object has no
resolved type, so the UI's subsequent GET cannot erase the report. A known
explicit DPT choice resolves that warning; reading diagnostics changes neither
the project nor the catalogue.

Parameter changes, new IDs and new objects form one undo step. Undo removes only
that step's appended objects and restores the prior parameter state; redo restores
the same object IDs. ID high-water marks remain reserved. Native save/load retains
the semantic result without a schema migration. The Core append inverse refuses
linked objects and a changed suffix rather than deleting unrelated state.

Preparation does not hold project and catalogue locks together. Publication
rechecks the exact source project plus any supplied revision binding. Missing
object declarations, invalid object numbers and allocation failure refuse the
whole write, without spending IDs or applying the parameter.

## Compatibility limits

The algorithm has no MDT or other manufacturer/product special cases. Positive
object activation remains governed by the existing evaluator, including its
unknown/skipped constructs and diagnostics. Missing active module-scoped objects
are an explicit write refusal until their instance/number mapping is supported;
no ModuleInstance, repeat expansion or number offset is guessed. This is not
universal manufacturer-database, manufacturer-script, ETS export or physical
programming compatibility. Product data and original retained sources are not
rewritten by presentation or parameter editing. No bus connection is made.

## Regression coverage

- `crates/knx-core/tests/add_com_objects.rs`: ordering, exact inverse/redo,
  defaults, duplicate IDs, wrong owners and linked-object refusal.
- `apps/knx-server/tests/http_parameter_panel.rs`: parameter-controlled missing
  object creation, metadata, one-step undo/redo, repeated switching, missing
  declaration/module/identity refusal, local-ID and project-alias nonduplication,
  preservation of existing links/overrides, failed
  durable-commit rollback, stable DPT warnings across pure GETs and semantic
  native save/load.
- `NavigationSection.test.tsx`, `App.test.tsx`, `ParameterPanel.test.tsx`:
  independent folding, retained nodes, grouped menu actions, immediate enum
  commit and grouped editor retention.
- `devices-navigation.e2e.ts`: real App parent, intercepted fictional API,
  object refresh after dropdown changes, grouping, navigation/menu, EN/DE,
  wide/narrow screens and overflow. Interception is UI evidence, not backend
  or physical-device acceptance.

Private project/source files and detailed records remain outside Git. Aggregate
real-copy checks are separate from public synthetic regression assertions.
