# LS-07 — Optional hardening and catalogue-cleanup extensions

**Status:** Later scope. Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is the [ledger](../status/LEDGER.md).

Each needs a named requirement before assignment; existing safe refusals stay
in force.

## Hardening

- Native hung-webview recovery design.
- Redacted authorisation/restart status presentation.
- The quiet-poll `Lagged` regression.
- Descriptor-pinned backup paths.
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
