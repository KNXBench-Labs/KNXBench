# GATE-01 — Test-gate speed and stability follow-ups

**Status:** Open; parked until release preparation (owner decision 8 November 2026). Alpha 21 preparation (13 November 2026) passed its complete release gate 23/23 on the first full run without taking these items up; they await re-prioritisation. Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is the [ledger](../status/LEDGER.md).

Small and optional; none blocks a delivery.

## Open

- Production browser suites now run in the tag lanes
  ([ADR-0154](../adr/0154-production-browser-suites-use-tag-lanes.md),
  [record](../status/2026-10-26-production-browser-lanes.md)). Their functional
  lane runs case-parallel since `3af5ea9e4` (production 681.7 → 455.7 s;
  [ADR-0155](../adr/0155-production-functional-case-parallelism.md),
  [record](../research/2026-10-27-production-case-parallelism.md)). Concurrent
  suites were assessed on 27 October 2026 and are not admitted as a small
  change: aggregate worker scheduling, global exclusive isolation and receipt
  writing need a separate design; no overlap speedup was measured.
  Default-lane case parallelism was assessed on the same date and is not admitted:
  retained four-worker scheduling has only about 28 s of fixed-duration model
  headroom, not a measured gain
  ([assessment](../research/2026-10-27-default-browser-parallelism-assessment.md)).
  Reopen on a material new tail or an explicit priority for the larger scheduler.
- Three historical `file-menu-submenus` timeouts remain unresolved. The run
  requested eight workers; active concurrency and a cold-Vite cause were not
  established. A [27 October source/compiled-inventory assessment](../research/2026-10-27-cold-start-worker-shape.md)
  finds only three groups for the current same-file repeat-three selection,
  including with an eight-worker cap. Its focused four-cap/eight-cap scopes
  are identical, so that comparison is not admitted as a load discriminator.
  Exact historical failed identities/log and runtime provenance are missing;
  no browser probe, timeout fix or policy change is claimed.
- The cause of the earlier observed SIGTERM-test timeouts remains unproven;
  no field timeout is attributed to the separately repaired spawn/signal window.

The spawn/signal correction was delivered as `ed156ef9b`; its
[acceptance record](../status/2026-10-26-browser-tmp-spawn-signals.md) owns the
bounded result. It is no longer pending work; the two performance bullets and
unresolved historical timeout causes above remain open.

