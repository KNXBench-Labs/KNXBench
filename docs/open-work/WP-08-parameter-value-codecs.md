# WP-08 — Parameter value codecs (Date, Colour, Picture)

**Status:** Blocked: evidence. Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is the [ledger](../status/LEDGER.md).

## Task

Separately admitted codecs and checked editors for `TypeDate`, Colour and
Picture parameter forms (PDB-04, PDB-05). One subpackage per exact type/form,
not one "support all widgets" task.

## Current state

Unverified forms keep their inert original-value fallback; `TypeDate` is
edited as free text. The packed `TypeTime` fallback is deliberate. The exact
Colour/Picture type definition found during research is behind a login.

## First deliverable

Documented lexical/native/resource encoding, width, invalid-value and
roundtrip vectors for one exact type/form.

## Done when

Independent vectors and malformed/boundary cases gate the codec, a
source-backed checked editor and all image consumers. A DPT bus codec is not a
product-parameter codec.

## References

[Admission contract](../contracts/parameter-editor-admission.md) ·
[ledger residue review](../status/2026-10-17-ledger-residue-review.md) ·
[product-database research](../research/product-database.md) ·
[history](../history/OPEN_WORK_2026-10-25.md#wp-08--parameter-value-codecs)
