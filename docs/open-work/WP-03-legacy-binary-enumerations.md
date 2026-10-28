# WP-03 — Legacy binary enumeration values

**Status:** Implemented in source (28 October 2026), bounded to validated
long-enum storage and retained-source backfill. Broader KL-128 boundaries stay
in the [ledger](../status/LEDGER.md).

## Delivered scope

Legacy EX-IM atomic type 5 now stores exact binary bytes as canonical Base64 in
`parameter_type_enum.binary_value`, the column used by `.knxprod` since v25.
Strict hex, byte-length and type-width checks preserve leading zeros;
`REAL_VALUE`, identity, text and display order remain separate and unchanged.
Invalid binary values remain NULL with a diagnostic and retained source.

Product database v30 also backfills existing v29 legacy data from authenticated
retained payloads. It checks source hashes, namespace, program owners, type shape
and enum keys/values/text/order before filling NULL columns. Ambiguity or a
conflicting non-NULL value refuses the whole payload's backfill with an explicit
finding, without changing historical reports or diagnostics. Fresh import,
upgrade, reopen, skipped re-ingest, idempotence and refusal regressions are in
`knx-productdb/tests/it/legacy_binary.rs`.

The independently reviewed [mapping evidence](../research/2026-10-28-legacy-enum-binary-mapping.md)
closes the former prerequisite for this bounded representation rule. It does
not prove conversion equivalence for the seven unpaired types or all legacy
files. Ordinary atomic type 4 binary/length fields remain uninterpreted and
explicitly diagnosed, rather than copied into the XML-shaped column.

## Remaining boundaries (not a new programming permission)

- Missing, corrupt or ambiguous retained data is not reconstructed. A skipped
  re-ingest does not retry a previously refused backfill.
- Large-payload migration memory/time has not been benchmarked.
- No live legacy download, PR/VD2 import, broad ETS-equivalence or hardware
  acceptance is claimed. Existing source-bound image guards remain unchanged.

The earlier G16 diagnostic slice (`b13b0ff9`) is unchanged: the `MissingMaster`
refusal names the missing master and CLI usage names `.vd1` for inspection only.

## References

[KL-128](../limitations/KL-128-legacy-product-import-boundaries.md) ·
[ADR-0094](../adr/0094-legacy-exim-product-files.md) ·
[ADR-0141](../adr/0141-placements-and-enumeration-attributes-reach-columns.md) ·
[history](../history/OPEN_WORK_2026-10-25.md#wp-03--legacy-binary-enumeration-values)
