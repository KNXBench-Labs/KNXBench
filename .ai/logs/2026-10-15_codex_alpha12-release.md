# Alpha 12 release preparation — 15 October 2026

Owner request: release Alpha 12, refresh the main website with current UI media,
and review README/documentation without changing the established dry engineering tone.

## Cutoff and ownership

- Baseline: published main `bee4f016`; previous release `v0.1.0-alpha.11`.
- Sole admitted in-flight prerequisite: P4 parameter comparison plus its separately
  committed corpus corrections (`a6cdfd9d`). The package is already in final gates.
- All other sessions pause new integration/publication until release or explicit abort.
- Release branch/worktree owned by this task; dirty selected root is not synchronized.
- No KNX traffic, hardware permission, Story content changes or vendor contact.

## Required acceptance (pending)

Full source-bound release gate, actual browser media review and localized website
acceptance; AppImage/MCP and multi-architecture Docker workflow dry runs with publishing
skipped; exact candidate presented for final publication decision. Only after that:
annotated immutable tag, hand-written prerelease, artifact checksum/download verification,
registry version/latest digest and isolated published-image smoke, and live Pages readback.

Self-review will be labelled as such, not independent approval. Prior package gates
remain historical evidence, never relabelled as an Alpha 12 run.
