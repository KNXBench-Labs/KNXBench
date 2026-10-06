# KNXBench Alpha — independent whole-product review (AR18)

- **Reviewer:** fresh Claude session started by the user from
  [AR18_REVIEW_BRIEF](AR18_REVIEW_BRIEF.md); no prior history with this code,
  no subagents.
- **Date:** 2026-10-06, 09:30–10:30 CEST.
- **Reviewed revision:** `f2b315388036d196f1a1bb1a5cbbc6080f73e1e7` (the
  `origin/main` tip when the review started; it did not move during the
  review). `git diff --name-only 4b9e913e f2b31538` lists only documentation,
  handover files and `apps/knx-server/tests/http_device_compare.rs` — no
  product source, no build file. Verified.
- **Artifact:** `KNXBench_0.1.0-alpha.4_amd64.AppImage`, copied out of the
  evidence store; SHA-256 recomputed `70bbb6b6…f72f81b`, matches the brief.
  The binary embeds the build stamp `4b9e913e`.

## 1. Verdict

**`READY_WITH_CONDITIONS`**

No CRITICAL finding. The gates reproduce green on my own target directory,
offline; the 142 private-corpus tests pass; the device-write path is gated
the way the documents say (plan first, device-specific phrase, durable
activity record, backup read and fsynced before the first write, refusal
before a tunnel opens); individual-address programming, address reset and
serial writes are refused unconditionally. Layering is clean.

Four IMPORTANT findings stand between the candidate and a tag. Each must be
**fixed, or explicitly accepted by the user and disclosed** before AR19:

| # | Condition |
| --- | --- |
| C1 | **F1** — Open / Import silently throw away unsaved edits. Fix (refuse or confirm, as *New project* already does), or accept and say so in the manual and known issues. |
| C2 | **F2** — Duplicate and case-colliding archive member names lose or substitute preserved bytes without a report. Fix (refuse like the nested ZipCrypto path already does), or accept and qualify "never silently dropped" in README and manual. |
| C3 | **F3** — A crafted `.knxproj` of 1–2 MB drives import to 2–3 GB of memory (no actual-size bound, no cumulative budget). Fix, or accept and disclose "do not import untrusted files; the server shares this risk with every logged-in user". |
| C4 | **F4** — `knx import --store <existing.knxdb>` replaces an existing project without a word. Fix (refuse an occupied store without an explicit flag) or accept and document. |
| C5 | Any code change for C1–C4 re-runs the affected gates (fmt, clippy, workspace tests, Vitest/Playwright when the Web changes, the corpus subset) and **rebuilds the AppImage with the same `--remap-path-prefix` flags** — the artifact reviewed here is `4b9e913e`. |

The MINOR findings (§2.2) are not conditions; M5 (documentation drift) is
cheap enough that I would fix it before tagging anyway.

On classification: F1 is the closest call. The brief defines CRITICAL as
"data loss", and edits are lost. I rated it IMPORTANT because nothing that
was ever written to disk is harmed and the loss follows an explicit user
action — but it is undisclosed, two clicks away, and inconsistent with the
guard the project built for *New project*. If the user reads "data loss"
literally, F1 makes the verdict `NOT_READY` until fixed.

## 2. Findings

### 2.1 IMPORTANT

#### F1 — Opening or importing a project discards unsaved edits without asking (ran)

- **Where:** `apps/knx-web/src/App.tsx:786-798` (`pickProject`,
  `openNativeProject` call `runLoad` with no modified-state check);
  `apps/knx-server/src/routes.rs:703-722` (`import_project`) and `:823-837`
  (`open_native_project`) replace the project unconditionally, while
  `POST /api/project/new` refuses with 409 (`routes.rs:811-818`,
  `domain.rs:675-686`).
- **Scenario (UI, ran):** real `knx-server` (candidate debug build) + the
  production web build in Chromium, offline. Import the fictional sample →
  Topology → *Add area* "Garage (unsaved edit)" → the area is shown → File →
  *Open project…* → pick another `.knxproj`. No prompt; the import request is
  sent at once (200); the area is gone; the server reports `is_modified:
  false`.
- **Scenario (API, ran):** after an edit (`is_modified: true`),
  `POST /api/project/new` → 409 "save it first or resend with
  discardChanges"; `POST /api/project/import` → 200 and the edit is gone;
  the same for `POST /api/project/open`.
- **Docs:** `docs/manual/user-guide/02-projects.md:56` and `:189-203`
  describe the guards for *New project* and *Quit* and the unguarded browser
  tab; Open/Import are not mentioned. Autosave (5 min, only for projects
  that already have a file) does not cover a freshly imported project.

