# System-2 own-run hint and refusal reasons — 25 October 2026

Status: implementation/review candidate; integrated acceptance and publication pending.
Base: `866cb5ca6`. This diagnostic slice follows GitHub issue #30; it does not
close [S2-02](../open-work/S2-02-gateway-busy-after-failed-runs.md).

## Contract and scope

- The partial activity endpoint names an admitted `Running` System-2 coordinator
  before its existing lock observations. It never reports `Uncertain` as running.
- The download hint suppresses its own System-2 reservations. Without the marker,
  observed foreign holders retain their existing hint. The non-atomic
  admit-to-reserve interval can temporarily suppress a genuine foreign hint;
  authoritative start/worker admission still refuses conflicts.
- Required nullable status `refusal` retains only the current worker's stable
  API-error kind. New attempts and historical reattachment reset it; it is not
  persisted, copied from an earlier operation, or execution/recovery authority.
- The panel shows worker and structured `4xx` start refusals, keeping lost-response
  uncertainty and no automatic resend. No reservation, journal, cancellation,
  transport cleanup, mutation, recovery or family gate is relaxed.
- `gatewayBusy` still conflates shared-holder conflicts and failure to acquire
  the process lease, including lease-file/runtime I/O errors. Actual reporter-side
  failure and the reported HTTP stall remain unresolved.

## Review and focused evidence

Independent Opus source/text review approved with no in-scope blockers. Its three
low-severity comment/documentation findings are corrected; confirmation and final
package gate are pending. Review was read-only, not independent test execution.

Preflight evidence (private receipts, logs, source snapshots and retained binaries
under `<profile-scratch>/issue30-busy-20261025/`):

- Original-runtime web regressions: three named failures (self-holder hint,
  missing start reason, missing worker reason). Final acceptance must exercise
  the corrected source; later comment/coverage edits do not rewrite that RED.
- Full web unit preflight: 4,398 passed across 301 files. Browser preflight: five
  intercepted real-app cases, including EN/DE at 400/1440 pixels. Those captures
  precede the final no-journal copy correction; final captures remain pending.
- Four opt-in, digest-pinned public-fixture worker regressions passed: shared lock,
  process lease, unavailable activity and stale source. They assert the reason,
  no connector/transport/write call, unchanged device/evidence tree and settled
  ownership. They need explicit fixture selection and are not default CI coverage.
- Removing reason propagation makes all four fail at the intended status
  assertion. This is omission-mutant discrimination, **not** a run on untouched
  original runtime.
- Supplemental module run: **13 passed, 1 failed**, including opted-in cases.
  The pre-existing cancellation/original-recovery test times out. An untouched
  base-source run also fails that test; different runs reach different wait
  phases. This confirms a baseline failure, not its cause or equivalence of the
  failing phase. No recovery success is claimed or safety assertion weakened.
- The serial supplemental run used a network namespace with only loopback,
  nonzero UID/EUID and zero capabilities; the actual child state was recorded.

## Remaining boundaries

Worker-level recovery refusal and post-journal connection-error propagation have
no new dedicated regression here. The existing opt-in recovery timeout remains
unresolved and outside this diagnostic acceptance; it must not be reported as a
green full-module run. No actual device or installation was contacted. Software
fixtures do not establish hardware compatibility, ETS parity or recovery.

The practical workflow is in the [device guide](../manual/user-guide/05-devices-and-products.md#a-busy-hint-is-not-the-system-2-failure-reason-unreleased-source);
DTO/lifetime details are in the [HTTP contract](../contracts/alpha16-system2-http-mount.md).
