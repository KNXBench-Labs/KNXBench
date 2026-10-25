# Restart snapshot

Updated: 2026-10-25 02:22 CEST. Operational snapshot, not an append-only journal.

## Issue #30 delivered (2026-10-25)

- System-2 mode selector/readiness scoping merged `946871e8b` (feature `501f59793`).
- Opus review REQUEST CHANGES closed, re-review APPROVE; change-scoped gate 11/11 incl. web 32
  stages/897 browser cases; on merge docs/xtask/local-info re-run (upstream delta Rust/docs only).
- Hardware outcome of System-2 stays unverified (#22/WP-12); issue #30 answered, see its thread.

## System-2 pointer-mismatch package: CLAIMED (2026-10-25 12:33 CET, top priority)

- Owner go: a device prestate differing from the source shape (PID 7 pointer / load state) must
  not refuse a System-2 download. Branch `fix/system2-pointer-mismatch-20261025`.
- Claimed paths: `crates/knx-net/src/commissioning/property_recovery.rs`, `property_download.rs`,
  `crates/knx-app/src/property_validation_*`, `apps/knx-server/src/device_system2/`,
  System2 web panel/API/copy, alpha16 System-2 contracts. Other sessions: do not edit these.
- Scope: record object + expected/observed pointer/state, mark original-state recovery unavailable,
  surface `mutationMayHaveStarted` (pre-mutation refusal = nothing written). Serial, program
  version, memory verification and confirmations stay hard. No bus operation by agents.

## Current assignment and authority

- Owner's six-task batch is delivered in five packages; all claims released. No new feature assignment is started by closeout.
- Source delivery only: no release, deployment, native desktop acceptance or real bus/device operations.
- No vendor contact, Ollama/model downloads or excluded models authorised; the globally excluded device remains off-limits.

## Delivered batch

| Package | Verified main delivery |
| --- | --- |
| Shared matrix object-column width | `38d1bf54d` |
| CLI diagnostic device-address selector | `eaa459b3a` |
| Inspector Diagnostics analysis/report entries and async cleanup | `4303e4112` |
| Diagnostic archive identity/chunk stop-reason validation | `5a86ac228` |
| PID marker race and detached stage-process cleanup | `913a812ed` |

- Independent review and maintained integrated acceptance completed for each package; details in [batch receipt](../docs/status/2026-10-25-six-task-delivery.md).
- Gate hygiene uses original complete acceptance plus CLI/UI and reader delta gates, not a fresh full run on the final SHA.
- Evidence remains in `<profile-scratch>/batch-20261024/` and linked review directories; preserve acceptance evidence during cleanup.
- Closeout: documentation checks and own worktree cleanup only. No worker or gate remains active.

## Saved-project-folder grouping: separate closeout owner

- Delivered `e6f227c4`, record `da36b77b9`; [owner log](logs/2026-10-24_codex_wp06-folder-grouping.md).
- Its claim is released; separate owner's result-only documentation acceptance/evidence cleanup was not independently established by this batch. Do not reopen product work.

## Remaining boundaries and preservation

- Family mapping, codecs, manufacturer folders/modules, import witnesses, priority schema and native/hardware acceptance retain their named prerequisites.
- System-2 memory diagnostics stay CLI-only. New inspector entries expose the existing generic identity/report flows, not that profile.
- Supervisor SIGKILL recovery remains limited; handled cancellation cleanup and resource-accounting boundaries are documented in the web gate contract.
- WinZip AES sources exist; outright refusal remains, not verified AES support.
- Preserve root private/untracked files, the separate Alpha 16 worktree and other owners' evidence.
- Prior post-16 closeout: [owner log](logs/2026-10-24_codex_post16-closeout.md).

## Handover policy

- Follow `AGENTS.md`; preserve other tracks. Repo docs own status, vault durable knowledge, sessions conversation history.
