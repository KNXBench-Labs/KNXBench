# Mounted System-2 recovery-authority timeouts under load — 8 November 2026

Scope: GATE-01 diagnosis of the `caption_mounted::recovery_authority` tests in
`apps/knx-server` (`caption_mounted_recovery_authority_{retained,standalone}{20,23}`
and their `wrong_serial` siblings). Diagnosis only: no source, test, deadline,
simulator or gate policy changed. Measured on `2b70040a4`.

## Observed failures

- 7 November 2026, one change-scoped package gate: 4 of the 4 authority cases
  failed with `Elapsed` from the 10 s join deadline in
  `settled_with_timeout` (`device_system2/tests/mounted_hardware.rs`); load
  average about 10 with an unrelated VM.
- 7 November 2026, a focused author run of `knx-server --lib` (all 872 tests,
  default test threads) during another session's package gate: 7 of the 8
  cases failed the same way. The same filter alone passed 8/8.

## Method

A throwaway source copy of `origin/main` with three test-only probes:
elapsed time and poll count of the join loop, the management-frame counters
before and after recovery, and total time and calls spent in
`knx_app::property_validation::persist` (checkpoint, `head-next`, rename,
directory-chain fsync and readback). Runs used `cargo test -p knx-server --lib --
caption_mounted::recovery_authority --nocapture` with scratch XDG directories.
Synthetic CPU and fsync load phases were prepared but skipped: another
session's package gates ran almost continuously, and extra load would have
put their timing at risk. Results are therefore bounded to the conditions
below.

## Results

| Condition | Test threads | Authority recovery (each) | Result |
|---|---|---|---|
| Load average about 3 | 1 | 0.64–0.65 s, of which `persist` 0.17 s over 58 calls | 8/8 |
| Load average about 4 | 4 | 1.39–1.48 s | 8/8 |
| Another session's gate running, load 14–20 | 1 | 0.83–1.22 s | 8/8 (two runs) |
| Another session's gate running, load 14–20 | 4 | 1.80–3.84 s | 8/8 (three runs) |
| Full-suite gate or full `--lib` run under a parallel gate | default | more than 10 s | 4/4 and 7/8 failed |

- Every authority recovery exchanged exactly the same management traffic:
  connects 1 → 2, disconnects 1 → 2, frames 238 → 430 (192 frames). There were
  no retransmissions or protocol timeouts despite the simulator's tight
  session timing (5 ms response, 30 ms connection timeout).
- The `wrong_serial` cases settle as `restoreFailed` in 0.44–1.74 s with 5
  `persist` calls.
- Duration scales with machine contention, not with protocol behaviour.

## Interpretation

The failures are waiting time — CPU scheduling of the runtime and blocking
pool, and fsync-bound checkpoint persistence, while dozens of other tests
and a second gate compete — not a hung worker or a protocol defect. At rest
the 10 s deadline has about fifteen-fold headroom; under a full-suite gate
alongside another gate it does not.

Not established: the split between CPU and storage contention under gate
load (the load phases could not run), and whether the join loop's
`yield_now` busy wait adds to contention. Its poll counts varied between
about 2,300 and 99,000 per recovery.

## Options (owner decision; nothing applied)

1. Raise the join deadline of these eight cases to the existing 180 s
   `settled_slow` deadline and wait with a short sleep instead of
   `yield_now`, keeping every state and frame assertion. This changes the
   deliberately unchanged original-simulation deadline recorded for the
   System-2 recovery acceptance.
2. Keep the deadline and run these cases outside the parallel Rust bundle
   (serial lane).
3. Keep everything as is and treat a red gate here as known, rerunning
   the filter alone.

The 10 s deadline stays as it is until the owner decides.
