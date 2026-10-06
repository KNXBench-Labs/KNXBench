# KNXBench Alpha — independent re-check, round 3 (N7–N10)

- **Reviewer:** a fresh Claude session started by the user from
  [AR18_RECHECK_BRIEF §Round 3](AR18_RECHECK_BRIEF.md#round-3-n7n10). I wrote
  none of the fixes and neither of the earlier re-checks. I used no
  subagents and changed no product code.
- **Date:** 2026-10-06, 22:20–23:05 CEST.
- **Candidate:** `8a79791b`, the `origin/main` commit that last changed the
  brief. `git diff --name-only 754a66dd 8a79791b` lists only `.ai/`,
  `alpha-release-goal.md` and `docs/` (verified), so the product code is the
  code of `754a66dde05db9dd522720baa67d6bbc39290dbb`.
- **Artifact:** a copy of `KNXBench_0.1.0-alpha.4_amd64.AppImage`. I
  recomputed its SHA-256, `37015eb6…2a58`, and it matches the brief. Its
  server reports `0.1.0-alpha.1+g754a66dd`; the desktop binary carries the
  stamp `754a66dd`.
- **Environment:** my own detached worktree, a fresh `CARGO_TARGET_DIR`, my
  own scratch directory. The three leases (alpha gate, workspace gates,
  browser fixture) were held for the whole gate run. No gateway or device
  variables. `XDG_DATA_HOME` isolated. Every test, CLI run, server and the
  AppImage ran in `unshare --net` with loopback only. No gateway, no device.

## 1. Verdict

**`READY_WITH_CONDITIONS`**

The round-2 archives are all refused now: every N7 scenario (outer, nested,
forged member first or last, Unicode Path in both headers), every
data-bearing or file-named directory, every local/central *name*
disagreement, and two central records sharing one local record. N8 and N10
are fixed (one stale line left, §3.2). The gates reproduce the dossier's
counts. The corpus subset passes, and no real project or product package is
refused.

**But R4 is met only for directories whose central record admits its
bytes.** The brief asked for "whatever else would let one record look like
two things". That is still possible:

- **N11 (IMPORTANT, same class as N1/N7):** the project container compares
  only the *name* in the local header with the central record. It does not
  compare sizes, CRC, method, flags, the data descriptor or the local
  Unicode Path field. The `zip` reader takes every size from the central
  record. So the round-2 attack works again with one more lie: give the real
  `P-0001/0.xml` a central Unicode Path to `P-0001/old/` **and central sizes
  and CRC of 0**, keep its local header and bytes truthful, and give another
  member a Unicode Path to `P-0001/0.xml`. The "directory" passes the new
  data check (`size() == 0`), its name is no file's name, and the import:
  - reads the forged member as the topology document;
  - drops the real one (13,254 bytes) without a report line;
  - exits 0 with 0 errors, through the CLI, the release server and the
    AppImage's own server (`200`), outer or nested, with or without a data
    descriptor (`0x08`).

  No directory is even needed: renaming the real `0.xml` to a *file*
  `P-0001/BinaryData/old.bin` whose central record says "stored, 0 bytes"
  gives the same result, with an empty opaque row where the real bytes
  were. `bsdtar` reads the real `0.xml` from the same file.

The product reader, the sibling the round-2 condition pointed to, refuses
exactly this ("local and central CRC or sizes differ"). The corpus shows that
the stricter rule costs nothing: 0 of 1,517 real records (outer and
unencrypted nested) have any local/central disagreement.

| # | Condition |
| --- | --- |
| R6 | **N11** — in `check_decoded_names` (`crates/knx-etsproj/src/container.rs:788-842`), compare each local header with its central record the way `crates/knx-productdb/src/package.rs:1088-1127` does: flags and method equal; CRC and both sizes equal, or zeros plus a matching data descriptor when bit 3 is set; local and central Unicode Path fields equal. This covers the outer archive and the nested payload, because both call the same function. Or: accept it, and qualify §159's "a record's parts must agree" and the manual's "never silently dropped". |
| R7 | Any code change for R6 reruns the knx-etsproj and workspace tests and the corpus subset, and rebuilds the AppImage with the same `--remap-path-prefix` flags. |

The MINOR findings (§3.2) are not conditions. N6 still should not block
(§2).

**Does anything still open block AR19?** Only N11, through R6 or an explicit
acceptance by the user. It is the third bypass of the same rule (N1 → N7 →
N11), and the product reader shows the complete check already exists in the
codebase, so I would fix it rather than accept it. Nothing else should
block: N6 is a UX gap with the F1 guard intact; N12 and N13 are MINOR; the
~3× in-budget memory peak (§159) and the in-place upgrade of older stores
(§157) are disclosed.

## 2. N7–N10

"Ran" means my own inputs against the candidate's release binaries
(`knx`, `knx-server`) or the AppImage, not the owner's tests.

| Finding | Result | Evidence |
| --- | --- | --- |
| **N7** a Unicode Path turns `0.xml` into a "directory" (R4) | **fixed as specified; bypass → N11** | **Ran, round-2 archives:** the real `0.xml` as `0.xml/` plus a forged `0.xml`, forged first or last, Unicode Path in the central header or in both, outer or in a ZipCrypto payload → each refused with `InconsistentRecord … a directory record carries data`. `Project.xml` as a directory → refused. A data-bearing directory (outer and nested), a file turned into a directory by its Unicode Path → refused. A directory beside a file of the same name, or its case variant (`0.XML/`), or `BinaryData` file + `BinaryData/` directory → refused (`a directory has the same name as a file`). A Unicode Path that makes a directory collide with `0.xml` → refused by the count check. Through the release server and the AppImage: `422 projectNotImportable`. **Not covered:** a directory whose *central* sizes are 0 while its local record carries the bytes → N11 |
| **N8** importing a missing path answers `500` | **fixed** | **Ran (release server and AppImage):** `422 projectNotImportable`, message `No such file or directory (os error 2)`. Over an unsaved edit: without `discardChanges` → `409 projectUnsavedChanges`; with it → `422`, and the edit survives (`is_modified` still true, the renamed installation still there). *Open* of a missing file: `409` / `422 projectNotOpenable`, as before. The other `409` (a load already running) still has no `kind`, so the two stay apart |
| **N9** local and central headers not compared | **fixed for names only → N11** | **Ran:** a local name differing from the central one (different length, same length), with and without a data descriptor (`0x08`) → refused. Two central records sharing one local record → refused (`al1`, and both round-2 alias bombs, which now stop at 10 MB instead of reaching 1.6 GB). A central `0.xml` pointing at a forged local record named `hidden.xml` → refused. **Accepted silently (ran):** local CRC/sizes differ from the central ones; local method differs; local flag `0x08` without the central one; a data descriptor whose CRC and sizes differ from the central record; a local Unicode Path field that differs from the central one. Each of these alone leaves KNXBench's own view intact (the real device is read), but together with a size lie they hide bytes (N11) |
| **N10** two stale statements | **fixed, one residue → N13** | Read: `apps/knx-server/src/domain.rs:271` now names `with_user_product_db`. `docs/ALPHA_CANDIDATE.md:53` carries the `422` annotation. But `docs/ALPHA_CANDIDATE.md:119` (§6) still says "the 500 status for malformed files (unchanged, see §3)" |
| **N6** reload shows the welcome page over a held project | **not changed; should not block** | Unchanged since round 2 (UI owner). Same reasoning: a UX gap, not a guard failure. F1 holds over a real edit (row N8 above) |

### Regressions the brief asked about

- **Corpus subset:** `tools/run_corpus_tests.py` with `OriginalData/` and
  `project_dump.json` linked (removed afterwards): **143 of 143 in 31 targets** (§4).
- **Real projects (read-only census, aggregates only):**
  - **3 `.knxproj`:** 2 exit 0. 1 exits 2 ("imported with report errors" in
    its own data), as in rounds 1 and 2. None is refused for identity,
    budget, size or an inconsistent record.
  - **The same 3 through `knx products ingest`** (the project-archive
    route): 3/3 exit 0.
  - **103 `.knxprod`:** 103/103 ingested.
  - Every corpus file is unchanged (size, mtime and SHA-256 compared).
- **Empty ETS6 directory records:** the corpus has 126 directory records,
  all stored with 0/0 bytes. None is refused. My synthetic stored empty
  directories (`P-0001/`, `P-0001/BinaryData/`) import, and so does a
  directory named `/`. An empty directory written *deflated* (a 2-byte empty
  stream, as Java's `ZipOutputStream` writes them) is refused → N12.
- **Structural census of the corpus (aggregates only), for R6:** 106 outer
  archives with 1,404 records, plus 7 unencrypted nested archives with 113
  records. 0 data descriptors, 0 zip64 extras, 0 local/central name, flag,
  method, CRC or size disagreements. **2 nested records carry an Info-ZIP
  Unicode Path field**, in both headers, consistent with the raw CP437 name
  and with a valid CRC. So real ETS archives do use the field that N1, N7 and
  N11 abuse. An R6 check that compares local and central fields would have
  refused none of the 1,517 records.
- **Data descriptors in general:** a sample with every member using `0x08`,
  with and without the descriptor signature, outer and nested → imports
  normally.
- **Prefixed archives:** a 4 KiB self-extractor-style prefix still imports.
- **Zip64:** a legitimate all-zip64 archive and a zip64-EOCD-only archive
  import; a lying zip64 size (outer and nested) and a 2⁶³ declaration are
  refused.
- **Bombs (peak RSS via `wait4`, 8 GiB cap, CLI):** 30 × 60 MiB nested,
  1.8 MB file: 11 MB, refused. 40 ZipCrypto members: 12 MB. 200 members,
  12 MB file: **31 MB**. Outer 420 + nested 120 MiB: 10 MB. A nested member
  that declares 100 B and inflates to 1 GiB: 10 MB. The 71 KB alias bombs
  that round 2 saw reach 1.6 GB inside the budget: now refused at 10 MB.
  Inside the budget, 8 × 60 MiB still peaks at 1,603 MB (disclosed, §159).
  Release server `VmHWM` 37 MB after the 12 MB file; AppImage desktop
  process 244 MB at start, 261 MB after both nested bombs.
- **Round-1 N1 set, rerun:** every collision, CP437/UTF-8 and `0.xml`
  substitution case is still refused, outer and nested.

## 3. New findings

### 3.1 IMPORTANT

#### N11 — A record whose central sizes are 0 hides its local bytes; with a Unicode Path the real `0.xml` is replaced again (ran)

- **Where:** `crates/knx-etsproj/src/container.rs`:
  - `:813-824`: the local header is read only for its name; flags, method
    (`header[8..10]`), CRC and sizes (`header[14..26]`), the descriptor and
    the local extra field are never compared;
  - `:825-828`: "carries data" is judged on the central `size()` and
    `compressed_size()` only;
  - `:829` and `:836`: the file/directory clash trims only `/`, while
    `zip`'s `is_dir()` also treats a trailing `\` as a directory
    (`zip-8.6.0/src/spec.rs:1000`). This one is harmless on its own (such a
    directory is empty), but it means the clash check is not what stops
    N11 either.

  `zip` 8.6 reads every member with the central `compressed_size` and uses
  the local header only to skip its name and extra field
  (`zip-8.6.0/src/types.rs:235-276`).
- **Scenarios (CLI `knx import --no-product-db --store`, fictional sample):**
  1. Real `P-0001/0.xml`: central Unicode Path `P-0001/old/`, central CRC
     and sizes 0, local header and deflated bytes truthful. Raw
     `P-0001/zz.xml` with a Unicode Path to `P-0001/0.xml` and a changed
     device name:
     - exit 0, 0 errors;
     - the stored project's first device is `FORGED DEVICE NAME`;
     - the real `0.xml` is gone, and no report section mentions it.

     The same happens:
     - when the forged member comes first;
     - when the "directory" is named `P-0001/0.xml\` (a backslash);
     - with a data descriptor (`0x08`) on the real record;
     - inside a ZipCrypto-protected payload, with and without the
       descriptor;
     - through the release server and the AppImage's own server (`200`,
       and the project shows the forged device);
     - through `knx products ingest` on the project-archive route (exit 0).
  2. **No directory needed:** the real `0.xml` renamed by a Unicode Path to
     `P-0001/BinaryData/old.bin`, central "stored, 0 bytes": exit 0, the
     forged project is imported, and the report lists an empty opaque
     `old.bin`. Release server and AppImage: `200`, forged.
  3. A directory record `P-0001/BinaryData/hidden/` with 4,500 bytes in its
     local record and central sizes 0: exit 0, the bytes are dropped
     silently (outer and nested, with and without a descriptor). Even
     without any rename, a plain member whose central record says "stored,
     0 bytes" while its local record holds 3,200 bytes is stored as an empty
     opaque row.
- **How other readers see scenario 1:** Python's `zipfile` and Info-ZIP
  `unzip` honour the Unicode Path field and the central sizes, so they agree
  with KNXBench (forged). `bsdtar` ignores the field and lists
  `P-0001/0.xml` (13,254 bytes) and `P-0001/zz.xml`; it extracts the real
  `0.xml`, streaming cleanly and seekable with a delayed error exit. So two tools read two different projects from one file — the
  round-2 N7 result, reached with one more lie.
- **The sibling already refuses it:** the same trick in a `.knxprod`
  (`Hardware.xml` hidden as a 0-byte directory plus a forged twin), and a
  data-bearing directory with central sizes 0, are each refused by the
  product reader with "invalid product ZIP: local and central CRC or sizes
  differ".
- **Why IMPORTANT:** the same class as N1 and N7, on a path that R4's fix
  does not cover. The §159 update promises "a record's parts must agree",
  and the manual promises "never silently dropped"; both are contradicted.
  ETS is not known to write such archives (the corpus has none), but the
  corpus shows ETS does write Unicode Path fields, so this is not a field
  nobody uses.

### 3.2 MINOR

- **N12 — An empty directory written deflated is refused (ran).**
  - What happens: a directory record with method 8 and the 2-byte empty
    deflate stream (`compressed_size` 2, `size` 0) fails with
    "inconsistent archive record …: a directory record carries data"
    (`container.rs:826`, which also tests `compressed_size()`).
  - Who writes this: Java's `ZipOutputStream` and the `jar` tool do by
    default. ETS does not (126 of 126 corpus directory records are stored
    0/0), so it bites only an archive repacked with such a tool.
  - The product reader accepts the same record in a `.knxprod`, so the two
    readers disagree on one shape, and the message says "carries data" for
    a directory that carries none. With R6 in place, checking the
    uncompressed size (or reading the payload and requiring it empty) would
    be enough.
- **N13 — One stale statement left (read).** `docs/ALPHA_CANDIDATE.md:119`
  (§6 "Not covered") still lists "the 500 status for malformed files
  (unchanged, see §3)". After N5 that is `422 projectNotImportable`, as the
  same file's §3 update says. The smoke table's "Open a missing file | 400"
  row (`:52`) is a historical record but, unlike the row below it, has no
  note that *Open* now answers `422 projectNotOpenable`.

### 3.3 Observations (no finding)

- **Unreferenced local records.** A real `P-0001/0.xml` local record left
  out of the central directory, followed by a forged local record with the
  same name that the central directory does point at: KNXBench, Python's
  `zipfile` and `unzip` read the forged one; a streaming `bsdtar` lists two
  `0.xml` and extracts both. This follows APPNOTE (the central directory is
  authoritative), and the product reader does not check gaps either. A
  .NET-based reader, as ETS is, also reads the central directory. Listed
  because R6 does not cover it.
- Round-2 observations still hold: a lone member renamed to `0.xml` by a
  Unicode Path (no real `0.xml`) is read as the topology; NFC/NFD twins are
  two rows; `/P-0001/0.xml`, `P-0001\0.xml`, `P-0001/./0.xml` and a name with
  a NUL stay separate opaque rows; a Unicode Path only in the local header
  is ignored (the central name wins).
- A ZipCrypto member that inflates beyond its declaration is reported as
  "wrong password", as documented.

## 4. Gate table

Revision `8a79791b` (product code = `754a66dd`). Fresh target directory,
three leases, offline namespace, `XDG_DATA_HOME` isolated. Run 22:23–22:46
CEST.

| Command | Exit | Counts (mine) | Dossier §10 |
| --- | --- | --- | --- |
| `npm ci`, `npm run build`, `tsc --noEmit`, `check:flow-study`, `check:theme-fixtures` | 0 ×5 | build: chunk-size warning only | exit 0 |
| `npx vitest run` (offline) | 0 | 2,076 passed in 117 files | 2,076 / 117 |
| `npx playwright test` (Chromium, offline) | 0 | 142 passed | 142 |
| `npm audit` | 0 | 0 vulnerabilities | 0 |
| `cargo build --release -p knx-cli -p knx-server` | 0 | binaries for the case runs | — |
| `cargo fmt --all -- --check` | 0 | | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | | 0 |
| `cargo test --workspace --no-fail-fast` (offline) | 0 | **3,367 passed, 0 failed, 178 ignored**, 192 blocks | 3,367 / 0 / 178, 192 |
| `xtask check-layering` | 0 | all boundaries ok | ok |
| `xtask check-headers` | 0 | 557 with header, 155 without (ceiling 155), 42 generated | 557/155 |
| `xtask check-anchors` | 0 | 638 links in 300 files | 634 (the candidate has docs-only commits after `754a66dd`) |
| `xtask check-ledger` | 0 | 190 rows | 190 |
| `xtask check-corpus-gates` | 0 | 393 Rust files | 393 |
| `cargo deny check` | 0 | advisories, bans, licenses, sources ok | ok |
| `python3 -m unittest tools.tests.test_run_corpus_tests` | 0 | 4 tests | — |
| `tools/run_corpus_tests.py` without corpus | 2 | refuses: "nothing was run, and that is not a pass" | — |
| `tools/run_corpus_tests.py`, `OriginalData/` and `project_dump.json` linked (removed afterwards) | 0 | **143 of 143 in 31 targets** | 143/143, 31 |
| `xtask check-appimage --artifact-dir <copy>` | 0 | `AppImage ok: version 0.1.0-alpha.4` | ok |
| AppImage contents (extracted from a second copy) | — | 337 files, 305 MiB. 0 files containing `/home/knxbench`; no `/mnt`, worktree or `OriginalData` path. Build stamp `754a66dd` in `knx-desktop` | 337 files, no home path |
| AppImage offline start, the manual's Wayland recipe | alive after 25 s | Window renders the start page with `v0.1.0-alpha.4` (screenshot inspected, not kept: it shows the reviewer's desktop). Server `0.1.0-alpha.1+g754a66dd`. The only socket is a TCP listener on `127.0.0.1`. The user's real app-data directory is unchanged (mtime) | rendered alpha.4, no non-loopback socket |

## 5. What I did not check

- Anything on a live bus or device.
- The unmodified AppImage under X11 (no X server here; KL-158). A rebuild
  of the AppImage from `754a66dd`, or a byte comparison with it.
- The owner's 5-mutant sweep. I wrote my own inputs instead.
- The web UI and the desktop's native dialogs (no web change since round 1).
- How ETS (or .NET's `ZipArchive`) resolves the N11 archives — in
  particular whether it honours the Unicode Path field and whether it
  compares local and central sizes. I had no ETS to try them against.
- Split or multi-disk archives, AES-encrypted members, and encrypted nested
  payloads of real projects (the census inspected only unencrypted ones).

## 6. Reproduction notes

- **Inputs:** built from `tools/manual_sample_project.py`'s fictional
  sample with round 2's hand-written ZIP writer, extended so that the local
  header, the central record and the data descriptor can each carry their
  own flags, method, CRC and sizes.
- **Peak RSS:** CLI via `wait4` under an 8 GiB `RLIMIT_AS`, spawned from a
  process that holds no archive in memory. Servers via `VmHWM`.
- **Byte identity:** SHA-256 before and after.
- **Corpus:** aggregates only. No name, id, path or value left the census,
  and the census only read the files.
- **Scripts and logs:** kept outside the repository in the local evidence
  directory `ar18-recheck-round3-20261006` (ZIP writer, case runners, gate
  logs, result JSON). No archive, corpus data or screenshot is kept there.

*The reviewer's one-line summary: the letter may no longer call itself a
folder — unless the envelope also swears the folder is empty.*
