# Large product packages: measured profile (KL-151)

Status: measurement of 2026-10-05 for [ADR-0082](../adr/0082-large-product-packages-are-a-cli-opt-in.md).
Built from `origin/main` `942d2680` plus the uncommitted ADR-0082 change, release
profile (`cargo build --release -p knx-cli`), run in a network-less namespace.

## Input

The 853 public manufacturer downloads of the crawler run (2026-10-03,
[corpus run](../PRODUCT_DATABASE_CORPUS.md#public-crawler-corpus-run-2026-10-03)),
read in place, never copied or committed. A ZIP central-directory scan (no
extraction) selects the 15 files whose declared sizes exceed the standard
profile: 7 by member only, 5 by total only, 3 by both; one file is not a ZIP.
No file exceeds the 256 MiB raw bound. This matches the research selection of
2026-10-03 (15 files, same split).

## Method

Each selected file, ordered by content SHA-256, is installed twice with the
same binary, each time into a fresh database: `knx products ingest <file>`
(standard) and `knx products ingest <file> --allow-large-package` (large).
The child's peak RSS comes from `wait4`, the time from a monotonic clock, the
database size from all `products.sqlite*` files; an install is followed by
`knx products verify`. Twenty standard-sized packages of supported schemes
(first 20 by content hash) are run the same way as a control. All originals
are rehashed afterwards. Rows are anonymised by index; no names, ids or
values are recorded.

## Result

| # | member > 64 MiB | total > 256 MiB | standard | large | s | peak RSS MiB | DB GiB | verify |
|---|---|---|---|---|---|---|---|---|
| 0 | ✓ | ✓ | size limit | installed | 256.03 | 760.5 | 7.18 | ok |
| 1 | ✓ | ✓ | size limit | installed | 90.52 | 401.6 | 2.51 | ok |
| 2 | ✓ |  | size limit | installed | 11.21 | 463.9 | 0.31 | ok |
| 3 |  | ✓ | size limit | installed | 57.83 | 347.3 | 1.64 | ok |
| 4 | ✓ |  | size limit | installed | 12.45 | 529.5 | 0.36 | ok |
| 5 | ✓ |  | size limit | installed | 12.32 | 579.2 | 0.36 | ok |
| 6 | ✓ |  | size limit | installed | 6.14 | 289.7 | 0.17 | ok |
| 7 | ✓ |  | size limit | installed | 12.32 | 560.0 | 0.33 | ok |
| 8 |  | ✓ | size limit | installed | 28.91 | 145.2 | 0.84 | ok |
| 9 | ✓ |  | size limit | installed | 6.2 | 300.8 | 0.19 | ok |
| 10 | ✓ | ✓ | size limit | installed | 156.27 | 725.1 | 4.45 | ok |
| 11 |  | ✓ | size limit | installed | 27.71 | 145.4 | 0.75 | ok |
| 12 |  | ✓ | size limit | installed | 50.41 | 346.0 | 1.47 | ok |
| 13 | ✓ |  | size limit | installed | 5.6 | 277.9 | 0.17 | ok |
| 14 |  | ✓ | size limit | namespace refusal (§153) | 0.06 | 45.9 | 0.0 | — |

- Standard: 15 / 15 refused with the typed size limit; every refusal prints
  the `--allow-large-package` hint.
- Large: 14 installed, all pass `knx products verify`; 1 still refused by
  its scheme-10 namespace (§153), atomically. Siemens' complete bundle is row
  1 (91 s, 402 MiB, 2.5 GiB).
- Maxima: 760.5 MiB peak RSS, 256 s, 7.18 GiB database; 734 s for all 15.
- Control: 20 / 20 installed under both profiles with byte-identical summary
  lines.
- Originals: 35 / 35 rehashed unchanged.

## Whole-corpus effect

The last full 853-file run (ADR-0072 receipt) installed 690. The flag changes
only the two expansion bounds, so with it the 14 packages above install and
nothing else changes: 704 of 853. This total is derived from that run plus
this measurement; it is not a new full 853-file run.