#### F2 — Duplicate and case-colliding archive members are lost or substituted silently (ran)

- **Where:** `crates/knx-etsproj/src/container.rs:483-507` (`open_raw`
  records every central-directory name with no duplicate check — the nested
  ZipCrypto path does check, `:315-317`); `container.rs:545-587` (`read`
  resolves through `find`, the *first* case-insensitive match);
  `crates/knx-etsproj/src/opaque.rs:121-148` (each walked path is read via
  `read`; the regenerated-entry skip at `:125` is case-insensitive). The
  `zip` 8.6 reader collapses identical names into one `IndexMap` slot
  (`zip-8.6.0/src/read/zip_archive.rs:49-52`).
- **Scenarios (CLI `knx import --no-product-db`, fictional sample, ran):**
  1. `P-0001/BinaryData/a.dat` ("lower-case-bytes") plus `…/A.DAT`
     ("UPPER-CASE-BYTES"): exit 0, 0 warnings, two opaque rows — **both with
     the bytes of `a.dat`**, and each row's SHA-256 matches its (wrong)
     bytes, so hash verification cannot notice the substitution.
  2. Two physical entries named `P-0001/BinaryData/x.dat`: exit 0, one
     opaque row (the second copy); the first copy vanished, no report line.
  3. An extra `P-0001/0.XML` (case variant of the topology document, other
     content): exit 0, 0 warnings, 0 unknown; neither parsed nor kept as
     opaque.
  The same three files through the AppImage's own server API: 200, 0
  warnings. The UI session log for case 1 mentions neither the collision
  nor the lost bytes.
- **Why IMPORTANT, not CRITICAL:** ETS is not expected to write such
  archives, so real projects are unlikely to hit it. But README.md:33-35,
  `docs/manual/reference/03-troubleshooting.md:121` and
  `docs/manual/reference/04-faq.md:32` state that unknown data is "never
  silently dropped", and for these inputs it is.

#### F3 — Unbounded memory on crafted archives (ran)

- **Where:** `container.rs:545-579` — the outer read checks only the
  *declared* uncompressed size (`MAX_ENTRY_SIZE`, 64 MiB) and then
  `read_to_end`s whatever the deflate stream yields; `zip` 8.6 checks the
  CRC, not the length (`zip-8.6.0/src/read/readers.rs:265-295`). The nested
  ZipCrypto path does bound inflation (`container.rs:620-651`); the common
  path does not. `opaque.rs:109-154` keeps every member in memory with no
  cumulative budget, and `crates/knx-app/src/import.rs:162` clones every
  opaque payload once more.
