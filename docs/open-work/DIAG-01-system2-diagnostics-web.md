# DIAG-01 — System-2 memory diagnostic profile in the web UI

**Status:** Open. Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is the [ledger](../status/LEDGER.md).

## Task

Offer the read-only `system2-declared-segments-v1` diagnostic profile
([ADR-0143](../adr/0143-declared-segment-memory-evidence-profile.md)) in the
web UI. Today it is CLI-only (`knx device diagnostics --device
<area.line.device>`).

## Current state

The **Analyse device** button on a device's Diagnostics tab opens the
existing export dialog, which offers only `generic-identity-v1`.

## Done when (proposed)

The web dialog offers the profile with the same admission, device-selection
and consent rules as the CLI, covered by simulator and browser tests. No new
collector request, recorded reply or archive format change; no device is
contacted during development.

## References

[Diagnostic export contract](../contracts/device-diagnostic-export.md) ·
[UP258 goal](../goals/up258-programming-2026-10-18.md) ·
[ADR-0134](../adr/0134-generic-read-only-diagnostic-export.md) ·
[history](../history/OPEN_WORK_2026-10-25.md#exact-siemens-up-258d12-programming-work-18-october-2026)
