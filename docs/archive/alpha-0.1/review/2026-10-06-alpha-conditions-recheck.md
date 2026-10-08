# KNXBench Alpha — independent re-check of the AR18 conditions

- **Reviewer:** fresh Claude session started by the user from
  [AR18_RECHECK_BRIEF](AR18_RECHECK_BRIEF.md). I did not write any of the
  fixes, had no history with this code and used no subagents. I changed no
  product code.
- **Date:** 2026-10-06, 15:05–15:45 CEST.
- **Candidate:** `bd691b82126f3fefe4fef4e1f2b81120c72b3888`. This is the
  `origin/main` commit that last changed the brief. `git diff --name-only
  faa3955f bd691b82` lists only `docs/`, `.ai/` and `docs/archive/alpha-0.1/alpha-release-goal.md`
  (verified), so the product code is the code of `faa3955f`.
- **Artifact:** a copy of `KNXBench_0.1.0-alpha.4_amd64.AppImage`. I
  recomputed its SHA-256, `41ad3880…0285f7`, and it matches the brief. The
  binary carries the build stamp `faa3955f`.
- **Environment:** my own worktree. A fresh `CARGO_TARGET_DIR` and my own
  scratch directory. Both leases (and the browser lease) held for every
  Cargo run. No `KNX*` variables set. `XDG_DATA_HOME` isolated. All tests,
  servers and browsers ran in `unshare --net` with loopback only. No
  gateway, no device.

## 1. Verdict

**`READY_WITH_CONDITIONS`**

F1, F4 and M1–M9 hold up against my own inputs. Two exceptions:

- M6 is fixed for *download* but not for its sibling *restore*.
- M8 is fixed only for `AppState::default`.

The gates reproduce green on my target directory with the dossier's counts.
The corpus subset is green, and no real project is refused.

**F2 and F3 are fixed only for the inputs the review used.** One way around
each fix brings the same failure back:

- **N1 (F2):** two members that the `zip` reader *decodes* to the same name
  get past the new byte-level name check. This happens through the Info-ZIP
  Unicode Path field, or through CP437 against UTF-8 names. The import then
  silently swaps or drops bytes again. One such member silently replaces
  the project's own topology document.
- **N2 (F3):** the 512 MiB budget is checked only *after* a protected
  project's nested payload has been fully unpacked. A 12 MB file drove an
  import to 8 GB (my cap). A 1.8 MB file drove the AppImage's own server to
  2.2 GB.

Both are the same class as the findings they come from. So they carry the
same weight: **IMPORTANT, fix or explicitly accept before AR19.**

| # | Condition |
| --- | --- |
| R1 | **N1** — check member identity on the names the reader actually uses, e.g. refuse when the central-directory record count differs from `archive.len()`, plus the case-insensitive check on decoded names in `open_raw`. Do this in the outer archive and in the nested payload. Or: accept it, and qualify "never silently dropped" and KNOWN_LIMITATIONS §159. |
| R2 | **N2** — keep a running total of what the nested payload has unpacked *inside* the loop, before each member is unpacked. Or: accept it and disclose it in §159. |
| R3 | Any code change for R1/R2 reruns the affected gates (knx-etsproj and workspace tests, the corpus subset) and rebuilds the AppImage with the same `--remap-path-prefix` flags. |

The MINOR findings (§3.2) are not conditions.

## 2. F1–F4 and M1–M9

"Ran" means my own inputs against the candidate's binaries, not the owner's
tests.

