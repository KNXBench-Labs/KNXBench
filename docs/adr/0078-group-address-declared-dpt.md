# ADR 0078: A group address keeps its declared DPT, and resolution reports it beside the linked objects instead of choosing silently

Date: 2026-10-05
Status: Accepted (implemented 2026-10-05, AR09)
Session: alpha-release-goal, AR09 (`KL-61`)

## Context

`resolve_group_address_dpt` (`crates/knx-core/src/dpt/resolve.rs`) infers a
group address's datapoint type only from the communication objects linked to
it, because `GroupAddressEntry` (`crates/knx-core/src/group.rs`) has no DPT
field. KNOWN_LIMITATIONS §61 records that ETS projects at schema 21 and later
can state the type on the group address itself. Today the importer keeps that
attribute as an opaque retained attribute (ADR-0006) and resolution ignores
it.

### E1 — what the schema says

`Project Schema23 v01.00.00.pdf` (KNX Standard v3.0.0, local `knx-spec-kb`),
§1.2.7 `GroupAddress_t`, page 59:

> `DatapointType` `knx:IDREF` optional — Optional datapoint type
> specification. A reference to DatapointType or DatapointSubtype. If the
> group address is linked to any DeviceCommunicationObjects, the sizes must
> match.

Two facts follow:

1. The group address states **at most one** reference (`IDREF`). The
   communication-object instance's attribute is `knx:IDREFS` (one or more,
   same document, line 2011 of the text extract).
2. The schema's only consistency rule is **size equality**. A group address
   declared 9.001 and linked to an object declared 9.004 satisfies the schema.
   A group address declared 1.001 and linked to a 1-octet object does not.

### E2 — what the fixtures carry

Measured on 2026-10-05 with a read-only scan:

- `KV v2.5 - demo.knxproj` (schema 21): **13 of 13** group addresses carry
  the attribute, using 5 distinct `DPST-*` values. All 13 are linked, by 26
  module-based `ComObjectInstanceRef` elements, and **none of the 26** states
  an instance DPT. Inside the project file the declaration is the only type
  for these addresses; a program-level DPT can only come from an installed
  product database.
- `Unser Zuhause` (schema 11 and the schema-23 re-export): **0 of 514**
  (unchanged from §61).

### E3 — what the store already holds

The attribute survives today as an `opaque_entry` row: `kind =
"RetainedAttribute"`, `name = "DatapointType"`, the exact value as `bytes`,
and an `xpath` keyed to the element instance,
`/KNX/Project/Installations/Installation/GroupAddresses/GroupRanges/GroupRange/GroupAddress[@Id='<ETS id>']`
(`crates/knx-etsproj/src/xpath.rs`, `group_address`, in use since
2026-09-20). The `group_address` table stores the same ETS id as
`source_ets_id`, so a row can be attributed exactly. ADR-0020 allows a
migration to re-derive what stored bytes determine, so these rows can be
lifted.

A store written by an import before the keyed xpaths existed holds the
generic path without the `[@Id=…]` predicate. Those rows cannot be
attributed to one address; pairing them by row order would be a guess.

*Corrected 2026-10-05, same day:* the first published text of this ADR said
no row carried element identity. That was read from the parser's generic
`real_path` and missed the re-keying in `map.rs`; the keyed form above is
what the importer actually stores.

## Decision

### D1 — model the declaration as an override, never as a guess

`GroupAddressEntry` gains `declared_dpt: Override<DptRef>` (ADR-0010):

| Source | Model |
| --- | --- |
| attribute absent | `Absent` |
| attribute `""` | `Empty` |
| exactly one `DPT-<main>` or `DPST-<main>-<sub>` | `Value(Resolved { layer: Instance })` |
| anything else (several tokens, unknown form, out-of-range number) | `Malformed(raw text)` plus one import report entry |

The value is never dropped or rewritten. A user edit later gets
`Layer::UserEdit`; inference never writes this field.

### D2 — resolution returns both sources and a named outcome

`resolve_group_address_dpt` stays as the inference-only function under its
current contract. A new function returns the declared value, the
`GroupAddressDpt` inferred from the linked objects, and exactly one outcome:

