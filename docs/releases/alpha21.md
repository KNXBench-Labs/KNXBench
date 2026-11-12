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

RELEASE-GATE-PENDING

## Acceptance

RELEASE-GATE-PENDING
