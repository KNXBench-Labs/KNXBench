# System-2 comparison bytes: diagnostic-bundle admission boundary

Date: 29 October 2026. **Raw-admission findings are static, not hardware acceptance.**
The original boundary examines `cada10a3cf89d4a66ad57041e5d6b0b958c637dd`.
The later saved-project adapter and closed App metadata increment leave the
underlying comparison/persistence semantics unchanged. Bounded metadata execution
below is separate from raw-admission or whole-archive acceptance.

## Decision

**Full private raw-evidence export is NOT READY.** Existing persistence retains
both sides of an allocation mismatch, but neither the expected nor returned
memory has an established credential-free classification. Consent is not an
exception to [ADR-0156](../adr/0156-operation-recorder-and-diagnostic-bundle.md)'s
unconditional exclusion of keys/passwords. Do not enable raw serialization on
that basis. The closed metadata-only App projection described below is a bounded
local increment, not completion of DIAG-03's requested byte evidence.

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

## App admission foundations; consumers still open

The earlier App-only increment implements
[`read_snapshot`](../../crates/knx-app/src/property_validation_private_projection.rs)
with one strict fresh `OperationStore::open` against the expected Binding.
Original minimized progress and separately owned closed metadata come from that
same locked store. Public DTOs and persistence grammar are unchanged.

Only the closed projection serializes: typed comparisons with persisted bytes
are `recordedBytesWithheld` with an explicit reason/loss; a typed mismatch without
persisted comparison bytes is `notRecorded`, not a claim that no read occurred.
Missing witnesses, empty lists and `Finished`/`Restored` remain `unclassified`.
Open failures expose neither progress nor free error text. No payload bytes,
lengths, hashes, masks or differing slices are exported; no additional journal
size budget or authenticity guarantee is claimed. The current App source adds
`admit_snapshot`: it consumes the available snapshot once, validates the fixed
format/kind and Download/optional-Restore ordering against the same progress,
serializes the closed projection once at no more than 4 KiB, and returns the
original owned progress with private admitted bytes. The admitted type implements
neither `Serialize` nor `Debug`.

The same source increment adds bundle-only admission for historical import reports
and saved native projects. Both entry points reuse the existing parser/sanitizer;
the native path retains its one bounded immutable source snapshot. An internal,
non-serialized observation flag is propagated through discovery, enforcement,
children and withheld/discarded branches. Recognized non-empty credential fields,
report credential digests without the free-access witness, and `BusAccess`
credential components set it before mutation. Any observation yields one generic
unavailable result before owned body or notes escape. Only the documented free-
access `BCUKey` sentinel is exempt. This is a narrow instance witness: ordinary
sanitizer success does not admit a bundle source and does not certify arbitrary
opaque, encoded or encrypted material.

Bounded App-only verification completed on 30 October 2026. The unavailable
scaffold ran 92 focused tests (81 passed, with 11 intended positive-admission
failures); the first implementation passed 92/92. Nine edge regressions then gave
the intended 94-pass/7-fail result, and the corrected source passed 101/101. Under
one temporary C10 final-gate bypass, 75 credential-refusal expectations failed as
intended while 26 tests passed: this included all 24 `Password123`
body/`source_path`/`xpath` witnesses in literal, hex, base64 and percent forms, and
all seven benign controls remained green. The exact production bytes were restored,
rebuilt and passed the restored 101/101 run.

The default App run passed 431 library tests, 584 integration tests and nine
doctests (1,024 total without adding focused/default overlap), with 47 integration
tests ignored. Six new compile-fail doctests emitted `E0277` for the intended lack
of `Serialize`/`Debug` on the actual newtypes. App all-target Clippy with
`-D warnings` covered library, library-test, example and integration targets; the
formatting, header, layering, test-layout, documentation, local-info, anchor, diff,
allowlist and guard checks passed without suppression. Ignored, corpus and hardware
cases were not executed. This verifies only the App foundation, not consumer or
whole-archive acceptance.

Both server hooks and the CLI still lack the attachment. They must consume one
coherent snapshot and retain the same owned bytes through text/review/export;
existing target/version/context, fingerprint and consent guards remain required.

The parent reports the bounded R4 ambient-identity prerequisite independently
approved for the real preview/unselected-recording regression. R4 is prerequisite
closure only; it does not authorize export. Raw P1/P2, the original fourteen-case
matrix, consumer integration and whole-goal acceptance remain open.

A future positive raw variant is blocked by P1/P2. The original **14-case matrix
remains open**; the bounded metadata tests above do not constitute fourteen new
passing acceptance cases. Server, CLI and cross-writer proposals remain unexecuted:

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
