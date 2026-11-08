# Patient join for the S2 recovery-authority tests — 8 November 2026

Scope: GATE-01, owner decision of 8 November 2026 (option 1 of the
[load-timeout diagnosis](../research/2026-11-08-s2-recovery-authority-load-timeouts.md)).
Test code only.

## Change

- `mounted_failure_diagnostic.rs`: `settled_slow_status` returns the settled
  status after the existing `settled_slow` wait (180 s, 20 ms sleep polling,
  then `join_finished` and the exact terminal-state assertion);
  `settled_slow` now delegates to it, so its other callers are unchanged.
- `caption_mounted/recovery_authority.rs` (four authority cases) and
  `recovery_authority/wrong_serial.rs` (four `wrong_serial` cases) join the
  recovery worker through `settled_slow_status` instead of the 10 s
  `settled`, which keeps its deadline for every other caller.
- Every assertion after the join (state, settlement, frozen download facts,
  binding, connect/disconnect/frame counters, read-only grammar, durable
  checkpoints and evidence) is unchanged.
- No production code, simulator timing, protocol timeout, recovery policy or
  gate configuration changed.

## Executed checks (author, before review)

- `cargo fmt --check`; clippy `-D warnings` for `knx-server` (all targets).
- `knx-server --lib -- caption_mounted::recovery_authority`: three runs, 8/8 each.
- Full `knx-server --lib`: 872 passed, 0 failed, 19 ignored (load about 9).
- The full parallel suite was not forced under a second gate again; the
  change only widens the wait, so the earlier red condition is not
  reproduced as a positive witness.

## Independent review

Focused review of `83a62976a` in a separate copy: **APPROVE**, nothing
blocking. Every line after the join matches the base; other `settled` callers,
including the original 10 s recovery test, are unchanged. Reviewer runs: the
filter 3 × 8/8, `device_system2` 87 passed / 16 ignored, clippy and fmt clean.
Mutants: returning an empty status and expecting a wrong outcome in either
file are killed; removing the helper's state assertion survives, as it did
with the old `settled` (that assertion is the only check of the outcome
string, and the restored outcome is additionally checked through settlement
and the operation store). The stale options heading in the diagnosis was
reworded.

## Limits

- A slower recovery is no longer reported as a deadline failure up to 180 s;
  a genuinely hung worker is still reported, after 180 s instead of 10 s.
- The CPU versus storage share of the delay under gate load remains
  unmeasured (see the diagnosis).
