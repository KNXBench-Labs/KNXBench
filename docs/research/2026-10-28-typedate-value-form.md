# `TypeDate` value form (WP-08) — 28 October 2026

Evidence for the first WP-08 subpackage: one exact parameter type, `TypeDate`.
It records what is documented, what the local collection shows and what stays
unestablished. No device, bus or ETS was used.

## Sources

- **Project Schema23 v01.00.00**, §1.1.3.19 `simpleType Value_t`, printed page
  31: the value-encoding table spells a `TypeDate` value `yyyy-mm-dd`. It is the
  only `TypeDate` occurrence in that document; the schema PDF does not describe
  the `TypeDate` element, its attributes or a memory layout. `[D]`
- **03_07_02 Datapoint Types v02.02.01**, §3.12 "Datapoint Type Date", printed
  page 41: DPT 11.001 `DPT_Date` is three octets (`r3N5 r4N4 r1U7`: day 1…31,
  month 1…12, year 0…99), with the century rule that a year octet ≥ 90 means
  the 20th century and < 90 the 21st, i.e. 1990…2089. `[D]` This is a bus
  datapoint format. Nothing in either document says that a product parameter
  declaring `Encoding="DPT 11"` stores these octets, or in which order, at its
  `Memory` placement.

## Local collection (read-only, aggregate)

A read-only scan of every local product archive found exactly one `TypeDate`
shape, `<TypeDate Encoding="DPT 11" />`, in two application programs of one
manufacturer database (the census addendum's two declarations). Parameters of
that type carry hidden (`Access="None"`) defaults in `yyyy-mm-dd` form; at least
one default lies **before 1990**, outside the DPT 11 year range. `[V]`

So the stored value is not limited to what DPT 11 can carry, or that default is
a sentinel whose device meaning is unknown. Either way, a DPT 11 range check on
the stored value would be an invented rule. The hidden default alone does not
establish whether that value reaches device memory or how it would be encoded.

## Decision

- **Model:** `TypeDate` becomes parameter kind `Date` (product database v29,
  data-only migration). `Encoding` and every other `TypeDate` attribute stay
  reported as unmodelled, with samples, like `TypeFloat/@Encoding`. For an
  extended-package source, promotion first compares affected keys, occurrences
  and samples under targeted and package-mode replay. If they differ, historical
  operation ownership is unavailable: the source savepoint is rolled back,
  `Other` and all original evidence remain, `ParameterKindBackfillError` names
  the refusal, package-counter contributions stay unchanged and affected
  measured reports become unavailable. Other sources continue independently.
- **Edit validation:** a `Date` value must be exactly `yyyy-mm-dd` (four, two
  and two ASCII digits, two hyphens) naming a day that exists in the Gregorian
  calendar. No sign, time, zone, whitespace or other spelling; no year range.
  The calendar check is the conservative reading of "a date"; the schema text
  gives only the spelling.
- **Download image:** unchanged refusal by name. A placed `Date` parameter is
  refused for its type; no DPT 11 octets are written.
- **Web editor:** unchanged plain text field; the server's validation is the
  authority. No date picker: its WebKitGTK behaviour is unverified (WP-11).

### Why the mode check is necessary

An independent genuine-v28 witness installed identical project/21 (also /23)
program bytes in a scheme-14 package and an extended package. A deciding
`TypeDate` declaring `xmlns:y="urn:example"` has different raw/expanded attribute
evidence in the two ingest modes. Reusing a global package-mode delta omitted
the targeted namespace row and left its report falsely measured (1 instead of
2 distinct/encountered rows). The retained global ledger cannot authenticate
absence of an earlier targeted ingest, so even one extended package is not
proof that its mode was the only mode. The bounded refusal above avoids guessing
history; mode-invariant plain dates remain admitted. Regression tests cover
both schemes, unchanged source/counters, report downgrade, reopen and an
independent source continuing. No change to the released v15 Color/Time pass.

## Still open

The memory encoding of a `TypeDate` parameter (octet order, the handling of
years outside 1990…2089, `SizeInBit`/placement width) needs an independent
source or a discriminating product image before any image consumer may write
one. Colour and Picture forms remain without a documented value encoding.

## Verification

- `knx-productdb` `date_kind` integration tests: fresh install, v28→v29
  migration equal to a fresh install (one and two packages sharing a blob),
  damaged blob, a surviving row whose sample changes (report downgraded), a
  repeated program id (first declaration keeps authority, both contribute
  unknown evidence), inconsistent measured reports (header, missing count row,
  wrong count), an ambiguous cross-member sample and a no-op database. A report
  gaining a row with a conflicting historical sample is downgraded instead of
  assigning the merged sample to a member without evidence. The hand-written v28 rewinds were compared once with a
  database written by the actual v28 parser (`origin/main` at `7e8c4a0ed`):
  identical snapshots for both fixtures.
- Server `date_accepts_only_schema_spelled_calendar_dates`, image
  `a_date_parameter_is_refused_for_its_type`, `parameter_kinds` additions.
- Seven deliberate source mutants (migration unregistered, package counter or
  report deltas dropped, backfill owning no child, parser keeping `Other`, no
  calendar check, sample-change downgrade ignored) each fail at least one test.
