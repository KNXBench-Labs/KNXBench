# GATE-01: frame the spawn fixture's stdout records

Date: 27 October 2026. **Test-fixture correction only.** The production
`browser_tmp` wrapper, signal dispositions, replay order, deadlines, retries,
lane policy and application are unchanged. GATE-01 remains open; this does not
attribute the earlier SIGTERM timeouts or historical cold-Vite failures.

## Actual failure and mechanism

The cold-start assessment's main-first candidate `83871cd88` stopped in the
maintained `tool-tests` stage. Its raw report names
`SpawnSignalRealChild.test_default_disposition_wrapper_forwards_a_signal_from_the_spawn_boundary`,
subcase `SIGHUP`: `IndexError` in the fixture's `line.split()[1]` PID reader.
The original failed receipt/log is retained separately; no publication followed.

The fixture polls a regular stdout file while its driver is writing it. Python
`splitlines()` returns an unterminated trailing fragment too. Thus a snapshot
`child ` satisfies the prefix check but has no PID token. A snapshot `child 1`
can likewise be mistaken for PID 1 before the actual row becomes
`child 123 ready\n`. This is an output-framing defect in the test harness, not
evidence that the production signal-forwarding wrapper dropped SIGHUP.

The corrected loop polls the wrapper **before** reading and accepts only rows
terminated by a newline. If that poll observes exit, the following file read
drains the final output before deciding the driver ended without an identity.
If the poll observes a live wrapper and the snapshot is incomplete, the existing
bounded loop continues. Malformed complete rows and an exited driver with only
a fragment still fail. No success oracle, timeout or signal assertion is removed.

## Discriminating evidence

The initial four synthetic controls execute the actual fixture `run_case`
reader with a scripted stdout file and fake wrapper/process helpers. They
launch **no process**. On the unchanged original reader, three encounter the
same PID-token `IndexError`; the complete-malformed-record control passes by
asserting that refusal. The corrected controls and the then-current owning
module passed: four focused controls and 13 module methods.

A parent-added fifth control covers a partially written numeric PID. Final
tests were calibrated against the exact original `run_case` AST in a fresh
process: five registered controls produce **one assertion failure and three
errors**, with the remaining malformed-complete control passing. The added
failure explicitly reports PID 1 instead of 123. This is an expected nonzero
calibration, not five failed product tests or another random SIGHUP reproduction.

The restored final owning module passes **14 methods, zero skips**. It includes
the five synthetic framing controls and the unchanged real-child signal tests;
those still check actual signal receipt, status 7 and their existing pidfd-bound
cleanup. Synthetic regressions cannot leak a child because their Popen and
process helpers are stubbed. This is not a new general cleanup guarantee.

Parent checks establish original-reader AST equality in the original, RED and
old-reader calibration snapshots, final-source binding, exact raw summaries
and the one-file implementation surface. The [machine-readable record](../evidence/gate01-spawn-record-2026-10-27.json)
binds the final fixture and original observations. Private sources and full logs
remain at `<profile>/evidence/gate01-spawn-record-20261027/`; the original failed
integration remains in `<profile>/evidence/gate01-cold-start-20261027/`.

## Acceptance boundary

Independent whole-package review, branch gate, main-first integrated gate and
publication are pending until their actual receipts are recorded. The deterministic
controls reproduce the parser mechanism, not the original OS scheduling. No
browser/application test, bus operation, new retry, relaxed assertion or production
wrapper change is claimed.
