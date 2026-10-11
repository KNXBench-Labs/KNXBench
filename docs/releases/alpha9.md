# KNXBench v0.1.0-alpha.9

Alpha 9 packages a frozen published-source snapshot after Alpha 8, with import,
native-history and everyday usability repairs. Less friction, no compatibility
fairy dust.

## Highlights

- **Large imported projects stay editable.** Bounded native working-history
  images now account for retained project context without silently losing it;
  import diagnostic counters stay separate from mapping/retention counters.
  Save, reopen, edit, undo and redo share the existing application services.
  [History](../PROJECT_HISTORY.md) · [Import integrity](../IMPORT_INTEGRITY.md).
- **DPT selection is easier to read.** The communication-object editor offers
  named English/German datapoint choices while preserving exact stored values
  and distinguishing project overrides from application defaults.
  Display vocabulary is not new codec or download support.
  [Contract](../contracts/dpt-selection.md).
- **Imported installation nodes are understandable.** Unnamed installations
  get translated display-only labels; selecting their label opens the existing
  name properties while arrows expand the tree. Imported names are not rewritten
  just to improve a label. [Contract](../contracts/installation-node.md).
- **Login and notification details have clearer context.** Login shows the
  application logo and frontend build version. Message actions navigate to exact
  browser-local details, with raw text copying and a selected-text fallback.
  Browser notification history remains distinct from server JSON logs.
  Streamed file uploads and raw evidence admission reach 1024 MiB, with separate
  parser/member/manufacturer/disclosure limits unchanged.
  [Contract](../contracts/LOGIN_FEEDBACK_AND_FILE_LIMITS.md).
- **Import progress and download configuration are clearer.** Import feedback
  explains the current operation; the four download-scope radio controls retain
  compact labels and keyboard behavior. This does not change download planning,
  confirmation requirements or supported hardware.

## Upgrade: independent backups first

**Native/model schema 13 is required.** Alpha 8 used schema 12. Schema 13 adds
native device-instruction overlay tables, separate from exact imported originals.
The predecessor migration and native saved/working/history/version preservation
are covered by automated regression tests; this is not downgrade support.
Alpha 8 and earlier refuse upgraded files.

Close the application and independently back up native project files and the
Docker data directory before upgrading. Returning to Alpha 8 requires the
pre-upgrade backup; in-file undo/version history is not a disaster backup.
Update the standalone `knx-mcp` binary together with the application.
[Model/storage decision](../adr/0112-native-instruction-overlays.md).

## Included foundations, not promised workbench features

The frozen source includes bounded instruction read/preview/apply services,
native overlay persistence and isolated tested instruction/parameter components.
**Those editors are not mounted in the production workbench in this snapshot.**
Do not expect a new instruction editing tab or a fully integrated time/slider
editor. Colour/picture codecs and module-scoped specialized mapping remain
unsupported. [Instruction host contract](../contracts/instruction-editor-host.md)
· [Parameter host contract](../contracts/parameter-editor-host.md).

Local-only import-report/payload/metadata work, unfinished Functions/Sites,
project notes and later parallel packages are not swept into this release.
Manufacturer-component research is not a functional parameter host and does
not add product compatibility.

## Distribution

- Linux x86-64 AppImage: `KNXBench_0.1.0-alpha.9_amd64.AppImage`.
- Linux x86-64 read-only MCP server: `knx-mcp-x86_64-linux`.
- Verify downloads with `SHA256SUMS`; for a partial download use
  `sha256sum -c --ignore-missing SHA256SUMS`.
- Docker Hub: `knxbench/knxbench-server:0.1.0-alpha.9`, for `linux/amd64` and
  `linux/arm64`. `latest` follows published releases, including alphas; it does
  not follow moving `main`. Pin a version for controlled upgrades.
- The tag is `v0.1.0-alpha.9`. Build/publication and independently downloaded
  artifact verification are recorded separately; these notes alone do not
  certify an unfinished build.

## Still alpha — explicit boundaries

No ETS-compatible `.knxproj` export, signing, complete ETS semantics, new
physical-device certification or expanded commissioning eligibility is claimed.
Unsupported data stays preserved/reported within the documented adapter scope.
Browser/CI evidence is not an interactive native accessibility/Orca session,
Raspberry Pi deployment or a large-project peak-memory guarantee.
Existing running containers and user data are not automatically replaced.
