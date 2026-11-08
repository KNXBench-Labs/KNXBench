# Alpha 20 — pre-release

Alpha 20 brings everything since `v0.1.0-alpha.19`. The headline items: group
addresses from ETS CSV exports with a choice of text encoding and a new
description field, a Linux USB connection to the KNX bus, more devices that
KNXBench can prepare a download for, priorities you can see and change on
communication objects, much faster editing in saved projects, and an AppImage
that starts again for every user, not just for the one who built it.

**Before you upgrade, keep a copy of your projects and data.** Projects opened
with Alpha 20 cannot be opened with Alpha 19 any more (see [Upgrade](#upgrade)).

## At a glance

| Area | What is new |
|---|---|
| Group addresses | Import ETS6 CSV exports, choose UTF-8 or Windows-1252, new **Description** field |
| Bus connection | USB/serial TP-UART adapter on Linux, connection status in the status bar |
| KNXnet/IP | Optional TCP tunnelling for the bus monitor and group telegrams; UDP stays the default |
| Downloads | More devices can be prepared: push-buttons, room controllers, property-based actuators, dimmers |
| Communication objects | Priority shown, editable and filterable; saved folder names visible |
| System-2 | A verified download keeps its "done" status after unrelated edits |
| Speed | Editing a saved project takes about 2 s per change instead of up to 16 s |
| Look and feel | New **Rail** and **Rail Dark** themes, **Compare** straight from the device editor |
| Packaging | The AppImage starts for every user again and runs on older Linux systems |

## For reporters first

| Issue | Status in Alpha 20 |
|---|---|
| [#35](https://github.com/KNXBench-Labs/KNXBench/issues/35) Dropdown stays disabled after a change | **Fixed.** Nothing was stuck; every edit was just very, very thorough. See [Faster editing](#faster-editing). |
| [#34](https://github.com/KNXBench-Labs/KNXBench/issues/34) System-2 panel shows another device's operation | **Fixed.** The panel now says clearly when a kept operation belongs to another device or project. |
| [#30](https://github.com/KNXBench-Labs/KNXBench/issues/30) System-2 download for UP 258D12 | **Improved, still open.** The one byte the device clears itself after loading (`Download_Flag`) is now expected for this exact application; every other byte is still checked strictly. Only new download preparations use the new rule. |
| [#22](https://github.com/KNXBench-Labs/KNXBench/issues/22) Siemens presence detector (`MV-0025`) | **Partly.** The same `Download_Flag` rule covers its shared application; nothing else is new. |
| [#10](https://github.com/KNXBench-Labs/KNXBench/issues/10) Devices in the reporter's project | The follow-up work ships here: more than 50 of the project's devices moved from *unsupported* to *untested*. None was written to real hardware for this work. |
| [#7](https://github.com/KNXBench-Labs/KNXBench/issues/7) ETS 5.7 project import | **Nothing new for the importer.** Schema-20 projects now show their saved folder names. |

## Group addresses

- **Import ETS6 CSV exports.** Group-address lists exported from ETS6, including
  the "CSV (ETS3 Format)" variant and tab-separated files, can be imported
  directly. Files without a header line are still refused.
- **You pick the encoding; KNXBench never guesses.** No more `GrÃ¼n` instead of
  `Grün`. Choose **UTF-8** (default) or
  **Windows-1252 (ETS export)** next to the import button. On the command line:
  `knx ga-import … --encoding windows-1252`. If the file does not match the chosen
  encoding, the import stops and names the position of the first bad byte.
- **New description field.** Every group address can now carry a description. It
  is filled from `.knxproj` imports and from CSV files with a description column,
  so "1/2/3" can finally explain what it is for.
  An empty CSV cell never deletes an existing description. The inspector shows the
  description; an editor for it follows later.
- **Datapoint type for new addresses.** If the CSV names a datapoint type, it is
  applied to the addresses the import creates. Existing addresses keep theirs.

More: [CSV guide](../manual/user-guide/04-group-addresses.md#group-addresses-as-csv) ·
[ADR-0164](../adr/0164-group-address-description-and-csv-encoding.md) ·
[KL-038](../KNOWN_LIMITATIONS.md#catalogue-kl-038)

## Bus connection

- **USB on Linux.** Besides KNXnet/IP gateways, the bus monitor and group telegrams
  can now use a USB/serial TP-UART adapter. Choose **Passive monitor** (only
  listens) or **Active group communication**. One adapter type is supported:
  Busware TUL with NCN5130 USB-serial firmware. KNX USB HID interfaces and other
  adapters are not supported yet. Downloads and scans still need KNXnet/IP; the USB
  stick is a good listener, not yet a programmer.
- **Find your adapter.** **List USB-serial candidates** shows the serial devices the
  server sees, without opening or probing them. Looking is not touching.
- **Connection in the status bar.** The footer shows whether the bus is connected
  and offers Connect and Disconnect. Changing the interface in Settings never
  switches a running connection.
- **Clear "busy" answers.** While a download or another bus job is running,
  starting the monitor is refused right away instead of waiting in the background.

More: [Bus guide](../manual/user-guide/07-bus-and-interfaces.md#bus-connection-in-the-status-bar) ·
[USB on the command line](../manual/user-guide/10-command-line.md#usbserial-tp-uart-linux) ·
[USB acceptance and limits](../status/2026-11-04-usb-serial-ui.md) ·
[KL-062](../KNOWN_LIMITATIONS.md#catalogue-kl-062)

## KNXnet/IP TCP tunnelling (optional)

Some KNXnet/IP interfaces also offer tunnelling over TCP. KNXBench can now use it,
on request, for a first and deliberately small set of jobs:

- **Bus monitor** in the web app and desktop, with group read/write from the same
  session: choose **TCP (optional)** in the IP transport fields before connecting.
- **Command line:** `knx bus monitor` and `knx bus write` with
  `--tunnel-transport tcp`.

**UDP stays the default**, and nothing switches to TCP on its own. If TCP is
refused, KNXBench says so instead of quietly falling back to UDP; the session
label shows the transport actually in use, not the one you wished for.
Downloads, device management, scans, discovery and routing still use UDP.

Worth knowing: TCP has no tunnelling acknowledgement, so "sent" means the network
accepted the frame, not that a device on the bus acted on it. There is no
automatic reconnect. Plain TCP is not KNX IP Secure. This was tested against
local test peers, **not yet against a real KNX interface** — so it is
TCP for the curious, not TCP for the commissioning deadline.
[Guide](../manual/user-guide/07-bus-and-interfaces.md) ·
[Command line](../manual/user-guide/10-command-line.md) ·
[Contract](../contracts/tcp-tunnelling.md)

## Downloads

KNXBench can now prepare downloads for more kinds of devices. Each was compared
with downloads recorded from ETS 6.4 in KNXBench's simulator, not on real devices,
so they are marked **untested** and need your explicit confirmation
(`--accept-untested` on the command line).

| Now supported | Typical devices | Details |
|---|---|---|
| Numbers stored low byte first ("little-endian") | Push-buttons, room controllers | [ADR-0159](../adr/0159-little-endian-numbers-and-fixed-load-steps.md), [KL-170](../KNOWN_LIMITATIONS.md#catalogue-kl-170) |
| Text without a declared encoding, written as Windows-1252 like ETS does | Devices with text labels | [KL-171](../KNOWN_LIMITATIONS.md#catalogue-kl-171) |
| Parameters kept in device properties, device reset the ETS way | Property-based actuators | [ADR-0161](../adr/0161-property-placed-parameters-and-master-resets.md) |
| Time parameters and fixed property values | Dimmers | [ADR-0162](../adr/0162-time-parameters-and-inline-property-writes.md) |

Property-based devices cannot use restore or partial download yet. When a
device's program cannot be read, the reason given is now more precise.

## Communication objects

- **Priority you can see.** The object editor shows each object's priority and
  where it comes from: your change, the value saved in the ETS project or the
  product default. Unknown cases say why instead of silently showing "Low".
- **Priority you can change.** Set Low, High or Alert in the object editor; undo
  works as usual. An optional **Priority** column with a filter is available in the
  object table.
- **Programming stays careful.** KNXBench does not write a changed priority to a
  device yet, so programming such a device is refused with a clear message
  ([KL-188](../KNOWN_LIMITATIONS.md#catalogue-kl-188)).
- **Folder names.** Objects from ETS projects (schema 20) show the folder names
  saved in the project, and the table warns when a folder assignment may be
  outdated because the device was changed in KNXBench.

## System-2

- A verified download no longer loses all of its "done" status to an unrelated
  edit. Renaming, or editing another device, keeps it; only the parts you really
  changed become pending again. A typo fix is no longer a full re-download.
- A product update that changes only the program's name still counts as the same
  program.
- Recovery after an interrupted download runs faster, with the same checks.

System-2 stays experimental and limited to the admitted application.

## Faster editing

In saved projects with a long undo history, each change used to take 6 to 16
seconds on the reporter's project
([#35](https://github.com/KNXBench-Labs/KNXBench/issues/35)). KNXBench now keeps
the unchanged parts of the history instead of rewriting them, and the same project
needs about 2 seconds per change, however long the history is. Your undo history
stops reciting itself in full on every keystroke. A row that is being saved shows
**Saving…**.
[ADR-0163](../adr/0163-journal-writes-keep-unchanged-undo-rows.md)

## Workbench, desktop and web

- **Rail and Rail Dark:** two new built-in themes with a sharper look, a bus-cable
  green accent and a measuring rail under the toolbar; their effects turn off with
  reduced motion.
- **Compare from the device editor:** a **Compare** button next to Download opens
  the read-only comparison for this device.
- **Import notes after reopening:** the session log shows a project's import
  findings again when you reopen it.
- **Date parameters:** a checked date field with an explicit Apply button.
- **Fixes:** group-address views below a selected range no longer appear empty,
  project-history limits are shown readably (`953.7 MiB` instead of eleven
  decimals), catalogue jobs stop polling once they are done, and device backups are
  written more safely.

## AppImage

The Alpha 19 AppImage was a little shy: it could only be started by the user who
owned its files ("Permission denied" for everyone else) and needed a very new Linux system
(glibc 2.43). Both are fixed. The AppImage is built on Ubuntu 24.04 again and
needs glibc 2.39 or newer. Before publication, every release image is checked for
file permissions and started as a different user without network access.
[Packaging record](../status/2026-11-08-appimage-catalog.md)

## Changed behaviour

- Programming refuses a device whose object priority in the project differs from
  the product's priority ([KL-188](../KNOWN_LIMITATIONS.md#catalogue-kl-188)).
- Devices that were refused because of little-endian numbers, property-based
  parameters, time units or undeclared text encoding now appear as *untested*.
- `.knxproj` imports store group-address descriptions as real fields.
- Starting the bus monitor while the bus is busy is refused immediately.
- No changes to ports, data directories or Docker tags.

## Upgrade

| What | Alpha 19 | Alpha 20 | What it means for you |
|---|---|---|---|
| Project file (`.knxdb`) | schema 16 | schema 19 | Converted automatically when opened for editing. **Alpha 19 refuses converted files.** |
| Product database | version 31 | version 32 | Converted when KNXBench starts. **Alpha 19 then starts without products.** |
| System-2 journals | — | new entries for the `Download_Flag` rule | Alpha 19 cannot read those new entries. |

What the conversions do:

- **Project schema 17 to 19** add room for saved priorities, your priority changes
  and group-address descriptions. Nothing existing is changed. Descriptions from
  older imports are not filled in automatically: import the `.knxproj` again to get
  them.
- **Product database 32** corrects a few parameter types that an earlier version
  wrongly marked as color or time values, only where the original product file
  proves it. [Details](../contracts/color-time-successor-repair.md)

**Before upgrading, close KNXBench and copy** your projects, the product database,
the whole data directory (desktop, server or Docker volume), device backups and the
original import files. Going back to Alpha 19 means restoring those copies;
reinstalling the older program alone is not enough. Update `knx-mcp` together with
the app.

## Boundaries

- **Not verified:** the new download types, priorities, descriptions and System-2
  changes were not tested on real devices; *untested* means exactly that, not
  "probably fine". USB was
  tested with one adapter and one firmware only. TCP tunnelling was not tested
  with a real KNX interface. The new screens were not tested in
  the native desktop window (WebKitGTK/Tauri) or with screen readers. Going back
  from Alpha 20 to Alpha 19 is not tested; restore your copies instead.
- CSV: files without a header, other encodings and clearing descriptions are not
  supported yet.
- The AppImage's glibc requirement is not a promise for every Linux distribution or
  graphics driver.
- KNXBench is KNX-compatible. There is no KNX Secure support and no `.knxproj`
  export.

## Distribution

| Download | Name |
|---|---|
| GitHub pre-release | `v0.1.0-alpha.20` |
| Linux AppImage (amd64) | `KNXBench_0.1.0-alpha.20_amd64.AppImage` |
| MCP server | `knx-mcp-x86_64-linux` |
| Checksums | `SHA256SUMS`; check with `sha256sum -c --ignore-missing SHA256SUMS` |
| Docker (amd64 and arm64) | `knxbench/knxbench-server:0.1.0-alpha.20`; `latest` follows releases |

To use USB in Docker, device and container access must be configured on the Linux
host. [Installation](../manual/getting-started/04-installation.md) ·
[Docker update](../manual/user-guide/11-web-and-docker.md#updating-a-docker-hub-installation) ·
[Known issues](../manual/known-issues.md)

## By the numbers

RELEASE-GATE-PENDING

## Acceptance and public verification

RELEASE-GATE-PENDING
