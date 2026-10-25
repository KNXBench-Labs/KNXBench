# Partial-download identity comparison: fixed five-octet width

Bounded safety fix of 2026-10-25 (branch `fix/partial-application-identity-20261025`),
on top of [hardware-identity-comparison](hardware-identity-comparison.md).

## Evidence

*Resources* (`03_05_01` v01.10.01):

- §4.2.13, printed pp. 33-34: `PID_PROGRAM_VERSION` (PID 13) is
  `PDT_GENERIC_05`, one five-octet identifier: manufacturer ID (two octets),
  device type (two), application program version (one). §4.2 holds the
  properties that do not depend on the object type.
- §4.20, Table 89 (printed p. 287) and Table 90 (p. 288): the Application
  Program Object lists Application Version as PID 13, `PDT_GENERIC_05`.
- §4.18.8.2, printed p. 275: the Application Program Object's object index is
  3 (stated there for the Easy Controller group object table).

The code previously cited "RES §4.19" for the Application Program Object table.
In v01.10.01 §4.19 is the Easy Parameter Block Table; the citation in
`properties.rs` now names §4.20 and §4.2.13.

## Contract

- `expected_response_octets(3, 13)` is `Some(5)` (`PROGRAM_VERSION_OCTETS`).
  `resource_matches` therefore requires exactly five octets from object index
  3, PID 13, before the generic `property_matches` rule. The hardware rule
  (object 0, PID 78, six octets) is unchanged.
- The helper takes no mask. Index 3 is the Application Program Object in the
  mask-`070nh` layout memory-download plans address; the width does not depend
  on the object (§4.2.13). PID 13 at any other index, and every other
  property, keeps the generic rule.
- Padded `InlineData` stays valid where the generic rule permits it; the
  `InlineData` length is never read as the answer's wire length.
- `derive_partial_plan` already emits the identity precondition with exactly
  five octets, taken from the complete plan's application task segment.
  `compare_with_plan` already required exactly five octets and is unchanged.

## Consumers

- `knx-net` memory download executor, `CompareProperty` step. In a partial plan
  the identity precondition is the first check before any device change: an
  answer of 0-4 or 6+ octets is a `PropertyMismatch` that carries the received
  octets, before the backup read, before the backup callback and before any
  memory or property write.
- A complete plan gains no identity precondition. An unbacked complete
  download reads no object 3, PID 13 unless the product's own
  `LdCtrlCompareProp` asks for it, and then the five-octet rule applies; a
  preview (`read_what_the_plan_overwrites`) and `compare_with_plan` without an
  explicit compare do not read it either. A backed-up complete run
  (`run_memory_download_with_backup`) performs exactly one additional
  PID 13 read before the first change, whether or not the plan has a matching
  compare; see [backup-application-identity](backup-application-identity.md).

## Tests

- Core: widths 0-4 and 6-8 refused against five-octet and padded data; five
  matching octets accepted; another application of five octets refused;
  no fixed width for PID 13 at indices 0, 1, 2, 4 and FFh or for PID 12 at
  index 3; PID 13 at indices 0 and 4 and PIDs 12 and 78 at index 3 compare
  exactly as the generic rule.
- Executor, simulator path, partial plan derived from a complete plan with the
  zero-tailed identity `00 83 27 00 00`: widths 0-4 and 6-8 refused at the
  plan's own precondition step with the received octets, no backup callback,
  no memory read or write, no property write. Before the fix the three-octet
  answer `00 83 27` ran the whole partial download. Five matching octets
  download; `00 83 27 00 01` is refused.
- Executor, complete plan with a product compare against eight padded octets:
  five octets download, four and six are refused (the generic rule alone admits
  both; six is the case unpadded data cannot show).
- Executor, unbacked complete plan without an identity compare: downloads to a
  device with a truncated or absent PID 13 and never reads it. (Historical
  claim of the original fix, which predates the backed-up identity read; it
  still holds for unbacked runs only. Backed-up runs are covered by the
  backup-application-identity contract.)

## Restore follow-up

The same five-octet identity is now also recorded in a backed-up run and required by
`restore_plan`; see [backup-application-identity](backup-application-identity.md). A legacy
partial backup (no recorded identity) still restores only through the precondition above, and
only when that exact compare sits before the plan's first change.

A product whose own `LdCtrlCompareProp` compares PID 13 against a padded identity (more than five
octets) is conservative on restore: `application_identity` does not accept a padded compare as a
single five-octet identity, so `restore_plan` refuses with `PlanApplicationUnknown`, even when the
backup recorded evidence of the application. The download itself is unaffected.

## Limits

- Tested only against the simulator. No device, bus or network access.
- Tests needing the private product corpus were not run.
