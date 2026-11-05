# LS-07 — Optional hardening and catalogue-cleanup extensions

**Status:** Later scope. Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is the [ledger](../status/LEDGER.md).

Each needs a named requirement before assignment; existing safe refusals stay
in force.

## Hardening

- Native hung-webview recovery design.
- Redacted authorisation/restart status presentation.
- Quiet-poll `Lagged` coverage is software-accepted in the [bounded test-only record](../status/2026-11-05-quiet-poll-event-loss.md):
  four real-overflow/healthy tests, independent review, calibrated mutations and
  the selected 14-stage main-first gate passed on 5 November 2026. Runtime and
  physical acceptance are unchanged; no remaining work is assigned by this bullet.
- Descriptor-pinned JSON backup paths are implemented in three bounded parts:
  held directory-chain acquisition/synchronization, device-memory writer, and
  service-control writer. The [contract](../contracts/descriptor-pinned-json-backups.md)
  preserves static aliases and `..`, uses same-file-descriptor readback and
  refuses detected final identity changes. Focused software tests passed;
  maintained package acceptance and publication remain pending in the
  [execution record](../status/2026-11-05-descriptor-pinned-json-backups.md).
  Catalogue SQLite/list/delete/restore and the legacy property-validation
  recorder are excluded. Namespace confinement, permanent paths and power-loss
  recovery are not established.
- Profiled large-device navigation.

## Catalogue cleanup (not built, by decision)

- A persistent "hidden" marker so a different package declaring the same
  placements does not bring them back (needs a product-DB migration).
- Usage counts across saved projects (the preview counts the open project).
- Removing whole application programs or products, with a project scan.
- Measured provenance/cleanup cost on a real large catalogue.

## References

[history: hardening](../history/OPEN_WORK_2026-10-25.md#deliberate-later-scope-and-owner-decisions--not-ready-made-coding-jobs) ·
[history: catalogue cleanup](../history/OPEN_WORK_2026-10-25.md#catalogue-cleanup-follow-ups-19-october-2026)
