# K13 public reset durable-recovery gate

- Agent: codex
- Date: 2026-09-30 22:12 UTC
- Scope: public CLI safety gate for MP §2.18; no live KNX or Web change.

## Change

The earlier user-approved `1.1.67` K13 live reset remains historical evidence, not standing permission for another reset. The production CLI previously opened a tunnel after validating targets and phrase without producing a complete durable pre-write backup for every affected device. Its pressed-device guard prevents an unexpected target set but is not recovery storage. Added an application-layer fail-closed precondition after input/phrase validation and before runtime/tunnel opening. Plan-only mode remains, but no longer suggests that flags can currently enable a reset. The protocol-layer simulator remains available. No HTTP/UI reset was enabled.

ADR-0058, RESEARCH §23, implementation status, KNOWN_LIMITATIONS §140 and `goal-commission.md` distinguish the single historical live run and its external backup from the missing reusable per-device recovery contract. Reopening requires supported device/mask-specific affected-storage evidence, exact durable pre-send backup/readback per pressed device and a deliberate recovery plan; no key may be guessed.

## Verification

- TDD RED: CLI integration initially failed (exit 101): the ungated confirmed command attempted a connection to the local test endpoint and timed out; the test had not yet reached its UDP receive assertion. Plan text still offered flags. No real gateway address was used.
- GREEN: integration tests cover no UDP packet before refusal, plan-only success and invalid address/phrase before the recovery gate. Final focused CLI suite passed.
- `cargo test --workspace --no-fail-fast`: 138 suites, 2,787 passed, 0 failed, 161 ignored, no `SKIP:` markers; private corpus was read only through temporary worktree symlinks.
- `npm ci`, Web production build and Vitest: 82 files / 1,295 tests passed; no Web source changed.
- Strict workspace Clippy, `cargo fmt --all --check`, layering, headers, anchors (405 links / 224 markdown files), corpus gates and diff check passed. The first Clippy run caught a now-test-only phrase helper; moved it into the test module and reran the focused suite, Clippy and workspace tests green.

## For the goal.md session:

The public K13 CLI availability changed: `reset-address --confirm` now refuses before a tunnel until full durable per-device recovery exists. Please keep shared status/limitation recounts aligned with ADR-0058 and KNOWN_LIMITATIONS §140. The `goal.md`-owned group-write activity finding remains in `.ai/logs/2026-09-30_codex_group-write-activity-boundary.md`.

## Remaining boundary

Do not treat the previous MDT address round trip, an unchanged application dump, the programming-button read, a confirmation phrase or an unacknowledged restart as a complete backup/receiver-effect witness. The low-level `knx-net` simulator/protocol API is outside this public CLI gate. K13 HTTP/UI reset, K14 and serial-address write remain blocked. **Next separate commissioning safety audit:** `run_device_program_address` in `apps/knx-cli/src/main.rs:2399-2452` still reaches tunnel construction after phrase validation without visible automatic pre-write backup; verify its full recovery contract before trusting public confirmed use. No bus or real device was touched, and no access key or credential was stored.
