# browser_tmp spawn/signal window

Delivered on 26 October 2026 as main-first merge `ed156ef9b` on `fb26c47c3`
(feature `2adbf63f5`, test/documentation correction `b12d93bc5`). Independent
whole-feature review and focused correction review: APPROVE. Local, tracking and
live `main` read back equal to `ed156ef9b`.

## Change

`run_command` in `tools/browser_tmp.py` now installs the SIGTERM/SIGINT/SIGHUP
forwarders before `Popen`. A signal that arrives while no child identity exists
is queued and forwarded to the child group right after the spawn. Handlers are
restored in `finally`, also when the spawn fails; the SIGKILL fallback for a
child that outlived its wait is unchanged.

## Evidence

`tools/tests/test_browser_tmp_spawn_signals.py` injects the signal
synchronously at the spawn boundary (after the child exists, before `Popen`
returns). Real-child cases run the wrapper as a separate process, use a
readiness pipe instead of sleeps, and pin the child by pidfd plus an
environment marker before cleanup.

- Original source: in-process contract and all real-child cases fail
  (wrapper exit -15/-2/-1 with the child never signalled; with SIGINT ignored
  the wrapper survived and the child never saw it). The refused-spawn handler
  restoration and SIGKILL-fallback guards pass on both.
- Fixed source: initially 6 of 6 pass; review-added characterizations increase
  the final module to 9 of 9. A calibration mutant that drops the replay is
  rejected by the same contract. Two additional in-memory runtime variants
  (reap omitted; only one pending signal replayed) fail the named assertions;
  unchanged/restored production passes. These are test calibrations, not new
  original defects or executions against exact final test bytes on the old base.
- `test_browser_tmp_launch`: 12 of 12 pass, none skipped; final combined run 21/21.

## Package acceptance

Branch and merge each passed the maintained change-scoped package gate: 11/11
outer stages and 65/65 nested web stages, frozen source/build/tool identities,
no touched or unchecked user data. The merged run passed 4,539 Vitest tests,
852 functional and 69 exclusive default browser cases, and 557 cases across
all 16 production configurations (1,478 browser executions total).
Rust runtime, private corpus and bindings were not selected; this is software
and browser-tooling acceptance, not a hardware or release claim. The earlier
RED used the original five-test fixture revision; it is historical semantic
RED, not an exact-final-test-byte run against the old implementation.

## Boundaries

- The injected signal proves a missed forward, and for default dispositions a
  wrapper death. An existing custom handler only shows the missed forward.
- A signal cannot be injected between the kernel `fork` and Python's return from
  `Popen`; the test uses the nearest synchronous point.
- A signal during a refused spawn has no group to reach and is dropped.
- The wrapper replaces its prior SIG_IGN with a callable forwarder before spawn;
  that former ignored disposition is therefore not inherited by the executable.
  A child that independently ignores SIGINT does not act on forwarded SIGINT.
- SIGKILL of the wrapper can still orphan the group (web test gate contract).
- This does not close GATE-01; the speed and `file-menu-submenus` bullets stay open.
