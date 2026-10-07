# 2026-09-28 — Claude — iaw: memory download (plan + executor)

**Nothing was written to the real device.** All of it is offline, against the simulator.

## What
- `knx-core` `commissioning::memory_download`: the plan data type, `property_matches`, `unmasked_runs`.
- `knx-productdb` `download_plan`: `LoadProcedure` + `DownloadImage` → plan.
- `knx-net` `commissioning::memory_download`: the executor. Checks go before writes; state checks are strict; every write is read back.
- `knx-net` `restart_basic_as` (crate-internal) lets the closing restart run under the `Download` scope.
- Simulator `preset_load_state`.
- `apps/knx-cli/tests/memory_download_simulated.rs`: option C end to end.

## Why this way
- The MDT procedure has no write step. CP §3.9.2.2.2 (the BIM M112 procedure, pp. 67–68) writes each table between AllocAbsDataSeg and TaskSeg. The plan follows that order (see RESEARCH §19.3).
- `PID_HARDWARE_TYPE` is PDT_GENERIC_06 (RES §4.3.28 p. 78), but the product carries 10 octets of InlineData. The comparison rule is [A] and documented; 1.1.67 satisfies it.
- The composition test lives in knx-cli because that crate already depends on productdb, net and tempfile, so no manifest change was needed.

## Verification
- Unit: core 11, planner 13, executor 13; corpus 3; mutants 16/16.
- Workspace gate: see the commit.

## Open
- The hardware allowlist for `Download`, a dry-run review for the user, and a new go.
- Recovery after a failure (re-running the same plan) is not tested against real hardware.
