# ADR 0031: Project notes are a project-owned collection

Date: 2026-09-22
Status: Accepted
Session: Goal-completion, Task 21

## Context

KNXBench has no domain concept for user-written project notes. The root
aggregate currently owns exactly `schema_version`, `strings`, `info`,
`installations`, `devices`, and `ids` (`crates/knx-core/src/project.rs:183-191`).
The existing in-application help is static product prose (ADR-0024), while
`knx-report` derives an HTML document from project data. Neither can preserve
an installer's explanation of why a project decision was made.

A project-only text field would be cheap, but would not let a note identify the
line, device, group address, or communication object it explains. Putting a
field on every entity would spread one concern across the model, persistence,
import, diff, and command code. User-written notes also cannot be represented
in `.knxproj`; KNXBench deliberately does not write that format (ADR-0028).
Any future conversion to a format without notes must therefore treat them as
unsupported user data, never as disposable decoration.

## Decision

The future domain model will add one ordered, project-owned `ProjectNote`
collection. Each note has a stable internal `ProjectNoteId`, a typed target, a
plain-text title and body, and an `include_in_documentation` flag. The target
is either the project itself or a stable user-facing project entity:
installation, area, line, building part, device, group range, group address,
or communication-object instance. Parameters and module instances are not
initial targets: they are imported technical substructure rather than
independently navigable project entities. Adding another target later requires
an explicit model change; an untyped string reference is not accepted.

The collection belongs to the `Project` aggregate instead of to individual
entities. Persistence uses a separate note table keyed by note ID, with a
tagged target kind and target ID plus an explicit position. The table is part
of the native project store and requires the normal next schema version,
ordered migration, and frozen predecessor fixture. Existing projects migrate
to an empty collection. This ADR intentionally does not reserve a schema
number and implements no field, table, migration, endpoint, or UI.

Deleting a targeted entity must not silently cascade to its notes. The future
command must make the disposition explicit in the same undoable operation:
move affected notes to the project, retarget them, or delete them after user
confirmation. The store must reject dangling typed targets rather than load
them as valid notes.

Notes are UTF-8 plain text with preserved line breaks. They contain no HTML,
Markdown interpretation, embedded images, files, or other attachments. They
carry no author identity until KNXBench has a real user model. Scratch notes
do not automatically enter `knx-report`: `include_in_documentation` defaults
to `false`, and only notes explicitly marked `true` may be rendered in project
documentation.

Importing `.knxproj` creates no notes. Native save and load preserve every
note. If a future exporter or converter targets a format that cannot represent
them, its compatibility report must count and identify the unsupported notes
before the operation proceeds; it must never silently omit them or promise a
round trip that cannot exist.

## Alternatives considered

**One project-level text field.** Rejected because it makes the common case of
explaining a particular device, line, or group address depend on conventions
inside an unstructured document. Retrofitting entity targets later would still
require the collection and migration chosen here.

**A note field on every entity.** Rejected because it duplicates storage and
command handling across unrelated aggregates, makes new entity types repeat
the feature, and cannot naturally hold multiple ordered notes for one target.

**A generic string target such as `"device:42"`.** Rejected because typos and
renames would create undetectable orphans. A tagged target lets the domain and
store validate referential integrity exhaustively.

**Rich text, Markdown, or attachments.** Rejected for the first version. Rich
content adds sanitisation, rendering, file lifecycle, backup, and export
policy to a feature whose purpose is durable explanation. Plain text is
portable and sufficient; a later ADR may widen the content model with evidence
that the added machinery is worth owning.

**Render every note in `knx-report`.** Rejected because working notes can
contain incomplete reasoning or operational reminders not intended for a
delivered project document. Explicit opt-in keeps report output deliberate.

## Consequences

- Notes can describe the whole project or the user-facing entity they concern
  without changing every entity type.
- The next implementation needs coordinated `knx-core`, command, projection,
  `knx-store`, server, UI, report, migration, and regression-test work.
- Stable typed targets and explicit delete disposition prevent silent orphaning
  or loss of user-authored text.
- Native projects preserve notes, while unsupported external conversions must
  disclose the loss boundary before proceeding.
- Plain text keeps storage, sanitisation, and portability predictable, at the
  cost of no formatting or attachments.
- Report inclusion is intentional and testable; private scratch notes remain
  private to the project by default.
