# Six-task source delivery — 25 October 2026

Owner-authorised batch: six tasks in five focused packages, including two initial
Codex implementation assignments. Every package received independent review;
reported defects were reproduced and corrected before integration/publication.
This records source acceptance, not a release, deployment or hardware result.

| Task / package | Main delivery | Maintained integrated acceptance |
| --- | --- | --- |
| One shared persisted matrix object-column width | `38d1bf54d` | 11/11 package stages, independent review accepted |
| Diagnostic CLI `--device <area.line.device>` | `eaa459b3a` | 13/13 stages; 472 Rust passes, 17/17 selected corpus |
| Inspector Diagnostics: analysis/report entries | `4303e4112` | 11/11 package, 32/32 nested web; 4,316 Vitest and 885 browser passes |
| Identity/chunk stop-reason archive validation | `5a86ac228` | 14/14 stages; 2,467 Rust passes, 91/91 selected corpus |
| Complete PID marker publication | `913a812ed` | Included in gate-hygiene acceptance below |
| Cleanup of detached stage descendants | `913a812ed` | Original complete gate 10/10, nested web 71/71; CLI/UI delta 14/14 with nested web 32/32; reader delta 14/14 |

Gate-hygiene acceptance is a source-bound **chain**, not a fresh complete run on
its final commit. Unchanged frontend, build and gate inputs were checked across
the final reader delta. The final delta passed 2,467 Rust tests and 91/91 selected
corpus cases. Tool discovery registered 431 tests with six optional skips.
Counts from overlapping executions are not summed as unique coverage.
All final receipts report completed plans, zero stage exits, valid proofs,
frozen inputs and empty touched/unchecked user-data lists. Exact remote `main`
readback was verified after each publication.

## Review corrections and retained boundaries

- Duplicate-address CLI diagnostics name candidates but do not recommend bypassing
  project-wide uniqueness with an internal ID; the application guard is unchanged.
- Diagnostic jobs acquire token ownership synchronously, before React commits.
  Late responses and teardown cancel only their owned running token; terminal
  results release ownership. Independent pre/post-commit probes passed.
- Reader validation distinguishes identity, setup, memory-exchange and teardown
  failure phases. Real collector/simulator roundtrips retain valid partial,
  cancellation, timeout and observer states; no archive-version change.
- Stage cleanup defers handled cancellation through bounded kill/reap operations.
  Independent real-signal race tests preserve an unrelated sentinel. Forced
  supervisor SIGKILL and trampoline resource-accounting limits remain explicit
  in the [web gate contract](../contracts/web-test-gate.md).
- Inspector entries reuse existing consent/export flows; System-2 declared-memory
  diagnostics remain CLI-only. No real bus/device access occurred.
- Chromium evidence does not establish WebKitGTK/Orca/native acceptance. No new
  release artifact or container deployment is included.

## Evidence locators

Private retained evidence lives under `<profile-scratch>/batch-20261024/`:

- `matrix-integration-gate-02/receipt.json` (attempt 01 failed before web execution
  because npm dependencies were absent; retained, not relabelled green).
- `cli-integration-gate/receipt.json`.
- `ui-integration-gate/receipt.json`.
- `reader-integration-current-gate/receipt.json` (earlier-base run retained separately).
- `gate-hygiene-integration-gate/receipt.json`,
  `gate-hygiene-final-delta-gate/receipt.json`,
  `gate-hygiene-reader-delta-gate/receipt.json`, and
  `gate-hygiene-reader-delta-preparation/validated-final-candidate.json`.

These locators are not public test fixtures. Maintained contracts and tests remain
in the repository; task evidence and review probes stay private.
