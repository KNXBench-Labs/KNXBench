# WP-09 — Offline image evidence gates

**Status:** Blocked: evidence. Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is the [ledger](../status/LEDGER.md).

## Task

Settle each offline-image encoding question with defining evidence. Each is a
separate question; a probe for one does not settle the others:

- KL-170: per-type byte placement (byte order).
- KL-171: UTF-8 byte capacity, termination and cut rule.
- KL-172: `LoadedImage`/`CheckSums` payload.
- G10: hidden PID-78 parameter placement (write, compare or ignore).
- G14: segment `Data` shorter than `Size` (fill rule).

## Current state

The generic System-2 entry refuses an explicit `DownloadInvisibleParameters`
declaration; Alpha 16 scoped only the rules established for the exact UP258
path. A read-only byte-order probe and the `base-image-low-octet-first`
warning exist.

## Done when (per question)

A defining source or a separately approved, discriminating observation
establishes the exact rule, negative controls reject the plausible
alternatives, and a reviewed bounded implementation retains unsupported
cases. Never write PID 78, pad a segment, swap octets or treat retained
compressed bytes as download authority on suspicion.

## References

[System-2 integrity research](../research/2026-10-20-system2-download-integrity.md) ·
[source stop lines](../research/2026-10-16-offline-source-stop-lines.md) ·
[census 3](../research/2026-10-18-product-database-programmability-census.md) ·
[schema-23 boundaries](../research/schema23-offline-boundaries.md) ·
[byte order](../research/parameter-byte-order.md) ·
[KL-170](../limitations/KL-170-parameter-byte-order-download-boundary.md) ·
[KL-171](../limitations/KL-171-utf-8-text-download-encoding.md) ·
[KL-172](../limitations/KL-172-stored-loaded-image-semantics.md) ·
[history](../history/OPEN_WORK_2026-10-25.md#wp-09--offline-image-evidence-gates)
