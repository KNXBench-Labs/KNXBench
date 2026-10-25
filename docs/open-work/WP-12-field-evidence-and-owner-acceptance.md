# WP-12 — Field evidence and owner acceptance (incl. issue #22)

**Status:** Blocked: consent. Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is the [ledger](../status/LEDGER.md).

## Task

Separate cases, not one generic hardware test:

- Issue #22: the hardware outcome of the experimental configured System-2
  download for the exact UP 258D12 application (software-tested only), and
  the reporter's consented `system2-declared-segments-v1` run with its offline
  comparison.
- KL-184: a fully silent device during identity reads, and firmware
  responses.
- KL-186: acceptance of adopted plug-in bytes by the originating plug-in
  (only the owner can check it).
- KL-170: probe output from ETS-programmed LittleEndian devices, and review
  of the first real tester download report.
- Device comparison: hardware acceptance of *project → device* and live
  *device → project* adoption.
- A first live legacy (`.vd3`–`.vd5`) download.
- Optional: custom multicast, bridge Route-Back tunnel, other specifically
  identified live gaps.

## Prerequisite

Operation-specific owner consent naming device and operation, or outside
input. No vendor or KNX Association contact.

## Done when (per case)

The exact build/device/application/operation and original evidence are
verified, outcomes include refusal/unknown, and only the witnessed boundary
is narrowed. Private projects, identity ZIPs and raw device evidence stay out
of Git.

## References

[UP258 goal](../goals/up258-programming-2026-10-18.md) ·
[detector diagnostics](../research/2026-10-18-up258-device-diagnostics.md) ·
[eleven devices](../research/2026-10-18-device-diagnostics-eleven-devices.md) ·
[house run](../research/2026-10-18-house-device-diagnostics.md) ·
[plug-in contract](../contracts/plugin-data-adoption.md) ·
[comparison adoption](../contracts/alpha16-comparison-adoption.md) ·
[ADR-0121](../adr/0121-tester-download-reports.md) ·
[history](../history/OPEN_WORK_2026-10-25.md#wp-12--field-evidence-and-owner-acceptance)
