# S2-05 — System-2: result in the device list, report, project settlement

**Priority:** P1. **Status:** bounded result/report and automatic project-belief
integration accepted on main-first `8c06ddc96`, including supervised terminal
settlement and application-level status/reload ownership. Feature and integrated
selected gates15/15 each passed; #30's allocation mismatch remains unresolved.
Origin: GitHub issue #31.

[Acceptance, counts and boundaries](../status/2026-10-28-system2-result-integration.md).

Contract: [system2-download-result](../contracts/system2-download-result.md).

## Open

- Splitting "final step did not finish" into verification mismatch, read failure and
  final write failure needs a typed failure in `knx-net` and a journal field (#30/#22).
- Per-part source equivalence: a verified run today clears memory beliefs only while the
  whole preparation is unchanged.
- Field verification by the reporter is not requested; no hardware retry is needed
  for the software change.
