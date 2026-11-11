# WP-38 — Pending-devices panel and batch download (#42)

External report by the issue reporter, 10 November 2026; triaged 11 November 2026.
Status: **open.** Depends on WP-34 copy.

## Packages

1. Read-only side panel listing every device with pending changes, what is
   pending, and a jump to its download dialog.
2. Sequential “download all changes” is a multi-device bus write: design and
   ADR first (per-device readiness before start, one batch consent, stop at the
   first failure, per-device results), then implementation with simulator
   tests only unless hardware is separately authorised.
