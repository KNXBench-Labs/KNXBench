# System-2 safe failure diagnostic (S2-01 projection)

## Scope

A refused PID 7 (table reference) answer under the one-element two-or-four-octet width
rule ends a System-2 download or explicit restore. This contract defines the typed,
non-raw facts of that refusal and how they travel from the original read through the
private journal, reopening, progress, the shareable report and HTTP status. The private
raw diagnostic stays private. Origin: [S2-01](../open-work/S2-01-pid7-raw-response.md).

## Source of the facts

Net's `PropertySimulationFailure::PointerWidth(PointerWidthRefusal)` carries a closed
reason, object, property identifier, element count and octet count, plus a capped raw
hex prefix that only `private_diagnostic()` exposes. App maps the typed fields with an
explicit `match` (`PointerFailureRecord::from_failure`). It never parses Display, Debug
or any string, never copies the raw prefix and never derives facts for another failure
kind. Legacy-policy refusals stay untyped `Recovery(String)` and carry no record.

A record is consistent only as Net produces it:

| Reason (`reason`)      | Required counts                          |
|------------------------|------------------------------------------|
| `zeroElementsWithData` | `elements == 0`, `octets > 0`            |
| `elementCount`         | `elements >= 2`                          |
| `unsupportedWidth`     | `elements == 1`, `octets` not 2 and not 4 |

`propertyId` is always 7. `object` is any octet the refusal names: bounded observed
context, deliberately not checked against the width policy's object list, so the grammar
claims no policy authority. A refusal that fails this check gets no record; its failure
checkpoint is still written, so no failure is lost to a projection problem.

A diagnosis is a bounded typed observation of one refused answer. It grants no device
support, no compatibility claim and no permission to retry.

## Private journal

`Checkpoint::Failed` and `Checkpoint::RestoreFailed` gain an optional `pointer_failure`
object (`reason`, `object`, `property_id`, `elements`, `octets`; closed keys). It is
omitted when absent, so every other checkpoint keeps its exact version-1 bytes.
The `local_diagnostic` text is unchanged: still `format!("{error:?}")`, raw prefix kept.

Key presence is strict: an absent key means no record and keeps the legacy shape; a
present key must hold a record. An explicit `"pointer_failure": null` is refused under
either envelope version, so no version-1 checkpoint carries a key that an older strict
reader would reject without a populated record behind it.

Envelope version:

- `format: 2` exactly when the checkpoint carries a populated `pointer_failure`;
- `format: 1` for every other checkpoint, including failures without a record.

The reader accepts both and refuses any other version/presence pairing (a populated
record under version 1, version 2 without a record). It also refuses an inconsistent
record, a record under any code other than `identity_or_recovery`, and a second record
for the same run. An older strict reader accepts only version 1 and closed keys, so it
refuses a populated record instead of silently dropping it.

The hash chain and `HEAD` detect corruption and missing tails. They do not authenticate
the journal against deliberate rewriting; validation on replay bounds what a rewritten
record can claim, it does not prove who wrote it.

`HEAD` stays at version 1: its grammar (sequence plus digest of the latest checkpoint)
is unchanged and independent of the checkpoint contents, so it needs no new version.

Old journals have no record and reopen with an empty list. Nothing is backfilled from
`local_diagnostic`.

## Projection

App exports `FailureDiagnostic`, `FailureDiagnosticRun`, `PointerFailureReason` and
`PointerFailureRecord`. One diagnostic serializes with exactly these keys:

```json
{"run": "download", "reason": "elementCount", "object": 1,
 "propertyId": 7, "elements": 2, "octets": 2}
```

`run` is `download` (`Failed`) or `restore` (`RestoreFailed`). It comes from the
checkpoint variant, never from the record.

`OperationProgress.failure_diagnostics` lists at most one diagnostic per run, download
first. The download diagnosis survives `RestoreStarted`; a restore adds its own and
never relabels the download one. A new operation starts with an empty list.

## Shareable report

`knxbench-property-validation-summary` moves to `format: 2`. It adds the top-level
`failure_diagnostics` list of the objects above, empty when nothing typed was recorded
or the operation predates the record. Identity and privacy boundaries are unchanged: no
path, key, binding hash, phrase, raw octets or private text.

Consumers: `knx device system2-validation --report <path>` writes it to a file. No decoder exists in
this repository; external readers must accept format 2.

## HTTP status

`GET /api/device-system2/status` (and every route returning the same view: start, cancel,
recover, select and the `operations` list) moves to `schemaVersion: 2` and adds `failureDiagnostics` with the objects
above. Every other field is unchanged. The status no longer flattens the private
metadata record, so it carries exactly one `schemaVersion` key. The private metadata
file and the preview stay at `schemaVersion: 1`. Old clients that require exact
version 1 refuse version 2; the web client accepts exact version 1 (normalized to an
empty list) and exact version 2 with the list required.

The client validates the diagnostic vocabulary, bounds, reason/count consistency and
run ordering, but does not cross-check each run against the outer operation state.
The server derives the run from the validated checkpoint; retained download facts may
remain visible during or after a restore. An unavailable row means only that no typed
pointer-read observation exists for the failed run, not that a pointer read caused it.

## Retained boundaries

No retry, resume or automatic restore follows a diagnosis. Recovery stays explicit and
once-only. No live-bus or hardware evidence backs this projection; it is tested on
hardware-kind simulated transports only.
