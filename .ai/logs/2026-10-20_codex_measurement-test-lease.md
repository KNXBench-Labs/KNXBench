# Isolate synthetic measurement test leases

20 October 2026; agent: codex.

The complete UP258 acceptance gate exposed a pre-existing test-fixture collision:
the full web runner owns the real checkout's workspace lease while synthetic
`StudyTests` call `run_study`, which tried to acquire that same lease again.
The unchanged test file was byte-identical to upstream. A direct synthetic study
succeeded without the outer lease; with that real lease held it produced an
explicit lease timeout and no observations. The failed complete-gate receipt is
retained and is not acceptance evidence.

The focused test-only correction gives each StudyTests case its own temporary
Git checkout and patches the measurement module's ROOT for that case, restoring
it through unittest cleanup. The study still acquires a real lock in that private
Git common directory. No production measurement, runner, parser, protocol or
budget changes; no mocked-away locking or relaxed acceptance assertions.

A new fixture-isolation assertion first failed before the correction. The module passes all 19 tests while the actual source-checkout outer lease is
held. Independent focused review ACCEPTS the fix and separately repeats 19 tests
with a real scratch outer lease, verifying ROOT restoration and directory cleanup.
Reversing just the setup/new-test additions reconstructs the original file bytes.
Final complete combined acceptance is recorded separately; no hardware or release
operation belongs to this correction.
