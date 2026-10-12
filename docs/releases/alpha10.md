# KNXBench v0.1.0-alpha.10

Alpha 10 packages a frozen published-source snapshot after Alpha 9. It carries
two user-reported fixes (an AppImage that would not start on Mesa systems, and
ETS 5.7 project import) plus the workbench features that were still parked
behind the Alpha 9 cutoff. Fewer crash reports, more things you can click.

## Fixes reported by users

- **AppImage starts on Mesa desktops again** ([#6](https://github.com/KNXBench-Labs/KNXBench/issues/6)).
  Alpha 9 bundled the build machine's `libwayland-client`; newer Mesa EGL
  vendors need newer symbols, so WebKit aborted with
  `Could not create default EGL display: EGL_BAD_PARAMETER` under X11 and
  Wayland alike (seen on Kubuntu 24.04). The image now uses the host's library,
  and a release-workflow check keeps it out.
- **ETS 5.7 projects import** ([#7](https://github.com/KNXBench-Labs/KNXBench/issues/7)).
  Project schema 20 is recognised; the import report names the evidence
  boundary. Password-protected ETS4/ETS5 exports, which keep their inner
  `0.xml` in the archive root, now decrypt. Schema 22 (ETS 6.1) is still
  unmeasured, and nested sub-module object numbers are not decoded.

## Highlights

- **Project notes.** Plain-text notes attached to the project, topology,
  devices, group ranges, group addresses or communication objects, edited in
  the workbench behind a preview and undoable. The "include in documentation"
  flag is stored with each note; no documentation export reads it yet. [Service contract](../contracts/project-note-service.md).
- **Group addresses four ways.** Tree, column browser, address atlas and
  connection matrix share one selection and Inspector; pick whichever fits the
  task. Members can be added to a group address, not only removed.
- **Device instructions and parameter editors in the workbench.** Installation
  instructions get their own tab. Checked time-count and integer-slider
  parameter editors open on request; colour/picture parameters keep the
  existing fallback.
- **Catalogue updates with a dress rehearsal.** A product-catalogue update can
  be previewed before installation, and installing keeps a restorable backup,
  from the CLI (`knx products update` / `restore`) and from the web catalogue.
  [ADR-0117](../adr/0117-previewed-recoverable-catalogue-updates.md) ·
  [ADR-0118](../adr/0118-catalogue-updates-through-the-web-server.md).
- **Import evidence that survives saving.** Historical import reports, retained
  source payloads and modeled project metadata stay in the native file and are
  shown in the Overview; recorded device state is shown as recorded, never as a
  live bus reading. Native export asks before carrying retained originals.
  [ADR-0113](../adr/0113-durable-native-import-evidence.md).
- **Clearer everyday screens.** Overview warnings open the matching log
  entries; unsaved-changes dialogs have labelled buttons; DPTs show their names
  first (raw XML ids on demand); the download tab suggests the discovered
  gateway and links from devices.
- **Release notifications.** The workbench checks GitHub's release list at startup
  and hourly and shows a notice for newer versions; nothing is downloaded or
  installed, and Settings → Updates switches the check off. [Contract](../contracts/RELEASE_NOTIFICATIONS.md).
- **MCP can rename group addresses, with a human nod.** `knx-mcp` stays
  read-only by default. An explicitly enabled writer applies literal
  prefix/suffix naming recipes only after a preview and a client-side
  confirmation form. [ADR-0116](../adr/0116-opt-in-mcp-name-recipe-writer.md).

## Upgrade: independent backups first

**Native/model schema 14 is required.** Alpha 9 used schema 13. Schema 14 adds
the project-note tables; existing data is not rewritten. The 13→14 migration is
covered by automated regression tests; this is not downgrade support.
Alpha 9 and earlier refuse upgraded files.

Close the application and independently back up native project files and the
Docker data directory before upgrading. Returning to Alpha 9 requires the
pre-upgrade backup; in-file undo/version history is not a disaster backup.
Update the standalone `knx-mcp` binary together with the application.

**Docker default port is now 8484**, and the documented Linux recipe uses host
networking. Containers created from older recipes with explicit `-p`/`KNX_PORT`
keep working; check your run command before replacing a container.

## Still alpha — explicit boundaries

No ETS-compatible `.knxproj` export, signing, complete ETS semantics, new
physical-device certification or expanded commissioning eligibility is claimed.
Functions/Sites and the remaining UX-feedback package are not in this snapshot.
Manufacturer-component research (SmartSensor, FastDownload) is research only and
adds no product compatibility.

## Distribution

- Linux x86-64 AppImage: `KNXBench_0.1.0-alpha.10_amd64.AppImage`.
- Linux x86-64 MCP server: `knx-mcp-x86_64-linux`.
- Verify downloads with `SHA256SUMS`; for a partial download use
  `sha256sum -c --ignore-missing SHA256SUMS`.
- Docker Hub: `knxbench/knxbench-server:0.1.0-alpha.10`, for `linux/amd64` and
  `linux/arm64`. `latest` follows published releases, including alphas.
- The tag is `v0.1.0-alpha.10`.
