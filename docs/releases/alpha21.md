# Alpha 21 — pre-release

Alpha 21 brings everything since `v0.1.0-alpha.20`: clearer feedback about
what still needs programming, comparisons with the last completed download,
safer batch programming, more responsive saved-project editing, and more
product detail. The amber dots now have a memory, not just an opinion.

**Before upgrading, close KNXBench and back up your projects and data.** Alpha 20
refuses project files and product databases converted by Alpha 21.

## At a glance

| Area | What is new |
|---|---|
| Pending changes | Device and tab marks, a pending-device panel, and **Download all pending…** with a review and explicit confirmation |
| Last download | Compare parameter values, links and flags with the last completed KNXBench download; navigation points to changed sections |
| Communication objects | Links first, compact toolbar, clearer Send/Receive choices and correct combined sending/receiving associations |
| Editing | Ordinary editor reads and edits no longer queue behind a project save; history writes run in the background |
| Group addresses | Edit descriptions in the inspector; explicitly confirmed CSV removal of blank descriptions; descriptions available through MCP |
| Product data | Manufacturer names and detailed product records, German parameter metadata and conditional block captions |
| Downloads | More declared parameter encodings and property writes can be prepared, still marked *untested* |
| Connections | Explicit TCP scans and read-only command-line lookups; TCP and USB/serial clearly labelled **experimental**, UDP supported |
| Feedback | Download progress and background activities in the footer; safer handling of obsolete editor replies |

## For reporters first