The two SIGTERM pidfd-identity fixtures in `tools/tests/test_browser_tmp_launch.py`
used timed lifetimes (4 s fixture or child, 8 s sentinel or wrapper). One failed
in a package gate on 27 October 2026 ("identities changed before pidfd
retention") and reproduced once in 12 runs under CPU oversubscription; a 5 s
stall injected before identity pinning fails both on the old fixtures and passes
on the new ones. Fixture lifetimes are now event-bound (parent death, or a
proceed file; 120 s only bounds a leak), and failure paths reap the direct
children. The cleanup test previously reached its timeout fallback only because
its fixture outlived a 3 s wait; it now forces that fallback with a simulated
first timeout and asserts the fallback ran. Reinstating the earlier Popen-signal
fallback in memory still fails it. This closes that flake only; it says nothing
about the older SIGTERM timeouts above.

Delivered as main-first merge `0791ddb77` on `909412381` (feature `081ccb1a5`,
review correction `953b2c9b8`). Independent review first blocked on a lost
cleanup-fallback oracle; the corrected test forces its first timeout and the
confirmation review approved it. The integrated change-scoped gate passed
10/10 stages with frozen inputs and no touched or unchecked user data:
461 registered tool tests, 455 passed and 6 optional skips. No additional Rust,
browser, bindings or corpus execution was selected for this test-only change.
Local, tracking and live `main` matched after publication. Original failed
attempts, stall probes, fallback mutation and accepted receipts remain in
`<profile>/evidence/gate01-sigterm-flake-20261027/`.

## SIGTERM fixture pinning before exec finished — 1 November 2026

The event-bound lifetimes above did not close the "identities changed before
pidfd retention" failure: it recurred in the OKX-05 package gate at load ~14.
Instrumented runs showed the missing identity is a process pinned right after
`subprocess.Popen`: the sentinel in all 22 failing runs, once also the
wrapper-reported child. Its `/proc/<pid>/environ` still lacked the marker or
still showed the parent's environment and command line, and was correct
shortly after. That much is observed; the exact kernel/CPython ordering behind
it is not proven. Pinned processes now
announce their own PID from the new image first (`spawn_announced`,
`ANNOUNCE_PID`); the wrapper's child reports itself instead of the wrapper
reporting it. Same load (12 parallel copies, 8 rounds, both SIGTERM identity
tests): the original fails 33 of 96 runs (41 of 192 test executions), the
corrected tests pass 96/96 with ResourceWarnings as errors. Removing the
pidfd PID check from `open_owned_process` still fails both tests, so they keep
proving the mismatched-identity rejection. This closes this pinning race only, not the older
SIGTERM timeout causes above.

## Spawn-record reader correction — 27 October 2026

A separate integrated package gate exposed a SIGHUP fixture `IndexError` while
reading an unfinished `child ` stdout record. The test-only reader correction
polls before reading and accepts newline-terminated records only. Final owning
module: 14 methods, zero skips; five deterministic synthetic framing controls
are calibrated against the original reader. These are not application tests or
an explanation of the older SIGTERM/cold-Vite timeouts. Published main-first as
`bc1b47a9a` after independent review APPROVE and branch/integrated gates10 green;
see the [bounded record](../status/2026-10-27-spawn-record-framing.md).

## 5000-device measurement load sensitivity — 28 October 2026

A new bounded [browser investigation](../research/2026-10-28-devices-browser-load-sensitivity.md)
reproduces the 120 s default-case timeout under synthetic CPU contention:
3/3 unchanged dev-fixture runs time out while opening the command palette,
compared with 3/3 passes at about 73 s without synthetic load. The test expands
the explorer before its table measurement; the attachment omits that setup.
A separate shared-lease-protected dev/production and expanded/closed comparison
finds substantial preparation cost. Closed-state variants are diagnostic only,
not coverage-equivalent replacements. No timeout, source, lane or concurrency
policy changed. The historical S2-03 red cause remains unproved. Production
preparation profiling was left open at this checkpoint; the bounded
5 November attribution below addresses that later question, not the old cause.

## Document-language fanout correction — 28 October 2026

The subsequent production profile found repeated global `lang` writes from
`useUiLanguage()` consumers. The effect now writes only if the actual document
attribute differs; the language store and all browser-case assertions are
unchanged. Original semantic tests red4 → green4, targeted language tests35
passed, inverted guard caught. A leased repeat-three production comparison
measured 41.803 → 21.434 s whole-case medians; the table's render attachment
remains about 10.4 s. Fixed dev cases also pass3 under 16 shell busy loops,
with limited headroom (99–105 s of a 120 s limit). These are bounded software
results, not arbitrary-load stability or a historical timeout-cause claim.
[Evidence, attribution and limits](../status/2026-10-28-document-language-fanout.md).
GATE-01 remains open; no scheduler, timeout or coverage policy changed.

## Crash-free timeout diagnostics — 30 October 2026

The browser-wrapper test diagnostic helper previously sent SIGABRT to obtain
Python faulthandler stacks. Its identity-safety regression exercised that helper
on purpose, so even a successful tool-test run could generate a desktop crash
report. It now captures kernel process state, wait channels, signal masks and
existing stderr before SIGKILL cleanup through retained fixture pidfds. No
system-wide crash configuration changes are needed. Python stack traces are no
longer collected by this helper; the underlying timeout assertions and
unrelated-process protection remain in place. Mocked regressions intercept all
signals (including against the old implementation), and a real stuck wrapper
and child verify cleanup without a crash-generating signal. The older timeout
causes above remain unproven.

## Proportionate input selection — 3 November 2026

Published main-first `703ec2904` refines ADR-0144 with evidence-bound font and
reader projections, representative production dialogs for font/app-shell HTML
changes, and an explicit nonzero diagnostic Rust stage for bounded theme
metadata. Complete web/default-browser checks and conservative full/unknown
fallbacks remain. The actual Rail plan changes from15 to12 package stages but
keeps16 production suites because of its shared dialog-test delta; no wall-time
speedup or retroactive coverage reduction is claimed.

Independent review and regression/mutation controls closed stage-order and
JSON-reader proof findings. A separately reviewed symlink-root fixture correction
keeps strict orchestration assertions. Full source490 acceptance and exact
composition plus successor703 delta acceptance retain separate runtime identities.
[Acceptance and boundaries](../status/2026-11-03-proportionate-gates.md).
This closes that bounded selection work only; the scheduler and historical
failure investigations above remain open under GATE-01.

## Production preparation attribution — 5 November 2026

The [bounded expanded-5000 production study](../research/2026-11-05-production-preparation-attribution.md)
executes one unchanged canonical control and two identically instrumented arms,
full snapshots versus snapshots-off diagnostic. All three pass the original
5000-row/filter/one-GET/no-unexpected contract at 120 s / one worker / zero retries.
Explorer action wall is 7796.886 ms versus 565.895 ms; conservative snapshotter-shaped
signed sample weights are 6917.029 ms versus 0, while table residuals remain
application-associated. These are single sequential observations, not a general
speedup, exact CPU allocation, historical-cause proof or product optimization.

The independent raw-evidence review approves this finite measurement only.
Snapshots-off also removes network recording; blank-URL ancestry lacks captured
Debugger script bytes; all six CPU profiles retain negative sampling deltas and
observer/action windows differ. Cleanup checks the owned temp directory, direct
children and accessible marker references, not a universal orphan audit. No
trace-default, coverage, scheduler, timeout or concurrency policy changed; the
diagnostic is not a gate replacement. This closes the previously requested
bounded production-preparation profile, not GATE-01's scheduler/historical
failure investigations above.

## Mounted System-2 recovery-authority timeouts under load — 7 November 2026

One change-scoped package gate (unrelated diff/server change, no `device_system2`
or `knx-app` edit) failed `rust-tests` with 868 passed / 4 failed: all four
`caption_mounted_recovery_authority_{retained,standalone}{20,23}` cases hit the
10 s `settled_with_timeout` wait (`Elapsed`) after their intended poisoning panic.
Load average was about 10 with an unrelated VM running. The four passed 3/3 in
isolation (about 13 s each) and the unchanged rerun passed all 3,664 Rust tests.
Cause not established; the timeout was not raised and the failed receipt is
retained privately. Reopen if it recurs in a gate.

It recurred the same day: a focused full `knx-server --lib` run during another
session's gate failed 7 of the 8 cases (alone 8/8). A bounded
[diagnosis](../research/2026-11-08-s2-recovery-authority-load-timeouts.md)
found identical traffic in every run (192 management frames, no
retransmissions) and durations that scale with contention: 0.65 s at rest (of
which 0.17 s for 58 fsync-backed checkpoints), 1.8–3.8 s at four test threads
during a parallel gate, over 10 s in the full parallel suite. This is waiting
time, not a protocol or worker defect. CPU versus storage share under gate load
is unmeasured. Owner decision (8 November 2026): these eight cases now use the
existing sleep-polled 180 s `settled_slow` budget with all assertions unchanged
([delivery record](../status/2026-11-08-s2-recovery-patient-join.md)); other
`settled` callers keep 10 s. This closes this flake's deadline question, not the
scheduler and historical items above.

## Known boundary

Killing the stage supervisor with SIGKILL can still orphan children; see the
[web test gate contract](../contracts/web-test-gate.md).

## References

[ADR-0144](../adr/0144-change-scoped-gates-tagged-browser-lanes-bundled-rust-tests.md) ·
[test-suite speed dossier](../research/2026-10-19-test-suite-speed.md) ·
[history](../history/OPEN_WORK_2026-10-25.md#test-gates-change-scoped-by-default-19-october-2026)
