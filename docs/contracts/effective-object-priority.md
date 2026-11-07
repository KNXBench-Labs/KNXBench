# Read-only effective object priority

Status: implemented; acceptance is recorded in the
[delivery record](../status/2026-11-07-effective-object-priority.md).
Scope: one WP-06 increment under [ADR-0158](../adr/0158-object-priority-provenance.md)
§3 (project-effective priority). It builds on the
[captured saved-instance contract](saved-instance-priority.md) and the existing
product-query provenance (`ComObjectView.priority`, `priority_layer`).

## Outcome and exclusions

The device detail gains one required, read-only field per communication object,
`ComObjectNode.effective_priority`. It states which source wins for the
project's priority and whether that winning literal is one of the three schema
tokens. It is project metadata, not a wire priority, not a device readback and
not permission to program anything.

Not admitted: editing, a domain command, a web object-table column (an owner
UX decision), a documentation-report or comparison consumer, module objects,
other source schemas, product-version staleness checks, image generation, bus
services and any new endpoint. Existing image priority policies (Low/missing,
documented Alert inference, High refusal) are unchanged and do not read this field.

## Evidence

Project Schema23 v01.00.00 (SHA-256 `8fe9193e…21144`, identity in the
[source investigation](../research/2026-11-06-object-priority-provenance.md)),
printed page 48/64, original page rendered and visually inspected on
7 November 2026:

- §1.2.5.13, `Priority` row: optional; when missing, the attribute of the
  underlying ComObjectRef or ComObject is used. This covers **AttributeOmitted**.
- §1.2.5.11, `ComObjectInstanceRefs`: an object instance that has all default
  settings and no associations need not be listed. An object without an
  instance element therefore has default settings. Per §1.2.5.13 the default for
  `Priority` is the underlying ComObjectRef/ComObject attribute. This combination of two
  clauses on the same page covers **InstanceElementOmitted**. The step is an
  inference from the schema text, stated here explicitly; no real ETS export
  was executed for it.
- No inspected clause gives a value when both the ComObjectRef and ComObject
  omit `Priority`. The product schema's own defaults are not in the local
  corpus. Both layers absent therefore stay unresolved; no `Low` default is
  assumed.
- No inspected clause says that an empty or unrecognised instance literal
  falls back to the product. Such declarations stay unresolved (ADR-0158 §3).

## Resolution rule

Input is the **projected** saved view (so the current owner/source binding check
already applies), then, for omissions only, the product query.

| Saved view | Effective result |
|---|---|
| Declared, token Known | `resolved`, value = literal, source `instance` |
| Declared, token Empty | `unresolved` `InstanceEmpty`, source `instance`, exact raw |
| Declared, token Unknown | `unresolved` `InstanceUnknown`, source `instance`, exact raw |
| Unavailable (any reason incl. SourceBindingMismatch) | `unresolved` `InstanceUnavailable` |
| Absent (either omission), before product lookup | `unresolved` `ProductNotConsulted` |

Server overlay for `ProductNotConsulted` objects only, in the device-detail route,
with the existing `knx_productdb::query::com_object_views` batch already used for
function texts (lookup id from program id, object `RefId` and module scope):

| Product situation | Effective result |
|---|---|
| Device states no program reference | `unresolved` `NoProgramReference` |
| No product database open | `unresolved` `NoProductDatabase` |
| Program reference not in the database | `unresolved` `ProgramNotInDatabase` |
| No product view for the object's reference | `unresolved` `ProductObjectNotFound` |
| Neither ComObjectRef nor ComObject states Priority | `unresolved` `ProductLiteralAbsent` |
| Selected literal Known | `resolved`, value, source `product` + layer + omission |
| Selected literal empty | `unresolved` `ProductEmpty`, source `product` + layer + omission, raw |
| Selected literal unrecognised | `unresolved` `ProductUnknown`, source `product` + layer + omission, raw |

The layer is `ProgramRef` or `Program`, taken from `priority_layer` (presence
wins, never value comparison). A product literal without a layer violates the
query invariant and fails the request rather than guessing an origin.
A resolved instance declaration is never replaced by product data; the overlay
touches only `ProductNotConsulted` objects. Token classification is exact
(`Low`, `High`, `Alert`; no trimming or case folding), shared with the saved view.
Consumers that build a device detail without the server overlay (CLI, report)
see `ProductNotConsulted` for omissions; none of them renders the field today.

## JSON shape

```text
EffectivePriorityView =
  { status: "resolved", value: "Low"|"High"|"Alert", source }
  | { status: "unresolved", reason, source: source|null, raw: string|null }
source = { kind: "instance" }
  | { kind: "product", layer: "Program"|"ProgramRef",
      omission: "AttributeOmitted"|"InstanceElementOmitted" }
```

`raw` and `source` are present on unresolved results exactly when a winning
literal was selected (instance Empty/Unknown, product Empty/Unknown).

## Editor row

The communication-object editor shows one read-only **Effective project priority**
row directly below the saved-priority row: the value or the closed unresolved
reason, the winning source (saved object, or product ComObjectRef/ComObject plus
the omission that made the product apply), and a fixed hint that this is not
what the bus uses. Unresolved literals are quoted exactly. A status, reason,
value, source kind or layer this build does not know is shown as unknown, never
mapped to a default. No control, command or extra API call is added.

## Executable obligations

1. Projection: every saved state maps as in the first table, including binding
   mismatch, all three known tokens, empty, whitespace and case-different literals.
2. Overlay: each product situation in the second table, both layers, both
   omissions, a reference repeating the program's literal (layer follows
   presence), and a counter-proof that a declared instance value survives a
   different product literal.
3. Mounted GET device route against a real product database built by the
   production ingesters, plus the no-database and no-program-reference paths;
   the saved field stays byte-identical.
4. Web: all states, quoting, unknown-state fallback, German text, prototype-key
   safety; all typed fixtures updated explicitly, no optional or `any` shortcut.
