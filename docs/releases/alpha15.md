# KNXBench v0.1.0-alpha.15

Alpha 15 makes the workbench easier to read and harder to misread: empty topology
cards step aside, parameter captions become useful, catalogue lookalikes explain
themselves, and device comparisons point at the bytes that differ. Product data
gets units and a less repetitive evidence ledger. None of this grants new
programming support.

Alpha 15 covers source through cutoff `21b9d46f`, relative to `v0.1.0-alpha.14`.

## For reporters first

- **Empty topology cards no longer bury the devices (#25).** The diagram hides
  lines without devices and areas without a populated line. Those ranges stay
  in the project tree, editable and unchanged; zero-object devices and populated
  unassigned sections remain visible.
  [Contract](../contracts/topology-metrics.md#device-empty-ranges-issue-25-source-builds-after-alpha-14).
- **Parameter headings say something useful (#26).** Area captions and separator
  text resolve supported placeholders through the exact referenced text
  parameter. Empty, marker-only and brace-bearing captions get readable
  fallbacks, with the original template and source reason in the tooltip.
  Area/group captions are searchable; separator text is not. Display only:
  settings and source metadata do not change.
  [Caption contract](../contracts/positional-text-templates.md#parameter-workspace-captions-issue-26).
- **An unavailable catalogue says why.** A product database from a future schema,
  or another open failure, no longer disappears behind an empty catalogue and a
  generic “not configured” message. The session log and affected refusals retain
  the actual reason. `doc-export`, the catalogue reading commands and
  `device readiness` no longer create or migrate the on-disk product database
  just to inspect it; older catalogues are copied and migrated in memory.
- **Catalogue “duplicates” have identity lines and a cleanup tool.** Entries show
  their application ID/version, catalogue section and source file name. A newer
  application is deliberately installed beside the older one. **Clean up
  catalogue** previews only the placements you select, starts with nothing
  selected, warns about the last placement for a product/application and takes
  a full backup before removal. It does not delete products, programs or package
  bytes, free disk space or switch a project's application version. Preview
  device counts cover the open project only.
  [Guide](../manual/user-guide/05-devices-and-products.md#cleaning-up-the-catalogue-since-alpha-15) ·
  [ADR-0145](../adr/0145-catalogue-cleanup-removes-placements-only.md).

## Workbench highlights

- **File menu focus, submenus and zoom.** Import & export, History & comparison
  and Help & diagnostics get keyboard-navigable submenus. Closing a dialog
  returns focus to File; submenus fit or become inline on constrained windows,
  including at 150% zoom. Mouse, touch and pen activation keep their targets.
  [Contract](../contracts/file-menu.md).
- **Controls with a visual hierarchy.** Primary, secondary, quiet and destructive
  actions have distinct feedback; separators are visible and routine provenance
  badges quieter. This changes presentation, not confirmation or write guards.
  [Contract](../contracts/controls-and-separators.md).
- **Topology numbers introduce themselves.** Device tiles label communication-object
  counts, keep the count visible in narrow building cards and use bounded widths
  on wide screens. They count objects, not devices.
  [Contract](../contracts/topology-metrics.md).
- **Resizable, measured table columns.** Pointer and keyboard resizing, resets and
  validated browser-local preferences cover the HTML data-table surfaces.
  Word floors, local scrolling and zoom-aware measurement keep columns usable;
  per-object workspace columns remain fixed. Pane and block-splitter drags also
  follow the pointer at 80–150% zoom.
  [Coverage and limits](../contracts/table-column-widths.md).
- **Overview measures storage honestly.** Current native-file size is separate
  from the unsaved flag. Catalogue size includes its SQLite side files, and
  canonical manufacturer/product/program counts are not placement counts.
  Loading, missing, busy, stale and error states stay explicit.
  [Contract](../contracts/project-status-metrics.md).
- **The monitor explains missing DPTs.** No project, unknown address, missing type,
  explicitly cleared program default and codec outcomes are distinct. A reason
  is not a guessed DPT or a changed decoding rule.
  [Contract](../contracts/dpt-selection.md#monitor-resolution-reasons-ui-01-18-october-2026).
- **Device comparisons open with differences first.** Differing raw bytes are bold
  and underlined on both sides; unread/not-comparable bytes use italic and dotted
  marks. Only proven-equal rows hide. **Show equal rows** reveals every original
  row and raw read range. Results and session tags are unchanged.
  [Guide](../manual/user-guide/05-devices-and-products.md#what-differs-wp-18-since-alpha-15).
- **System column in Devices.** Sort or filter by the system family or declared
  mask, using one cited core table. Missing products/programs/masks and unknown
  masks have explicit labels. This is product-declared metadata, not a device
  reading or proof of programming support.
  [Guide](../manual/user-guide/05-devices-and-products.md#system-column-since-alpha-15).
- **Nested module areas keep their identity.** Complete response-local evaluation
  paths separate nameless nested scopes, diagnostics, drafts and navigation.
  This is not a durable module-instance mapping or new parameter write authority.
  [ADR-0063](../adr/0063-parameter-scopes-preserve-evaluation-identity.md#web-adoption-wp-15-19-october-2026).

## Product data and import integrity

- **Product v25 placements and enumerations.** Parameter offsets/property placements
  and enumeration attributes reach typed columns; remaining attributes are
  reported. Storing `BinaryValue` or a property placement does not make it
  executable.
  [ADR-0141](../adr/0141-placements-and-enumeration-attributes-reach-columns.md).
- **Product v26 records unknown evidence once per identity.** Re-parsing a shared
  blob no longer multiplies its per-blob unknown rows. Migration merges repeated
  rows using the largest per-parse count and oldest sample, not their sum;
  distinct conflicts remain distinct. This repair alone leaves package-report
  totals unchanged.
  [ADR-0146](../adr/0146-unknown-evidence-rows-have-an-identity.md).
- **`SuffixText` units follow values.** Parameter/reference unit text is stored
  separately, translated and shown after the value in the row and checked editor.
  The reference's own declaration wins, even when empty. It never becomes part
  of a value, draft, validation or download. Its promoted evidence rows retire
  only on the supported hosts; this separately changes package-report totals.
- **Larger, measured bounds.** A retained file/member is bounded at
  **1,000,000,000 bytes**, the storage ceiling. Standard expanded archives are
  bounded at **4 GiB**; the CLI large-product profile at **16 GiB**. Product
  packages admit up to **65,535 members**. The general browser project upload
  remains **1024 MiB**. These are distinct limits, not an unlimited-file promise;
  larger files still cost memory and time.
  [ADR-0142](../adr/0142-file-size-limits-follow-the-storage-ceiling.md).
- **Legacy uppercase table names import.** Evidenced `.vd5` inputs no longer
  appear empty solely because their table names are uppercase. Legacy format
  and programming limits otherwise remain.
- **Empty ZIP directories must really be empty.** Project archives and protected
  nested payloads refuse inconsistent directory CRCs or incomplete/nonempty
  compressed streams before changing the project/catalogue. Valid stored and
  complete empty-deflate directories remain accepted.
  [KL-159](../limitations/KL-159-project-archive-budgets-and-duplicate-names.md).

## System-2 investigation, not programming support

The CLI can preview explicitly selected System-2 source fields with
`byte-order-probe --preview --system2-source`. It shows candidate source-derived
locations and byte orders without constructing a network client; the option
cannot override the live probe's refusal.
[Preview contract](../contracts/system2-source-probe-preview.md).

A separate **CLI-only** diagnostic profile, `system2-declared-segments-v1`, derives
a bounded read plan from ProductDB's admitted load-procedure shape. Preview
requires no bus operation; an actual read needs explicit confirmation and exact
mask/manufacturer/hardware identity checks before memory. Format-2 ZIPs retain
coverage and private memory evidence. The web/HTTP diagnostic export still offers
only the generic identity profile. Source-declared ranges are candidate locations,
not a backup or restore basis. This profile has offline/simulator evidence, not a
hardware run. [ADR-0143](../adr/0143-declared-segment-memory-evidence-profile.md).

## Upgrade: independent backups first

**Native/model schema stays 16.** The core/store diff was checked as well as the
version constants: the new types are diagnostic plans/display metadata, and
store changes raise existing byte budgets; no new persisted project kind or
native migration is introduced. **Opening Alpha-15-saved projects in Alpha 14
has not been verified**, especially files using the increased bounds.

The dedicated **product database migrates 24 → 26 on a writable application
open** (through v25's placement columns, then v26's evidence repair and unit
columns). Read-only CLI catalogue readers leave the on-disk file unchanged,
migrating an older copy only in memory. Older
builds that understand at most v24/v25 refuse product26 by their version check;
this is a source-established refusal, not an executed downgrade test.

Close KNXBench before independently copying `.knxdb` files and all Docker data,
including the product database. Keep the original `.knxproj`/`.knxprod` sources;
update standalone `knx-mcp` together with the app. Going back means restoring
pre-upgrade copies. In-file history and catalogue cleanup backups do not replace
an independent disaster backup. Port **8484** and host-network exposure are
unchanged; updating an image does not replace a running container.

<a id="still-alpha-explicit-boundaries"></a>

## Still alpha — explicit boundaries

- **Not verified:** no broader physical-device/gateway support, complete ETS
  semantics, native WebKitGTK/Orca feature acceptance or previous-reader
  compatibility follows from this release. Recorded identity-only hardware
  observations do not verify the new declared-memory profile or any write.
- **System-2 / `MV-0025` programming remains refused.** Complete image, source
  procedure, encoding and durable whole-mutation recovery prerequisites remain.
- **Product v23–v25 migration residues remain open.** Certain repeated program
  declarations can backfill a losing declaration, and older promotion steps can
  retire evidence that fresh ingest keeps. V26 fixes those patterns for its own
  suffix promotion, not a general repair of earlier promotions.
- Parameter fidelity remains partial: inline sections, in-cell buttons, object
  folders, object priority and a `TypeDate` editor are not added. Legacy EX-IM
  has no suffix mapping; stored `Suffix` is not displayed. Older servers without
  complete module paths cannot separate all nested nameless scopes.
- Remaining ZIP corners include trailing data after a deflated file stream and
  the product reader's directory equivalent; this is not universal ZIP validation.
- No new `.knxproj` export, KNX Secure, individual-address programming, firmware
  replacement or little-endian download admission is added. UI/zoom evidence is
  Chromium-bounded.

## Distribution

Linux x86-64 AppImage and standalone `knx-mcp` with `SHA256SUMS`; Docker
`knxbench/knxbench-server:0.1.0-alpha.15` for `linux/amd64` and `linux/arm64`.
`latest` follows published releases, including alphas, not moving source.
[Installation](../manual/getting-started/04-installation.md) ·
[Docker update](../manual/user-guide/11-web-and-docker.md#updating-a-docker-hub-installation) ·
[Known issues](../manual/known-issues.md).
