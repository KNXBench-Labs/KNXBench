# System-2 comparison bytes: diagnostic-bundle admission boundary

Date: 29 October 2026. **Static source findings, not runtime or hardware acceptance.**
Source examined: `cada10a3cf89d4a66ad57041e5d6b0b958c637dd`; the subsequent
saved-project adapter does not change the System-2 sources below.

## Decision

**Full private raw-evidence export is NOT READY.** Existing persistence retains
both sides of an allocation mismatch, but neither the expected nor returned
memory has an established credential-free classification. Consent is not an
exception to [ADR-0156](../adr/0156-operation-recorder-and-diagnostic-bundle.md)'s
unconditional exclusion of keys/passwords. Do not enable raw serialization on
that basis. A metadata-only projection with explicit withheld bytes is a possible
bounded increment, not completion of DIAG-03's requested byte evidence.

## Publicly auditable source evidence

- [Net evidence](../../crates/knx-net/src/commissioning/final_step_evidence.rs)
  stores allocation identity, complete expected/actual arrays, a written mask and
  classified differing ranges. [Final verification](../../crates/knx-net/src/commissioning/property_recovery.rs)
  returns the **first failing allocation per run**, not every compared allocation.
  Expected bytes combine captured prestate with executed writes; actual bytes
  come from `read_region`. Unreadable prestate placeholders are not predictions.
- [App evidence](../../crates/knx-app/src/property_validation_allocation_evidence.rs)
  retains whole-array hex and range classifications. [Journal replay](../../crates/knx-app/src/property_validation.rs)
  validates these against the executed download/recovery plan. This establishes
  coherent recorded comparison semantics, **not credential classification** and
  not authenticity against a malicious rewrite of an entire journal chain.
- [Execution](../../crates/knx-app/src/property_validation_execution.rs) skips
  explicit authorization at the relevant System-2 sites. The
  [AccessKey wrapper](../../crates/knx-core/src/commissioning/authorisation.rs)
  avoids serialization and redacts Debug. Neither fact proves arbitrary captured
  memory, parameter data or unexpected readback credential-free.
- [Wire redaction](../../crates/knx-net/src/wire_tap.rs) handles identified
  authorize/key-write APDU offsets. That is not a general classifier for memory
  or property response payloads, arbitrary error strings or derived hex slices.
- [Retained storage](../../apps/knx-server/src/device_system2/storage.rs) reopens
  the journal, but `progress` drops it after obtaining minimized progress.
  [Diagnostic projection](../../apps/knx-server/src/device_system2/routes.rs)
  returns progress and a minimized report. The
  [bundle collector](../../apps/knx-server/src/diagnostic_bundle_routes.rs) has
  **two** relevant paths: retained-operation resolution and recording-related
  operation sections. Neither currently attaches the typed comparison file.
- [CLI collection](../../apps/knx-cli/src/diagnostic_bundle_cli.rs) exports
  environment/recordings, but does not obtain typed mismatch evidence.
  [CLI System-2 execution](../../apps/knx-cli/src/device_system2_validation.rs)
  owns the store and writes the minimized report separately. General CLI bundle
  support exists; the missing part is shared typed private evidence attachment.
- The [native project-copy helper](../../crates/knx-app/src/secret_free_copy.rs)
  is a separate boundary. It does not sanitize allocator-memory evidence.

## Two unresolved admission requirements

**P1 — both byte sides.** No admitted producer invariant or existing helper
classifies every expected and actual octet, including unwritten/captured memory,
restore prestate and unexpected values at written offsets. Written does not mean
credential-free. Admit only proven permitted spans, or explicitly withhold/refuse
unclassified spans. Do not substitute a warning, hash, ASCII grammar, successful
serialization or an always-true safety flag for this evidence.

**P2 — all overlapping archive entries.** A withheld/redacted value must not
remain in a differing slice, recording memory datagram, numeric JSON array,
progress/error entry, note or another selected file. Existing exposure is not
authorization for another copy. A metadata-only new file does not certify the
rest of the ZIP. Keep original evidence local; disclose exported loss explicitly.

## Bounded next implementation, not an implemented API

Use one shared App-owned projection from the same successfully reopened store
as progress. Preserve the minimized public report and persistence grammar.
Explicitly distinguish recorded-but-withheld bytes, a typed mismatch with
`notRecorded` bytes, unclassified historical outcomes and unavailable journals.
Do not interpret an empty list as proven absence or infer restore stages from
counter arithmetic. Both server hooks and the CLI must use the same owned bytes;
normal target/version/context, review-fingerprint and consent guards still apply.

A future positive raw variant is blocked by P1/P2. The following **14 acceptance
cases are proposed and unexecuted**, not new passing tests:

| Scope | Proposed cases |
| --- | --- |
| App (6) | Unwritten mismatch; unreadable-prestate classes; separate restore expectation; known empty versus unclassified; replay admission/refusal; deterministic closed schema and explicit exported loss. |
| Server (4) | Synthetic executor mismatch through reopened journal to ZIP without recording; the same with recording/history; source/version/target/review admission; unchanged public/minimized reports. |
| CLI (2) | Real built CLI against a loopback synthetic gateway with mismatch and retained failure outcome; no-recording/refusal/no-flag/duplicate-attachment behavior. |
| Cross-writer (2) | All-entry recognized-key sentinels through the real tap; unclassified memory/error sentinels on both byte sides and every overlapping copy. |

Use synthetic fixtures and actual producers, replay, collectors and ZIP writers.
A surviving sentinel is a failure, not an accepted legacy leak. No private
corpus, real installation access, manufacturer contact or hardware inference is
needed or authorized by this static result.
