# Read-only run-state continuation after a regular download

This bounded PRG-01 contract extends the existing
[post-download check](post-download-check.md), without changing transfer,
restart, settlement or function-checklist semantics. Source facts and the
explicit candidate-location interpretation are in the
[original-source review](../research/2026-10-29-property-run-state-observation.md).
Executed acceptance and publication are separate from this contract.

A separately [design-admitted exact-0700 increment](post-download-run-state-0700.md)
adds descriptor qualification for an optional property. It is not implemented
at source `8d14de9ca` and does not change this 0701/0705 path, its schedules,
legacy datatype handling or serialized bytes. Its explicit 0700 scope-policy
supersession must not be mistaken for already-executed acceptance here.

## Admission and read schedule

Only exact masks `0701h` and `0705h` enter this continuation. The existing
read-only management session must already have matched the plan's identity and
completed every load-state read successfully. A successfully read state that
differs from the plan remains valid prior evidence; it is not a runtime gate.

Using candidate object index 3, in order:

1. Read PID 1, start index 1, count 1. Require one correlated element with
   exactly two octets naming Applicationprogram object type `0003h`.
2. Strictly reread PID 13 at that same candidate, start/count 1. Require one
   correlated element with exactly five octets equal to the plan application.
3. Read PID 6 at that candidate, start/count 1. Require one correlated element
   with exactly one octet. Decode only the six literal property-state values.

The strict PID 13 reread is mandatory: the older identity path's generic
property read is not used as evidence for the new count/start contract.
Candidate validation is not universal index mapping, full object discovery,
uniqueness, numbered state-machine selection or a proven memory-machine link.
Nondefault candidate layouts remain unsupported; no other object is guessed.
Legacy `0701h` property datatype declarations do not impose a new description
lookup on this one-octet read.

No new session, added wait/retry, scan, description request, Authorize,
Verify Mode, write, restart/reset or memory fallback is introduced. A failure
ends the runtime sequence, never the earlier successfully collected facts.

## Failure distinctions

- Other masks: not read, unsupported mask.
- A failed/skipped earlier load read: not read, load read failed; no runtime
  frames follow.
- A complete valid PID 1 naming another type: candidate mismatch, object type.
  This is not proof that no Applicationprogram object exists elsewhere.
- A complete valid PID 13 naming another application: candidate mismatch,
  application binding.
- A correlated count zero: unavailable, refused.
- Another correlated count or incorrect byte width: unavailable, malformed.
- Wrong object/PID/start echo: ignored by the existing strict matcher. Without
  a later matching response the outcome is no correlated answer, not proof of
  a malformed correlated response.
- Disconnect or other read failure: unavailable, with step and a closed cause;
  no raw error string is retained in shared evidence.
- Exact one-octet state outside `00`–`05`: undefined value, never bitmasked.

Previous identity/load readings and the plan-load verdict are retained under
both matching and differing load verdicts. Runtime evidence neither confirms
nor revokes programming flags, successful transfer, restart, recovery,
physical-exemplar identity or the independent installation-function checklist.

## Persisted, payload-free record

Historical `runtime: "unknown"` remains readable. Within the identity-match
record an optional `runState` field carries the new observation. Absence means
not observed by the older producer and must not infer success. Explicit null
is invalid. Legacy records reserialize without inventing a field.

The closed variants are:

| `status` | Additional fields |
| --- | --- |
| `notRead` | `reason`: `unsupportedMask` or `loadReadFailed` |
| `candidateMismatch` | `kind`: `objectType` or `application` |
| `unavailable` | `step`: `objectType`, `application` or `runState`; `cause`: `refused`, `malformed`, `noCorrelatedAnswer`, `connectionLost` or `readFailed` |
| `undefinedValue` | None |
| `observed` | `objectIndex`: exactly `3`; `state`: `halted`, `running`, `ready`, `terminated`, `starting` or `shuttingDown` |

No address, raw identity, property payload, secret or free-text error enters
this shape. The top-level `runtime` summary is one of the six observed states
only when matching structured evidence supports it. Otherwise it stays
`unknown`. Neither a known scalar without evidence nor unknown hiding an
observed known state is valid. Rust and web readers enforce exact agreement,
identity nesting, candidate index and successful-prior-read invariants. A
`differsFromPlan` verdict alone does not contradict an observed run state.

This is backward **read** compatibility, not forward compatibility: older
strict readers may refuse new nested fields or state values. No history/native
format migration, record rewrite or seamless downgrade is claimed.

## Consumers and wording

CLI, server status, reports and activity history use the same shared record.
The shared EN/DE web presentation identifies the reported property state of
the validated plan-matching Applicationprogram object 3. It does not announce
that the complete device or installation works. Load state, run state,
transfer outcome and functional verification remain distinct.

## Required software acceptance

- Literal schedules and all six state values on both selected masks; undefined
  `06`, `80`, `81`, `FF`; unsupported `0700h`, `0702h`, `070Fh`.
- Wrong/relocated candidate type, binding mismatch and no uniqueness claim.
- Incorrect widths/counts, wrong echoes, refusal, no answer and disconnect at
  each stage; first failure prevents later runtime requests.
- Zero prohibited operations from this continuation, no memory fallback or
  additional session; ordinary cleanup and prior evidence preserved.
- Failed prior load reads cause zero runtime frames; differing valid load
  states do not prevent observation.
- Shared Rust/web legacy, positive and contradictory wire cases, durable
  history reopen, actual CLI/server report composition and EN/DE presentation.
- Original semantic RED before changed behavior, discriminating controls,
  independent whole-feature review and maintained integrated package gates.

Simulator/isolated loopback acceptance is not field or hardware acceptance.
The memory-state encoding, nondefault object discovery, physical-exemplar
proof, broader parameter policies and whole-mutation recovery remain open.
