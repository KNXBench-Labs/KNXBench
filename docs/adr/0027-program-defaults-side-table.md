# ADR 0027: A program-defaults side table, not a layer stack on `Override<T>`

Date: 2026-09-20
Status: Accepted
Session: Goal-completion, Task 4 (§12 gap 2)

## Context

ADR-0012 recorded, and rejected for its own session, the "structurally
cleaner answer" to the same problem this ADR now closes: 497 of the
reference project's 907 `ComObjectInstanceRef` elements carry
`DatapointType=""` — `Override::Empty`, not `Absent` — and enrichment
correctly refuses to write into an `Empty` slot (overwriting it would
change what the exporter later produces). But the program's own value for
that attribute is not nothing; it is queryable
(`knx_productdb::query::com_object_view`) and, for 122 of the 497 dpt
slots and all 82 of the description ones, it resolves to exactly one
value. Re-measured this session against the same reference project, both
figures hold unchanged.

ADR-0012's deferred alternative was to grow `Override<T>` itself into a
layer stack, so `Program` and `Instance` could coexist explicitly on one
field. Revisiting it: `Override<T>` is matched exhaustively across the
whole workspace — the exporter (`is_exported()`), the parameter/flag
editors, every `EnrichmentIssue` site, `knx-diff`, the web DTOs. A fifth
variant, or a `Vec`-shaped replacement for `Value`, means every one of
those call sites has to be re-audited for whether it now silently does
the wrong thing with a coexisting pair — not a type error to catch, a
behavioral one to find by reading. That is a large, workspace-wide
surface for a feature that, per ADR-0012's own accounting, changes
nothing about what gets exported either way.

## Decision

`Devices` grows a second map, `program_defaults: BTreeMap<ComObjectInstanceId,
ProgramDefaults>`, keyed by the same id as `com_objects` but populated only
when `knx_productdb::enrich` finds a resolvable program value behind an
`Empty` `text`, `description` or `dpt` slot. `ProgramDefaults` holds
exactly those three optional `Resolved<T>` fields — nothing else, because
the flags and `size` were already correctly excluded from ADR-0012's gap
2 (an "empty" boolean flag is not a thing the source format expresses).

`ComObjectInstance`'s own `Override<T>` fields are untouched: an `Empty`
slot stays `Empty`, verified directly by a test that reads both the slot
and `program_defaults` in the same assertion. Every existing `Override<T>`
match site in the workspace continues to compile and behave exactly as
before, because the type itself did not change shape.

Persistence mirrors `com_object_override`'s own shape: a new table,
`com_object_program_default (com_object_instance_id, attr, value,
text_kind, layer)`, one row per populated attribute, written and read by
`knx-store::devices::{upsert_com_object_program_defaults,
load_all_program_defaults}`. This needs a migration —
`CURRENT_SCHEMA_VERSION` moves from 7 to **9** on this branch, not 8: a
concurrent task at merge time claimed 8 for the schema-≥21 export path.
`migrate_v7_to_v8` ships as a deliberately empty placeholder (mirroring
the existing v6→v7 precedent) so the coordinator's renumbering at merge is
a one-line change, not a re-derivation.

## Alternatives considered

**Extend `Override<T>` into a layer stack (ADR-0012's original deferred
alternative).** Rejected again, for the reason above: workspace-wide
blast radius for zero change in export behavior. If a future session
needs `Program` and `Instance` values to compose (not just coexist
read-only) — e.g. a UI that lets a user promote a program default into an
explicit instance override with one click — that composition can read
`program_defaults` and write `Override::Value` through the existing
`fill_absent`-shaped path; nothing here forecloses it.

**Store the lifted value back inside `Override::Value`, tagged with a new
`Layer::ProgramDefault` distinct from `Layer::Program`.** Rejected: this
is indistinguishable from writing into the slot, which is exactly the
behavior ADR-0012 forbids — `Layer::is_exported()` would have to special-case
a `Value` that must never export, which is a landmine for the next
`Override<T>` match written without reading this ADR first.

**Do nothing; leave the value queryable only via `com_object_view`.**
Rejected: 122 + 82 = 204 attributes across a single reference project are
not a corner case, and a caller wanting "what would this object show if it
had no override" already has to re-resolve the program and reconstruct the
lookup id itself, duplicating logic `enrich()` already has.

## Consequences

`Devices` now owns two maps instead of one for communication-object data;
`remove_com_object` must (and does) clean up both. That cleanup is pinned
by `devices::tests::removing_a_com_object_takes_its_program_defaults_with_it`,
which fails if the cleanup line is removed. Because the map lives beside
`ComObjectInstance` rather than inside it, `Command::DeleteDevice` also has
to carry the entries it destroys into its inverse explicitly
(`Command::CreateDevice::program_defaults`), or an undo would hand back a
device stripped of everything enrichment lifted for it.

`program_defaults` is additive and inert by default — an already-persisted
project with no `com_object_program_default` rows behaves exactly as
before, and a project with none needed writes none.

Nothing here surfaces a program default in `apps/knx-web` yet; that is a
UI decision (which field, which affordance, whether "empty" and "program
says X" render differently from "absent"), deliberately left for whoever
builds that screen. `KNOWN_LIMITATIONS.md` §12 is updated to record gap 2
as closed at the model/persistence layer, with the UI question open.
