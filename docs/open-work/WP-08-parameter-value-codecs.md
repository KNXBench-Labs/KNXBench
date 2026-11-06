# WP-08 — Parameter value codecs (Date, Colour, Picture)

**Status:** Blocked: evidence. Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is the [ledger](../status/LEDGER.md).

## Task

Separately admitted codecs and checked editors for `TypeDate`, Colour and
Picture parameter forms (PDB-04, PDB-05). One subpackage per exact type/form,
not one "support all widgets" task.

## Current state

Unverified forms keep their inert original-value fallback. The packed
`TypeTime` fallback is deliberate. The exact Colour/Picture type definition
found during research is behind a login.

**`TypeDate`, lexical slice (28 October 2026):** stored as kind `Date`
(product database v29, data-only migration that reproduces a fresh install);
edits must be Schema23's `yyyy-mm-dd` naming an existing Gregorian day. The
text field stays; the download refuses a placed date by name. The memory
encoding of `Encoding="DPT 11"` is **not** established: a collection default
lies before the DPT 11 year range, so DPT 11 octets are not assumed
([evidence](../research/2026-10-28-typedate-value-form.md)). The type's
native/image codec and Colour/Picture stay open. The bounded
[checked Date text editor](../contracts/checked-date-text-editor.md) adds only
source-authenticated project-value preflight and explicit Apply; no native date
picker, memory encoding or device compatibility follows.
Sources whose affected evidence differs between targeted and extended-package
modes are not promoted: they retain `Other`, original evidence and counter
contributions, with explicit backfill diagnostics and unavailable reports.

Related historical follow-up: the unchanged v15 Color/Time backfill has a
same-blob duplicate-program first-winner hole. The [authentic upgrade witness](../status/2026-11-03-wp08-authentic-color-time-upgrade.md)
recorded on 3 November 2026 installs fictional bytes with actual v14 code,
then opens its database through the current v31 chain. Two winner type rows
incorrectly inherit the losing declaration's Color/Time metadata; a fresh
current install keeps the first declaration authoritative. The unique-program
control and an additional reopen are recorded. This closes the authentic
pre-v15 reproduction prerequisite only, not the defect's acceptance.

**Bounded successor delivered on main (3 November 2026):** the new v32 migration
repairs source-authenticated, exact v15-bad Color/Time tuples back to the
first declaration's `Other`/`Date`, including NULL Time fields. It does not
rewrite original history or released migrations. Registered semantic RED/GREEN,
focused ProductDB suites and corrected-source real-v14/v31/fresh/reopen copies
were accepted after independent review, ten semantic mutant kills and complete
branch/main-first 14-stage gates. Published as `5c405bfde`; see the
[acceptance record](../status/2026-11-03-wp08-color-time-successor.md) and
[repair contract](../contracts/color-time-successor-repair.md).
Date/native/image and Colour/Picture codec evidence remains unchanged.

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
