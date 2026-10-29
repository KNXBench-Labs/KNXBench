# Regular restore: bind replay to the captured exemplar

## Status and sources

PRG-01 implementation contract, 29 October 2026. This is an admitted software
increment, not execution acceptance or proof of whole-device recovery. The
[source review](../research/2026-10-29-regular-restore-exemplar.md) separates KNX
resource facts from the conservative replay policy.

## Required behavior

A regular backed-up memory download records the Device Object PID 11 serial
on the pre-mutation connection, after prior application identity and before
load states and memory regions. The read uses object 0, PID 11, start 1, count 1
with strict object/PID/start correlation. Only count 1 and exactly six octets
produce `Recorded`; refusal, malformed correlated count/width and acknowledged
silence on a still-live connection produce `NotRecorded`. Other session failures
abort before `keep` and mutation, preserving the original error. There is no
retry, reconnect, authorization change or memory fallback. Unbacked downloads,
previews and comparisons do not add this observation.

This is an observation, not a new download-admission identity comparison.
An unavailable serial does not alone prevent an otherwise-admitted download;
it prevents a later restore. Zero and all-FF answers remain recorded but are
not bindable for replay, following the existing recovery policy.

The lowest public `restore_plan` derivation requires a recorded, bindable
six-octet serial, without weakening any existing product, shape, machine,
application or loaded-state checks. It inserts a typed read-only exemplar
precondition before the first device-changing step. Executing the returned plan
through any ordinary memory executor must enforce that precondition before
backup, `keep` and mutation. No caller-only optional flag may bypass it.
A strict live PID 11 response must equal the captured six octets; mismatch,
refusal, wrong count/width or no correlated answer refuses without a backup
callback or load/memory writes. Transport failures retain their original cause.
Stop/commit ordering, retry/wait policies and existing family/authorization gates
are unchanged. Diagnostics and step descriptions must not expose serial bytes.

Equal answers are a necessary replay guard, not proof of physical identity:
a device can misreport and the profile's read-only designation is not absolute
at every privileged access level. Matching does not prove function, complete
mutation coverage, recovery from interruption or restoration of properties,
allocations, task effects or unbacked memory.

## Persistence and procedure identity

Backup format 3 adds required nullable `exemplar_serial`: exactly twelve hex
digits or null. New serialization uses uppercase hex. Formats 1 and 2 remain
readable, without inventing serial evidence; they reject a smuggled new field.
Format 3 rejects missing, duplicate, non-string, wrong-width or non-hex fields;
unknown fields and versions remain refused. Existing create-new, owner-only,
sync, readback and directory-sync guarantees remain unchanged.

**Compatibility narrowing:** every format 1/2 backup, including partial backups,
and every format 3 backup without bindable serial evidence is not restorable.
There is no legacy exemption or bypass flag. Older format-2 readers reject
format 3; no seamless downgrade or rewriting of old files is claimed.

The synthetic runtime exemplar precondition is not a manufacturer load-procedure
step. Store and compare procedure identity through one explicit canonical helper
that excludes only this typed precondition, never source `CompareProperty`
checks. The serial is separately durable evidence. A backup made during a restore
must itself remain restorable under the same procedure and new captured serial.
Serial bytes stay in the private backup, not history, reports, HTTP DTOs or logs.

## Consumers and tests

CLI restore uses the guarded plan; CLI and HTTP downloads produce format 3 through
the shared executor/persistence path. HTTP has no restore route. Web, history and
reports retain their existing closed result shapes; no new exemplar-success
claim is added to the post-download check. Validate real consumers, not only
lexical source assertions; typecheck affected fixture configs if they change.

Required acceptance includes original-source semantic RED, then GREEN for:

- Same-product/application replacement target refused before writes and `keep`;
  matching target restores captured bytes and backup-of-restore can restore again.
- Missing, all-zero and all-FF evidence refused at public derivation; no partial
  exemption; existing shape/application/machine guards remain effective.
- Exact capture order; unavailable, malformed and failed observations;
  strict count/width/echo checks; no added preview/unbacked/comparison traffic.
- Format 1/2 read compatibility and restore refusal, format 3 roundtrip/null,
  malformed/missing/duplicate/smuggled field refusal, durability failures unchanged.
- Actual CLI pre-connector refusal, shared HTTP-produced backup field, unchanged
  payload-free history/report behavior and server shutdown/stop ordering.
- Independent whole-feature review, focused and maintained branch/main-first
  package gates, real server build and isolated loopback smoke.

Only offline/simulator/loopback execution is authorized. No hardware, bus, VM,
release or deployment acceptance follows from this contract.
