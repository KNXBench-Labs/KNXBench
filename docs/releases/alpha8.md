# KNXBench v0.1.0-alpha.8

Alpha 8 packages the published source changes since Alpha 7. More import evidence,
fewer imaginary defaults — the bus remains blissfully uninvolved in this release.

## Highlights

- **Merge selected lines or devices into the open project.** The File-menu flow
  and CLI share a previewed, conflict-checked application service. Required
  dependencies are mapped explicitly and the merge is one undo step. Consent
  explains that retained complete source archives can include unselected data
  and survive Undo. [Workflow](../SELECTIVE_IMPORT.md).
- **More robust schema-23 imports.** Independent root object lists and recursive
  channel/folder lists are handled without fabricating placement for unassigned
  devices. ZIP member lookup binds validated names to physical entries; encoding,
  collision, CRC and expansion guards remain intact. Both project XML files and
  unsupported source information are retained and explicitly reported.
  [Import integrity](../IMPORT_INTEGRITY.md).
- **Read-only stored module-instance values.** The Inspector, HTTP API and MCP
  readers expose each instance's own stored values separately from defaults,
  sibling instances and evaluated Dynamic values. This does not enable Repeat
  activation, broader parameter writes or additional download eligibility.
  [Design](../adr/0108-read-only-module-instance-values.md).
- **Larger project archive budget with clear feedback.** The cumulative expanded
  `.knxproj` budget is **1024 MiB**, up from 512 MiB. The **64 MiB per-member**
  limit and all archive integrity checks remain. The popup and persistent failure
  banner explain expanded size and the actual server limit in English or German.
  The compressed upload ceiling remains separate; large-project peak memory is
  not newly certified. [Limits](../IMPORT_EXPORT.md).
- **Honest preservation and regression evidence.** Additional synthetic and
  opt-in private tests cover source provenance, native Save/Reopen/Re-Save,
  schema-23 references, manufacturer retry reporting and outside-tree overrides
  without activating unsupported objects. Retained bytes are not complete ETS
  semantics. Diagnostics redact sensitive identities and endpoint data within
  their documented scope.

## Upgrade: back up first

**Native/model schema 12 is required.** The v11→v12 migration is an identity
migration with a completion-status vocabulary barrier, not extra project tables.
Earlier builds refuse upgraded files. Close the application and make an
independent backup of every native project and Docker data directory before
upgrading; downgrade requires the pre-upgrade backup. In-file history is not a
disaster backup. Update the standalone `knx-mcp` binary together with the app.

## Distribution

- Linux x86-64 AppImage: `KNXBench_0.1.0-alpha.8_amd64.AppImage`.
- Linux x86-64 read-only MCP server: `knx-mcp-x86_64-linux`.
- Verify downloads against `SHA256SUMS`; a partial download uses
  `sha256sum -c --ignore-missing SHA256SUMS`.
- Docker Hub: `knxbench/knxbench-server:0.1.0-alpha.8`, for `linux/amd64` and
  `linux/arm64`. `latest` follows published releases, including alphas, not
  moving `main`. Pin the version when controlled upgrades matter.
- The release tag is `v0.1.0-alpha.8`. Publication and artifact verification are
  recorded separately in the implementation history; these notes do not certify
  a pending build.

## Still alpha — explicit boundaries

No ETS-compatible `.knxproj` export, project signing, complete ETS semantics,
new physical-device certification or broader hardware commissioning is claimed.
Unsupported data remains retained/reported where technically possible. Read-only
cvexc analysis does not execute vendor rules. Real historical ETS revision-export
acceptance remains blocked on authorized exports and independently reviewed
baselines. Native accessibility, an interactive screen-reader session and
Raspberry Pi deployment are not established by browser/CI checks.

This release packages published source only; unmerged Functions/Sites work and
other parallel worktrees are not swept into it. Existing running containers and
user project directories are not automatically replaced.