- **Scenarios (CLI, 8 GiB address-space cap, offline, ran):**
  - 1,053,039-byte `.knxproj` whose one extra member inflates to 1 GiB while
    both headers declare 100 bytes (CRC correct): **peak RSS 2,078 MB**, then
    `import failed … string or blob too big` (SQLite's limit, not a guard).
    A member slightly under 1 GB would have been stored.
  - 1,480,248-byte `.knxproj` with 24 honest 60 MiB members of zeros: exit 0,
    **peak RSS 3,015 MB**, a 1,511,710,720-byte `.knxdb` written.
  - `knx products ingest` on the first file (project-archive route): peak
    RSS 1,055 MB.
- **Impact:** the desktop runs the server in-process, so an OOM kills the
  application with whatever is unsaved (see F1). The server accepts uploads
  up to 100 MiB (`apps/knx-server/src/fs_routes.rs:33`), far above what is
  needed. `docs/IMPORT_EXPORT.md:176-187` presents the declared-size guard
  as the zip-bomb defence; it covers only entries that *declare* their size
  honestly.

#### F4 — `knx import --store` overwrites an existing project silently (ran)

- **Where:** `apps/knx-cli/src/main.rs:330-350` opens/migrates whatever the
  path holds, `:374-402` imports and calls `save_project`, which deletes and
  rewrites all domain tables; `insert_opaque` and
  `insert_manufacturer_refs` replace theirs.
- **Scenario (ran):** `knx import sample.knxproj --store c.knxdb`, then
  `knx import other.knxproj --store c.knxdb` → exit 0, the stored project
  name changes from the first to the second, no warning.
  `docs/manual/user-guide/10-command-line.md:57` documents `--store` only as
  "Where to write the project file".
- **Related (read, not run):** the web Save As picker has no
  existing-file check (`apps/knx-web/src/FsPicker.tsx:255` onward) and
  `save_project_as_impl` overwrites by design
  (`apps/knx-server/src/domain.rs:490-508`). Whether the desktop's native
  GTK save dialog asks before replacing was not checked.

### 2.2 MINOR

- **M1 — Failure and read-only paths still modify files (ran).**
  (a) A *failed* CLI import into a v3 store upgrades it to v10 anyway
  (`main.rs:344` migrates before the import at `:374`). (b) Read-only
  commands (`knx doc-export`, `knx ga-export`) and GUI *Open* on a foreign
  SQLite file with `user_version 0` inject 22 KNXBench tables and set
  `user_version 10`; the file is changed and the command then fails with "no
  project has been saved". `migrate` (`crates/knx-store/src/migration.rs:572-605`)
  never checks the `schema_meta.created_by` marker before upgrading a v0
  file. (c) GUI *Open* of an empty v3 file: 500, file upgraded. (d) A
  mistyped path given to `doc-export`/`ga-export` leaves an empty migrated
  `.knxdb` behind (`Connection::open` creates it, `migration.rs:549`). A
  newer-schema file is refused untouched, and a migration failure rolls back
  byte-identically (both ran).
- **M2 — Save writes three transactions in place (read).**
  `domain.rs:503-506` commits `save_project`, `insert_opaque` and
  `insert_manufacturer_refs` separately into the target file; a crash
  between them leaves the new project beside the previous file's opaque
  evidence. No temp-file-and-rename.
- **M3 — A second project part is kept but not mentioned (ran).** An
  archive with `P-0001` and `P-0002` imports only the first
  (`container.rs:594-600`); the second's `0.xml`/`Project.xml` become
  ordinary opaque entries, with no unsupported-feature or warning line. Not
  lost, but easy to miss. I did not check whether ETS can produce such a
  file.
- **M4 — Wrong-password message carries a literal backslash, newline and
  indentation (ran).** `container.rs:182-183` (`\\` followed by a real line
  break). Visible in CLI stderr and the session log; the Web dialog shows
  its own clean text.
- **M5 — Documentation drift (read).** Limitation counts disagree:
  README.md:173 "119", `docs/manual/known-issues.md:11` "110 … as of
  2026-10-01", `docs/ALPHA_SCOPE_MATRIX.md:78` "109 numbered, 108 rated";
  `docs/KNOWN_LIMITATIONS.md` has 120 numbered headings (highest §158).
  `ALPHA_SCOPE_MATRIX.md:44` still cites the AR17 AppImage from `6b9b6818`.
  The brief (`AR18_REVIEW_BRIEF.md:17`) names "the commit that added this
  brief" (`0c14c4a7`, without the test fix) but describes `f2b31538`.
- **M6 — A download whose tunnel never opened is recorded as `unknown`
  (read).** The durable start precedes `connect_tunnel`
  (`device_download_routes.rs:399-412`, CLI `main.rs:2235-2263`); on connect
  failure the guard's `Drop` turns `running` into `unknown`
  (`crates/knx-app/src/commissioning_activity.rs:634-648`). Safe-side, but
  it tells the user a write might have happened when none could.
- **M7 — Corpus tests sit outside every routine gate (read).** The compare
  pair had been red "since at least `c58b2d0a`"
  (`docs/ALPHA_FINAL_GATES.md:68-69`) before anyone looked. The repo has the
  conventions (`check-corpus-gates`) but no schedule that runs them.
- **M8 — Tests read the developer's real product database (ran + read).**
  `AppState::default()` → `AppState::new` opens `knx_productdb::default_path()`
  (`domain.rs:198-200`, `:253-258`). My corpus run used a copy under a
  private `XDG_DATA_HOME`; the copy and the original stayed byte-identical,
  so no writes happened, but results depend on host state.
- **M9 — `npm audit`: 1 high (ran)** in `source-map-js`, a build-time
  dependency; nothing of it ships in the bundle as far as I can tell.
  `cargo deny check` is clean.
- **Known, already disclosed:** a non-ZIP import answers HTTP 500
  (`docs/ALPHA_CANDIDATE.md` §3). Reproduced through the AppImage.

### 2.3 Claims checked and found accurate

- **Compare-harness fix (brief §3).** Production builds `AppState::new`
  (`apps/knx-server/src/main.rs:54`,
  `apps/knx-desktop/src-tauri/src/lib.rs:191`), which installs a persistent
  activity history (`domain.rs:226-231`); `Default` deliberately replaces it
  with an unavailable one (`domain.rs:253-258`) → 503 at `start_download`.
  Reverting the fix in my worktree reproduced exactly the two failures with
  "activity history unavailable; not sent"; restored, 7/7 pass. **It is a
  stale harness, not a hidden product defect**; the product fails closed as
  AUDIT-01 requires.
- **Build paths.** No `/home/…`, no worktree or corpus path, no e-mail
  address in any of the 337 AppImage files; 500 `/cargo/registry` and 40
  `/rustup/` neutral paths remain in `usr/bin/knx-desktop` (the dossier's
  "515" is a different count of the same thing). Not release-blocking. A
  release built elsewhere must use the same flags or a CI runner.
- **Device writes.** Plan by default; `WriteAuthorisation::for_hardware`
  with the per-device phrase; untested programs need a second phrase;
  durable activity start before the tunnel; mask and manufacturer read
  before the first write; backup read, written `create_new` + `0600` +
  fsync + read-back + directory fsync before the first changing step
  (`crates/knx-net/src/commissioning/memory_download.rs:817-838`,
  `crates/knx-app/src/device_backup.rs:347-380`). Address programming,
  reset and serial writes return an unconditional precondition failure
  before any socket (`crates/knx-app/src/individual_address_programming_recovery.rs:11-13`
  and siblings). Simulator and corpus tests for these paths pass.
- **KNXnet/IP.** Tunnelling request: 1 s ACK wait, one repeat with the same
  sequence number, then disconnect; inbound: expected → ACK and deliver,
  expected−1 → ACK and drop, anything else → drop
  (`crates/knx-net/src/client.rs:454-488`, `:964-995`). cEMI L_Data
  control 0xBC / 0xE0 for group data, broadcast bit cleared for system
  broadcast. Read only; no live bus.
- **DPT encoder.** CLI `bus write --dry-run` (no socket) gave correct octets
  for 1.001, 5.001/5.003/5.004 (0.5 rounds up), 6/7/8/12/13/29 limits, 9.001
  (21.5 → `0C 33`, −30 → `8A 24`, −671088.64 → `F8 00`, half-away rounding),
  14 (1.5 → `3F C0 00 00`, NaN and 1e39 refused), 11.001 and 16.000 padding.
  9.001 refuses 670760.96, whose encoding collides with the reserved invalid
  value 0x7FFF — defensible. Decoding was not probed.
- **Layering.** `knx-core` depends on `chrono` only; `knx-net` on `knx-core`,
  `tokio`, `socket2`; `knx-etsproj` on neither store nor product DB. No
  product logic keyed to a manufacturer; the one embedded evidence file
  (`crates/knx-app/data/verified_downloads.json`) is data, not code.
- **Store.** Upgrade is one transaction: a planted table collision left a v3
  file byte-identical. CLI import → server open → Save As → open → Save As:
  all 22 tables equal at every step, including the three retained unknown
  constructs.
- **Import reporting.** Unknown elements and attributes are reported and
  retained byte-exact; duplicate ids, duplicate group and individual
  addresses, invalid addresses, broken XML, an unknown schema version,
  truncated archives and a bit flip in a deflate stream each produce a
  report entry or a clear refusal; ZipCrypto right/wrong/missing password
  behave as documented, and the import report says the protection is not
  preserved.
- **Honesty of claims.** README, manual and scope matrix stay inside what
  the code does on the points I checked (no full-ETS or certification claim,
  one device and one gateway of hardware evidence, Secure not claimed,
  address programming refused, flow view "Chromium only, Motion Off for large
  maps"), with the exceptions in F2 and M5.

## 3. Gate table

Revision `f2b31538`, fresh `CARGO_TARGET_DIR` in my scratch directory, both
leases held, every `KNX*` variable unset, tests in a loopback-only network
namespace. Run 09:36–09:45 CEST.

| Command | Exit | Counts |
| --- | --- | --- |
| `npm ci` / `npm run build` (apps/knx-web) | 0 / 0 | chunk-size warning only |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | |
| `cargo test --workspace --no-fail-fast` (offline) | 0 | **3,317 passed, 0 failed, 177 ignored**, 188 result blocks |
| `xtask check-layering` | 0 | 449 resolved packages |
| `xtask check-headers` | 0 | 552 with header, 155 without (ceiling 155), 42 generated |
| `xtask check-anchors` | 0 | 617 links in 296 files |
| `xtask check-ledger` | 0 | 190 rows |
| `xtask check-corpus-gates` | 0 | 389 Rust files |
| `cargo deny check` | 0 | advisories, bans, licenses, sources ok |
| `npx tsc --noEmit` | 0 | |
| `npx vitest run` (offline) | 0 | 2,071 passed in 117 files |
| `npx playwright test` (Chromium, offline) | 0 | 139 passed |
| `xtask check-appimage --artifact-dir <copy>` | 0 | `AppImage ok: version 0.1.0-alpha.4` — checks config, workflow, name, version and exec bit only, not contents |
| Private corpus, `-- --ignored`, offline, copied product DB | 0 per target | **142 passed, 0 failed** in 31 test targets |
| Compare-harness revert mutant | — | 5 passed, 2 failed (503), restored → 7 passed |
| `npm audit` | — | 1 high (M9) |

The 177 ignored tests: 142 are the corpus tests above. Not run (35): 9 live
gateway/device tests (forbidden by the brief), 20 that need
`KNXBENCH_PRODUCT_CORPUS` scopes, 2 explicit-env corpus research tests, 1
private telegram log, 1 private nested package, 1 heavy 55 MB build, 1
`perf_baseline`.

## 4. Coverage

**Checked deeply, by running:**

- every gate in §3 on the candidate;
- the 142 `OriginalData` corpus tests from the worktree with the corpus
  linked (symlink removed afterwards); aggregates only;
- 19 adversarial imports built from the fictional sample — truncations, a
  bit flip, broken XML, schema 99, duplicate ids/addresses, unknown
  constructs, invalid addresses, duplicate and case-variant members, two
  project parts, a lying-size bomb, a many-member archive, ZipCrypto
  right/wrong/missing — through the CLI and, for six of them, the
  AppImage's own server;
- store refusal paths, migration rollback, and a CLI → server → Save As
  round trip, compared table by table;
- the unmodified AppImage, offline, under a private Xvfb 21.1.24 (signature
  checked): start page, new/save/import/reopen, missing file, non-ZIP,
  password, old and foreign files; no non-loopback socket; still alive;
- the production web build in Chromium against the real server: import,
  group addresses, command palette, search, topology, log with unknown
  constructs, password dialog (wrong and right), Save As dialog, settings,
  and the unsaved-edit scenario of F1;
- a content and privacy scan of the extracted AppImage (337 files, 305 MiB),
  including a search for 629 label values taken read-only from the private
  reference projects: one hit, a generic German UI word;
- the compare-harness mutant; DPT encoding via `--dry-run`.

**Checked by reading only:** device download, restore and service-control
gating in CLI and server; backup durability; address-programming refusal;
tunnelling sequence/ACK/timeout handling and cEMI control fields; layering
manifests; debug-report redaction (local only, no upload); ADR-0078 / `KL-61`
(black-box only at schema 11, where a declared GA type is reported as an
unknown attribute and ignored for resolution); the AR21 flow acceptance
§13–§22 and its disclosure in the manual; the manual chapters on projects,
command line, known issues and troubleshooting against the behaviour I saw.

The committed tree contains 11 multi-word or long label values from the
private reference projects (65 file-value pairs, mostly in docs and tests):
ETS default names, generic room/function labels, a project title, a vendor
demo name — no personal names or street addresses found. Worth a glance
before the repository goes public; not a finding.

**Not checked:**

- anything on a live bus or device (forbidden); the hardware evidence is
  read from the documents only;
- KNX Data/IP Secure (not implemented, accepted boundary);
- adversarial `.knxprod` package ZIPs beyond the package/project
  classification; the 20 env-configured product-corpus tests including the
  compatibility matrix;
- the flow view's load and long-session measurements (not re-measured);
  WebKitGTK interaction beyond the AppImage start page; the native Wayland
  recipe; keyboard-only and screen-reader use; the desktop quit dialog and
  native save dialog;
- schema-21/23 synthetic projects for `KL-61`; DPT decoding;
- a reproducible rebuild of the AppImage from `4b9e913e`; the CI workflow;
  the Docker image; server authentication beyond reading `main.rs`;
- performance on large projects.

## 5. Reproduction notes

Scripts and outputs live in the reviewer's scratch directory, not in the
repository. Every adversarial input is the fictional sample from
`tools/manual_sample_project.py`, changed with Python's `zipfile`: extra or
renamed members, one edited attribute, or, for the bomb, a hand-written
local header and central directory whose size fields say 100 while the
deflate stream yields 1 GiB with a correct CRC. Peak RSS comes from
`wait4` under an 8 GiB `RLIMIT_AS`.

*The reviewer's one-line summary: the bus is treated with great care; it is
the archive and the open-file dialog that need a second look.*
