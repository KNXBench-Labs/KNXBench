# KNXBench v0.1.0-alpha.12

Alpha 12 packages the work since Alpha 11: files that bring their product data
along, project edits that remember what still needs programming, and quite a few
small UI repairs from people actually using the thing. The workbench is getting
better at explaining itself. It has not acquired a licence to guess at your bus.

## Fixes reported by users

- **A reopened `.knxdb` can resolve its application programs**
  ([#21](https://github.com/KNXBench-Labs/KNXBench/issues/21)). New native saves carry
  the retained manufacturer files needed to rebuild the catalogue. Docker keeps
  its product database in the mounted data directory, not in a replaceable image
  layer. Older files without those bytes cannot manufacture them retrospectively:
  keep the original `.knxproj` or install the matching `.knxprod` once.
- **AppImage downloads ask where to go.** Session-log and monitor-capture exports
  can use the embedded server again; downloads use a save dialog instead of
  silently replacing a same-named file in Downloads. This addresses the export
  follow-up on [#7](https://github.com/KNXBench-Labs/KNXBench/issues/7), not its
  remaining import-format questions.
- **Names stop showing raw `{{0:…}}` placeholders**
  ([#17](https://github.com/KNXBench-Labs/KNXBench/issues/17)). Verified text-parameter
  references fill supported channel, parameter-block/separator and object labels;
  the source and fallback remain inspectable. Unsupported expressions are not
  turned into plausible-looking names.
- **Create where you are working**
  ([#12](https://github.com/KNXBench-Labs/KNXBench/issues/12),
  [#15](https://github.com/KNXBench-Labs/KNXBench/issues/15),
  [#20](https://github.com/KNXBench-Labs/KNXBench/issues/20)). Buildings and group
  addresses get a single **+ New** menu; a range heading's **+** opens creation in
  that range and suggests the next free address. A full or conflicting range
  still says no.
- **Trees open and close predictably**
  ([#11](https://github.com/KNXBench-Labs/KNXBench/issues/11),
  [#16](https://github.com/KNXBench-Labs/KNXBench/issues/16)). Group ranges start
  compact and remember their open state; a closed object channel actually hides
  its children. Explorer chevrons, counts and optional structure-line motion make
  the hierarchy easier to follow.
- **The Log gets a clock, not a larger button**
  ([#13](https://github.com/KNXBench-Labs/KNXBench/issues/13)). Timestamps, even rows,
  readable link events, compact export and a 16,000-entry server log that drops
  less-important entries before errors. Repeated retained-file notices are
  grouped without losing their individual file list. Export-format radio buttons
  and hover feedback stay where you put them
  ([#14](https://github.com/KNXBench-Labs/KNXBench/issues/14)).
- **Navigation and loading say what is happening.** The download tab names the
  requested device while it loads
  ([#18](https://github.com/KNXBench-Labs/KNXBench/issues/18)); selecting the active
  Bus monitor or Log keeps its current workspace
  ([#23](https://github.com/KNXBench-Labs/KNXBench/issues/23)). The Hall of Fame stays
  a public GitHub link rather than a missing Docker build input
  ([#19](https://github.com/KNXBench-Labs/KNXBench/issues/19)).

## Highlights

- **Pending programming, per part.** Devices shows Adr/App/Par/GA marks, a Pending
  only filter and an explorer dot. Relevant edits and undo affect the marks in
  the same step; server writes settle only the parts they actually cover.
  Manual marks are logged and undoable. These are the project's belief, not a
  live device reading; external/CLI programming still needs reconciliation.
  [Guide](../manual/user-guide/05-devices-and-products.md#programming-marks-p3-source-build-after-alpha-11).
- **Project versus device, deliberately read-only.** Compare one device or the
  current filtered/sorted list after review and confirmation. Session-only tags
  distinguish equal, different, stale, wrong application, no answer and not
  comparable. Verified application identity comes before bounded memory reads;
  supported 0701 address/association tables are decoded and every raw range is
  retained. Cancellation drains the current read before stopping the queue.
  **PART-P4-PENDING:** named parameter comparison awaits the final published
  parameter/corpus prerequisite; no unpublished branch is claimed as shipped.
  [Contract](../contracts/device-comparison.md).
- **A useful report before a device can be programmed.** **File → Device support
  report** works offline without a project, matching product, executable plan or
  gateway. Optional fixed read-only diagnostics require separate target-specific
  consent. Review the preview, consent separately to disclosure, save a local ZIP
  and hand it over manually. No automatic upload, raw vendor/customer files,
  parameter values or programming unlock. This is separate from the older report
  about an ended download, which can contain values.
  [Guide](../manual/user-guide/08-reports-and-diff.md#universal-device-support-reporting-source-after-alpha-11).
- **Changes with context.** Revision-bound parameter/installation writes reject a
  stale edit and preserve the draft. A reloaded web tab resumes the project the
  server still holds; Save As asks before replacing a file. The desktop reload
  prompt protects note/instruction drafts too. Devices made in KNXBench can have
  instructions without an invented imported original.
- **Rooms get Functions.** Named groups of existing group addresses can be
  created and edited at a room, with source identity retained where imported.
  This is project organisation, not a scheduler, device program or new ETS export.
- **More considered day-to-day tools.** Previewed catalogue installs and explicit
  backup deletion, a literal batch naming recipe for group addresses, redacted
  stored bus-access inspection, German report headings, legacy text/long-enum
  parameters and a quiet shimmer on actual active-work text. No progress is
  fabricated; motion preferences still win.
- **A refreshed front door.** The bilingual website and README get current
  real-application stills with a fictional house. Earlier videos remain labelled
  as earlier recordings. The approved Evolution Story is unchanged.

## Upgrade: independent backups first

**Native/model schema moves from 14 to 16.** The migration adds project Functions
(schema 15) and retained manufacturer files (schema 16). **Alpha 11 and earlier
refuse upgraded native files.** Returning requires an independent pre-upgrade
copy; undo and versions inside that same file are not a disaster backup.

**The dedicated product database moves from schema 22 to 23.** It models the
verified text-parameter references and backfills from retained, hash-checked
source blobs. Unsupported or failed backfills remain explicit. Keep the whole
product database as well as project files when backing up Docker data; older
product-database compatibility is not promised.

Close KNXBench before copying `.knxdb` files and the Docker data directory. Keep
the original `.knxproj`/`.knxprod` too. Update standalone `knx-mcp` together with
the application. Read-only CLI operations use in-memory migration rather than
upgrading their input file in place. Older native files do not gain missing
manufacturer bytes until the matching data is actually available and saved.

The Docker port remains **8484**. With a password and Linux host networking it
listens on host interfaces; keep it on a trusted LAN/VPN behind your firewall.
No existing container is replaced merely because an image is published.

## Still alpha — explicit boundaries

- **Not verified:** no new physical gateway/device operation, broad hardware
  support or native WebKitGTK/Orca feature acceptance follows from this release.
  Current browser screenshots use a real server and fictional data in a
  loopback-only namespace, not a live installation.
- Siemens System-2 / `MV-0025` programming remains refused
  ([#22](https://github.com/KNXBench-Labs/KNXBench/issues/22)). Typed TaskCtrl2 and
  property-record handling are offline prerequisites, not complete image/table,
  loading-procedure or durable-recovery support. The new reporter does not
  verify, unlock or program the device.
- Little-endian parameter downloads remain refused. Comparison only decodes
  documented/admitted layouts; unsupported, absent and ambiguous facts remain
  unknown, not equal. A session tag is not continuous monitoring.
- ETS 5.7 import retains its earlier bounded evidence. Nested sub-module object
  IDs and ETS schema 22 remain open on #7; no universal ETS compatibility,
  `.knxproj` export, KNX Secure or individual-address programming is added.
- Colour/picture codecs and module-scope specialised parameter widgets remain
  outside this release. Imported source preservation is not vendor plug-in
  execution or complete manufacturer semantics.

## Distribution

Linux x86-64 AppImage and standalone `knx-mcp` with `SHA256SUMS`; Docker image
`knxbench/knxbench-server:0.1.0-alpha.12` for `linux/amd64` and `linux/arm64`.
`latest` is intended to follow the published release, not moving source.
[Installation](../manual/getting-started/04-installation.md) ·
[Docker update](../manual/user-guide/11-web-and-docker.md#updating-a-docker-hub-installation) ·
[Known issues](../manual/known-issues.md).
