# ADR-0106: Selective project import into the open project

- Date: 2026-10-09
- Status: Accepted; locally implemented, unpublished
- Scope: IMPORT-01, with separate IMPORT-02/IMPORT-04 evidence tooling

## Context

The owner approved importing selected lines/devices into the currently open
project, not creating a separate partial project. Existing ETS import replaces
an entire project. Native history already records normalized project snapshots
and retained-source context separately; reusing it avoids a second undo engine.
The shared root and concurrent release/Functions work must remain untouched.

## Decision

Parse the complete source using the existing bounded container/parser pipeline
without installing manufacturer data. Select one explicit source installation
and one explicit destination installation. Normalize selected device IDs and
line IDs, then remap every included domain ID from destination high-water marks.
Include communication objects, directional links in original order, parameters,
modules and their arguments, program-default slots, required topology and
building/group-range ancestors. Dependencies never import unselected devices.

Preserve destination project identity, installation settings and address style.
Reuse only unambiguous compatible topology/range/address identities; differing
existing metadata are never overwritten. Duplicate addresses, ambiguous owners,
dangling selected references, unsupported preservation shapes or incompatible
translations refuse the whole operation. No implicit address reassignment.

Keep selection/ID mappings and selected-versus-full-source counts separate from
the original importer report. Bind preview/apply to source bytes, target server
incarnation and revision, selection and the exact retained-source context.
Recompute/admit the plan on apply; a stale preview refuses without changes.
No manufacturer installation, protocol call or live device action is performed.

Retain the original archive and the existing importer's opaque/member evidence
under a content-addressed import prefix. SourceRef paths use the same prefix;
raw ETS identifiers and payload bytes stay unchanged. The retained source may
contain unselected devices and confidential information, explicitly disclosed
before confirmation. Source metadata are evidence, not automatic edit defaults.

Use the existing native snapshot history to record one undo step. Retained
source context is append-only across ordinary edit undo/redo, matching the
current history contract; undo removes imported model entities, not private
source evidence. Durable editor admission precedes publishing in-memory changes.
Save/reopen must preserve model, context and history exactly within existing
native limits. No native/model schema bump or new runtime dependency is introduced.
The Linux private regression harness reuses the workspace `rustix` dev-dependency.

GUI and CLI share one application service; the GUI exposes source inspection,
selection, preview and explicit confirmation. The CLI offers read-only preview
and a bound confirmation token against an existing native project.

## Separate evidence tracks

IMPORT-02 consumes only explicitly supplied authorized `.knxproj` exports of
restore-point revisions. It does not parse `.restorepoint`, execute ETS, modify
an internal ETS store or claim historical regression acceptance without exports.
Private inputs remain outside Git and tests fail loudly when explicitly invoked
without their required configuration. Synthetic harness checks are not ETS proof.

IMPORT-04 analyzes `ConverterExceptions` wrappers and project-namespaced
ApplicationProgram/Static/DeviceCompare/ExcludeMemory/Options declarations.
Observed records are not automatically applied as download/comparison policy;
signatures, option defaults, merge precedence and runtime semantics remain
unverified until primary documentation and authorized differential evidence
establish them. Unknown fields are retained/reported, never silently skipped.

## Acceptance

Required: exact selected entity/dependency counts and ID mappings; unchanged
unselected/target fields; malformed/duplicate/dangling/refused selection;
conflicts and stale previews; original-byte preservation; one-step undo/redo;
nonempty seeded store refusal/rollback; native save/reopen/history; separate
HTTP/CLI and GUI regressions; isolated built-app visual/runtime proof. Run
relevant owning suites, warning-denied lint, frontend build/type checks and
repository/documentation gates. Report self-review as self-review.

## Alternatives

Whole-project replacement contradicts the approved target. Silent overwrite,
first-candidate identity matching and automatic readdressing hide conflicts.
A second archive parser, import-specific model or undo system duplicates existing
infrastructure. ETS internal-store reverse engineering does not supply an
authorized interchange fixture and is outside this scope.