| Finding | Result | Evidence |
| --- | --- | --- |
| **F1** Open/Import over unsaved edits | **fixed** | **API (ran):** after an edit, import and open each answer `409` kind `projectUnsavedChanges`. The edit stays, also for a broken file, a password-protected file and the user's own `.knxdb`. `discardChanges: true` replaces the project. `"true"` as a string is refused with `422` (edit kept). Password after discard: no password gives `422 projectPasswordRequired` (edit kept). Wrong password gives `422 projectPasswordWrong` (edit kept). Password without discard gives `409`. Password with discard gives `200`. **Late refusal (ran):** a 480 MiB import was started over a clean project, and an edit was made while it ran. The import ends with `409`, the edit survives, and load-progress says `failed`. **UI (ran, production build + real server, Chromium):** a clean project gives no dialog and loads at once. With an edit: the dialog appears, *Cancel* sends nothing, and *Discard* sends `discardChanges: true`. The password retry keeps `discardChanges: true`. A late `409` brings up the same dialog. *Save and open* on a project with no file opens Save As, saves (the edit is in the file) and then imports. |
| **F2** duplicate / case-variant members | **fixed for raw names; bypass → N1** | **Ran, outer archive:** an exact duplicate, an ASCII-case variant and a case variant of `0.xml` are each refused with `DuplicateEntry`. **Ran, ZipCrypto-protected nested payload** (hand-encrypted, not stored plain): exact duplicate refused, case variant refused, CP437/UTF-8 pair refused (by the loop's own `find`). A pair of names in a Unicode Path field is **imported with one member lost** (N1). |
| **F3** unbounded memory | **fixed for the outer archive; bypass → N2** | **Ran (peak RSS via `wait4`, 8 GiB cap):** a member that declares 100 B and inflates to 1 GiB is refused, peak 17 MB (was 2,078 MB). One that declares 60 MiB and inflates to 1 GiB is refused, 74 MB. 24 × 60 MiB honest members are refused at open, 17 MB (was 3,015 MB). `knx products ingest` on the lying archive is refused, 17 MB (was 1,055 MB). 8 × 60 MiB (480 MiB, inside the budget) is imported, peak **1,608 MB** (3.3× the budget), with a 504 MB `.knxdb`. A protected project with plain nested members, **12 MB file: 8,172 MB, then "out of memory"** (N2). |
| **F4** `--store` overwrites | **fixed** | **Ran, SHA-256 before/after:** an existing store without `--replace` is refused and byte-identical. The same holds for a symlink to it, a dangling symlink (target not created) and a directory. `--replace` overwrites, and all 19 domain tables plus opaque and manifest are rewritten (read: `DELETE_ALL_TABLES`). `--replace` onto the source `.knxproj` itself, or onto a text file, gives "not a database", file byte-identical. `--replace` without `--store` is a usage error. |
| **M1a** failed import touches the store | **fixed** | Ran: a broken input into a new path leaves no file. Into an old store and into a v3 store with `--replace`, the file stays byte-identical. A wrong password with `--replace` leaves it byte-identical too. |
| **M1b** foreign SQLite gets tables | **fixed** | Ran with `user_version` 0, with 7, and with an alien `schema_meta` (`created_by = other-app`). `doc-export`, `ga-export`, `import --replace` and GUI *Open* each refuse ("not created by KNXBench", `422 projectNotOpenable`). All files stay byte-identical. |
| **M1c** empty / v3 store via Open | **fixed** | Ran: a v3 store, an empty v10 store and a zero-byte file. `doc-export`, `ga-export`, `diff` and *Open* each refuse with "no project has been saved". All stay byte-identical. A refused *Open* with `discardChanges` keeps the open project and its edits. |
| **M1d** mistyped path | **fixed** | Ran: `doc-export`, `ga-export`, `diff`, `ga-import --dry-run`, `device readiness` and *Open* each report the missing file. Nothing is created. |
| Legitimate files still open | **yes** | Ran: a normal v10 store, a read-only (0444) store, a new empty project that was saved and reopened, and its `doc-export`. Read: every KNXBench file carries the marker from v1 (`migrate_v0_to_v1`), and the migration chain existed from the first store commit. |
| **M2** three-transaction save | **fixed** | Ran: *Save As* over an existing project file, with the server killed by SIGKILL after 0.4, 1.0, 2.0 and 4.0 s. The file was always wholly old or wholly new: domain, opaque and manifest agree, and `integrity_check` is ok. Earlier kills left a hot journal that rolled back cleanly. |
| **M3** second project part | **fixed** | Ran: `P-0002` with full content, folder-only and `.zip`-only parts, and a lone `P-0009.signature`. Each gets its own "additional project part" line. |
| **M4** wrong-password message | **fixed** | Ran: one clean line on CLI stderr. |
| **M5** documentation drift | **not re-audited** | I read only §159 and the manual passages I quote. |
| **M6** never-connected download | **fixed for download; not for restore (N3)** | Corpus route test `a_tunnel_that_never_opens_is_recorded_as_failed_with_nothing_written` passed in my corpus run. Read: CLI `main.rs:2284-2293` and server `device_download_routes.rs:408-420` call `record_never_connected`. `TunnelClient::connect` sends only a CONNECT_REQUEST before it fails (`crates/knx-net/src/client.rs:331-389`), so "written: no" is true. **Not run by CLI:** none of the fictional sample's devices plans a download (all `unsupported`), and running it against the corpus project would have meant pointing the CLI at a private file that it might migrate in place. |
| **M7** corpus tests in no gate | **fixed** | Ran: `tools/run_corpus_tests.py` without `OriginalData/` exits 2 with "nothing was run, and that is not a pass". With the corpus it reports 143 of 143 in 31 targets. Its own unit tests: 4/4. 143 + 35 non-corpus = 178 ignored, which matches the workspace run. |
| **M8** tests read the developer's product DB | **partly fixed (N4)** | `AppState::default` opens none (read, and its test passes). But ran: with `XDG_DATA_HOME` unset and `HOME` pointing to an empty fake home, `cargo test -p knx-server --test http_fs_routes` and `--test http_project_routes` each **created `$HOME/.local/share/knx/products.sqlite`**. `--test cli_import` did not. |
| **M9** `npm audit` | **fixed** | Ran: 0 vulnerabilities. |

### Regressions the brief asked about

- **Real projects refused?** No. A read-only census of every `.knxproj` in
  `OriginalData/` (3 files) through the changed CLI path, in memory only:
  - all 3 are imported. One exits 2 because of report errors in its own
    data, not because of a refusal.
  - none is refused as a duplicate, as too large or as inflated.
  - none has non-ASCII names or the UTF-8 flag.
  - the largest declares 22.3 MiB in total.
  - all files are unchanged (size and mtime).

  The corpus subset passed 143/143.
- **Scripts that re-import into one store:** they now fail, with exit 1 and
  a message that names `--replace`. This is intended, and it is documented
  in the CLI usage and `10-command-line.md:58`.
- **409 against 409:** they do not collide. While a load runs, a second
  import gets the old "a project load is already running" `409`. That
  `409` has no `kind`, even with unsaved edits and even with
  `discardChanges`, and the client tells the two apart by `kind`
  (`api.ts`, `isUnsavedProjectConflict`). The *New project* `409` still
  carries no `kind`. It always lacked one, and the client does not need it.
- **Dialog on a clean project:** no dialog appears.

## 3. New findings

### 3.1 IMPORTANT

#### N1 — Names the `zip` reader decodes to one name still collapse or substitute members silently (ran)

- **Where:** `crates/knx-etsproj/src/container.rs:699-724`
  (`check_member_names` compares *raw* name bytes, ASCII-folded). The
  inventory and every lookup use the *decoded* name (`open_raw`
  `:514-525`, `find` `:604-608`). `zip` 8.6 decodes non-UTF-8 names as CP437
  and replaces the name with the Info-ZIP Unicode Path field (0x7075) when
  its CRC matches (`zip-8.6.0/src/read.rs:520-530`, `:706-713`). It then
  keys members by that decoded name. The nested loop (`:310-330`) only sees
  what `inner.len()` still holds after that collapse.
- **Scenarios (CLI `knx import --no-product-db`, fictional sample, ran):**
  1. Raw names `…/p1.dat` ("first-upath") and `…/p2.dat` ("second-upath"),
     both with a Unicode Path field of `…/dup.dat`: exit 0, 0 errors, one
     opaque row with the second member's bytes. The first is gone, with no
     report line. The same happens inside a ZipCrypto-protected nested
     payload.
  2. `…/\x82.dat` without the UTF-8 flag (CP437 "é") plus `…/é.dat` with
     the flag: exit 0, **two opaque rows, both with the UTF-8 member's
     bytes**. As in the original F2 case 1, each row's SHA-256 matches its
     wrong bytes.
  3. **The topology document itself:** a member `P-0001/zz.xml` carrying a
     Unicode Path field of `P-0001/0.xml` and a changed device name: exit 0,
     0 errors, 0 unsupported. The imported project contains the attacker's
     device name. The real `0.xml` was never read and is not retained,
     because it is regenerated. Through the AppImage's own server: `200`,
     and the project shows the substituted device.
  4. For contrast: invalid UTF-8 names that both decode to U+FFFD fail
     closed ("specified file not found"), and two distinct non-ASCII names
     import correctly.
- **Why IMPORTANT:** it is F2 with a different spelling. ETS is not known to
  write such archives. But README.md:34, `03-troubleshooting.md:121`,
  `04-faq.md:32` and KNOWN_LIMITATIONS §159 ("every member name must be
  unique") claim what scenarios 1–3 contradict. Scenario 3 makes another
  tool and KNXBench read *different projects* from the same file.

#### N2 — The 512 MiB budget is checked after a protected payload is fully unpacked (ran)

- **Where:** `container.rs:310-490`. The nested loop decompresses every
  member into `container.decrypted`, with a 64 MiB limit per member and no
  running total. `check_total_size` runs once at `:493`, after the loop.
  Plain (unencrypted) members in the nested payload go through the same
  loop, and they need no correct password (any password reaches them).
- **Scenarios (ran):**
  - CLI, 8 GiB address-space cap: a 1.8 MB `.knxproj` with 30 × 60 MiB
    nested members reached a peak of 1,936 MB, and only then the budget
    refusal came. With 40 ZipCrypto-encrypted members, a 2.5 MB file
    reached 2,577 MB. With 200 plain members, a **12 MB file reached
    8,172 MB, then "out of memory"**. Without the cap that is about 12 GB.
    The arithmetic limit is set by the 64 MiB nested blob at deflate's
    ~1000:1 ratio, i.e. tens of GB.
  - The AppImage's own server (desktop process), the 1.8 MB file: `VmHWM`
    went from 241 MB to 2,206 MB before the `500` refusal.
- **Mitigation that already exists:** thanks to F1, an import over unsaved
  edits is refused before the file is read. So a crash now costs unsaved
  edits only after the user chose *Discard*. The server is still shared by
  every logged-in user, and its upload limit is 100 MiB
  (`fs_routes.rs:33`).

### 3.2 MINOR

- **N3 — `knx device restore` still records a never-opened tunnel as
  `unknown` (read).** `apps/knx-cli/src/main.rs:2778-2784` returns on a
  connect failure without `record_never_connected`. The guard's drop then
  writes `unknown` (`crates/knx-app/src/commissioning_activity.rs:667-671`).
  This is the M6 defect on the sibling path. Not run: it needs a real
  backup file.
- **N4 — Server integration tests still create and migrate the developer's
  product database (ran).** `AppState::new` opens
  `knx_productdb::default_path()` with `open_and_migrate`
  (`apps/knx-server/src/domain.rs:196-199`), and 10 test files in
  `apps/knx-server/tests/` build their state with it. A plain
  `cargo test --workspace` without `XDG_DATA_HOME` therefore touches
  `~/.local/share/knx/products.sqlite`. The gate scripts set
  `XDG_DATA_HOME`, so they hide this, but a developer's run does not.
- **N5 — Refused crafted archives answer HTTP 500 (ran).**
  - Which refusals: `DuplicateEntry`, `TooLarge` and "inflated to more than
    the declared" reach the client as `500` internal errors.
    `load_failure_kind` (`apps/knx-server/src/domain.rs:461-472`) maps only
    the password errors, and `routes.rs:693` turns everything else into
    `internal`.
  - Why it matters: the new refusals are input errors, the same wart as the
    disclosed non-ZIP `500` (ALPHA_CANDIDATE §3), which covers only the
    non-ZIP case.
  - The message text itself is clear.
- **N6 — After a browser reload, the web UI shows the welcome page while
  the server still holds the modified project (ran).**
  - What happens next: an Open or Import from there is correctly refused,
    and the unsaved-changes dialog appears.
  - The problem: *Save and open* and *Discard* then act on a project the
    user cannot see.
  - What it is not: no data is lost. This is a UX gap, not a guard failure.

### 3.3 Observations (no finding)

- An import inside the budget still peaks at about 3.3× the declared total
  (480 MiB → 1.6 GB). The §159 statement that the budget "bounds what an
  import holds in memory from the archive" is true for the archive bytes,
  not for the process.
- `--store`: the existence check and the later create are not atomic. Two
  concurrent imports to the same new path would both pass. This needs two
  processes racing; not a practical risk.
- Reload aside, every F1 path I tried keeps the edit unless an explicit
  `discardChanges: true` was sent.

## 4. Gate table

Revision `bd691b82` (product code = `faa3955f`). Fresh target directory,
leases held, offline namespace, `XDG_DATA_HOME` isolated. Run 15:08–15:28
CEST.

| Command | Exit | Counts (mine) | Dossier §8 |
| --- | --- | --- | --- |
| `npm ci`, `npm run build`, `tsc --noEmit`, `check:flow-study`, `check:theme-fixtures` | 0 ×5 | build: chunk-size warning only | exit 0 |
| `npx vitest run` (offline) | 0 | 2,076 passed in 117 files | 2,076 / 117 |
| `npx playwright test` (Chromium, offline) | 0 | 142 passed | 142 |
| `npm audit` | 0 | 0 vulnerabilities | 0 |
| `cargo fmt --all -- --check` | 0 | | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | | 0 |
| `cargo test --workspace --no-fail-fast` (offline) | 0 | **3,350 passed, 0 failed, 178 ignored**, 191 blocks | 3,350 / 0 / 178, 191 |
| `xtask check-layering` | 0 | knx-core and knx-etsproj boundaries ok | ok |
| `xtask check-headers` | 0 | 556 with header, 155 without (ceiling 155), 42 generated | 556/155 |
| `xtask check-anchors` | 0 | 626 links in 298 files | 620 (the candidate has 6 more docs links than `faa3955f`) |
| `xtask check-ledger` | 0 | 190 rows | 190 |
| `xtask check-corpus-gates` | 0 | 392 Rust files | 392 |
| `cargo deny check` | 0 | advisories, bans, licenses, sources ok | ok |
| `python3 -m unittest tools.tests.test_run_corpus_tests` | 0 | 4 tests | 4 |
| `tools/run_corpus_tests.py` without corpus | 2 | refuses: "nothing was run" | — |
| `tools/run_corpus_tests.py` with corpus (symlink, removed afterwards) | 0 | **143 of 143 in 31 targets** | 143/143, 31 |
| `xtask check-appimage --artifact-dir <copy>` | 0 | `AppImage ok: version 0.1.0-alpha.4` (config, name, version and exec bit only) | ok |
| AppImage contents (extracted, 337 files, 305 MiB) | — | 0 files containing `/home/knxbench`. No worktree, corpus or `/mnt` path. `KNXBench-Labs` appears only as the public app identifier `com.knxbench.knxbench-labs`. 483 neutral `/cargo/registry` strings | 337 files, no home path |
| AppImage offline start, unmodified (X11 hook) | 101 | `Failed to initialize GTK`: no reachable X server in the namespace (accepted boundary KL-158, as documented) | — |
| AppImage offline start, the manual's Wayland recipe | alive after 25 s | Window renders the start page with `v0.1.0-alpha.4`. The only socket is a TCP listener on `127.0.0.1`. The user's real app-data directory is untouched (HOME redirected) | rendered alpha.4, no non-loopback socket |

## 5. What I did not check

- Anything on a live bus or device. M6 and N3 on real or simulated hardware
  beyond the corpus route test. The CLI M6 path was only read.
- A rebuild of the AppImage from `faa3955f`, or a byte comparison of it
  with the evidence copy. The CI workflow and the Docker image.
- The owner's mutation sweep (17 + 16 mutants). I did not rerun or audit
  it; I wrote my own inputs instead.
- M5 beyond §159 and the manual passages I quote. The rest of the
  documentation-drift audit was not repeated.
- The desktop's native GTK dialogs: Quit, native Save As, the replace
  dialog under WebKitGTK. The F1 UI checks ran in Chromium against the
  production web build.
- An old KNXBench store (v4–v9) *with* a saved project. Its in-place
  upgrade is accepted (§157); I only checked v3/empty/foreign refusals and
  current-version files.
- Other zip-level identity tricks beyond the three I built (Unicode Path,
  CP437 against UTF-8, lossy UTF-8). For example, local-header names that
  differ from central-directory names, or zip64 records. Directory entries
  that collide with file names.
- Whether ETS ever writes a multi-part archive or Unicode Path fields.

## 6. Reproduction notes

Every adversarial input is built from `tools/manual_sample_project.py`'s
fictional sample:

- **Name and member variants:** Python `zipfile`, or a small hand-written
  ZIP writer for headers, flags and extra fields.
- **ZipCrypto entries:** the writer encrypts them itself (PKWARE
  traditional, password used as is), so the nested payloads are really
  encrypted.
- **Peak RSS:** from `wait4` under an 8 GiB `RLIMIT_AS`.
- **Byte identity:** SHA-256 before and after.
- **Corpus:** only aggregates were collected. No names, ids, paths or
  values left the census, and the census only read the files.

The scripts live in the reviewer's scratch directory, not in the repository.

*The reviewer's one-line summary: the guards now check the names on the
envelope — the reader still delivers by the name on the letter inside.*
