# KNXBench Alpha — independent re-check, round 4 (N11–N13)

- **Reviewer:** a fresh Codex session in Hermes, not the release owner. I
  implemented none of the fixes and used no subagents. Product code remained
  read-only throughout.
- **Date:** 2026-10-07, starting at 06:21 CEST.
- **Brief:** [AR18_RECHECK_BRIEF, Round 4](AR18_RECHECK_BRIEF.md#round-4-n11n13),
  including the original brief's offline, lease, private-corpus and read-only
  rules. This is the narrow fourth re-check, not a new whole-product audit.
- **Candidate:** freshly fetched `origin/main`,
  `b8724d66acad05c30328ef54fd61cdd799e87cea`.
- **Product sources:** identical to
  `2254eed002e1c8a0988cd5bab89910559371342e` in `apps/`, `crates/`,
  `Cargo.toml` and `Cargo.lock` (verified by Git).
- **Artifact:** a copy of the brief's AppImage; SHA-256 recomputed as
  `138444b4cf7f5664ce2f64e88b82574f88dcc5b382dca7f2e49ff1eae7d6c3fc`,
  107,870,712 bytes. The packaged server reports
  `0.1.0-alpha.1+g2254eed0`; the newly built server reports
  `0.1.0-alpha.1+gb8724d66`.

**Candidate-binding note.** The brief's literal “only documentation and
handover files” description is no longer exact after the repository-cleanup
revert: the diff from `2254eed0` has 263 paths, including restored root notes,
`.claude/settings.json`, `.gitignore`, and agent-memory/cloud helper tooling.
There is no product-source or Cargo-manifest change. I gated the actual
`b8724d66` checkout, not merely the old code pin.

## 1. Verdict

**`READY`**

**R6 and R7 are met. No further mandatory condition from this round blocks
AR19.** The release owner can record the AR18 outcome; the release/tag
decision remains the user's. N14 is a nonblocking MINOR follow-up, not a
condition disguised as a ready verdict.

The direct round-4 probes are complete: N11, N12 and N13 are fixed against
my inputs. No new CRITICAL or IMPORTANT finding was reproduced. One new
MINOR observation is recorded below, without a release condition.

## 2. Findings against their fixes

“Ran” means my own fictional archives or the actual writer's output against
newly built release binaries, the standalone HTTP server, and the copied
AppImage's server. The owner's 35 tests and mutation sweep were not substituted
for these inputs.

| Finding | Result | Evidence |
| --- | --- | --- |
| **N11 / R6: local bytes hidden by central zero sizes; forged `0.xml`** | **fixed** | **Ran:** the real `P-0001/0.xml` hidden by central CRC and sizes of zero, renamed with Unicode Path to a directory, a trailing-`\` directory or an ordinary file, plus a forged replacement. Both archive layers, with and without bit 3, with Unicode Path only centrally or in both headers. Every variant refused; none imported the forgery or created a project store |
| **N11: disagreements in individual header fields** | **fixed** | **Ran:** local name, flags, method, CRC, compressed size and uncompressed size independently changed, with and without bit 3, outer and ZipCrypto nested. Local Zip64 sizes independently changed. Additional central Zip64 size disagreements also refused |
| **N11: data descriptors** | **fixed** | **Ran:** each descriptor value changed independently, signed/unsigned, 32-/64-bit sizes, both layers; missing, truncated, displaced descriptors and padding before/after them refused. Additional mixed local zero/nonzero tuples and descriptor values with nonzero high 32 bits refused. Valid zero tuples and full central-value tuples, with every signature/width combination, import |
| **N11: Unicode Path** | **fixed** | **Ran:** local-only, central-only, different local/central values and unequal multiplicities refused in both layers. Equal fields accepted |
| **N11: records must tile to the central directory** | **fixed** | **Ran:** gaps between records and before the central directory, an unreferenced middle local record, an early declared record end, overlaps and aliased offsets refused. An additional overlap deliberately has a *valid* embedded local header matching its central name/metadata: it reaches the extent check and fails with `does not end where the next record begins`, in both layers |
| **N12: deflated empty directories** | **fixed** | **Ran:** stored and deflated empty directory controls import, outer and nested. A one-byte directory, a deflate stream with trailing bytes, raw stored `03 00`, a positive declared size without output, and a trailing-`\` directory/file clash are refused. Actual Java `jar` output imports too. Empty-directory checksum/stream edge cases remain → N14 |
| **N13: stale HTTP status documentation** | **fixed** | **Read and ran:** `docs/ALPHA_CANDIDATE.md:52-69,116-121` distinguishes the historical smoke from today's statuses. In both servers: absolute missing Open → `422 projectNotOpenable`; relative missing Open → `400` (both Open probes use `discardChanges: true`); missing Import → `422 projectNotImportable`. Over an actual unsaved area rename: Import without discard → `409 projectUnsavedChanges`; with discard, the bad file is refused with `422` and the edit and modified flag survive |
| **R7: re-gate and rebuilt AppImage** | **met** | All 24 gate stages have their expected exit; selected corpus **143/143 in 31 targets**, `check-appimage` exit 0. The artifact hash and `2254eed0` stamp match; native offline start and all direct artifact probes pass. The sources remained frozen |

### Independent matrix and controls

The main manifest contains **181 distinct fictional fixtures**:

- **142 refusal cases:** CLI exit 1, no new project store. The standalone
  server and packaged server each return `422 projectNotImportable` for
  every case.
- **33 acceptance controls:** CLI exit 0, the genuine sample device remains,
  no forged device. Both servers return 200 and show the genuine sample.
- **6 exploratory cases:** empty-directory CRC/stream checks (N14) and
  trailing compressed bytes (§3.2). These are observations, not successful
  negative controls.
- **16 additional CLI cases:** all refused, with the expected guard message
  and no project store. These exercise central Zip64 values, a real
  overlapping header, mixed local descriptor tuples and wide descriptor
  high bits. They are separate from the 181-fixture three-surface matrix.
- All main CLI input files remained SHA-256-identical after the probes.

**Real writers, not hand-written imitations:** Info-ZIP 3.0, unencrypted and
ZipCrypto encrypted (the encrypted project files are placed in an ETS-shaped
nested payload); Python 3.14.7 `zipfile`, seekable/streaming with and without
`force_zip64`; Java `jar` 26.0.2.1. All seven outputs import, through CLI and
both HTTP surfaces. The Info-ZIP encrypted control independently confirms
why accepting full local values as well as zeros with bit 3 is necessary;
matching descriptors are still required.

**Products-only route:** all **94 outer** fixtures were also exercised through
`knx products ingest`: 71 refused, 20 valid controls ingested, 3 exploratory
inputs ingested. No refused case created the product database. The 87
protected nested fixtures are refused for lack of a password, including the
13 valid nested controls. This is **not** inner-layout guard evidence:
`apps/knx-cli/src/main.rs:1691` calls `import_knxproj`, without password
support on this command. The initial probe supplied `--password-stdin`,
which this path does not consume; after tracing the source I classified the
results as this existing boundary, not 13 compatibility regressions. Nested
structural checks are independently exercised by project import and both
HTTP surfaces with the synthetic password supplied.

## 3. Newly observed behavior

### 3.1 N14 — MINOR: empty directories bypass payload checksum/stream validation

**Where:** `crates/knx-etsproj/src/container/local_record.rs:132-151`, and
`crates/knx-etsproj/src/container.rs:858-860,882-885,569-570`.

**Ran, outer and nested:**

1. An empty deflated directory, compressed bytes `03 00`, with CRC **123456**
   in both headers. The local and central records agree, but the CRC of the
   empty output is zero. CLI exits 0; both HTTP surfaces return 200; no
   directory-specific diagnostic is emitted. `unzip -t` on the outer input
   exits 2 and identifies the directory's bad CRC.
2. A directory with method 8, zero declared size, zero compressed size and
   **no deflate stream**. The early `data.is_empty()` return accepts it, without
   asking `inflates_to_nothing`. CLI and both servers accept it; `unzip -t`
   on the outer input exits 2 with invalid compressed data.

This does **not** bypass the local/central agreement check: those fields
really agree. It is a missing validation of the empty directory's actual
payload, not the former N11 substitution attack. No useful directory
content is lost and no genuine project member is replaced in these probes.
The zero-byte/method-8 shape is also a reader-tolerance difference, not a
claim here about ETS behavior. Optional checksum/stream hardening or a
precise disclosure would improve diagnostic consistency. It does **not**
block AR19 on the evidence reproduced in this round.

### 3.2 Observations, not further release conditions

- A complete deflate stream followed by extra bytes *inside its declared
  compressed range* is accepted for an ordinary file, outer and nested.
  My extra bytes contain a local-record-shaped blob. No forged device is
  selected. Seekable and streaming `bsdtar`, and `unzip -t`, agree with the
  outer archive's central inventory. I did not establish a second-project
  interpretation or a missing declared member; this is not a new N11.
- A 4 KiB self-extractor-style prefix still imports. The accepted prefix
  exception, including unreferenced initial records, is disclosed in
  [KNOWN_LIMITATIONS §159](../../../KNOWN_LIMITATIONS.md#159-a-project-archive-may-unpack-to-at-most-512-mib-and-every-member-name-must-be-unique).
- N6 (welcome page after reload over a held server project) has no source
  change in this candidate. I did not repeat the interactive reload probe.
  The earlier nonblocking classification still applies: F1 is intact over
  an actual edit in this round's API checks.

## 4. Regression evidence

### Private corpus — aggregates only

My read-only CLI census, using isolated output databases:

| Scope | Measured result |
| --- | --- |
| 3 real `.knxproj` | 2 exit 0; 1 exit 2 (the documented report-loss exit); 0 inconsistent-record refusals |
| Same 3 through `knx products ingest` | 3/3 exit 0 |
| 103 `.knxprod` | 103/103 ingested with the ordinary limits |
| Input identity | All 106 inputs unchanged: size, mtime and SHA-256; discovery set unchanged |
| Structural census | 106 outer archives / 1,404 records; 7 unencrypted nested payloads / 113 records |
| Directories | 126, including the real ETS empty-directory records |
| Local/central names, flags, method, CRC, both sizes | 0 disagreements in each category |
| Gaps/overlaps, prefixes, data descriptors, Zip64 records | 0 in each category of this census |

The census did not decrypt encrypted real-project payloads. It provides
regression and admission evidence, **not** proof of complete semantics,
lossless import or general ETS compatibility. No private names, individual
hashes, paths, values or databases are published.

**Selected corpus regression suite: 143 of 143 passed in 31 targets, exit 0.**
`OriginalData/` and the additional oracle `project_dump.json` were linked only
for that run; both links are now removed (verified). The absent-corpus
negative control returned exit 2, explicitly saying that nothing ran and
this is not a pass.

## 5. Gate table

Gate run **06:23:53–06:46:16 CEST on 2026-10-07**. All **24** stages have
their expected exit, with absent-corpus exit 2 treated only as a negative
control. Product/tool sources remained frozen (`git diff --exit-code HEAD
-- apps crates Cargo.toml Cargo.lock tools`, exit 0).

Candidate `b8724d66`; my own worktree, fresh `CARGO_TARGET_DIR`, short private
`TMPDIR` and isolated `XDG_DATA_HOME`. The alpha, workspace and browser
fixture leases are held in that order for the entire gate script. All tests,
CLI probes, servers and native application runs use `unshare --user
--map-root-user --net` with loopback brought up. Dependency fetch/install,
audit and dependency advisory checks are setup operations, not offline test
claims. No gateway/device variables or live KNX contact.

| Command / check | Exit | Measured result |
| --- | --- | --- |
| `npm ci`, `npm run build`, `npx tsc --noEmit`, flow-study and theme-fixture checks | 0 each | Actual install/build and type checks; build chunk warning only |
| `npx vitest run` | 0 | 2,076 passed, 117 files |
| `npx playwright test` | 0 | 142 passed, Chromium, no retries |
| `npm audit` | 0 | 0 vulnerabilities |
| `cargo fetch --locked` | 0 | Dependency setup |
| `cargo build --release --locked -p knx-cli -p knx-server` | 0 | Fresh binaries used for the probes |
| `cargo fmt --all -- --check` | 0 | Formatting |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | 0 | Strict lint |
| `cargo test --locked --workspace --no-fail-fast` | 0 | **3,402 passed, 0 failed, 178 ignored**, 192 result blocks |
| `xtask check-layering` | 0 | Boundaries checked |
| `xtask check-headers` | 0 | 560 with header, 155 without (ceiling 155), 42 generated skipped |
| `xtask check-anchors` | 0 | Before review: 640 links / 301 Markdown files; final review/handover: 642 / 302; none dead |
| `xtask check-ledger` | 0 | 190 rows |
| `xtask check-corpus-gates` | 0 | 394 Rust files, no silent missing-corpus return |
| `cargo deny check` | 0 | Advisories, bans, licenses, sources |
| `python3 -m unittest tools.tests.test_run_corpus_tests` | 0 | 4 tests |
| `tools/run_corpus_tests.py`, corpus absent | 2, expected | Explicit refusal; not counted as a pass |
| `tools/run_corpus_tests.py`, corpus and oracle linked | 0 | **143 of 143 passed, 31 targets**, separately selected ignored tests |
| `xtask check-appimage --artifact-dir <copy>` | 0 | `AppImage ok: version 0.1.0-alpha.4` |
| AppImage hash / source stamp / extracted content inspection | checked | Brief's digest; stamp `2254eed0`; 337 regular files; no `/home/knxbench`, `/mnt/`, worktree or `OriginalData` path; no project/archive/database/key-file extensions |
| Native offline AppImage start | 0, probe | Wayland recipe; window visibly renders `v0.1.0-alpha.4`, no crash dialog; all 181 API cases and 13 smoke steps completed; still alive before controlled shutdown; 0 non-loopback sockets |
| Fresh standalone server probe | 0 | Same 181 cases and 13 smoke steps; edit survives refused discard-import; 0 non-loopback sockets |
| Private CLI census | 0 | Aggregates in §4; inputs unchanged |

The extracted count excludes 22 symlinks, not their targets; counting
`is_file()` alone misleadingly reports 359. PEM-label strings were found only
in the bundled `libgnutls.so.30`, not as separately packaged key files.
The native screenshot was inspected: actual KNXBench welcome page,
navigation, inspector and `v0.1.0-alpha.4` status bar, with no error dialog.
No user-desktop screenshot is published or retained in the permanent report.
The smoke uses the real application API: sample import, Save As, area rename,
refusals with/without discard, absolute/relative missing Open, reopen, New,
Save As and reopen. It is not evidence that every native dialog was tested.

The final review/handover also passed `check-ledger` (190 rows),
`check-headers` (560/155, 42 generated skipped), and staged whitespace
validation. The inherited `.ai/CURRENT_STATE.md` body remains a byte-exact
suffix; only this review's entry is prepended.

## 6. Coverage limits and reproduction

Not checked in this narrow round:

- Live bus, gateways, hardware or hardware writes.
- ETS or .NET's own interpretation of the synthetic archives; no new
  certification or full-compatibility claim.
- AES, split/multidisk archives, or encrypted real-project nested payloads.
- The unmodified AppImage under X11 (no Xvfb available here); the tested
  extracted Wayland recipe is the existing KL-158 boundary.
- A new AppImage rebuild or byte-for-byte reproducible-build comparison.
- The owner's 27-mutant sweep; my independent byte fixtures were used instead.
- A fresh independent re-audit of every F1–F4/M1–M9 item, browser-native file
  dialogs, screen readers or the N6 interactive reload. Broad regression
  suites and the specific edit/refusal probes do not replace those audits.
- A new measurement of the disclosed approximately three-times in-budget
  memory peak. No claim that the new structural check changes that boundary.

**Harness provenance.** An initial generator attempt stopped on a struct
packing typo before creating its final manifest. The first native harness
refused before launch because it inspected inherited sysfs rather than
namespace-aware `ip -j link`. A later attempt completed the archive matrix
but stopped in auxiliary smoke code that used the wrong save endpoint and
projection field. I read the actual routes/projection, corrected the harness,
and reran all 181 cases and the complete smoke on both servers. These were
harness failures, not product failures or admitted passes.

**Reproduction:** fictional input from `tools/manual_sample_project.py`;
independent local/central/descriptor ZIP writer, producer controls and case
runners. Compact scripts and aggregate receipts are retained outside Git in
maintainer evidence `ar18-recheck-round4-20261007`; no private archive,
extracted customer content, customer database or screenshot is retained.
The repository deliverable is this review plus its handover, on
`review/alpha-recheck-4`, not a product fix or a merge into `main`.

*The envelope and letter finally agree. The empty folder still gets away
without having its checksum homework checked.*
