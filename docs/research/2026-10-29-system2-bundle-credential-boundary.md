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
  operation sections. The bounded consumer candidate now feeds both from one
  admitted snapshot and request-local cache, without adding raw recording files.
- [CLI collection](../../apps/knx-cli/src/diagnostic_bundle_cli.rs) now owns an
  invocation-local admitted capture slot. The System-2 simulator path supplies the
  closed projection after releasing the operation store; raw mismatch evidence is
  still not admitted.
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

## App admission foundations and bounded consumer candidate

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

The bounded consumer candidate uses one coherent reopened snapshot and retains the
same owned bytes through text, review and export. Each direct/current/history edge
validates provenance before request-cache reuse; source disagreement is distinct
from target mismatch. Project/import copies are moved from their helper-owned
admission result and the collector keeps raw recording bodies and all other raw
sources fixed unavailable. The CLI slot has explicit empty/captured/missing/
unavailable/duplicate/consumed states and refuses an existing destination before
loopback contact.

Local execution includes one real semantic history RED, 60 focused server tests,
the default server and CLI suites, owning all-target Clippy, formatting and static
gates, full Web Vitest/type/build, and 11 intercepted-API Playwright feature cases.
That browser lane verifies all four private lifecycle cells in English and German
at wide and narrow widths, but is not the still-open captured real-router response
mounted through the production build. The large-recording test characterizes
bounded private output but does not instrument exact reader-call/byte counts; that
R3 proof remains open.

The parent reports the bounded R4 ambient-identity prerequisite independently
approved for the real preview/unselected-recording regression. R4 is prerequisite
closure only; it does not authorize export.

**31 October 2026 owner decision:** System-2 bytes never leave raw. The bundle
carries the closed projection and the typed report only, so P1/P2 are not needed
for it; they stay the prerequisites of any future raw System-2 export.

## Original fourteen-case matrix (executed 31 October 2026)

All fourteen cases now have a test on synthetic data with real producers and
replay; no private corpus, real installation, manufacturer contact or hardware.
The App cases end at the closed projection; the Server, CLI and cross-writer
cases go through the real collectors and ZIP writers, where a sentinel surviving
in any entry, entry name, preview or section text fails the test. Each new case
was also run against a deliberately broken production path and failed.

| Case | Test (file::name) | Proof |
| --- | --- | --- |
| A1–A6 | `knx-app/src/property_validation_private_projection_tests{,_more}.rs`, `property_validation_private_admission_tests.rs`, `tests/it/property_validation_execution/allocation_evidence.rs` | Unwritten/written/unreadable-prestate/restore mismatches, unclassified and refused replay, closed 4 KiB schema with explicit loss; projection scanned in every encoding. |
| S1 | `knx-server/src/device_system2/tests/diagnostic_bundle_mismatch.rs::a_real_mismatch_without_a_recording_exports_neither_byte_side` | Real App executor on the configured synthetic project, OLD simulator, lying final read; the journal holds template, lie and prestate; preview, section text, download name and every ZIP entry hold none. |
| S2 | same file `::real_mismatch_recordings_and_history_export_neither_byte_side` | The exact exchange as current and earlier System-2 recordings (sentinels present there as a precondition); no raw recording leaves. Mutants: journal shipped as a report, recording in a note. |
| S3 | same file `::a_system2_review_is_bound_to_the_journal_state_it_read`, `::a_system2_run_refuses_a_foreign_device_and_never_projects_one`; source conflict and build fields: `diagnostic_bundle_fallback.rs` | The review exports, also after a recovery claim alone; a real restore after the preview refuses export and section text. A foreign device is refused in the run's own unchanged context; after a real restart, with a project open in which that device is projectable, it is still not projected and no device file leaves. |
| S4 | same file `::a_real_mismatch_keeps_status_and_report_public_and_unchanged_by_export` | After a restart, status/report carry only the typed failure, export changes neither, the bundled report equals the public one except the unadmitted execution build. |
| C1 | `knx-cli/tests/it/device_system2_validation/gateway.rs::a_final_mismatch_is_bundled_with_neither_byte_side` | Built CLI against the loopback gateway; simulator knob `memory_after_load_completed` (property and memory-mapped load control, pinned in `knx-net/tests/it/simulator_system2_state.rs`); exit 1, journal keeps both sides, ZIP holds projection and report only. Mutant: the CLI System-2 guard lifted leaks the lie. |
| C2 | same file `::without_the_flag_a_failed_run_writes_no_bundle_and_no_recording`, `::a_doubled_bundle_flag_is_refused_before_any_datagram`; unit tests in `diagnostic_bundle_cli.rs` | No flag writes nothing; doubled or repeated destinations are refused with zero gateway datagrams. The process-global Duplicate slot is unit-covered only. |
| X1 | `knx-server/tests/it/http_bundle_key_redaction.rs` | The service-control read sends the project BCU key over a real tunnel; `A_Authorize`/`A_Key_Write`, sent and received, through the production tap; both raw recordings exported and every entry scanned, octet-wise in both byte orders and as whole numbers. Mutants: tap names but does not zero; tap spares only the routed key. |
| X2 | S1/S2 tests | Both byte sides, the expected side's differing slice on its own, and every overlapping copy (journal, recordings, projection, report, notes, every section text). |

**Scanner limit:** the sentinel scanner finds contiguous runs. A recording stores
memory in the pieces the device answered (12 octets here), so a sentinel spanning
two responses would not appear contiguously. The fixtures keep every sentinel inside
one response. That is enough while System-2 recordings never leave raw; a future
raw export would need a scanner over reassembled memory.
