# System-2 safe PID 7 diagnostic projection — 26 October 2026

## Delivery status

**Delivered as `536e492bc`.** Feature commit `eeba38bf` (production `5c469a5ec`),
independent whole-feature, test-coverage and gate-correction reviews all APPROVE.
Branch gate, 15 stages, passed: Rust 2731 passed / 129 ignored, corpus 91/91, vitest
4553 passed (306 files), browser inventory 914. Integrated gate on the merge:
Rust 2731 passed / 129 ignored, corpus
91/91, browser inventory 914.
These result documents carry the accepted runtime evidence; their publication is
separately guarded by the maintained documentation-delta gate.
The client run/state cross-check and unavailable wording are documented boundaries,
not hardware evidence. [S2-01](../open-work/S2-01-pid7-raw-response.md) is delivered
as a projection slice and no longer listed in [OPEN_WORK](../OPEN_WORK.md); physical
outcome and recovery remain unverified. S2-02/S2-03 and issue #30 remain separate open work.

Not done and not claimed: release, deployment, live-bus operation, vendor contact,
reporter retry, hardware support, physical recovery.

## Scope

A refused PID 7 (table reference) answer ends a System-2 download or explicit restore.
The implementation carries only its closed typed facts — a six-key object `{run, reason,
object, propertyId, elements, octets}` — from the read through the private journal,
progress, shareable report, HTTP status and the mounted EN/DE panel
([contract](../contracts/system2-safe-failure-diagnostic.md)).

- Reasons are closed: `zeroElementsWithData`, `elementCount`, `unsupportedWidth`.
  `object`, `propertyId` and `elements` are u8, `octets` is u32; counts may be 0.
- Conversion is a typed match on Net's refusal. Nothing parses text; old journals get
  no backfill; legacy-policy and all other failures stay untyped.
- The raw prefix, arbitrary exception text, paths, keys, recovery payloads and raw
  bytes never reach report, HTTP or UI. The private journal keeps its prior raw evidence.
- `run` separates a download from a restore and keeps the download entry when a
  restore follows. The same operation is not retried; a new operation starts empty.
  Resetting the project is not a bus command.
- Existing pointer observations may show four or eight hex digits, as delivered with
  the width fix (`54b052885`). This is not a new raw-diagnostic allowance.

## Compatibility

| Surface | Version | Content |
| --- | --- | --- |
| Private failure journal envelope | 1 or 2 | 2 exactly when the failure checkpoint holds a typed record; 1 otherwise, including all old journals |
| Journal `HEAD` | 1 | Unchanged |
| Shareable report | 2 | Adds `failure_diagnostics` (empty when untyped/legacy) |
| HTTP Status | 2 | Adds `failureDiagnostics`; exactly one `schemaVersion` key |
| HTTP Preview | 1 | Unchanged |
| Private `metadata.json` | 1 | Unchanged |
| Web client | v1 and v2 | Strict route: exact v1 normalizes to an empty list, exact v2 requires the list, v1 with the key is refused |

An older strict journal reader refuses a populated v2 record; a client requiring exact
Status v1 refuses the new Status. The mounted panel shows the diagnostic per run and
an *unavailable* row for a failed run without a typed entry.

## Evidence

Development execution is retained separately: App semantic RED before behaviour edits,
final focused App and server/CLI checks, and the original private raw-prefix evidence.
The additional replay-negatives pass against the unchanged production logic; this is
coverage strengthening, not a newly reproduced product defect. The duplicate-per-run
guard is redundant with the outcome guards and is not claimed individually
mutation-tested.

**Gate history, not relabelled.** Branch gate 1 on `5c469a5ec` was RED: the first nine
stages passed and the nested functional browser run exited 0 with 847 passed / 4
skipped, but the verifier refused (`non-passing browser expectation is not admitted`).
The four skips were the new private-capture opt-in cases without their env. No browser
assertion failed; later stages did not run. That receipt is retained and is not green.
The test-only correction `eeba38bf` (1 file, +3/-2) registers those cases only when the
capture env is set; production code is unchanged. Corrected branch gate 2 passed
all 15 stages.

**Browser evidence, two separate kinds.** The 9 ordinary intercepted-API cases are part of the
standard lane. The 4 opt-in capture cases run separately against unchanged real captured
bytes (production web bundle, real router/worker, simulated hardware-kind transport,
journal reopening, separate download/restore failures; EN/DE x 1440/400 px, frozen
inputs). This join uses intercepted transport; it is not a live backend browser session
or hardware proof. The original capture invocation had a separate new fixture failure,
corrected before the final server selection passed; it is not relabelled wholly green.

**Visual check.** An independent reviewer inspected the German narrow screenshot, and a
child observer reports pixel inspection of all four final EN/DE x 400/1440 screenshots:
new diagnostic entries readable, distinguished, no clipping or overlap. The parent
vision API failed, so there is no parent pixel-inspection claim.

**Translation boundary.** The new diagnostic vocabulary and entries are translated EN
and DE. The pre-existing static paragraph "Original retained operation; no automatic
resume or retry. Recovery remains experimental best-effort." stays English in the German
panel; it is the same source line and renderer as on the immutable base (verified
unchanged). No full-panel translation is claimed.

One known, **pre-existing, ignored** pinned test,
`pinned_public_http_worker_cancellation_reopens_original_once_only_recovery`, exceeds
its 10 s settle budget while a restore is still progressing. It is red on the
unchanged base and on the candidate with no proven cause, so it matches base. It is
neither turned green nor fixed nor waived here, the standard gates do not select it, and
the unrelated shared helper stays untouched.

## Boundaries and residual risks

- A diagnostic is a bounded observation, not a hardware-support grant. Objects 2/3
  stay source-declared hypotheses; the journal hash chain detects corruption, it does
  not authenticate against deliberate rewriting.
- Old-reader rejection is proven by a copied grammar mirror, not an executed old
  binary. The explicit-null refusal has no pre-edit semantic RED; its final refusal
  and absent-field controls passed. These are separate evidence boundaries.
- [S2-02](../open-work/S2-02-gateway-busy-after-failed-runs.md) and
  [S2-03](../open-work/S2-03-up258-image-interpretation.md) remain separate open work. Physical recovery, masked-byte survival and complete firmware
  behaviour remain open ([validation boundaries](../contracts/up258-system2-validation.md)).
