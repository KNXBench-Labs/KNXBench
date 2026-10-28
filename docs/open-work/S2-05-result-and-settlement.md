# S2-05 — System-2: result in the device list, report, project settlement

**Priority:** P1. **Status:** bounded result/report and automatic project-belief
integration accepted on main-first `8c06ddc96`, including supervised terminal
settlement and application-level status/reload ownership. Feature and integrated
selected gates15/15 each passed; #30's allocation mismatch remains unresolved.
Origin: GitHub issue #31.

[Acceptance, counts and boundaries](../status/2026-10-28-system2-result-integration.md).

Contract: [system2-download-result](../contracts/system2-download-result.md).

## Open

- Splitting "final step did not finish" into verification mismatch, read failure and final
  write failure is implemented in source (see the dated note below); which cause lies behind
  the allocation mismatch in #30/#22 stays unresolved and unproven on hardware.
- Per-part source equivalence: a verified run today clears memory beliefs only while the
  whole preparation is unchanged.
- Field verification by the reporter is not requested; no hardware retry is needed
  for the software change.

## Update 28 October 2026: final-step failure category (source, focused proof)

Net classifies a failure of the last `Disconnect` step's own operations from the typed
operation in flight; App journals it (envelope version 3, closed `final_step` record, strict
replay), reopen and the reports carry it, status schema 4 serves `finalStepFailures`, and
the panel shows it in EN and DE with an explicit "unavailable" for older journals. Scope,
grammar and limits: [contract](../contracts/system2-download-result.md#final-step-failure).
Proven so far by focused Net, App, server and web runs on simulators and intercepted
HTTP only; the package and integration gates have not been run for this change, and no
hardware evidence exists.
