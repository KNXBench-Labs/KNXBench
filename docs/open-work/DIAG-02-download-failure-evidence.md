# DIAG-02 — Retain download failure evidence in the log and report

**Priority:** P1. **Status:** Planned only; implementation explicitly not authorised.
Listed in [OPEN_WORK](../OPEN_WORK.md). Owner request: 26 October 2026.

## Observed gap

An Alpha 16 regular download report (`0.1.0-alpha.16+g02c437110`)
for application `M-0083_A-0024-15-EF03`, mask `0701`, records a complete-scope
plan with 25 steps, stopped in step 21, `written: partially`, and no restart
result. It contains no failure reason or failing step description.
The supplied `DeviceDownload` log entry only says that the download stopped
partway and programming flags were cleared; `severity` is `info`, `location`
and `detail` are null. The owner saw a **yellow**, not red, message and confirms
there is no additional error entry. Do not assume a missing red dialog or
require a fresh hardware attempt to establish this diagnostic gap.

These observations establish lost diagnostic context, not the device failure's
cause. Timeout, rejection, verification failure and device state remain unknown.
Do not copy installation addresses, room/device names, parameter values or the
private attachment into public fixtures or documentation.

## Bounded outcome

1. Trace the regular executor's terminal error through server status, project
   log, UI and exported report. Retain one correlated, structured terminal
   diagnostic rather than only the programming-flag settlement summary.
2. Include the actual available cause (stable category/code and safe readable
   explanation), 1-based failing step number and description, phase/operation,
   attempted/confirmed write status and restart outcome. Missing observations
   remain explicitly unknown; never infer successful writes from progress alone.
3. Keep that diagnostic accessible after closing the download panel and in the
   exported report. Define retention/reopen behaviour explicitly; a volatile
   status alone is insufficient. Reuse existing log/history/report mechanisms,
   not a second competing journal. Link the failure and flag-settlement entries
   without presenting settlement as the cause.
4. Audit severity mapping and the yellow UI presentation. A failed download
   must be recognisable as failed, distinct from warnings, cancellation and
   success. Colour is supplementary, not the diagnostic contract. Do not just
   recolour the existing summary or promote every informational entry to error.
5. Define a bounded, payload-free diagnostic projection: preserve useful causes
   without blindly publishing raw exceptions, credentials, access keys, network
   endpoints, local paths, memory bytes or project/customer data. Document any
   report schema/version changes; old reports remain readable with missing
   evidence explicitly labelled unavailable, never fabricated retrospectively.

## Acceptance for the future implementation

- Software-only regressions inject a pre-write refusal, partial-write failure,
  readback mismatch, timeout/transport loss, cancellation and unexpected task
  termination; successful completion and restart-unconfirmed outcomes are controls.
- For each case, assert consistent cause/category, step and write uncertainty
  across executor/server status, durable log/history, UI and exported JSON.
  Failures before a plan step retain an absent step, not an invented index.
- Close/reopen the panel and exercise documented project/server reopen retention;
  the retained diagnostic stays available without a new download. Cover repeated
  status polling so one terminal failure does not produce duplicate log entries.
- Assert EN/DE accessible labels and severity semantics independently of colour;
  cover legacy reports without the new diagnostic and sanitisation of sensitive
  error inputs. Preserve backup references only through existing private surfaces.
- Review and run the relevant scoped gates at implementation time. No real bus,
  device write, timeout/retry relaxation or protocol change is part of acceptance.

## Coordination and exclusions

Coordinate shared commissioning/log/report surfaces with
[PRG-01](PRG-01-programming-gaps.md). This is the regular-download diagnostic
path, not a duplicate of [S2-02](S2-02-gateway-busy-after-failed-runs.md)'s
System-2 early-start refusal work. Reuse its conventions where appropriate,
without claiming it already covers this failure.

No repair of the original hardware failure, compatibility promotion, automatic
retry/restart/restore, release assignment, deployment or vendor contact is
included. Next step is a separately authorised implementation package with a
fresh source/ownership check. This planning entry grants no implementation Go.
