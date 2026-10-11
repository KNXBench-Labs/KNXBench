# ADR 0111: Native instruction overlays keep imported text authoritative

Date: 2026-10-11
Status: Proposed (implementation and acceptance still pending)

## Context

Schema23 §1.2.5.1 admits plain/RTF `DeviceInstance/@InstallationHints`;
[the bounded adapter contract](../contracts/rtf-text.md) records the primary
source, supported subset and retention witnesses. Other text fields are not
admitted by analogy. Original logical attributes and whole XML already live in
the native opaque store. Its generic XPath does not identify a particular
device, so selecting the first matching attribute would bind the wrong owner.
The parsed source's `SourceDevice::other` is owned by the actual source device;
its source path and exact ETS ID match `DeviceInstance::source`.

The domain has no instruction field. Mutating opaque evidence would falsify
original-source provenance. Storing an editor draft only in UI/localStorage
would lose durable history and break native Save/Reopen. Putting RTF syntax in
Core would couple an external format to the normalized model.
ADR0031 deliberately keeps new project notes plain text; it is not a rich-text
back door and does not authorize broadening those notes.

## Proposed decision

Add a narrowly named, project-owned map of device instruction overrides keyed
by stable `DeviceId`. An override is a normalized text document with only
literal UTF-8 text and bold/italic/underline spans; it carries no RTF/HTML,
attachments, URLs, program parameters or source-format interpreter in Core.
The map avoids adding imported-format fields to every device constructor.
An absent override means use imported source. A present empty document means
explicitly empty instructions; these states must never be collapsed.

Read original instructions through the existing bounded project source parser,
selected by exact source path and ETS ID, requiring one matching device and
one admitted InstallationHints occurrence. Missing/duplicate/foreign/refused
source is explicit, not first-match fallback. Parse schema23 only for initial
RTF admission; other schemas require evidence before interpreted display.
Unchanged source and opaque entries are never rewritten or relabeled as edits.

Application services produce the original/derived/effective view and a
revision-bound preview of changes. Supported-only edits may replace the
effective normalized text. Unsupported or malformed originals require an
explicit acknowledged conversion, bound to the inspected original digest and
current project/server/load generation; the retained original remains exact.
Reset explicitly removes the override and returns to the original. Commands
validate device ownership, document limits and exact inverse restoration; one
user action is one undo step, including durable native working history.

Persist overrides and ordered spans in dedicated native tables with strict
row validation, typed device FK, full atomic save/load and next ordered schema
migration. Freeze the actual current predecessor (currently schema12), migrate
old projects to an empty map, and never silently drop unknown/future/corrupt
rows. Native history snapshots, semantic equality, selective-import closure,
entity delete/restore, diff and any unsupported conversions must account for
this new model field before user-edit admission. Do not publish a schema bump
with an incomplete representability/history barrier.

Device deletion must remove its override in the same reversible operation and
restore it exactly on undo; dangling targets are rejected by Core/store. No
automatic discard because an imported source entry moved or became unreadable.
No inference that instructions belong in an ETS export or hardware download.

## Alternatives and remaining acceptance

- Edit the opaque original: rejected because retained-source claims become false.
- Add raw RTF to Core or every entity: rejected format coupling/duplicated concern.
- Store only a UI draft: rejected missing native persistence and history.
- Reuse rich project notes: rejected ADR0031 scope and mixed semantics.

Proposed, not implemented. Required gates: nonempty original-bound application
reads, ambiguous/missing/refused source, conversion consent/conflict/no-op
preservation, exact edit/reset/undo/redo, nonempty predecessor migration and
corrupt-row refusal, repeated native save/reopen and durable history, complete
model/opaque equality, deletion/selected import/unsupported converter barriers,
real server operation and isolated browser at wide/narrow EN/DE. Review and
accept this decision from actual execution before claiming a working editor.
