# Commissioning: server download worker outcome (offline)

Date: 2026-10-01
Agent: codex
Scope: server download worker bookkeeping and existing commissioning documentation.

## Verified changes

- Polling a finished worker without a terminal device result produces failure,
  rather than a permanent Running snapshot. Progress and backup path survive.
- The first changing step is derived from the prepared plan once. A stopped
  worker before that boundary is `written:no`; afterward `partially` means
  conservative unknown attempted-write outcome, not send/device-effect evidence.
- A terminal result is not a tunnel-release boundary: the task continues through
  the outer tunnel disconnect. `is_running` retains the reservation until task
  completion, and existing terminal/restart evidence is not overwritten.
- A cancelled join wait retains its handle; no public cancellation route is added.

## In-session review (subagents prohibited)

Re-read the complete diff and ADR-0045; trace Shared progress to `record`, the
prepared plan to `MemoryDownloadStep::changes_device`, and exclusion to the
routes' `DeviceDownloadSession::is_running`. No public wire or UI change.

IMPORTANT finding: `apps/knx-server/src/device_download.rs:282` originally took
and removed the JoinHandle before awaiting it. Dropping that wait while a terminal
result's cleanup was pending detached the task and made exclusion release early.
The new RED regression failed at `the live worker must remain tracked`; borrowing
the handle through the await and removing it only afterward closes this finding.
No remaining blocking findings in the reviewed final code. This is an in-session
review, not an independent reviewer verdict.

## Executed focused evidence

- Initial RED: 1 passed / 4 failed for dead-worker polling, tunnel reservation,
  terminal-result preservation and pre-mutation write classification.
- Additional cancelled-join RED: 0 passed / 1 failed, then fixed.
- GREEN: 8 synthetic offline worker regressions passed; no bus connector or
  hardware was used. Assertions include the actual serialized failure shape,
  retained backup path and unconfirmed restart evidence.
- Four temporary mutations each compiled and failed the corresponding tests:
  task-reservation check, dead-task polling reconciliation, and protection of an
  existing terminal result, plus the first mutation-step boundary. All restored;
  no mutation left in the source.
- First expanded HTTP run: 12 passed / 1 failed before any worker started in
  that test. The fixture's group-address partial scope was already in shipped
  verified evidence, contrary to its old untested assertion. Keep the evidence;
  choose an installed unverified plannable sibling at runtime, with a fresh
  default-only project. All missing/wrong/other-target acknowledgement refusals
  still assert no tunnel/write; the exact one asserts one simulated tunnel.
  Its subsequent executor outcome is not hardware-success evidence for that
  sibling. No private fixture values were added or copied into Git.
- Rerun full workspace: 139 suites / 2,809 passed / 0 failed / 161 ignored /
  0 `SKIP:`; explicit simulator HTTP download fixture sweep: 13 passed / 0
  failed. Strict workspace Clippy checked the changed server crate; fmt,
  npm install/resource build, layering/headers/anchors/corpus gates and diff
  check all exited zero.
- Rebased over two upstream documentation/handover commits, preserving both
  handovers. Diff against the originally gated candidate contains only the
  upstream handover and open-items document, no source changes. Rebased
  anchors: 375 links / 226 Markdown files, none dead; headers/fmt/diff green.
- Code commit `8e3f3ecec6f025da6a400c47b357024c339f8610` published to
  `origin/main`; push/fetch exited zero and exact local/remote SHA matched.
  Task-owned cleanup follows the final handover publication receipt.

## Limits

This preserves server-lifetime evidence only. No guaranteed disconnect after task
abort/panic, no process-crash recovery, no durable audit, no automatic retry/restore,
no new hardware/write permission and no credentials. A retained successful device
result cannot imply that a later tunnel cleanup succeeded.

## Next inspection

Keep the address-write recovery gates fail-closed. Independently inspect durable
backup publication for newly created parent directories; the existing final-file
and immediate-directory syncs are not a proof of an entire newly created path.
Do not change that contract without primary OS documentation and failure tests.
