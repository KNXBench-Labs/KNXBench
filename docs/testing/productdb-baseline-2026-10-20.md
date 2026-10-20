# ProductDB synthetic release baseline — 20 October 2026

## Scope and provenance

This records the actual WP-04.1 synthetic study, independently accepted without
findings. It is not a vendor-corpus benchmark, production capacity guarantee,
ETS compatibility statement or parser-optimisation result. The production
schema, parser, budgets and device admission were unchanged.

Measured source: `9834daff772a790be5a57c2e92d72aefd3b9b0a5`.
Source fingerprint: `98673b059ef842178354ecb2b008ad11af4b8267aa775f5c5babb49353b10cbf`.
Offline, locked release builds produced these authenticated executables:

| Executable | SHA256 |
| --- | --- |
| CLI | `596ead11ab3fd107485224b745c1355fc75f394f17ab2bb682d4989e3b421caa` |
| Init/resolve probe | `aaa37fb1c3713042d6e83f7d82d82d631162e2a29319761eda6f85bb78554857` |
| Native supervisor | `3c0df694360bd07d4d0a726cee8e8fccbe7ac5cdabc1749baf24d6c161acfd0b` |

Source and executable identities were unchanged through execution and independent
review. The [measurement method](productdb-measurement.md) defines the generated
fixtures, target observation boundary, supported SQLite layout, complete
accounting and refusal rules. Detailed measurements, source material, typed
streams, databases and raw process evidence remain outside Git.

The exact release supervisor passed five controls (calibration, cancellation,
TERM-ignore escalation, timeout and actual wait failure); the native-enabled
regression selection passed all 20 tests, without skips or ResourceWarnings.
Calibration target peaks for fresh/held-parent/freed-parent/heavy-target/after
were 2396/2368/2544/78872/2580 KiB. The separately recorded wrapper peak rose
from 18952 to 84568 KiB: wrapper memory was not credited to the target.

## Measured ladder

Three complete repetitions of ten cases produced 54 observations: 45 installs
and nine resolves. RSS is **absolute native-launched target peak in KiB**,
including the fresh native pre-exec floor, not a parser allocation delta.
Elapsed time is the external target launch/reap/checkpoint interval. OS cache
was uncontrolled: no cache flush or warmup. Builds, generation, setup and
read-only accounting are outside target timings.

| Fixture label | Decoded application bytes | Parameter rows | Peak RSS min / median / max KiB | Elapsed min / median / max ms | Final database bytes | Growth from blank bytes |
| --- | ---: | ---: | --- | --- | ---: | ---: |
| 1 MiB | 1049220 | 1592 | 17380 / 17500 / 17764 | 45.493 / 46.167 / 139.747 | 4874240 | 4329472 |
| 4 MiB | 4194780 | 6358 | 39752 / 39952 / 40008 | 145.845 / 168.931 / 283.490 | 17891328 | 17346560 |
| 16 MiB | 16777413 | 25376 | 109064 / 109172 / 109528 | 543.072 / 601.834 / 636.509 | 69918720 | 69373952 |

The largest final database retains 16779114 original-package BLOB bytes and
16778716 deduplicated decoded-source BLOB bytes. Normalized parameter text adds
13068640 logical UTF-8 bytes. Physical growth concentrates in `parameter`
(17367040 bytes), `package` (16793600), `source_file` (16793600) and the parameter
identity index (1638400). Its 12970 btree pages plus 4100 free pages reconcile
exactly at 4096 bytes/page. File growth therefore includes retained originals,
decoded sources, normalized data, indexes and free pages; it is not just live
parameter payload. This does not identify an internal parser allocation peak
or establish the benefit of a storage/parser replacement.

## Evidence, retries, rollback and resolver boundary

- Two distinct shared archives retain two package originals, six member
  references, three deduplicated decoded sources and 16 normalized parameters.
  Same-path retries preserve every typed row; an explicit alias adds exactly
  one package-source-name pair without duplicating package/source/evidence rows.
- Named `FutureMeasure` global keys/occurrences remain 1/16; the two packages
  retain 2/32 named report keys/occurrences. These are named subsets, not all
  global evidence: full global evidence has two rows. Repeated versus distinct
  fixtures have 1 versus 16 named keys at 16 occurrences, with two versus 17
  full evidence rows. Their common 585728-byte file size does not imply equal
  evidence content.
- The seeded late-wrapper refusal preserves complete logical state with no
  early/late package/program/source residue and zero file growth. Each of the
  three raw processes retains exit 1 and its matched diagnostic. The complete
  raw process receipt correctly stays unsuccessful; expected-refusal study
  acceptance is separate, not a rewritten successful process exit.
- Resolver sources of 11398 and 8384512 bytes produce exactly five expanded
  steps, no unplaced/omitted issues and `executable=false`; median target peaks
  are 6120 and 22544 KiB. The installed 8392704-byte source exceeds the separate
  8388608-byte resolver ceiling: its successful query reports `unavailable`,
  zero steps and exactly `source-byte-limit`, still non-executable. All nine
  resolve observations preserve the database. This is not an install refusal
  or evidence for changing the resolver limit.

## Independent checks and remaining boundaries

Independent review reconciled all 54 raw target observations, 108 before/after
accounting endpoints and 30 final read-only databases. Every endpoint includes
all 121 objects (47 tables including `sqlite_schema`, 74 indexes including
50 automatic indexes), even zero-growth objects. Rows, UTF-8 TEXT/BLOB totals,
retained-byte aggregates, evidence counters, deltas, pages and sidecars match.
Integrity and foreign-key checks pass; no unexplained pages/slack remain.

A separately implemented XML/SQL/typed-stream DEFAULT oracle verified all
54 observations, including normalized parameters/types/values/text, master
rows, original bytes, named evidence/report units, retry, alias and rollback.
Independent read-only re-execution returned identical output and empty stderr.
Its semantic verdict deliberately does not accept resource metrics or a
production workload baseline. The parent separately authenticated source,
build and native resource proofs. All 253 recorded process IDs were absent at
review. The review froze 29 artifact hashes and verified 4521 original files
unchanged; detailed evidence is private, not bundled with this public summary.

The first private controller attempt stopped before workload execution because
it omitted the requested native-test evidence parent directory. Five fixture
setup errors are retained, not called product regressions. The successor only
adds that directory and a fresh attempt output; reviewed sources and binaries
are unchanged. Private verifier projection mistakes are retained separately,
not treated as failed product measurements or silently overwritten.

This establishes only the generated ladder, AP1 boundary and evidence-preservation
baseline. It does not establish vendor-corpus/ETS compatibility, production-scale
capacity, controlled-cache timing, internal-phase RSS, transient disk/journal
peaks, desktop/bus/device behavior or an optimisation. WP-04.2 must inventory
resolver byte/node/depth/expansion bounds before deciding whether any parser
change is justified; WP-04.3 and WP-19 remain separate. The
[delivery entry](../IMPLEMENTATION_STATUS.md#wp-041-synthetic-measurement-baseline-20-october-2026)
records the separately passed maintained integration gate and published source;
these are not supplied by the study review.
