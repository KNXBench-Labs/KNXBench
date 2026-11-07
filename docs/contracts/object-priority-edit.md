# Editable object priority with a programming guard

Status: software-accepted on main `0ad3dc8cf` (7 November 2026); acceptance in the
[delivery record](../status/2026-11-07-object-priority-edit.md).
Scope: one WP-06 increment under [ADR-0158](../adr/0158-object-priority-provenance.md).
It builds on the [captured saved-instance contract](saved-instance-priority.md)
and the [effective object priority contract](effective-object-priority.md).
Owner decisions, 7 November 2026: priority becomes editable as a separate user
layer (native 17 → 18); a stated instance priority that differs from the
product literal makes the image builder refuse the device instead of writing the
product priority. No new priority encoding.

## Outcome and exclusions

A user can set or clear one communication object's project priority (`Low`,
`High`, `Alert`) in the device editor. The edit is a validated, undoable domain
command, saved natively, carried by history images and compared by project diff.
It never rewrites the captured saved declaration (ADR-0158 §4 of the future
requirements: user edits must not overwrite retained source facts).

The image request now carries every priority the project states for an object.
The builder refuses an active object whose stated priority is not exactly the
product's selected literal. Admitted images stay byte-identical to before.

Not admitted: a web object-table column, writing `High` or `Alert` from a
project edit, any change to the existing product-priority encoding (`Low`/absent
→ low, documented `Alert` inference, `High` refused), bus services, a
documentation-report or CSV consumer, a batch editor, and module-scope
evidence beyond the existing module identity.

## Domain

- `knx_core::ObjectPriority = Low | High | Alert`, the three tokens of
  Project Schema23 §1.1.2.4 (`ComObjectPriority_t`). It is a project token, not
  `TransmissionPriority`; no cast between them exists.
- `ComObjectInstance.priority_edit: Option<ObjectPriority>`; `None` = no user
  edit. Import, enrichment, catalogue creation and every constructor set `None`.
- `Command::SetComObjectPriority { com_object, priority: Option<ObjectPriority> }`
  sets (`Some`) or clears (`None`) the edit. It refuses an unknown object and a
  change to the value already set (`CommandError::PriorityUnchanged`, no empty
  undo step). Its inverse is the same command with the previous value, so
  undo/redo restore the exact prior edit state. `saved_instance_priority` is
  untouched in every case.
- Programming state: the command changes whether the device image is admitted,
  so `ProgrammingContent` carries each object's edit and the command marks the
  owning device's group-object part pending, like a flag edit.

## Effective priority

The read-only resolution gains one row ahead of all others:

| Instance | Effective result |
|---|---|
| `priority_edit = Some(t)` | `resolved`, value = `t`, source `userEdit` |
| otherwise | unchanged rules of the effective contract |

A user edit therefore also resolves an Empty/Unknown/Unavailable captured
declaration; the captured row in the editor keeps showing the source fact.

## Image request and builder

`ImageRequest.priority_statements: BTreeMap<ComObjectRef id, String>` lists the
priority each object instance states, by the same `com_object_lookup_id` key as
`flag_overrides`:

| Instance | Statement |
|---|---|
| `priority_edit = Some(t)` | `t` |
| captured Declared, Known token | the exact literal |
| captured Declared, Empty or Unknown | refused: `ProjectRequestError::UnreadablePriority` (device, object number, raw) |
| captured Absent or Unavailable, no edit | none (the product literal applies, as today) |

The request is built before activation is known, so an Empty/Unknown
declaration refuses the device even on an object the parameters leave
inactive, and it also stops read-only previews built from the same request.
Two instances sharing one `ComObjectRef` key (one module object in two module
instances) must state the same value or the request is refused
(`ConflictingPriority`); active module refs are refused by the builder anyway.

The builder looks up the statement of every activated `ComObjectRef`. A
statement is admitted only when the product's selected literal is present and
equal; otherwise it refuses with `ImageError::PriorityDiffers { ref_id, project,
product }`. An absent product literal is not treated as `Low` here: the project
states a value the product does not, and nothing proves equality. A statement
for an inactive ref has no octet and is ignored, like a flag override. The
existing product-priority policy then runs unchanged, so a stated `High` equal
to a product `High` is still refused by the existing High rule.

Unavailable captured facts (older native files, other schemas, module scope)
cannot be checked and keep today's behaviour; this is a named limitation
([KL-188](../KNOWN_LIMITATIONS.md#catalogue-kl-188)), not an admission that the
device holds the product priority. A captured declaration counts only while its
source still binds to the device and object (`SavedPriorityBindings`), as in
the effective priority. The serialized S2 `RequestRecord` is unchanged:
statements decide admission only and never reach an image octet.

## Storage and history (native 18)

- New table `com_object_priority_edit (com_object_instance_id INTEGER PRIMARY
  KEY REFERENCES com_object_instance(id), priority TEXT NOT NULL CHECK(priority
  IN ('Low','High','Alert'))) STRICT`. The migration 17 → 18 only creates it;
  nothing is backfilled or derived from retained attributes or captured facts.
- Whole-project save writes one row per `Some` edit and deletes the table with
  the other object tables. A row naming no object or an unknown token is refused
  on load.
- Read-only open migrates an in-memory copy only (existing mechanism).
- History images of versions 12–18 decode; semantic identity of a store
  without edits equals its native17 identity (the empty table is skipped), so
  existing working history and named versions keep their hashes.
- An authentic native17 predecessor (produced by the unchanged v17 writer,
  with captured declarations and undo/redo/named history) migrates to 18 with
  every captured fact, image and history row unchanged and every edit `None`.

## Comparison and API

- Project diff compares `priority_edit` as its own field; captured priority keeps
  its existing comparison.
- `POST /api/com-object-priority` with `{ comObjectId, priority }`, priority one of
  `"Low"`, `"High"`, `"Alert"` or `null`. Another token, an unknown object or a
  no-op is a 400; a body without `priority` or with an unknown field is refused
  by the JSON extractor (422). It returns the project tree like the flag endpoint.
- `EffectivePrioritySource` gains `userEdit`.

## Web

A *Your priority* select follows the effective-priority row in the
communication-object editor: *Not set by you* plus the three tokens, preselected
only from an effective value whose source is `userEdit`. Its help text says that
KNXBench does not program a changed priority and refuses the device while the
project priority differs from the product's. The captured-declaration row stays
read-only.

## Tests and counter-checks

- Core: set, clear, unchanged refusal, unknown object, undo/redo exactness,
  captured fact untouched, programming-state candidate.
- Store: save/load/reopen, invalid rows refused, v17 → v18 migration from the
  authentic predecessor, history decode of v17 images, legacy semantic identity.
- Image: statement mapping for each table row, refusal on differing/absent
  product literal, admission on equal `Low`/`Alert`, inactive ref ignored,
  byte-identical image without statements.
- Server/web: endpoint validation, effective source `userEdit`, select round
  trip; mutants for the edit precedence, the equality rule and the Empty/Unknown
  refusal.
