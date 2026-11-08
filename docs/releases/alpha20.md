# Alpha 20 — pre-release

Alpha 20 covers changes since `v0.1.0-alpha.19`: ETS6 group-address CSV import
with an explicit encoding and group-address descriptions, a Linux USB/serial
TP-UART bus transport, download planning for property-placed and little-endian
programs, editable object priorities, saved folder captions, much faster edits in
saved projects and corrected AppImage packaging. Project schema moves 16 → 19 and
the product database 31 → 32; Alpha 19 refuses both. Keep a copy before upgrading.

## For reporters first

### Issue #30: System-2 download for UP 258D12 / `M-0001_A-9205-04-9270`

[Issue #30](https://github.com/KNXBench-Labs/KNXBench/issues/30): the reporter's
read-only memory snapshots showed that every written byte matched except the hidden
`Download_Flag` octet, which the firmware clears to `00h` after loading. A **new**
System-2 preparation of this exact application now declares exactly that octet
(object 3, `3CF4h`, accepted `00h`) and tolerates it in the final verification;
every other byte stays strict, and preparations made with older builds keep the
old strict check (`b635a58d8`, `a580c87d1`, merge `046ce3fef`). The preview names
the octet before consent; the panel lists written, observed and prestate values
after the run; the shareable report names only run and object.
[Contract](../contracts/system2-download-result.md#declared-post-load-octets).
The repository records simulator proof for this change. The reporter's own run
on a pre-release `main` build is reported in the issue; these notes do not turn it
into hardware acceptance for other devices or configurations.

### Issue #22: Siemens presence detector (`MV-0025`)

[Issue #22](https://github.com/KNXBench-Labs/KNXBench/issues/22): the
`Download_Flag` declaration above addresses the shared #30/#22 final-check
mismatch for application `M-0001_A-9205-04-9270` only. **Nothing else new for
#22:** no wider device admission or other System-2 profile.

### Issue #35: Restriction dropdown stays disabled after a change

[Issue #35](https://github.com/KNXBench-Labs/KNXBench/issues/35) is fixed
(`82aee5617`, merge `4ca7ca981`). Nothing was stuck: in a saved project every edit
re-encoded the entire undo history and decoded it twice more before it was shown,
6 to 16 s per change with 33 to 56 undo steps on the test project. Journal writes
now keep every undo state they can prove unchanged; rows the server already holds
are checked by their SHA-256 instead of being decoded again. Same project: 2.1 to
2.5 s per change, independent of history length. A pending row now says
**Saving…**. The remaining cost grows with project size, and the write still holds
the project lock. [ADR-0163](../adr/0163-journal-writes-keep-unchanged-undo-rows.md) ·
[History limits](../limitations/persistent-native-working-history.md).

### Issue #34: retained System-2 operation of another device

[Issue #34](https://github.com/KNXBench-Labs/KNXBench/issues/34) is fixed
(`8c4aa8ff1`, merge `fb5706a22`). The System-2 panel says when the retained
operation belongs to another device or project, and a refused configured
preparation points to **Ordinary memory download**. Wording and placement only:
admission, retained lifetime and recovery are unchanged.

### Issue #10: devices in the reporter's project

[Issue #10](https://github.com/KNXBench-Labs/KNXBench/issues/10) itself was closed
before Alpha 19; its follow-up work on the reporter's project ships here. More than
50 of its devices (push-buttons, room controllers, property-based actuators and a
dimmer) moved from *unsupported* to *untested*; see
[Downloads](#downloads-property-placed-and-little-endian-programs). None was
written to hardware by KNXBench for this work.

### Issue #7: ETS 5.7 project import (schema 20)

[Issue #7](https://github.com/KNXBench-Labs/KNXBench/issues/7) stays open.
**Nothing new since Alpha 19 for the importer.** Schema-20 projects now display
their saved folder text (below), which is display only. The authentic ETS 5.7
evidence gap remains [KL-177](../KNOWN_LIMITATIONS.md#catalogue-kl-177).

## Group addresses: ETS CSV, encoding and descriptions

- **ETS6 CSV headers are read** (`41433448a`). Real ETS6 6.4.1 exports matched the
  four full import profiles; the four name/address-only shapes ETS writes as
  "CSV (ETS3 Format)" and tab separators are now accepted too. Header-less files
  stay refused.
- **The encoding is your choice, never a guess** (`2749b2537`, `f53894c59`, merge
  `21f4a62bc`). Group-address CSV import reads UTF-8 (default) or
  **Windows-1252 (ETS export)**: CLI `knx ga-import … --encoding windows-1252`,
  server body field `encoding`, a select beside the web import button. A non-UTF-8
  file under UTF-8 is refused with the offset of its first invalid byte; the five
  bytes Windows-1252 leaves undefined are refused with their offset; a UTF-8 BOM
  contradicts a Windows-1252 choice. Confirmation tokens bind bytes and encoding.
  Other code pages, UTF-16 and detection are unsupported.
- **Group addresses have a description** ([ADR-0164](../adr/0164-group-address-description-and-csv-encoding.md)).
  It is byte-exact and set through an undoable command. Schema 20–23 `.knxproj`
  imports map `GroupAddress/@Description` into it instead of keeping it as opaque
  data. CSV profiles and a new optional native `Description` column apply a filled
  cell on create and update; an empty cell never clears one. Range descriptions are
  reported, not imported. Diff, `.knxproj` comparison, selective import and the
  documentation export include it; the inspector shows it read-only, with no
  editor yet.
- **Declared type for new addresses.** A valid profile `DatapointType` becomes the
  declared type of an address the import creates; for an existing address it stays
  a read-only comparison. The native `DatapointType (read-only)` column never applies.

[Profiles contract](../contracts/group-address-csv-profiles.md) ·
[Guide](../manual/user-guide/04-group-addresses.md#group-addresses-as-csv) ·
[KL-038](../KNOWN_LIMITATIONS.md#catalogue-kl-038). No general ETS/ESF
interoperability or hierarchy roundtrip is claimed.

## Bus access

- **USB/serial TP-UART on Linux** (`eb951166e`, merge `81b0a2092`; `3f956e401`,
  merge `eab79cc5b`). `knx bus monitor` and `knx bus write` accept
  `--serial <device>` with a mandatory `--source <individual address>` instead of
  `--gateway`. Settings, the status bar and Bus Monitor offer
  **USB/serial TP-UART** with **Passive monitor** (no sends, no ACKs) or explicit
  **Active group communication**, which auto-acknowledges the local address
  **and all group telegrams**. Only a Busware TUL with transparent NCN5130
  USB-serial firmware (19,200 baud 8E1, 9-bit UART contract) is admitted; KNX USB
  HID, FT1.2 and arbitrary USB-UART adapters are not. The path is the server's
  (and its container's) device. Uncertain sends close the link and are never
  retried; there is no IP fallback. Device management, downloads and scans stay
  KNXnet/IP only.
- **Candidate listing without probing** (`c760b5d85`, merge `b3d1a0a7c`).
  **List USB-serial candidates** reads `/dev/serial/by-id` on the server only when
  pressed; it opens no device. Rows are unverified candidates, not compatible
  adapters.
- **Status-bar connection** (`4a47be6ef`, merge `7ea7ff6dd`). The footer shows the
  shared bus session's state with separate Connect/Disconnect. Choosing an
  interface only saves the preference; a running connection is never switched.
  `POST /api/bus/monitor/stop-bound` stops only the named session.
- **Busy means now, not later** (`87440d6b3`, merge `038845ef8`). A monitor start
  while a download, management or System-2 run holds the bus answers
  `409 gatewayBusy` immediately instead of queueing behind it.

[Bus guide](../manual/user-guide/07-bus-and-interfaces.md#bus-connection-in-the-status-bar) ·
[CLI](../manual/user-guide/10-command-line.md#usbserial-tp-uart-linux) ·
[USB acceptance and limits](../status/2026-11-04-usb-serial-ui.md) ·
[KL-062](../KNOWN_LIMITATIONS.md#catalogue-kl-062).

## KNXnet/IP TCP tunnelling (optional)

TCP-PENDING: filled from WP-29's delivered commits or removed if TCP is deferred.

## Downloads: property-placed and little-endian programs

All of this was recorded against KNXBench's own simulator and compared with ETS 6.4
download images; affected products plan as **untested** and need
`--accept-untested`.

- **LittleEndian numbers** (`e5306e023`, `195ff4fda`, merge `781afbf0d`).
  Whole-octet `LittleEndian` numbers in memory layouts are written low octet
  first; fixed `LdCtrlWriteProp` writes one element as wide as the device reports;
  `LdCtrlDelay` waits; a `choose` whose controller is inactive selects nothing.
  A comparison behind a leading property write still runs before any write.
  [ADR-0159](../adr/0159-little-endian-numbers-and-fixed-load-steps.md) ·
  [ADR-0160](../adr/0160-inactive-choose-controllers-select-nothing.md) ·
  [KL-170](../KNOWN_LIMITATIONS.md#catalogue-kl-170).
- **Undeclared text encoding is Windows-1252** (`2d9816966`, merge `704d1997d`).
  Programs without `TextParameterEncoding` get non-ASCII text in Windows-1252, as
  ETS wrote it, instead of a refusal; the plan names
  `undeclared-text-windows-1252`. Declared `utf-8` stays refused.
  [KL-171](../KNOWN_LIMITATIONS.md#catalogue-kl-171).
- **Property-placed parameters and master resets** (`a727c2c31`, merge
  `ebc3a861f`). On mask `0701h`, `LdCtrlMasterReset` is sent as the function
  property command ETS uses; a parameter-filled property is read, only
  parameter-owned bits are merged, and the rest is written back as the device
  holds it. Invisible parameters get their default under
  `DownloadInvisibleParameters="Background"`. **Restore and partial download refuse
  these plans**, and a pre-download backup cannot restore property values.
  [ADR-0161](../adr/0161-property-placed-parameters-and-master-resets.md).
- **Time parameters and inline property writes** (`df8e492f0`, merge
  `c03f9dbde`). A plain-unit `TypeTime` value is written as its unit count; packed
  units stay refused. All-zero `InlineData` property writes need no read, and a
  property whose every bit is determined is not read.
  [ADR-0162](../adr/0162-time-parameters-and-inline-property-writes.md).
- **Clearer refusal precedence** (`c3b662763`, merge `ac636f22f`). When a program's
  code cannot be read, a stored mask that no download loads is reported as
  not-memory-mapped instead of a product-data error. The product coverage pins
  now use a documented 122-package / 758-program characterisation baseline.
  [Baseline](../research/commissioning/19-22-product-coverage-characterisation-baseline.md) ·
  [KL-006](../KNOWN_LIMITATIONS.md#catalogue-kl-006).

## Communication objects: priority and folders

- **Priority you can see** (`c376f0347`, `7b7e72df1`, `c55147aab`; merges
  `edd41cd25`, `ed0cf7737`, `40fab41b6`). Fresh schema-23 imports keep each
  object's saved priority declaration. The object editor shows an effective
  priority with its winning source (your edit, saved declaration or product
  literal); unknown cases stay unresolved with a reason, never defaulted to Low.
  Project comparison and the documentation export include saved priority
  (`b60b5a0fd`, `263c55b32`).
- **Priority you can set** (`947e342af`, merge `0ad3dc8cf`). Low/High/Alert as an
  undoable edit in the object editor (`POST /api/com-object-priority`). An optional
  **Priority** column with filter, off by default (`3017c51bb`, merge `b6b0d29b6`).
  Programming never writes a changed priority; see Changed defaults and
  [KL-188](../KNOWN_LIMITATIONS.md#catalogue-kl-188).
  [Contract](../contracts/object-priority-edit.md) ·
  [Effective priority](../contracts/effective-object-priority.md) ·
  [ADR-0158](../adr/0158-object-priority-provenance.md).
- **Saved folders get words** (`e4aa31945`, merge `61e28d225`; `1e3b441d3`, merge
  `252b11c3c`). Schema-20 projects show the literal saved folder text beside the
  folder IDs; already assigned folders can also show an authenticated installed
  product's literal caption, with its provenance disclosed. Placeholders stay
  literal; nothing is translated. IDs stay visible and searchable.
  [Contract](../contracts/declared-product-folder-captions.md).
- **Saved folders admit their age** (`1b85f4df1`, merge `b33f95145`). When a device
  changed in KNXBench since import, the object table says the saved folders may
  be outdated; placement is never recomputed or hidden.
  [Contract](../contracts/saved-folder-currency.md).

## System-2

- **Selective settlement** (`3f19ebe65`, merge `0d6299187`). A verified configured
  run no longer loses every credit to an unrelated edit: a rename or another
  device's edit keeps all parts; link or flag edits leave group addresses pending;
  parameter edits leave parameters pending. A changed or refused source, or a
  switched product reference, credits nothing.
- **Caption-only product changes** (`9efa07832`, merge `288bd8005`). If the
  retained product source differs only in the application program's `Name`
  caption, settlement can still credit; every other byte must be identical.
  Failed runs never earn credit.
- **Faster recovery conversion** (`41b1bf61d`, merge `1e0d0a6de`). Each recovery
  conversion reconstructs its plan once instead of several times, keeping every
  validation.

## Workbench, desktop and web

- **Rail and Rail Dark** (`58760ae32`, merge `ed01e2bd3`). Two built-in sharp
  themes with Geist type, a bus-cable green accent and a measuring rail under the
  toolbar; their effects stop under motion **Off** or OS reduced motion.
  [ADR-0157](../adr/0157-rail-built-in-presentation.md) ·
  [Limits](../KNOWN_LIMITATIONS.md#catalogue-rail-presentation).
- **Compare from the device editor** (`3173e60d7`, merge `080f2723d`). **Compare**
  beside Download opens the existing read-only comparison for that device; it
  starts no automatic read.
- **Import findings after reopening** (`e91d232df`, merge `3455fdb9c`). Reopening a
  native project shows its stored import findings in the session log again, with
  their original provenance.
- **Checked Date editor** (`b19dfb131`, merge `76dc954a1`). Source-declared DPT 11
  date parameters get a calendar-checked text field with explicit Apply; no new
  image encoding.
  [Contract](../contracts/checked-date-text-editor.md).
- **Fixes:** group-address Columns/Atlas sub-ranges no longer render empty under an
  Explorer range (`f1578474a`, `9da80b693`); project history shows its 1 GB limits
  as `953.7 MiB` instead of eleven decimals (`7fd3cdc94`); catalogue job polling
  stops after a job finishes and a stale poll cannot touch a newer job
  (`09833e8b7`, `1213e6ff9`); a source-label binding refusal now names its cause
  as `passiveLabelsUnbound` (`555263e77`).
- **Backups pinned to their directory** (`9980aee67`, merge `98d2e6953`).
  Device-memory and service-control JSON backups are created, verified and synced
  through held descriptors; a path swapped underneath refuses the receipt.
- **AppImage packaging** (`f00afadb3`, `76d697151`; merges `0da90db34`,
  `cac2b92c9`). The Alpha 19 AppImage shipped `AppRun.wrapped` with mode `0770`,
  so other users got *Permission denied*, and bundled libraries required glibc
  2.43. Release images are now built on Ubuntu 24.04, rejected if any ELF needs
  glibc above 2.39, mode-checked and started as a different user without
  networking before publication.
  [Packaging record](../status/2026-11-08-appimage-catalog.md).

## Import evidence

The authorised revision-export harness (IMPORT-02) passed once on real input:
three ETS6 6.4.1 / schema-23 exports of a fictional project, including one
restored through ETS's own archive (`2380fa081`). Other ETS versions, schemas
and protected projects remain unverified.
[Run record](../AUTHORIZED_RESTORE_EXPORTS.md#run-record-8-november-2026).

## Changed defaults

- **Programming refuses changed priorities.** A device is refused while an active
  object's project priority (your edit, else a captured saved declaration) is not
  exactly the product literal. A captured empty or unknown declaration refuses until
  you set a priority. Objects without captured priority write the product priority
  as before. [KL-188](../KNOWN_LIMITATIONS.md#catalogue-kl-188).
- Programs without a declared text encoding now download non-ASCII text as
  Windows-1252 instead of refusing; LittleEndian, property-placed and time-unit
  programs that were refused now plan as *untested*.
- Group-address CSV import accepts tab separators and Windows-1252 on explicit
  choice; UTF-8 stays the default.
- Fresh `.knxproj` imports store group-address descriptions in the model, no
  longer as opaque retained attributes.
- `POST /api/bus/monitor/start` refuses a held bus immediately with
  `409 gatewayBusy`.
- The object-table Priority column is off by default. No port, data-directory or
  Docker tag rule changes.

## Upgrade

**Native/store schema moves 16 → 19.** All three steps are additive:

- **17** adds the captured saved-instance priority table. Only fresh schema-23
  imports fill it; older projects stay *not captured*.
- **18** adds user priority edits, starting empty.
- **19** adds the group-address description table, empty.
  [ADR-0164](../adr/0164-group-address-description-and-csv-encoding.md):
  descriptions an older import kept as retained `GroupAddress/@Description` data
  are **not** lifted, so recorded history stays valid. They appear after you
  import the `.knxproj` again.

Opening a project for editing migrates the file in place; read-only openers
migrate an in-memory copy. **Alpha 19 refuses schema 17 and later as a newer
schema (`FutureSchemaVersion`).** Comparing a project saved before priority capture
with a fresh ETS6 import lists `saved_priority` for every captured object, so
`knx diff --exit-code` reports a difference; that is recorded knowledge, not an edit.

**The product database moves 31 → 32** (`0ded8d5f9`, merge `5c405bfde`). A
data-only repair corrects parameter types that an earlier migration wrongly
promoted to Color/Time where the authenticated retained source says otherwise. It
changes four existing type columns only; uncertain sources are left untouched with
a named diagnostic. **Alpha 19 refuses v32 with `FutureVersion` and the server
starts without products.**
[Repair contract](../contracts/color-time-successor-repair.md).

**Before upgrading, close KNXBench and keep independent copies of the product
database and the complete server/desktop/Docker data directory**, native projects,
private operation journals and device backups, plus original import files.
Going back means restoring those copies, not just the older binary.

Other downgrade boundaries:

- **System-2 journals:** a run with a tolerated post-load octet records it in
  checkpoint envelope 5; older strict readers refuse it. Status schema 5 and report
  format 4 carry the tolerated octets; Alpha 19's web report decoder accepts
  exactly format 3.
- Journal reuse ([ADR-0163](../adr/0163-journal-writes-keep-unchanged-undo-rows.md))
  changes no file format.

Update `knx-mcp` together with the app.

## Boundaries

- **Not verified:** no new download planning, priority, description or System-2
  change has physical-device acceptance; *untested* means exactly that.
  Simulator/ETS image comparison is not hardware verification. USB/serial has
  bounded receive and single group-write witnesses on one adapter and firmware
  build only; bridge parity/error forwarding, other adapters and long runs are
  unverified. Native WebKitGTK/Tauri, Orca and Firefox acceptance for the new UI
  is open. Returning an upgraded data directory to Alpha 19 is not a tested
  rollback.
- System-2 stays experimental and limited to the admitted application.
- Property-placed plans have no restore or partial download; cross-octet fields,
  packed time units and declared `utf-8` text stay refused.
- CSV: header-less files, other code pages, range descriptions and clearing
  descriptions are unsupported; there is no description editor yet.
- AppImage: a glibc 2.39 ceiling and an offline cross-user start are not universal
  Linux or graphics-driver compatibility.
- KNXBench is KNX-compatible. No KNX Secure, `.knxproj` export or broader
  device-family admission is added.

## Distribution

Release channels retain the Alpha 19 layout: GitHub pre-release
`v0.1.0-alpha.20`, Linux amd64 AppImage
`KNXBench_0.1.0-alpha.20_amd64.AppImage`, `knx-mcp-x86_64-linux`, and
`SHA256SUMS` covering both binaries. Verify a single download with
`sha256sum -c --ignore-missing SHA256SUMS`. The versioned Docker image is
`knxbench/knxbench-server:0.1.0-alpha.20` (without `v`), for `linux/amd64` and
`linux/arm64`; `latest` follows published releases, including alphas. For USB/serial
in Docker, device and container access must be configured on the Linux host.
[Installation](../manual/getting-started/04-installation.md) ·
[Docker update](../manual/user-guide/11-web-and-docker.md#updating-a-docker-hub-installation) ·
[Known issues](../manual/known-issues.md).

## By the numbers

RELEASE-GATE-PENDING

## Acceptance and public verification

RELEASE-GATE-PENDING
