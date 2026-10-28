# S2-05 — System-2: result in the device list, report, project settlement

**Priority:** P1. **Status:** typed result, automatic project-belief settlement, shared
status lifetime (application-level owner, acknowledged reload), supervised terminal settlement and
report implemented in the tree for review; #30's allocation mismatch
unresolved. Origin: GitHub issue #31.

Contract: [system2-download-result](../contracts/system2-download-result.md).

## Open

- Splitting "final step did not finish" into verification mismatch, read failure and
  final write failure needs a typed failure in `knx-net` and a journal field (#30/#22).
- Per-part source equivalence: a verified run today clears memory beliefs only while the
  whole preparation is unchanged.
- Field verification by the reporter is not requested; no hardware retry is needed
  for the software change.
