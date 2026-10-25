# Alpha 16 System-2 UI transport contract

Status, 24 October 2026: shared App backend and UI adapter are mounted, with
bounded A16-03 software acceptance in the
[current release matrix](../goals/alpha16-release-plan-2026-10-23.md#current-acceptance-disposition--24-october-2026).
This supersedes earlier unmounted/backend-pending wording below. UI fixture
evidence remains separate from the final mounted HTTP/worker TestHardware proof;
neither verifies physical recovery or ETS parity. Local amd64 software/package
acceptance is complete, including both formerly blocked pinned-input passes;
extra local ARM64 preacceptance is explicitly deferred and unverified, without
changing the normal amd64+arm64 publication workflow. Publication Go is conditional on
final readiness; parent closeout and public readback remain pending.
Ordinary device-download endpoints retain their current semantics.

## Required user workflow

The existing Devices → Download entry must offer the selected System-2 device's
experimental configured download. It prepares offline; opening/mounting/polling
never contacts a bus. UI calls the shared App service through the following
same-origin authenticated endpoints, never invokes a CLI process and never
uploads caller-constructed plans or backup contents.

The mandatory scope is the exact supported application/profile identified by the
shared backend, using actual project parameters, group links and flags. There
is no default-configuration substitution. Source selection is explicitly retained
source closure; if it is not available, preparation refuses without silently
falling back to product defaults. Standalone-package CLI selection remains a
separate supported input, not a silent UI fallback.

## DTO vocabulary

All responses use `schemaVersion: 1`; JSON object properties below are required
and unknown properties/closed-enum tokens are refused by this new client. Small
IDs/counts are safe nonnegative integers; opaque preparation/operation IDs are
32 lowercase hex, revisions are canonical decimal u64 strings. No server paths,
serials, credentials, raw memory, parameter values or recovery bundle contents
are in these transport DTOs. The exact confirmation phrases are deliberate
operator-facing target/plan acknowledgement strings, not credentials.

`Binding`:
`{serverIncarnation: string, projectIncarnation: u64-string, projectRevision: u64-string, deviceId: safe-integer}`.
Server incarnation is the existing opaque nonempty server identity, not a new
request-supplied identity authority. Preparation is retained server-side; public
binding alone cannot authorise execution.

`Preview`:
`{schemaVersion, preparationId, binding, address, deviceName, programId,
profileId, configurationSha256, planSha256, parameterValues, groupLinks,
flagOverrides, steps, dataOctets, allocationOctets, notWrittenOctets,
warnings, confirmationPhrase, hypothesisPhrase, downloadPhrase, releasePhrase}`.

`steps` and `warnings` are bounded string arrays. Addresses are canonical
individual-address text from the shared backend. Digests are 64 lowercase hex.
The projection may refuse before retaining a preview if it cannot fit a bounded
public response; it never truncates required plan/risk information.

`Status`:
`{schemaVersion, operationId, binding, address, deviceName, state,
completedSteps, totalSteps, backupAvailable, recoveryPhrase, downloadPhrase,
releasePhrase, message, refusal, mutationMayHaveStarted, prestateDiffers,
pointerObservations, programVersionObservation}`.

Issue #30 own-run diagnostic follow-up (unreleased source): `refusal` is a
required nullable stable error-kind token, ASCII alphanumeric with 1–64 characters
when present. It records only a refusal from the current in-process worker and
is cleared on a new attempt or historical reattachment; it is not persisted or
inferred from an old selected journal. The panel shows this reason separately
from conservative write/recovery evidence. A named `4xx` start refusal is also
shown without automatic resend; missing/malformed/network/server-error responses
retain unknown-outcome handling. The gateway hint recognizes the admitted
`system2Operation` owner in the partial activity snapshot rather than blaming
its own shared programming lock. No safety admission or bus behavior changes.

Additive fields, 2026-10-25 (GitHub issue #30). `mutationMayHaveStarted` is the
server's conservative flag: `false` only when no write could have been
sent (including a refusal before connection/journal creation, or no recovery
checkpoint and no mutating step). It is not a fabricated journal for a refused
pre-connect attempt. The panel shows "Refused
before any write — nothing was written" for `failed`/`cancelled` only when it is
`false`; otherwise the conservative state text stays. `prestateDiffers` is true
when the captured prestate differed from the source allocation shape or
application; the server then offers no recovery phrases and the panel shows
"original-state recovery unavailable" instead of the recovery form.
`pointerObservations` is an array (at most 256) of `{run: "download"|"restore", phase:
"prestate"|"inProcedure"|"final", object, operationIndex, expected, observed,
loadState}` with eight lower-case hex digits per pointer; `observed` may be
`"refused"`, and `operationIndex` is a number exactly for `inProcedure` entries
and null otherwise. `programVersionObservation` is `{expected, observed}` (hex, `observed`
empty when the read was refused) or null. `prestateUnreadableAllocations` is an
array (at most 256) of `{object, address, length}` with a four lower-case hex
digit start address, naming declared allocations whose prior memory read was
refused or short. Their stored bytes are zero placeholders: the panel then
replaces the "backup evidence is retained" line with "Prior memory could not be
read — no backup" and lists them. A non-empty array implies `prestateDiffers`
and no recovery phrases; the client refuses anything else. The client refuses a status that has
prestate evidence without `prestateDiffers`, or that offers recovery phrases
together with `prestateDiffers`. `prestateDiffers` may be true without evidence
in the DTO: a captured all-zero/all-FF serial, which the DTO never carries, also
makes recovery unavailable. Serial numbers stay out of the DTO.

`downloadPhrase` and `releasePhrase` are required nullable strings retained from
this exact original operation, not reconstructed from the current project's or
latest preparation's plan. Both must be present with `recoveryPhrase` whenever
recovery is offered. Otherwise they are null. Read-only status reattachment must
provide these original acknowledgements after navigation or a lost start response;
missing terms refuse recovery, never trigger guessed phrases or a fresh download.
This additive pre-release contract correction closes the UI handoff's recovery
projection gap; it is not yet evidence of mounted backend acceptance.

State is one of `running`, `finished`, `cancelled`, `failed`, `interrupted`,
`restoring`, `restored`, `restoreFailed`. `completedSteps` is a safe nonnegative
integer, `totalSteps` a safe integer or null, and completed<=total when known.
Only actual durable completed checkpoints contribute progress, never a synthetic
percentage. `recoveryPhrase` is string or null; `message` is a safe bounded
operator-facing summary or null, not raw exception/debug output. BackupAvailable
means retained actual-prestate evidence is available for explicit recovery,
not a guarantee of physical restorability. Restoring requires the same retained
operation and never automatically retries an earlier attempt.

## Operations

- `POST /api/device-system2/prepare` body
  `{deviceId, sourceMode:"retainedSources"}` → Preview. Offline only. Fresh
  retained preparation replaces no active operation and is bound to the current
  exact project/config/source. Refuse absent/unsupported/ambiguous source or
  unsupported partial scope rather than choosing a different plan.
- `POST /api/device-system2/start` body
  `{preparationId, gateway, confirmation, hypothesisConfirmation,
  downloadConfirmation, releaseConfirmation, recoveryTrack}` → Status.
  RecoveryTrack is `knxbenchBestEffort` or `externalETS`. Validate all phrases,
  retained source/project binding, active operation availability and normal bus
  guards before acquiring a tunnel. Consume the preparation once admission
  succeeds; never auto-retry a lost response or let another plan reuse it.
- `GET /api/device-system2/status` → Status or JSON null. Read-only local
  snapshot; no filesystem/bus/SQL work under a long-running operation lock.
- `POST /api/device-system2/cancel` body `{operationId}` → Status. Request
  cooperative cancellation, retain job ownership until settlement; no claim
  that cancellation undoes prior writes. Never permits a later step by retry.
- `POST /api/device-system2/recover` body
  `{operationId, gateway, recoveryConfirmation, downloadConfirmation,
  releaseConfirmation}` → Status. Explicit separately confirmed operation using
  the original actual-state backup and target identity, not the current project's
  desired defaults. All normal admission/backup-integrity/identity checks remain.

Each mutating body is bounded and rejects unknown/duplicate fields. All routes
are covered by existing auth/same-origin policy; no new CORS or cookie authority.
Concurrent/invalid/stale preparation is a typed refusal. A timeout or HTTP loss
means unknown-to-client: poll status, never automatically resend start/recover.

## Lifetime and safeguards

The server owns the operation task/cancel flag and retains its status and recovery
record independently of a component/request lifetime. A new preparation cannot
replace a running/restoring operation. Navigation does not cancel or erase it;
returning to Download polls its status. Browser reload cannot imply rollback.
On server restart, a durable unfinished journal is interrupted and can only be
recovered through explicit validated operation selection, never automatically
resumed. Parent integration must provide actual restart recovery selection before
claiming that part accepted; current in-memory status alone is insufficient.

All bus uses join existing single-tunnel/device-write admission and activity
history. Fresh complete actual prestate is persisted/read back before setup as
well as load controls/data. The new UI does not bypass ADR-0152: full required
verification decides Restored, and residual masked/indirect effects are stated as
best-effort, not ETS compatibility or certified firmware support.

## UI integration

Render the experimental route within the existing device-download workflow,
using the stable selected DeviceId/current project context and gateway controls.
Keep the ordinary complete/partial memory path intact. Explain the risk and both
recovery-track choices, show plan/range/count summaries and require all exact
phrases plus the existing programming consent dialog. Do not prefill phrases or
make a remembered general consent replace the experimental target-specific ones.

Keep request-generation guards for preparation and status so stale replies cannot
paint another device/project. Retain/display active operation identity even if
selection changes; disabling controls is not a substitute for server checks.
Provide explicit cancel and recovery controls, EN/DE messages, keyboard-labelled
inputs and visible error/unknown outcome. No private bundle or raw protocol is
automatically uploaded or requested in a public issue.