| Outcome | When | Effective DPT |
| --- | --- | --- |
| `Declared` | declaration `Value`; every linked object with a stated DPT has the same main-type width | the declared one |
| `DeclaredDiffersFromLinked` | as `Declared`, and at least one linked object states a different DPT of the same width | the declared one; the difference stays visible |
| `SizeConflict` | declaration `Value`, and at least one linked object states a DPT of a different width (the schema rule in E1 is broken) | **none**: decoding and writing refuse, both sides are shown |
| `Unverifiable` | declaration `Value`, and the width of either side cannot be stated (variable-length 24/28, a main type outside the codec) | the declared one, flagged as not size-checked |
| `DeclarationNotLifted` | declaration `Absent` in an installation whose migrated store still holds unlifted declarations (D3) | today's `GroupAddressDpt`, flagged as possibly incomplete |
| `Inferred` | declaration `Absent`, `Empty` or `Malformed` otherwise | today's `GroupAddressDpt`, unchanged |

Width comes from a per-main-type format width that the codec exposes for
this purpose (today it appears only in `WrongLength`), checked by AR09
against DPT-AS (`docs/spec-audits/2026-10-05-dpt-format-widths.md`). The
schema allows a same-width difference, so following the declaration there is
the standard's own rule, not an override. The outcome still names the
difference, so nothing is silent. A width difference is never resolved
towards either side.

### D3 — persistence and migration

- Store schema v10 adds the declaration to `group_address` with the same
  `state` / `value` / `layer` encoding as `com_object_override`. `Malformed`
  keeps its exact text.
- A new import takes the attribute into the model and no longer writes an
  opaque row for it, so the value is stored once.
- `migrate_v9_to_v10` lifts every `RetainedAttribute` / `DatapointType` row
  whose `xpath` is exactly the keyed path of a stored group address (E3)
  into that address's columns, parsing it as the importer does (`Value`,
  `Empty` or `Malformed` with the exact text), and deletes the lifted row in
  the same transaction. Rows without an exact match (the pre-keying form)
  stay untouched and are counted in
  `project_info.unlifted_group_address_dpt_declarations`. While that count
  is non-zero, an `Absent` declaration resolves as `DeclarationNotLifted`
  instead of `Inferred`: the project shows that unattributed declarations
  exist and that a re-import from the source file is needed to use them.
- Native save stays exact-or-refused (ADR-0074). A save/load roundtrip must
  return the identical `Override` in all four states.

### D4 — consumers

- Bus-monitor decoding and the CSV export's read-only `DatapointType` column
  use the effective DPT from D2. The CSV export writes nothing for
  `SizeConflict` and adds a warning, as it does for linked conflicts today.
- The projection's existing `dpts` field carries the effective type, so
  the web shows and fingerprints what the server decodes with, without a
  web change. Additional API fields for the declared value and the outcome
  would land in the generated web bindings, which belong to the current web
  lock holder; the AR09 implementation therefore does not add them and hands
  them, together with any display of the difference, to the UI owner.
  *Update 2026-10-06:* added by the Alpha owner under the Web lock as
  `GroupAddressNode.dpt_detail` (declared state and text, linked types, the
  outcome mirrored one to one) and shown in the group-address Inspector; the
  field is optional on the wire and `dpts` is unchanged.
- The CSV *import* still never applies `DatapointType`
  (IMPORT_EXPORT.md); changing that is out of scope.

## Consequences

- Schema-21+ projects that declare their types (KV demo: 13 of 13) get a DPT
  where inference alone found none.
- A project that breaks the schema's size rule is shown as a conflict instead
  of being decoded with either size.
- One store migration. Older KNXBench builds cannot open v10 files, as with
  every earlier schema step.
- Projects saved before v10 get their keyed declarations lifted by the
  migration. Only stores from imports before the keyed xpaths keep
  unattributed rows; the `DeclarationNotLifted` outcome says so.

## Test contract (before implementation is accepted)

Explicit, inferred, same-width divergence, width conflict, missing, empty,
malformed (several tokens, unknown form) and unsupported main types; the
import report entry for `Malformed`; the v9→v10 migration from a store with
keyed opaque rows (lifted in all three non-absent states, rows deleted)
and with unkeyed rows (left untouched, counted, `DeclarationNotLifted`
reported); native save/load equality for all
four `Override` states; codec refusal on `SizeConflict`; CSV export column
and warning.

## Alternatives rejected

- **Always prefer linked objects.** Leaves the 13 KV-demo addresses untyped
  and ignores the field the schema provides for this purpose.
- **Always prefer the declaration, without a width check.** Would decode a
  1-octet telegram with a 1-bit type when the project breaks the schema rule.
- **Keep the attribute opaque and read it at resolution time.** Puts XML
  parsing in the resolver and leaves user edits impossible.
- **Lift unkeyed opaque rows by row order.** Those rows carry no element
  identity (E3); a wrong pairing would assign a type to the wrong address.
