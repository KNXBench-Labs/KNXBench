# DIAG-01 — System-2 memory diagnostic profile in the web UI

**Status:** Implemented on branch `feat/diag01-system2-web-20261026` (26 October
2026); independent review and integration pending. Simulator-verified only, no
hardware run. Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is
the [ledger](../status/LEDGER.md).

## Task

Offer the read-only `system2-declared-segments-v1` diagnostic profile
([ADR-0143](../adr/0143-declared-segment-memory-evidence-profile.md)) in the
web UI. Until this change it was CLI-only (`knx device diagnostics --device
<area.line.device>`).

## Delivered

- **Server:** `POST /api/device-diagnostics/preview` requires `profile`
  (`generic-identity-v1` or `system2-declared-segments-v1`; another value is
  `400 diagnostics-unknown-profile`). The memory profile requires the server's
  product database (`400 diagnostics-product-db-required`, the CLI's
  `--product-db` rule) and is prepared by the same `prepare_memory` call as the
  CLI, whose refusals become `400` with their leading token. The job holds an
  opaque plan of exactly one profile and an `Export`; `acquire_memory` runs with
  the identity profile's lock order, lease, `deviceDiagnostics` activity row,
  timeouts, cancellation and disconnect handling. Views always carry `memory`
  (`null` for format 1) and `exportable`. Export uses `bundle_export`, so format 1
  bytes are unchanged.
- **Web:** a *Diagnostic profile* choice before Preview (default generic
  identity) with a translated description; a change invalidates the preview like a
  device or gateway change. The memory preview lists application, required
  identity, declared ranges and total; progress counts memory chunks; the result
  shows coverage per range. Same read and share consents. Known preview refusals
  are explained in English and German. The client admits format 1 and 2 strictly.
- **Not exportable case:** format 2 admits no caller setup stop reason, so a
  memory run stopped before its tunnel (gateway busy, connect failure/timeout,
  context change, server stopping) is shown without a save action; the CLI
  writes no archive in the same case.

No new collector request, recorded reply or archive format change; no device
was contacted. Evidence: [contract](../contracts/device-diagnostic-export.md),
route tests `apps/knx-server/tests/it/http_device_diagnostics.rs`, browser spec
`apps/knx-web/e2e/device-diagnostics.integration.ts` (real server, regular
simulator, loopback inside an isolated network namespace).

## Remaining

- A separately authorised read-only hardware run of the memory profile (the
  CLI and web share this gate; see KL-184).

## References

[Diagnostic export contract](../contracts/device-diagnostic-export.md) ·
[UP258 goal](../goals/up258-programming-2026-10-18.md) ·
[ADR-0134](../adr/0134-generic-read-only-diagnostic-export.md) ·
[history](../history/OPEN_WORK_2026-10-25.md#exact-siemens-up-258d12-programming-work-18-october-2026)
