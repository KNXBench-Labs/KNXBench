# System-2 safe PID 7 diagnostic projection — 26 October 2026

## Delivery status

**Reviewed candidate; package/integrated acceptance and publication pending.**
Independent whole-feature review and the focused test-coverage re-review both APPROVE.
The re-review closes the App replay count/width negative-test gap without a production
change; the client run/state cross-check and unavailable wording are documented
boundaries, not hardware evidence. The maintained package gates have not yet completed.
[S2-01](../open-work/S2-01-pid7-raw-response.md) stays open until that delivery.

Not done and not claimed: release, deployment, live-bus operation, vendor contact,
reporter retry, hardware support, physical recovery.

## Scope

A refused PID 7 (table reference) answer ends a System-2 download or explicit restore.
The candidate carries only its closed typed facts — a six-key object `{run, reason,
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

Complete package acceptance is **PENDING**. Development execution is retained
separately: App semantic RED before behaviour edits, final focused App and server/CLI
checks, and the original private raw-prefix evidence. The additional replay-negatives
pass against the unchanged production logic; this is coverage strengthening, not a
newly reproduced product defect. The duplicate-per-run guard is redundant with the
outcome guards and is not claimed individually mutation-tested.

The controller also executed the production web bundle with unchanged HTTP response
bytes from the real router/worker over a simulated hardware-kind transport, including
journal reopening and separate download/restore failures: all four EN/DE × 1440/400 px
cases passed with frozen inputs and unchanged captured bytes. This browser join uses
intercepted transport; it is not a live backend browser session or hardware proof.
The original capture invocation had a separate new fixture failure, corrected before
the final server selection passed; that invocation is not relabelled wholly green.
The German narrow status screenshot was visually inspected by the independent reviewer;
other screenshots are retained without a blanket visual-acceptance claim.

One known, **pre-existing, ignored** pinned test,
`pinned_public_http_worker_cancellation_reopens_original_once_only_recovery`, exceeds
its 10 s settle budget while a restore is still progressing. It fails identically on
the unchanged base, so it matches base. It is neither turned green nor fixed here, and
the unrelated shared helper stays untouched.

## Boundaries and residual risks

- A diagnostic is a bounded observation, not a hardware-support grant. Objects 2/3
  stay source-declared hypotheses; the journal hash chain detects corruption, it does
  not authenticate against deliberate rewriting.
- Old-reader rejection is proven by a grammar mirror, not by running an old binary.
- [S2-02](../open-work/S2-02-gateway-busy-after-failed-runs.md) remains a separate
  investigation. Physical recovery, masked-byte survival and complete firmware
  behaviour remain open ([validation boundaries](../contracts/up258-system2-validation.md)).
