# ADR 0012: Enrichment fills only `Override::Absent` slots

Date: 2026-09-03
Status: Accepted
Session: 4

## Context

ADR-0010 gave every overridable communication-object attribute its own
`Override<T>` with four states: `Absent`, `Empty`, `Value`, `Malformed`.
Measured against the reference project (ADR-0010): 497 of 907
`ComObjectInstanceRef` elements carry `DatapointType=""` — present,
explicitly cleared, not merely unstated — and 149 carry no `DatapointType`
attribute at all.

Now that the product database can resolve a communication object's
`Program`/`ProgramRef` values (ADR-0011), enrichment has to decide what
happens when an instance-level slot already holds something. The four
states are not equally safe to write into.

## Decision

`knx_productdb::enrich` writes a resolved value into a slot **only when
that slot is `Override::Absent`**. `Empty`, `Malformed` and
`Value(Resolved { layer: Layer::Instance, .. })` are left exactly as they
were.

The reason is concrete, not merely cautious: `Override::Empty` writes as an
empty attribute on export, and a bare (non-`is_exported()`) `Override::Value`
is skipped entirely by the writer. If enrichment overwrote an `Empty` slot
with a program value, the exporter would then write that attribute as
*absent* instead of *empty* — a file different from the one that was read,
for 497 communication objects in the reference project alone. Overwriting a
`Value(Instance)` slot would silently discard what the user's own project
file said, which CLAUDE.md forbids regardless of what the product database
believes the default should be.

The four tests in `crates/knx-productdb/src/enrich.rs` pin this down
directly: `an_absent_text_is_filled_at_the_program_layer` and
`an_absent_datapoint_type_is_filled_and_a_size_is_set` prove the positive
case; `an_empty_instance_attribute_is_never_overwritten` and
`an_instance_value_is_never_overwritten` prove the two states enrichment
must leave alone.

A second rule follows the same shape: nothing is guessed. `ComObjectRef/
@DatapointType` can hold a space-separated list of acceptable alternatives
(RESEARCH §4.2). When it does, enrichment fills nothing and records
`EnrichmentIssue::AmbiguousDpt`
(`a_datapoint_type_list_fills_nothing_and_is_reported`) — deciding between
two or more alternatives from one sample would be invented compatibility,
not resolved data.

## Alternatives considered

**Overwrite whatever is there.** Rejected for the reason above: it changes
the exported file for the 497 `Empty`-slot objects in the reference
project alone, and it discards genuine instance-level data the moment a
product database happens to be present — behavior that would depend on
whether enrichment ran at all, which is exactly the kind of
non-deterministic import CLAUDE.md's data-integrity rule rules out.

**Extend `Override<T>` into a full layer stack, so `Program` and
`Instance` values can coexist explicitly on the same attribute.** This is
the structurally cleaner answer and was considered seriously: it would let
an `Empty`-slot program value become visible without ever being confused
for the instance's own statement. Rejected for *this* session because it is
a domain-model change requiring a schema migration (`knx-core`'s
`CURRENT_SCHEMA_VERSION` and `knx-store`'s migration chain both bump) for
a gain — visibility of the value behind an already-cleared attribute — that
does not change what gets written back on export either way. Recorded
below as the deferred alternative, not silently dropped.

## Consequences

For an `Empty` slot, the corresponding program-layer value stays queryable
in the product database (`knx_productdb::query::com_object_view`) but is
never baked into the project model. A future UI can show "the program
default is X, this device clears it" by querying both sources side by
side; today's model cannot express that pairing on one field, and
KNOWN_LIMITATIONS §12 records this explicitly as what remains, with the
layer-stack extension above as its lift condition.

`ComObjectInstance.size` is not an `Override<T>` and is filled freely
(overwritten by a later, more specific value) rather than guarded by this
rule — its own doc comment already says it exists to be filled this way,
and it is never exported (`Layer::Program`/`ProgramRef` are excluded by
`is_exported()` regardless).
