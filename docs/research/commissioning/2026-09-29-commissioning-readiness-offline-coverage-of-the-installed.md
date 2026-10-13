# 2026-09-29 — Commissioning readiness: offline coverage of the installed product corpus

[Research index](../commissioning.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="2026-09-29--commissioning-readiness-offline-coverage-of-the-installed-product-corpus"></a>
<!-- BEGIN preserved record-2-008 -->
## 2026-09-29 — Commissioning readiness: offline coverage of the installed product corpus

- **[V]** `knx products coverage --product-db <db>` evaluated the **103**
  locally available `.knxprod` packages installed into a fresh database:
  **246** application programs, **55** complete default/no-link memory plans
  (one program with a cited hardware run, 54 without), **191** refused with
  named reasons. Within `MV-0701` and `MV-0705` specifically, **55 of 181**
  planned (28/40 and 27/141 respectively); 126 were refused. Outside those
  masks, no program planned. Reasons across all 246 programs:
  `not-memory-mapped` 24, `procedure-style` 41, `unmodelled-step` 1,
  `parameter-evaluation` 59, `parameter-value` 33, `image-structure` 33.
  This is **not** a figure for all KNX devices: the denominator is the
  installed local sample, not the market. A different corpus or configured
  project can yield different counts. The earlier count of 203 `070n`
  applications was obtained under a different corpus/counting boundary and
  must not be substituted for this 181-program database run.
- **[V]** This evaluation is repeatable via
  `crates/knx-app/tests/download_coverage_corpus.rs` (ignored unless the
  private corpus is present). A plan only says the product's **default**
  image, no group links, and its load procedure are representable. It does
  not prove a user-configured image or hardware response.
- **[V]** `M-0083_A-0027-15-0BAC` has a cited live complete download
  (RESEARCH §19.4) and parameter-only partial download (§19.8) on one MDT
  device. Other plans are `untested` even if they share its mask or vendor;
  group-address partial has no hardware evidence. The implementation ships
  only these two scoped evidence records. See [ADR-0049](../../adr/0049-download-readiness-is-per-plan-and-backups-are-pre-write.md).
- **[V]** Pre-write region backup, file read-back, failure-before-mutation,
  restore, and CLI/HTTP support gates passed simulator/corpus tests.
- **[V] Live roundtrip on `1.1.67` (2026-09-29 15:17–15:26, after the user's
  "Hardware go").** Read-only dump before: 180 lines, identical to
  `post-k15` (option C, load states `01 01 01`). (1) `knx device download`
  of the K7 project wrote the backup JSON (1416 octets in 4 regions:
  `4000h`/1, `4003h`/510, `4201h`/511, `4400h`/394, all three machines
  `Loaded`) after the identity checks and **before** step 3 (the first
  write); 0 of the 1416 backed-up octets differ from the independent
  pre-run dump. Download 1416/1416 read back, restart unconfirmed as
  usual; the dump after ~45 s equals `post-k7-cli` (the K7 config).
  (2) `knx device restore <backup.json>` took its own pre-write backup
  (again 0 differing octets against the post-download dump), wrote
  1416/1416; after ~45 s the independent dump is **byte-identical to the
  pre-run dump in all 180 lines**, load states `01 01 01`. Proves on this
  one device and program that the backup is taken before the first write
  and restores the overwritten regions. The failure-before-mutation path
  stays simulator-only (not provoked on hardware). Logs:
  `OriginalData/DeviceBackups/1.1.67_MDT-0701_2026-09-29_{pre-backup-live,
  backup-live-download,post-backup-live-download,backup-live-restore,
  post-backup-live-restore}.txt` and the two JSON files under
  `1.1.67-prewrite/` and `1.1.67-prerestore/` (gitignored).
- **[V] Live roundtrip, partial download (parameters), 2026-09-29
  16:02–16:12.** Pre-dump = option C. (1) `--partial parameters` with the
  option C project: backup (1 region, `4400h`, 394 octets; `partial`
  recorded as parameters only; load state of the application `Loaded`) was
  written after the identity and all-`Loaded` checks (steps 1–6) and before
  step 7; 0 differing octets against the pre-dump; dump after the write
  unchanged; `knx device restore` of it wrote 394/394, dump unchanged
  (180/180). (2) The discriminating case: `--partial parameters` with the K7
  project: backup 394 octets, 0 differing against the device; after the
  write the dump equals `post-k7-cli` (3 dump lines changed, the K7
  parameters). `knx device restore` of that backup (own pre-write backup
  first) wrote 394/394; after ~40 s the dump is **byte-identical to the
  pre-dump in 180/180 lines**, load states `01 01 01`. Restart unconfirmed
  every time, as usual. Logs `…_{pre-backup-partial,backup-partial-*,
  post-backup-partial-*}.txt`, JSON under `1.1.67-prewrite-partial*/` and
  `1.1.67-prerestore-partial*/`.

---


<!-- END preserved record-2-008 -->
