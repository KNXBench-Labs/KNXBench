# Commissioning track closing self-review

Timestamp: 2026-10-05 09:58. Reviewer: this session (self-review; no subagents by user rule, so not an independent review). Tree: `origin/main` `eea146f7` (source gated at `ae567d00`).

Against `goal-commission.md` §4 and the user scope decision of 2026-10-04.

| Requirement | Evidence | Verdict |
|---|---|---|
| Pending device-specific safety contracts verified in code and tests | Address programming (ADR-0059), serial write (ADR-0057) and reset (ADR-0058) refuse before any tunnel: `crates/knx-app/src/individual_address_programming_recovery.rs`, `individual_address_reset_recovery.rs`, `serial_address_recovery.rs`; CLI `tests/cli_address_programming_recovery.rs`, `tests/cli_address_reset_recovery.rs`; server `address_programming_routes.rs` returns 412 before locks | closed as fail-closed boundary |
| Property recovery before any property write, incl. `PID_DEVICE_CONTROL`; failed backup blocks writes | `crates/knx-app/src/service_control_backup.rs` (format 2 keeps PID 8 and PID 14), CLI `device_service_control.rs`, server `service_control_routes.rs`; compiled mutants recorded in E1 | closed (property scope only) |
| Caller / long-session lifecycle coverage (item 1) | CLI download, restore, service-control write, compare and serial read; server download session (Drop-guarded `DownloadGuard`), compare, serial lookup, service-control read/write; tests `cli_activity_history.rs`, `cli_compare_activity.rs`, `cli_read_activity.rs`, `device_download_task_tests.rs`, `commissioning_activity.rs` unit tests | closed offline |
| Recovery/abort/restore contracts offline (item 3) | K7 restore-file guard, returned-error disconnect, dead-worker `Failed` with unknown outcome, prior-incarnation `unknown` rows, history/input alias refusal (identical, parent, unresolved leaf) | closed offline; process/power-loss recovery unproven (user notice) |
| Web/client adoption (item 2) | Handed to the UI owner by user decision 2026-10-05; contract table in COMMISSIONING_ALPHA_LEDGER | open, owned by goal-ui (KL-142, UI-04) |
| Reset UI | Accepted unsupported boundary by user decision 2026-10-05 (KL-140), user notice in the manual | closed as boundary |
| Required gates on the integrated tree | `docs/evidence/commission-integrated-gates-ae567d00-2026-10-05.json`: workspace 3238/0/177 (14 stages), Vitest 2001, Chromium 131, selected private 68/0, release versions | passed |
| Handover records unsupported operations and validation boundary | `.ai/CURRENT_STATE.md` entries of 2026-10-05; manual *Commissioning validation boundary* and *There is no address reset in the app* | done |

Findings: Critical 0, Important 0, Minor 1 — `docs/IMPLEMENTATION_STATUS.md` top still described the serial-lookup caller as an unpublished candidate; corrected by a new top entry in the same commit.

Verdict: the commissioning track's own work is complete at its documented scope. The goal is not closed while the UI half of KL-142 / UI-04 is open in `goal-ui.md`, and no ETS parity, certification, general device compatibility or power-loss recovery is claimed.
