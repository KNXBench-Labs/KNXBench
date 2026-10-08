# KNXBench Alpha — independent re-check, round 2 (N1–N6)

- **Reviewer:** a fresh Claude session started by the user from
  [AR18_RECHECK_BRIEF §Round 2](AR18_RECHECK_BRIEF.md#round-2-n1n6). I wrote
  none of the fixes, did not write the first re-check, used no subagents and
  changed no product code.
- **Date:** 2026-10-06, 17:00–17:55 CEST.
- **Candidate:** `863f72ad`, the `origin/main` commit that last changed the
  brief. `git diff --name-only 3ede4817 863f72ad` lists only `docs/`, `.ai/`
  and `docs/archive/alpha-0.1/alpha-release-goal.md` (verified), so the product code is the code of
  `3ede481741acbfcd5904f67c80e443077d1151c0`.
- **Artifact:** a copy of `KNXBench_0.1.0-alpha.4_amd64.AppImage`. I
  recomputed its SHA-256, `ca3101fb…78591e`, and it matches the brief. Its
  server reports `0.1.0-alpha.1+g3ede4817`; the desktop binary carries the
  stamp `3ede4817`.
- **Environment:** my own detached worktree, a fresh `CARGO_TARGET_DIR`, my
  own scratch directory. All three leases held for every Cargo run. No
  `KNX*` variables. `XDG_DATA_HOME` isolated. Every test, CLI run, server and
  browser ran in `unshare --net` with loopback only. No gateway, no device.

## 1. Verdict

**`READY_WITH_CONDITIONS`**

N2–N5 are fixed against my own inputs. Every round-1 N1 scenario is now
refused, in the outer archive and in a ZipCrypto-protected payload. The
gates reproduce the dossier's counts on my target directory. The corpus
subset passes 143/143. No real project or product package is refused.

**N1 is fixed only for names that stay file names.** The brief asked about
"directory entries that collide with file names". That is the gap:

- **N7 (IMPORTANT, same class as N1):** a record whose *decoded* name ends in
  `/` counts as a directory. Both identity checks and the inventory skip it,
  together with its bytes. Give the real `P-0001/0.xml` a Unicode Path field
  that makes it `P-0001/0.xml/`, and give another member a Unicode Path field
  that makes it `P-0001/0.xml`. The import then:
  - reads the second member as the topology document;
  - drops the real one without a report line;
  - exits 0 with 0 errors, through the CLI, the release server and the
    AppImage's own server (`200`), outer or nested.

  `bsdtar` ignores the field and reads the real `0.xml` from the same file.

| # | Condition |
| --- | --- |
| R4 | **N7** — in `open_raw` and in the nested loop, refuse (or at least report) a record whose decoded name is a directory but that carries bytes, and refuse a decoded directory name that equals a file name once the trailing `/` is removed. The product reader already does both (`crates/knx-productdb/src/package.rs`: "unsafe product ZIP member", "duplicate product ZIP member"). Or: accept it, and qualify "never silently dropped" and KNOWN_LIMITATIONS §159. |
| R5 | Any code change for R4 reruns the knx-etsproj and workspace tests and the corpus subset, and rebuilds the AppImage with the same `--remap-path-prefix` flags. |

The MINOR findings (§3.2) are not conditions. N6 should not block (§2).

## 2. N1–N6

"Ran" means my own inputs against the candidate's release binaries
(`knx`, `knx-server`) or the AppImage, not the owner's tests.

| Finding | Result | Evidence |
| --- | --- | --- |
| **N1** decoded names collapse or substitute members | **fixed for file names; bypass → N7** | **Ran, outer archive:** Unicode Path twins → `NameCollision` ("12 member records … only 11 distinct names"). CP437 `\x82` against UTF-8 `é` → `DuplicateEntry`. CP437 against a Unicode Path to `é` → refused. A Unicode Path onto `P-0001/0.xml`, whether placed before or after the real one → refused. Onto `p-0001/0.XML` → refused. Two invalid UTF-8 names (both U+FFFD) → refused. A Unicode Path twin of `KNX_MASTER.XML` → refused. Two distinct non-ASCII names → imported, both rows with their own bytes. **Ran, ZipCrypto-protected payload** (hand-encrypted, not stored plain): the same twins, CP437/UTF-8 and `0.xml` substitution are each refused. So are plain members inside the payload, and a nested Unicode Path onto an outer member (`KNX_MASTER.XML`). **New identity tricks (ran):** local-header names that differ from the central directory are accepted silently (→ N9). Zip64: a legitimate all-zip64 archive and a zip64-EOCD-only archive import normally. A zip64 extra that declares 100 B for a 200 MiB member is refused, outer and nested. One that declares 2⁶³ is refused by the budget. A zip64 EOCD count one short is refused. Directory entries: a plain `P-0001/0.xml/` directory beside the file is harmless, but directories that carry bytes, or that a Unicode Path field creates, are not → **N7** |
| **N2** budget checked after unpacking | **fixed** | **Ran, peak RSS via `wait4`, 8 GiB cap, CLI:** 30 × 60 MiB plain nested members (1.8 MB file): 11 MB, refused before unpacking (round 1: 1,936 MB). 40 ZipCrypto members (2.5 MB): 12 MB (round 1: 2,577 MB). **200 plain members (12 MB file): 31 MB** (round 1: 8,172 MB, then out of memory). Outer 420 MiB plus nested 120 MiB: refused by the combined pre-count, 10 MB. 9 central-directory names aliasing one 60 MiB local record: refused, 10 MB. A nested member that declares 100 B and inflates to 1 GiB: refused, 10 MB. **Release server:** `VmHWM` 37 MB after the 12 MB file. **AppImage desktop process:** `VmHWM` 244 MB at start and 261 MB after the 1.8 MB and 12 MB files (round 1: 2,206 MB for the 1.8 MB file). Inside the budget, 8 × 60 MiB still peaks at 1,602 MB (disclosed in §159) |
| **N3** `device restore` records a never-opened tunnel as `unknown` | **fixed (read)** | `apps/knx-cli/src/main.rs:2780-2788` now calls `record_never_connected` on a connect failure, like download (`:2290`) and the server (`device_download_routes.rs:415`). **Not run:** a restore needs a backup that plans against real product data. None of the fictional sample's devices plans a download, and I did not point the CLI at private corpus data |
| **N4** `AppState::new` touches the developer's product DB | **fixed** | **Ran:** the whole workspace test suite with `XDG_DATA_HOME` unset and `HOME` set to an empty directory: 3,359 passed, and the fake `HOME` was still empty afterwards (§4). Read: `AppState::new` (`apps/knx-server/src/domain.rs:211-213`) opens no product database. Only `knx-server/src/main.rs:54` and the desktop's `lib.rs:191` call `with_user_product_db` |
| **N5** refused archives answer `500` | **fixed, one residue → N8** | **Ran (release server and AppImage):** a non-ZIP file, an empty file, a Unicode Path collision, a lying zip64 size and both bombs each answer `422 projectNotImportable` with the original message. A missing password still gives `422 projectPasswordRequired`, and a wrong one `422 projectPasswordWrong`. **But** an import of a path that does not exist answers `500` with no `kind` (N8) |
| **N6** reload shows the welcome page over a held project | **not changed; should not block** | No web change since round 1 (`git diff bd691b82 3ede4817 -- apps/knx-web/src` is empty), so the gap stands as described. Disclosed in [KNOWN_LIMITATIONS §82](../../../KNOWN_LIMITATIONS.md#82-the-diagnostics-companions-stale-lock-sees-one-browser-profiles-own-windows-and-nothing-else), item 3. **Why not blocking:** it is a UX gap, not a guard failure. I reran F1 over a real edit (below): nothing is replaced without an explicit `discardChanges: true`, and round 1 saw the unsaved-changes dialog appear in this state. The desktop window does not normally reload. Better home: it sits inside a section about the diagnostics companion, where a user looking for it will not search |

### Regressions the brief asked about

- **Corpus subset:** `tools/run_corpus_tests.py`, **143 of 143 in 31
  targets**. My first run linked only `OriginalData/` and got 137/143. All
  six failures said "`project_dump.json` … not present". That was my setup
  (the owner's gate links the oracle too), not the product.
- **Real projects (read-only census, aggregates only):**
  - **3 `.knxproj`:** 2 exit 0. 1 exits 2, meaning "imported with report
    errors" in its own data, as in round 1. None is refused for identity,
    budget or size.
  - **The same 3 through `knx products ingest`** (the project-archive
    route): 3/3 exit 0.
  - **103 `.knxprod`:** 103/103 ingested.
  - Every corpus file is unchanged (size, mtime and SHA-256 compared).
- **Prefixed archives:** a self-extractor-style 4 KiB prefix does not
  trigger the new count check; the sample imports normally.
- **F1 over a real edit (release server, ran):** after renaming an area,
  each of these answers `409 projectUnsavedChanges` and keeps the edit:
  non-ZIP, collision and missing-file imports, and a missing-file open. With
  `discardChanges: true`:
  - the non-ZIP, collision and bomb imports answer `422`, and the edit
    survives;
  - the missing-file import answers `500`, and the edit survives (N8);
  - the missing-file open answers `422 projectNotOpenable`.

  A second import while a load runs still gets the old `409` without a
  `kind`, so the two `409`s stay apart.

## 3. New findings

### 3.1 IMPORTANT

#### N7 — A Unicode Path field can turn the real `0.xml` into a "directory" and another member into `0.xml` (ran)

- **Where:** `crates/knx-etsproj/src/container.rs`:
  - `:754`: the raw-name check skips names that end in `/`;
  - `:788-790`: `check_decoded_names` skips `is_dir()` entries;
  - `:554-556`: the inventory skips them, so their bytes never reach
    `entries`, the opaque store or the report;
  - `:335` and `:358-360`: the same skips in the nested loop.

  `zip` 8.6 applies the Info-ZIP Unicode Path field (0x7075) to the name and
  then decides `is_dir()` on the decoded name.
- **Scenarios (CLI `knx import --no-product-db --store`, fictional sample):**
  1. Raw `P-0001/0.xml` with a Unicode Path of `P-0001/0.xml/`, plus raw
     `P-0001/zz.xml` with a Unicode Path of `P-0001/0.xml` and a changed
     device name:
     - exit 0, 0 errors, 8 opaque entries;
     - the stored project's first device is `FORGED DEVICE NAME`;
     - the real `0.xml` (13,254 bytes) is gone, and no report section
       mentions it.

     The same happens:
     - when the forged member comes first;
     - with the Unicode Path field in both the local and central headers
       (the consistent form);
     - inside a ZipCrypto-protected payload;
     - through the release server and the AppImage's own server (`200`,
       and the project shows the forged device).
  2. A directory record `P-0001/BinaryData/hidden/` with 4,500 bytes of
     payload: exit 0, and the bytes are dropped silently. So is a file
     whose Unicode Path ends in `/`.
  3. For contrast: a Unicode Path that makes a *directory* record collide
     with `0.xml` is refused by the count check. So is a raw case variant
     next to the "directory".
- **How other readers see it:** Info-ZIP `unzip` and Python's `zipfile`
  honour the field and list `0.xml/` plus `0.xml`. `bsdtar` ignores it and
  reads the real `0.xml` plus `zz.xml`. So two tools read two different
  projects from one file, which is exactly N1 scenario 3.
- **The sibling already handles it:** the product reader refuses the same
  trick in a `.knxprod` ("duplicate product ZIP member:
  M-7FF0/Hardware.xml") and a data-bearing directory ("unsafe product ZIP
  member"). `knx products ingest` on the project-archive route goes through
  the project container and accepts both.
- **Why IMPORTANT:** this is the condition-R1 class, met on a path R1's fix
  does not cover. ETS is not known to write such archives. But §159 and the
  manual's "every member name must be unique", and "never silently
  dropped", are contradicted again.

### 3.2 MINOR

- **N8 — Importing a path that does not exist answers `500` (ran).**
  - What happens: `std::fs::read` fails with `ImportFailure::Io`
    (`crates/knx-etsproj/src/lib.rs:181`). `load_failure_kind` maps no `Io`
    variant (`apps/knx-server/src/domain.rs:476-499`), so `routes.rs:693`
    returns `internal`.
  - What *Open* does instead: the same missing path answers
    `422 projectNotOpenable`.
  - In the UI: practical only in a race (a file picked, then removed), but
    it is the N5 class.
- **N9 — The project container does not compare local and central headers
  (ran).** All three cases import with exit 0 and no warning:
  - **(a)** a local-header name (`P-0001/evil.xml`) that differs from the
    central name `P-0001/0.xml`;
  - **(b)** two central records that share one local record;
  - **(c)** a central `P-0001/0.xml` that points at a forged local record,
    while the real one is left unreferenced. The forged project is
    imported.

  How other readers see it: Python's `zipfile` refuses (a) and (c) ("File
  name in directory … and header … differ"). A streaming `bsdtar` lists
  the local names. KNXBench follows the central directory, as APPNOTE
  intends, so nothing is lost *in its own view*. But the archive is
  ambiguous, and the product reader already refuses local/central
  disagreement ("local and central Unicode path fields differ").
- **N10 — Two stale statements (read).**
  - `docs/ALPHA_CANDIDATE.md:53` still lists "Import a non-ZIP file | 500"
    as the smoke result. After N5 it is `422 projectNotImportable`, as
    ALPHA_FINAL_GATES §9 itself records.
  - `apps/knx-server/src/domain.rs:271` says "production always calls
    `AppState::new`". After N4, production calls `with_user_product_db`.

### 3.3 Observations (no finding)

- A lone member that is renamed by a Unicode Path field to `P-0001/0.xml`
  (no real `0.xml` present) is read as the topology document. That follows
  APPNOTE 4.6.9, and nothing is lost. A reader that ignores the field sees
  a project without `0.xml`.
- NFC `é.dat` and NFD `é.dat` are two distinct members and are kept as two
  opaque rows whose names look identical.
- `/P-0001/0.xml`, `P-0001\0.xml`, `P-0001/./0.xml` and a name with an
  embedded NUL are each kept as a separate opaque row. They are not read as
  `0.xml`. Whether ETS (Windows) would normalise any of them onto `0.xml`
  is unknown.
- A 71 KB file whose central directory names one 60 MiB local record 8
  times stays inside the budget: 1,603 MB peak, a 504 MB store. That is the
  disclosed ~3× rule, reached with a smaller file.
- A ZipCrypto member that inflates beyond its declaration is reported as
  "wrong password". That is documented ("a damaged encrypted entry is
  reported the same way").

## 4. Gate table

Revision `863f72ad` (product code = `3ede4817`). Fresh target directory,
three leases, offline namespace, `XDG_DATA_HOME` isolated. Run 17:04–17:50
CEST.

| Command | Exit | Counts (mine) | Dossier §9 |
| --- | --- | --- | --- |
| `npm ci`, `npm run build`, `tsc --noEmit`, `check:flow-study`, `check:theme-fixtures` | 0 ×5 | build: chunk-size warning only | exit 0 |
| `npx vitest run` (offline) | 0 | 2,076 passed in 117 files | 2,076 / 117 |
| `npx playwright test` (Chromium, offline) | 0 | 142 passed | 142 |
| `npm audit` | 0 | 0 vulnerabilities | 0 |
| `cargo fmt --all -- --check` | 0 | | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | | 0 |
| `cargo test --workspace --no-fail-fast` (offline) | 0 | **3,359 passed, 0 failed, 178 ignored**, 192 blocks | 3,359 / 0 / 178, 192 |
| same, `HOME` = empty directory, `XDG_DATA_HOME` unset (N4) | 0 | 3,359 passed, 0 failed, 178 ignored, 192 blocks; the fake `HOME` is still empty afterwards (no `~/.local/share/knx`) | — |
| `xtask check-layering` | 0 | all boundaries ok (449 packages) | ok |
| `xtask check-headers` | 0 | 557 with header, 155 without (ceiling 155), 42 generated | 557/155 |
| `xtask check-anchors` | 0 | 632 links in 299 files | 626 (the candidate has docs-only commits after `3ede4817`) |
| `xtask check-ledger` | 0 | 190 rows | 190 |
| `xtask check-corpus-gates` | 0 | 393 Rust files | 393 |
| `cargo deny check` | 0 | advisories, bans, licenses, sources ok | ok |
| `python3 -m unittest tools.tests.test_run_corpus_tests` | 0 | 4 tests | — |
| `tools/run_corpus_tests.py` without corpus | 2 | refuses: "nothing was run, and that is not a pass" | — |
| `tools/run_corpus_tests.py`, `OriginalData/` and `project_dump.json` linked (removed afterwards) | 0 | **143 of 143 in 31 targets** | 143/143, 31 |
| `xtask check-appimage --artifact-dir <copy>` | 0 | `AppImage ok: version 0.1.0-alpha.4` | ok |
| AppImage contents (extracted) | — | 337 files, 305 MiB. 0 files containing `/home/knxbench`; no `/mnt`, worktree or `OriginalData` path. `KNXBench-Labs` only as the app id `com.knxbench.knxbench-labs`. 500 neutral `/cargo/registry` strings | 337 files, no home path |
| AppImage offline start, the manual's Wayland recipe | alive after 25 s | Window renders the start page with `v0.1.0-alpha.4` (screenshot inspected, not kept: it shows the reviewer's desktop). The only socket is a TCP listener on `127.0.0.1`. The user's real app-data directory is unchanged (mtime) | rendered alpha.4, no non-loopback socket |

## 5. What I did not check

- Anything on a live bus or device. N3 by running: it needs a backup that
  plans against real product data.
- The unmodified AppImage under X11 (no X server here; KL-158). A rebuild
  of the AppImage from `3ede4817`, or a byte comparison with it.
- The owner's 6-mutant sweep. I wrote my own inputs instead.
- The desktop's native GTK dialogs, and the F1 dialog in a browser this
  round (no web change since round 1, which ran it).
- Whether ETS ever writes Unicode Path fields, directory records with
  bytes, zip64 or backslash names, and how ETS resolves the N7/N9 archives.
  I had no ETS to try them against.
- A data-descriptor (`0x08`) variant of N9, and split or multi-disk
  archives.

## 6. Reproduction notes

- **Inputs:** built from `tools/manual_sample_project.py`'s fictional
  sample with a hand-written ZIP writer. The writer controls central and
  local names separately, extra fields per header, declared sizes,
  ZipCrypto (PKWARE traditional, encrypted by the writer itself), zip64
  records and aliased offsets.
- **Peak RSS:** CLI via `wait4` under an 8 GiB `RLIMIT_AS`, spawned from a
  process that holds no archive in memory. Servers via `VmHWM`.
- **Byte identity:** SHA-256 before and after.
- **Corpus:** aggregates only. No name, id, path or value left the census,
  and the census only read the files.
- **Scripts and logs:** kept outside the repository in the local evidence
  directory `ar18-recheck-round2-20261006` (ZIP writer, case runners, gate
  logs, result JSON). No archive, corpus data or screenshot is kept there.

*The reviewer's one-line summary: the letters are now checked by the name
on the envelope and by the name inside — but a letter can still mark itself
"folder" and be filed unread.*
