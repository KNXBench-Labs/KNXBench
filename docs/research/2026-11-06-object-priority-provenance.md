# Object-priority provenance — 6 November 2026

## Question and bounded admission

WP-06 requires an ADR before modelling raw and effective object priority.
The first increment carries the origin of an existing **product-query literal**;
it does not add project priority, a priority editor or a new wire encoding.

## Primary source actually inspected

`Project Schema23 v01.00.00.pdf`, SHA-256
`8fe9193e701350643be5527644955d23b6933c1c4cc66264a3bb234301c21144`,
was read from the authorised local specification corpus. Text was extracted with
`pdftotext -layout`; original PDF pages 7 and 48 were rendered with `pdftoppm`
and visually inspected. Their printed page numbers are respectively 7/64 and
48/64. Licensed extracts and renders remain private.

- Section 1.1.2.4, printed page 7, enumerates `Low`, `High` and `Alert` for
  `ComObjectPriority_t`. It does not give protocol bit patterns.
- Sections 1.2.5.11–13, printed page 48, describe project object-instance
  references and an optional `Priority`. An omitted priority uses the underlying
  product reference/object attribute. The inspected passage does not establish
  how an invalid or empty priority should execute, nor an on-wire encoding.
- These are schema-23 project rules. They do not independently establish every
  manufacturer-package schema, legacy project schema or module binding.

### Addendum, 7 November 2026: omitted instance elements

The same original PDF (same SHA-256) was reopened and printed page 48/64 was
rendered and visually inspected again for the effective-priority increment.
§1.2.5.11 describes `ComObjectInstanceRefs` and states that an object instance
with all default settings and no associations need not be listed. Assuming the
writer omits instances only under that rule, an unlisted instance carries default
settings, and §1.2.5.13 defines the default
for a missing `Priority` as the underlying ComObjectRef/ComObject attribute.
KNXBench applies the product literal to `InstanceElementOmitted` on that
two-clause basis, which is an explicit inference from the schema text. No real
ETS export was executed for this. The Project Schema document contains no
`Priority` occurrence outside pages 7 and 48 (case-sensitive whole-document
text search). The product-schema default for a ComObject without `Priority` is not in the local
corpus, so a product with neither layer declaring it stays unresolved.

## Existing implementation, not a new format claim

At base `87642b851`, `knx-productdb::parse::comobject` stores separate nullable
`priority` columns on `com_object` and `com_object_ref`. The batch query
`com_object_views` joins each reference within its program and selects
`raw.cor_priority.or(raw.co_priority)`. Present empty, whitespace and unknown
literals therefore win just as any other present reference literal does.
The single-object query delegates to that batch query.

`ComObjectView.priority` carries the selected text but not its origin, unlike
text, size and flag fields nearby. This is the concrete missing metadata in the
admitted increment. Source selection remains an existing KNXBench query policy;
this investigation does not promote that implementation into new format proof.

`knx-core::ComObjectInstance` has no priority property. The project source
parser does not take `Priority` into a typed instance slot. Existing generic
attribute retention already preserves unmapped instance declarations: both
mapping paths re-key `other` attributes to the owning object locator, and the
importer emits hashed `RetainedAttribute` opaque entries. Raw retention is not
new work; typed promotion and its source/ownership proof remain open. This is
a source-path inspection, not fresh project-import or save/reopen execution.
Product image
construction already has a separate policy: `Low`/missing, inferred `Alert`
and unsupported other values. In particular, `High` stays refused. None of
those writer choices is changed or independently revalidated by these pages.

## Decision and executable obligation

[ADR-0158](../adr/0158-object-priority-provenance.md) separates source literals,
product-query selection, project-effective priority and wire priority. Add only
optional selected-product-layer provenance to `ComObjectView`: a present value
identifies Program or ProgramRef; no value has no asserted origin. Never use
presence of that metadata as permission to program or as proof of a valid token.

Required tests must exercise the production importer and query, not only a
selector helper: missing values, each winning layer, equal values in both
layers, explicit empty, whitespace, unknown/case-different tokens, single/batch
agreement and close/reopen of an isolated catalogue. Equal values are important:
the layer must follow presence, not value comparison. Existing image tests keep
the old High refusal and Alert-inference boundary. Execution and publication
results belong in the acceptance record; this source admission alone is not
software acceptance.

## Still open

Typed promotion of already-retained project-instance declarations and provenance-preserving migrations,
core/model defaults, user-edit commands with undo, parameter-object refresh,
API/UI/report/compare consumers and real-export witnesses remain separate work.
No new bus, device, ETS, native UI, release or deployment action is admitted.