| Issue | Status in Alpha 21 |
|---|---|
| [#36](https://github.com/KNXBench-Labs/KNXBench/issues/36) Editing while saving | **Improved; report remains open.** Saving and other file work leave the asynchronous runtime, and ordinary editor requests no longer wait for the save queue. The exact reporter failure was not reproduced; the reverse-proxy question remains open. This is not a promise that every request is nonblocking. |
| [#39](https://github.com/KNXBench-Labs/KNXBench/issues/39) Download scope | The plan warns about pending parts that the selected operation leaves out. Unknown parts stay unknown. |
| [#40](https://github.com/KNXBench-Labs/KNXBench/issues/40), [#42](https://github.com/KNXBench-Labs/KNXBench/issues/42) Pending changes | Device/tab marks, the pending-device panel and last-download comparisons make unfinished work visible. |
| [#43](https://github.com/KNXBench-Labs/KNXBench/issues/43) Progress | A running download keeps its progress in the status bar, with completed work distinguished from the current percentage. |
| [#44](https://github.com/KNXBench-Labs/KNXBench/issues/44), [#46](https://github.com/KNXBench-Labs/KNXBench/issues/46) Object editor | Links come first; less toolbar, more room for the actual objects. |
| [#45](https://github.com/KNXBench-Labs/KNXBench/issues/45) Empty parameter choice | Lists no longer invent a “(none)” choice the product never declared. |
| [#47](https://github.com/KNXBench-Labs/KNXBench/issues/47) Link direction | The first new link sends; further links receive. Direction changes are explicit, one association per address. |
| [#48](https://github.com/KNXBench-Labs/KNXBench/issues/48) Finding changed parameters | Areas, scopes, sections and collapsed groups point to values that differ from the last completed download. The issue remains open for feedback on this alpha. |
| [#41](https://github.com/KNXBench-Labs/KNXBench/issues/41) German wording | One consistent German word for programming a device: **programmieren**. |
| [#38](https://github.com/KNXBench-Labs/KNXBench/issues/38), [#10](https://github.com/KNXBench-Labs/KNXBench/issues/10) Device coverage | More declared product layouts can be prepared, including the bounded Hager push-button application. Software evidence is not a hardware test; affected devices remain *untested*. |
| [#7](https://github.com/KNXBench-Labs/KNXBench/issues/7) ETS 5.7 import | **No importer change.** These projects already import within the documented schema-20 scope. Synthetic coverage and three public samples do not establish compatibility with every ETS 5.7 project or the reporter’s project. |

## What still needs programming?

- The opened device names its pending parts and marks the relevant tabs. The
  **Pending programming** panel lists devices in address order and opens the
  selected device or its download page.
- **Download all pending…** previews a bounded sequence of device operations and
  always asks for confirmation. It stops at the first doubt rather than guessing
  its way through a building. The normal device-specific safety checks still apply.
- Parameter rows show the value used by the last completed KNXBench download.
  Counts and navigation marks include values hidden in unopened sections or by a
  search. Manufacturer fields have their own count.
- Communication-object rows mark changed links and flags. Expand a row to see
  added, removed or redirected links and the flags at that download. Group marks
  and a summary help find changes hidden by filters or collapsed groups.
- Each comparison says when it was recorded or why it is unavailable. An imported
  project does not suddenly acquire a fictional download history. Links, flags
  and parameters keep independent evidence; a partial write only updates the
  parts it actually covered.
- Restoring an older project version does not claim that a physical device was
  restored too. Saved comparisons are retained only where their identity still fits.

[Pending-change guide](../open-work/WP-37-pending-changes-across-ui.md) ·
[Batch programming](../adr/0172-batch-programming-stops-at-the-first-doubt.md)

## Editing, history and background work

Ordinary editor reads and edits can proceed while a project save is working.
File work is moved away from the server’s asynchronous runtime, and journal
writes acknowledge edits before writing the history in the background. A failed
journal writer is reported rather than silently presenting a successful shutdown.
Transactions, history changes and unrelated routes retain their own serialization;
this is not “everything runs in parallel”.

**Keep saving and backing up.** A crash can lose the most recent history that has
not reached disk. The footer shows retained background work and errors; it is an
observer, not a new cancellation or safe-exit guarantee. Obsolete editor replies
are rejected without discarding edits the server already committed.

[History contract](../adr/0166-commands-acknowledged-from-memory-journaled-in-background.md) ·
[Save-queue scope](../open-work/WP-31-device-editor-unavailable-after-save.md)

## Product data and download preparation

Product views expose manufacturer names and the retained technical, application
and source metadata. Parameter metadata is localized in German, and supported
conditional rename leaves can retitle parameter blocks while active.

Download preparation admits more source-declared forms: property writes without
an explicit count use the declared size; assigned and unnamed controllers,
short segments and mid-byte unions have bounded support; floating-point values
use their declared encoding, including decimal rounding for DPT 9. Matching
conditional branches and invisible assigned parameters are handled within the
recorded evidence boundaries. No general ETS equivalence follows from this.

These changes were exercised in software, not written to real devices for this
release. *Untested* is a status, not a euphemism for “go ahead”.

## Connections and everyday fixes

- TCP is an explicit choice for line/device scans, including the server/web
  scan, and for read-only command-line serial lookups and the byte-order probe.
  UDP remains the default; there is no silent fallback or automatic reconnect.
- TCP and Linux USB/serial TP-UART are labelled **experimental** wherever their
  selected or observed transport is shown. UDP tunnelling remains supported.
  No new real-interface interoperability is claimed.
- The download-holder Stop action targets the monitor actually named by the
  current state, rather than a legacy generic stop.
- New projects receive independently generated identities instead of all sharing
  the same identifier. File-menu notices no longer cover menu actions; the
  last-download caption stays put so a click lands where it was aimed.
- Group-address descriptions are editable, can be cleared through the explicitly
  confirmed CSV option, and are exposed read-only through MCP. Empty CSV cells
  still keep existing descriptions unless that removal option is selected.

## Upgrade

| What | Alpha 20 | Alpha 21 | What it means |
|---|---|---|---|
| Project file (`.knxdb`) | schema 19 | schema 21 | Converted on opening for editing. Alpha 20 refuses converted files. Older files start with unknown last-download comparisons; nothing is inferred from imported programming flags. |
| Product database | version 32 | version 33 | Converted on opening for editing; retained source data supplies conditional parameter-block captions. Alpha 20 refuses the converted database and starts without products. |
| Other retained data | Existing formats | No additional versioned sidecar-format change in this release | Keep the complete data directory, not just the project file. Newer project semantics are not a downgrade contract. |

Close the app/server before copying projects, the product database, the whole data
directory or Docker volume, device backups and original import files. To return
to Alpha 20, restore those copies; reinstalling the old executable alone is not
enough. Update `knx-mcp` together with the app. No port or data-directory default
changes, and no new Docker channel policy.

## Boundaries

- **Receipt:** the [sanitized public receipt](../evidence/release-alpha21-2026-11-13.json) records the candidate, the dry runs, the tag publishers and the checks of the final public bytes: downloads and `SHA256SUMS` match, `knx-mcp` and the server report `0.1.0-alpha.21+g616d26d`, the Docker version and `latest` share one index for amd64 and arm64 with the exact revision label, an isolated amd64 container passed the authenticated smoke (create, save as, reopen), and the extracted AppImage started headless with frontend and API answering. The owner's own container was not touched.
- **Not verified:** physical KNX hardware or real TCP/USB interface interoperability
  for these changes, native desktop-window interaction, ARM runtime execution,
  FUSE launch, universal ETS compatibility or downgrade without restored backups.
- System-2 remains experimental and application-specific; issues #30 and #22
  remain open. KNX Secure and `.knxproj` export are not supported.
- Comparison marks describe retained software evidence, not a live readback from
  the device. New download forms still require explicit acceptance of *untested*.
- The AppImage requires glibc 2.39 or newer, not a promise for every Linux system
  or graphics driver.

## Distribution

| Download | Name |
|---|---|
| GitHub pre-release | `v0.1.0-alpha.21` |
| Linux AppImage (amd64) | `KNXBench_0.1.0-alpha.21_amd64.AppImage` |
| MCP server | `knx-mcp-x86_64-linux` |
| Checksums | `SHA256SUMS`; run `sha256sum -c --ignore-missing SHA256SUMS` |
| Docker (amd64 and arm64) | `knxbench/knxbench-server:0.1.0-alpha.21`; `latest` follows releases |

[Installation](../manual/getting-started/04-installation.md) ·
[Docker updates](../manual/user-guide/11-web-and-docker.md#updating-a-docker-hub-installation) ·
[Known issues](../manual/known-issues.md)

## By the numbers

| | Alpha 21 |
|---|---|
| Commits since Alpha 20 | **241** (203 without merges) |
| Files changed | **600**, +48,661 / −2,560 lines |
| Estimated active development time | **about 50 hours** in 34 working sessions, over about 4 calendar days* |
| Rust tests | **8,949 passed, 0 failed**; 252 ignored ones are reported, not counted as coverage |
| Frontend unit tests | **5,737 passed** |
| Browser test executions | **1,229** fixture (1,132 + 97 exclusive), **844** production in 37 configurations, **45** extra, **12 + 12** native — overlapping suites, not one unique total |
| Corpus tests | **155/155** in 9 targets, plus the 4 whole-corpus product modules (15 tests), the legacy-name check and the download-coverage baseline |
| Alpha 20 vs Alpha 21 on 28 real product files | **28/28 identical** ingest results (27 install with both, the same 1 refuses with both) |
| Upgrade probes | Alpha 20 project and product database converted by Alpha 21, saved and reopened: **0 failures**; Alpha 20 then politely refuses both |
| Release candidates | **1**, **23/23 green** on its first complete gate run — nothing to fix between freeze and tag |
| Gate time | about **1 h 25 min** for the complete run; the first attempt lost its connection mid-gate and is kept as aborted, not counted as a result |
| Packaging dry runs | **2** (AppImage + Docker), **0 accidental publications** |

\* Estimated from commit times (a gap over 45 minutes starts a new session, each session counts from 15 minutes before its first commit) — not time tracking.

## Acceptance and public verification

Published on **13 November 2026** from immutable `616d26d2ec86ceeadd86a3441a7e3b2bfaa9b6cc` (branch `release/alpha21-20261113`, source cutoff `938e895c5`).

- **Source campaign:** the complete maintained package gate (complete, plan completed, inputs frozen, no user data touched) plus the release-only browser, native, product-corpus and download-coverage stages: **23/23 green** with the candidate unchanged from start to end.
- **Historical product files:** on a representative 28-file cross-section of the archived pinned product selection, restored byte-exact and hash-checked, the legacy-name check passed unchanged, and Alpha 20 and Alpha 21 ingested every file identically (28/28). The full 115-instance compatibility matrix needs the complete selection and was not run; the comparison replaces it.
- **Upgrade:** a project written by Alpha 20 converts to the new project schema, and an Alpha 20 product database to the new product-database schema; both reopen, the originals stay unchanged and Alpha 20 refuses the converted copies.
- **AppImage (dry run of this commit):** highest glibc requirement 2.39 across 165 bundled ELF files, CI smoke bound to the downloaded image, loopback only. `knx-mcp` reports `0.1.0-alpha.21+g616d26d`.
- **Docker (dry run of this commit):** amd64 and arm64 built; publishing was skipped until the tag.
- **Not verified:** physical KNX hardware and real KNX interfaces (including TCP tunnelling and USB/serial), native WebKitGTK/Tauri windows, ARM runtime execution, FUSE AppImage launch and the full 115-instance pinned compatibility matrix (replaced by the 28-file comparison above).
