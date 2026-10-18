# KNXBench v0.1.0-alpha.14

Alpha 14 packages the work since Alpha 13: a quieter, folded workbench, a real
parameter workspace that sits parameters where the product data seats them, a
polite read-only diagnostic export for bug reports, and safer deletes for
projects with notes. Product databases get a closer reading. Nothing here turns
a parameter screen into a download, however tidy the screen has become.

## For reporters first

- **Send diagnostic data in five steps**
  ([#22](https://github.com/KNXBench-Labs/KNXBench/issues/22)). A read-only device
  diagnostic export picks one device, reads its descriptor, manufacturer ID and
  hardware type over the existing tunnel lock, hangs up and saves a verified
  local ZIP for the maintainers. No writes, no keys, no restarts, no memory
  reads; Stop keeps what was already read. This helps triage #22; it does not
  program `MV-0025`, which stays refused.
  [Guide](../manual/user-guide/08-reports-and-diff.md#send-diagnostic-data) ·
  [Contract](../contracts/device-diagnostic-export.md) ·
  [ADR-0134](../adr/0134-generic-read-only-diagnostic-export.md).

## Product data: manufacturer ZIPs install in one step

Most manufacturers ship a ZIP with one or more `.knxprod` packages and their
manuals inside, which earlier alphas refused as "missing knx_master.xml". Now
the catalogue picker (preview first, as before) and `knx products ingest`
install every package inside such a ZIP together, each byte-exact, or none of
them if one is refused, and the message names it. Manuals and other files are
listed with their size and digest and are not stored; macOS metadata files named
like a package are recognised by their content and listed too. A complete
manufacturer database too large for the default bounds installs with
`knx products ingest <file>.zip --allow-large-package`; the web keeps the default
bounds. On the maintainer's local collection: 33 such ZIPs, 30 install with the
default bounds, 2 with the large profile, 1 is refused for unsafe backslash paths.
[ADR-0140](../adr/0140-vendor-zips-of-product-packages-install-as-a-whole.md).

## Workbench highlights

- **Everything starts folded.** Navigation blocks, explorer branches, group-address
  ranges, parameter areas, manufacturer groups, Function entries and report details
  open on request; search, selection, remembered tree state and Expand all still
  reveal them. A splitter that measured a hidden block as 0 px no longer jumps on
  its first drag.
- **A parameter workspace instead of stacked accordions.** An area register built
  from the declared device/module scopes and parameter blocks, compact aligned
  label/value rows, search across all areas and a read-only information pane.
  Editors stay mounted, so switching areas never loses a draft or a conflict.
  [Contract](../contracts/parameter-workspace.md).
- **Declared parameter grids.** Table/Grid blocks render as a matrix of the existing
  rows at their declared cells; everything else stays in rows with a stated reason.
  [ADR-0137](../adr/0137-declared-parameter-grids-reach-the-workspace.md).
- **Block access and separators.** A block's declared `Access` now limits writes
  below it (refused with `parameterBlockAccessRestricted`, nothing written), and
  separator rows (headline, information, error, rule) appear in the workspace.
  [ADR-0139](../adr/0139-block-access-and-separators-reach-the-parameter-tab.md).
- **Function choices where the object lives.** An expanded communication object
  offers the manufacturer's supported choice parameters that control it directly
  below its DPT field, as the same bound row the Parameters tab uses (revision-bound
  write, inline errors, Undo). The separate *Device functions* disclosure is gone.
  Missing, ambiguous or module-local mappings give no control rather than a guess.
  [Contract](../contracts/parameter-object-refresh.md).
- **Indented rows and object sizes.** Rows the product data declares as indented
  (`IndentLevel`, one consistent declaration per block) are indented in the row
  view, and the object table names an object's size (for example *1 Bit*,
  *2 Bytes*) when the product data states it.
  [Contract](../contracts/parameter-workspace.md#row-indentation-census-p4-18-october-2026).
- **Functions in the comparison.** The project diff gains a per-Function table
  (matched by ETS id, else building-part path and name; ambiguity reported, never
  guessed) in CLI, web and MCP; MCP gains a read-only `list_functions` tool.
  [Contract](../contracts/project-functions.md).

## Data integrity

- **Notes get a say before a delete.** Deleting an entity that notes point at now
  asks what to do with each stranded note (move it to the project, retarget it or
  delete it) and applies the decisions and the delete as one undoable step. CLI, GA CSV delete
  rows and scan reconciliation refuse and name the notes instead.
  [ADR-0138](../adr/0138-notes-get-an-explicit-decision-before-a-delete.md).
- **Selective import carries notes along.** Notes of selected entities follow their
  ID mapping; notes outside the selection need an explicit skip/attach choice.
- **Plug-in data adoption.** `knx adopt-plugin-data` and a web dialog copy one
  device's opaque plug-in blob and numbered ExtraData table verbatim from a source
  `.knxproj` into an existing native project, after a digest-bound preview. It
  refuses unless product/application references, blob/table references and group
  links match; nothing is decoded or run. The web route records a version first,
  so the way back is the saved version, not Undo.
  [ADR-0136](../adr/0136-plugin-data-adoption-is-audited-opaque-replacement.md) ·
  [Contract](../contracts/plugin-data-adoption.md).
- **Product database v24.** Block/separator `Access` and separator `UIHint` are
  typed columns, backfilled from retained source bytes on first open.
- **Packed time values are left alone.** Time parameters whose unit packs several
  fields into one number (`Packed…`) are no longer offered as a decimal count:
  the original value is shown unchanged. Plain units keep their count control.
  [Contract](../contracts/parameter-editor-admission.md#packed-time-units-fall-back-2026-10-18).

## Repository housekeeping

The public repository no longer carries local machine details: tracked files were
scrubbed to placeholders, a guard checks files and commit messages, and the Git
history including the release tags was rewritten once (KL-185). **Commit hashes
quoted in earlier release notes refer to the old history**; they translate through
[COMMIT_MAP_2026-10-17](../history/COMMIT_MAP_2026-10-17.txt). Release assets and
images are unchanged.

## Upgrade: independent backups first

**No project schema bump relative to Alpha 13:** native/model schema remains
**16**. This follows the actual diffs, not just the version number: the core
changes since Alpha 13 add error and helper types for note-aware deletes, and plug-in
data adoption reuses existing opaque entries; no new persisted kind. The
dedicated **product database moves from 23 to 24** on first open (typed block
`Access` and separator hints, backfilled from the retained source bytes).
Alpha 13's code refuses a product database newer than the version it knows; that
was read in its source, not executed, so keep a copy of the product database if
you may go back. **Opening Alpha-14-saved projects in Alpha 13 has not been
verified.**

Close KNXBench before copying `.knxdb` files and all Docker data, including the
product database. Keep original `.knxproj`/`.knxprod` sources. Update standalone
`knx-mcp` together with the application. In-file history is not an external backup.
The Docker port stays **8484**; review firewall/LAN/VPN exposure when using host
networking. Publishing an image does not replace any existing container.

## Still alpha — explicit boundaries

- **Not verified:** no new physical device/gateway operation, broad hardware
  support, native WebKitGTK/Orca feature acceptance or universal ETS compatibility
  follows from this release. The diagnostic export was exercised against a
  loopback simulator; browser/native-evidence checks use isolated software servers
  and synthetic projects, not a live installation.
- Siemens System-2 / `MV-0025` programming remains refused.
- Plug-in data adoption is verified on synthetic archives and one private ETS6
  export pair; whether the originating plug-in accepts adopted bytes on a real
  installation is the owner's open check (KL-186).
- Parameter layout fidelity is partial: inline sections, `SuffixText` units,
  parameter-driven captions, in-cell buttons, object folders and object priority
  are not shown yet, and `TypeDate` has no date editor (KL-003). Block `Access`
  inheritance follows the schema's meaning, not a documented rule.
- Vendor ZIPs install all or nothing; a refused inner package refuses the ZIP, and
  files that are not product packages are listed, not stored. The large profile
  for complete manufacturer databases is command-line only. ZIPs inside ZIPs are
  not opened. Product databases can grow large (about 1 GB for each of the two
  complete databases measured); growth is not optimised yet, and size ceilings
  follow measurement in a later alpha. One measured DALI-2 vendor package holds
  a program file above even the large profile and has no install route yet
  ([census 3](../research/2026-10-18-product-database-programmability-census.md#installer-outcomes)).
- No new `.knxproj` export, KNX Secure, individual-address programming, firmware
  replacement or little-endian download support is added.

## By the numbers (a release, in six takes)

Alpha 14 needed **six release candidates**. Not for lack of confidence: every
retake found something real, and none of it was in the new product-data code.

| Take | What it found | Verdict |
| --- | --- | --- |
| 1 | The Docker image **would not have built** on either architecture: since the browser temp helper moved to `tools/` one day earlier, the web build reached for a file the image never copied. Caught by a dry run, not by you. | Fixed, plus a guard test |
| 2 | Two native browser specs still believed every tree starts unfolded. | Fixed |
| 3 | Two flaky tests that only misbehaved on a heavily loaded machine, and a product-database matrix still counting in schema v23. | Fixed, re-pinned |
| 4 | Lived four and a half minutes. | Retired before its gate |
| 5 | Green through web gate, Rust, native and corpus; superseded by the matrix re-pin. | Retired honourably |
| 6 | Nothing. That is the one you are downloading. | Tagged |

- **84 commits**, 446 files, +31,970 / −1,452 lines and 7 new ADRs since Alpha 13.
- **7 release-gate attempts** over about **6.4 hours** of gate time; the final one
  ran **36 of 36 stages green** in 97 minutes with its inputs frozen.
- **About 40,500 test executions** across those attempts: 14,968 Vitest, 19,528
  Rust, 4,064 browser, 1,332 tool and 612 private-corpus tests.
- **839 deliberate repeats** to corner two flakes: a test that retried a refused
  import before the first refusal had finished answering (219 runs), and a Linux
  multicast table that changed between two reads of the same file (620 runs).
  The first now fails every time its fix is removed; the second failed 2 times in
  160 under deliberate churn, and 0 times in 200 with the fix.
- **12 packaging dry runs**, 62 runner minutes, **0 accidental publications**.
- **33 real manufacturer ZIPs** run through the new installer: 30 install with the
  default bounds, 2 with the large profile, 1 is refused for backslash paths, which
  is the right answer.
- **6 of 102** product-matrix numbers moved for schema v24, each predicted in
  advance by an independent XML count.
- The final gate's own figures: Vitest 3,742, fixture Chromium 596, 15 production
  browser configs 341, 8 further configs 41, Rust 4,882 passed / 0 failed / 212
  ignored, native checks 24, tools 223, private corpus 153 of 153 in 33 targets,
  product corpus 16 cases in 6 targets.

## Distribution

Published on **18 October 2026** from the gated commit `8d104e2f`
([source acceptance](alpha14-acceptance.md),
[publication record](../evidence/release-alpha14-2026-10-18.json)).

Linux x86-64 AppImage and standalone `knx-mcp` with `SHA256SUMS`; Docker image
`knxbench/knxbench-server:0.1.0-alpha.14` for `linux/amd64` and `linux/arm64`.
`latest` follows published releases, including alphas, not moving source.
[Installation](../manual/getting-started/04-installation.md) ·
[Docker update](../manual/user-guide/11-web-and-docker.md#updating-a-docker-hub-installation) ·
[Known issues](../manual/known-issues.md).
