# KNXBench v0.1.0-alpha.11

Alpha 11 packages a frozen published-source snapshot after Alpha 10. It carries
three fixes reported by a tester (a group-address picker you can type into,
trees that indent, and a gateway that says who is holding it), the parked UX
feedback package, and the first half of KNXBench's answer to "little-endian
devices": a read-only probe, a named warning and a way for testers to tell us
how a download went. No new file format, so this one is mostly about manners.

## Fixes reported by users

- **Searchable group-address picker when linking** ([#8](https://github.com/KNXBench-Labs/KNXBench/issues/8)).
  The link row's long drop-down is now a combobox that filters while you type:
  every word must match the address (in the shown notation or a compatible
  spelling), its name, DPT or stored reference. Each option shows the address's
  DPT and marks a difference from the object's type (*Different type* /
  *Same encoding, different subtype*); nothing is hidden or reordered. Drag and
  drop and the direction choice are unchanged.
  [KL §178](../KNOWN_LIMITATIONS.md#178-the-group-address-picker-filters-and-marks-it-does-not-judge).
- **Expanded content indents under its parent** ([#9](https://github.com/KNXBench-Labs/KNXBench/issues/9)).
  Group-address tree rows take their level like the range headers (capped at
  twelve levels; flat views stay flat). In the devices view *By channel*, the
  channel header reads from the left and its objects start one step in.
  [KL §179](../KNOWN_LIMITATIONS.md#179-tree-indentation-is-capped-at-twelve-levels-flat-views-stay-flat).
- **"Gateway not found during programming" now says why** ([#10](https://github.com/KNXBench-Labs/KNXBench/issues/10)).
  The cause in the report was KNXBench's own bus monitor still holding the
  gateway's only tunnel, and the download tab learned so only after *Start*,
  in English server prose. Now the download tab says up front what holds the
  gateway (bus monitor with its gateway, line scan with progress, a download or
  address programming), offers a stop button and the monitor's gateway as a
  suggestion, and says when an empty search means the typed address is used
  directly. A gateway's own refusal is named from the KNXnet/IP specification
  instead of `status 0x24`; a silent gateway is called silent. Devices KNXBench
  cannot program yet (little-endian parameter byte order, see below) are marked
  with a plain reason before anything is prepared. The plan step is now
  **Prepare download** / **Download vorbereiten** (*nothing is written yet*), and
  the German download texts were reviewed.
  [ADR-0123](../adr/0123-structured-gateway-refusals.md) ·
  [KL §180](../KNOWN_LIMITATIONS.md#180-gateway-refusals-name-knxbenchs-own-sessions-and-the-gateways-stated-reason-nothing-more).
- **Issues #6 and #7: nothing new since Alpha 10.** Both fixes shipped in Alpha 10
  and are unchanged here. #7 stays open until the reporter's own ETS 5.7 project
  imports; nested sub-module object numbers and schema 22 (ETS 6.1) remain open.

## Highlights

- **Helping to verify a device.** Every download can produce a tester report:
  program, version and program-file hash, manufacturer, mask, scope, support
  level, the plan's inferences and warnings, the outcome, and the parameter
  values written. It never contains a project or device name, an individual
  address, a gateway, a key, a backup path or an error text. Reports go by hand,
  as a prefilled GitHub issue or by mail to `issue@knxbench.com`; a maintainer
  reviews each one before a program may ever be marked verified. CLI:
  `knx device download … --report <file>`; server:
  `GET /api/device-download/report`. In the device download tab, a plan's
  warnings appear in a **Before you write** box in plain words (the server text
  under *Technical details*), its assumptions are listed, and after every ended
  run **Help verify this device** walks through five steps: create the report on
  request, tick a checklist, save it, open the prefilled issue form or send it by
  mail. Nothing is sent automatically.
  [ADR-0121](../adr/0121-tester-download-reports.md) ·
  [review contract](../contracts/download-verification-reports.md).
- **Little-endian devices: evidence, not guesses.** Downloads of programs
  declaring `ParameterByteOrder="LittleEndian"` are still refused (KL-170). New:
  `knx device byte-order-probe` reads (never writes) an ETS-programmed device and
  reports per field whether it holds values high or low octet first, and a
  download whose own product base image contradicts its big-endian declaration
  carries the named warning `base-image-low-octet-first` (32 of 181
  memory-downloadable corpus programs, all ABB/Busch-Jaeger `0701h`), shown in
  the CLI, the server plan and the download tab.
  [ADR-0119](../adr/0119-read-only-byte-order-probe.md) ·
  [ADR-0120](../adr/0120-base-image-byte-order-warning.md) ·
  [KL §170](../KNOWN_LIMITATIONS.md#170-little-endian-parameter-byte-order-is-refused-for-download-images).
- **Steadier notifications.** At most three cards on screen, each card's time
  runs only while it is visible and unpaused, waiting cards are counted, and an
  unread error is no longer replaced by the next one or cleared by a selection
  or import. Import error/warning counts open the log narrowed to them.
- **Flow snapshots and clips.** With consent, a PNG of the selected Flow view or
  a 1–30 s GIF clip, encoded in the browser.
  [Contract](../contracts/flow-image-export.md).
- **Motion per category.** Toasts, achievements and Flow can each turn motion
  down (never up) from the global setting; the OS reduced-motion preference
  always wins ([ADR-0122](../adr/0122-category-motion-preferences.md), KL-43).
  Also four new achievement milestones and three fresh error jokes.
- **Notes on communication objects.** The notes editor gets its target list from
  the service, communication objects included; a browser tab asks before closing
  while a note or instruction draft is unsaved; the review names actions in
  English and German.

## Upgrade: independent backups first

**Native/model schema stays 14**, as in Alpha 10. The project store and the
domain model are unchanged since Alpha 10, so there is no migration and existing
files are not rewritten on open. Alpha 9 files (schema 13) are migrated to 14 as
in Alpha 10 (covered by automated regression tests; not downgrade support).
Opening Alpha 11 files in Alpha 10 was not tested.

Close the application and independently back up native project files and the
Docker data directory before upgrading anyway; in-file undo/version history is
not a disaster backup. Update the standalone `knx-mcp` binary together with the
application.

No changed defaults: the Docker port stays 8484. Server error responses keep
their `error` text; new structured fields (`kind`, `holder`, gateway status) are
added next to it, so existing scripts keep working.

## Still alpha — explicit boundaries

- **Not verified:** none of the new screens was exercised in the native
  WebKitGTK window (Chromium only). The #10 work was tested with a fake gateway connector
  and injected KNXBench sessions; no real gateway, device or bus was contacted.
  The tester-report flow has had no real tester report yet.
- Little-endian parameter downloads stay refused (KL-170); the warning and the
  probe add evidence, not support. Programs flagged by the warning remain untested
  downloads.
- The group-address picker marks type differences; it does not decide what is
  allowed beyond the existing linkable set.
- Gateway refusals can name only KNXBench's own sessions and the gateway's stated
  reason; other clients (ETS, visualisations) are invisible except through that
  reason.
- Flow GIFs use a fixed palette (approximate colours) and are refused, not
  thinned out, on a machine too busy to capture on time.
- No ETS-compatible `.knxproj` export, signing, complete ETS semantics, new
  physical-device certification or expanded commissioning eligibility is claimed.
  Functions/Sites and the dropped inventory cards (AP4) are not in this snapshot;
  colour/picture parameters keep their fallback (PDB-04).

## Distribution

- Linux x86-64 AppImage: `KNXBench_0.1.0-alpha.11_amd64.AppImage`.
- Linux x86-64 MCP server: `knx-mcp-x86_64-linux`.
- Verify downloads with `SHA256SUMS`; for a partial download use
  `sha256sum -c --ignore-missing SHA256SUMS`.
- Docker Hub: `knxbench/knxbench-server:0.1.0-alpha.11`, for `linux/amd64` and
  `linux/arm64`. `latest` follows published releases, including alphas.
- The tag is `v0.1.0-alpha.11`.
