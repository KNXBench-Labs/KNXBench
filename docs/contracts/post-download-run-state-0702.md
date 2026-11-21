# Read-only run-state continuation for exact `0702h` after a regular download

This bounded PRG-01-N1 contract extends the existing
[0701/0705 contract](post-download-run-state.md) and the
[descriptor-qualified exact-0700 contract](post-download-run-state-0700.md)
by one further exact mask, `0702h`, without changing any existing mask's
schedule, gates or serialized bytes. Source facts are in the
[original-source review](../research/2026-11-20-property-run-state-observation-0702.md).
Executed acceptance and publication are separate from this contract.

## Why `0702h`, and why descriptor-qualified

Resources §4.1.2.2 Table 7 (printed p. 25) approves only `0700h`/`0701h`/`0705h`
as BIM M112 masks, but the `070nh` low-nibble family restriction of MP §3.31.2
(printed p. 134) covers `0702h`, which the existing download path already
admits. Resources footnote 94 (printed p. 298) grants the `PDT_UNSIGNED_CHAR`
legacy exception only to masks `0020h` and `0701h`; `0702h` is not listed, so
its Type-1 run-state property is `PDT_CONTROL` like any non-legacy mask. The
same conservative qualification policy as exact `0700h` therefore applies,
now backed directly by the source rather than only by policy: independent
descriptor metadata must establish `PDT_CONTROL` before one PID 6 value read.
A one-byte value alone is not type evidence.

## Admission and read schedule

Only exact mask `0702h` enters this continuation; `0700h`, `0701h` and `0705h`
keep their existing schedules byte-for-byte, and every other `070nh` mask
keeps `unsupportedMask`. The existing read-only management session must
already have matched the plan's identity and completed every load-state read
successfully; a differing but successfully read load state remains valid
prior evidence and does not gate the continuation, a failed or skipped load
read does (`loadReadFailed`, zero runtime frames).

Using candidate object index 3, in order:

1. Read PID 1, start index 1, count 1. Require one correlated element with
   exactly two octets naming Applicationprogram object type `0003h`.
2. Strictly reread PID 13 at that same candidate, start/count 1. Require one
   correlated element with exactly five octets equal to the plan application.
3. Read the PID 6 property description (APCIs `3D8`/`3D9`), exactly as the
   exact-0700 contract specifies: named PID 6, request property index 0,
   correlate on object/PID only, response payload exactly seven octets. Apply
   the same qualification precedence: nonzero upper maximum-elements nibble →
   `unqualifiedElements`; zero upper nibble and zero count →
   `descriptionProblem` (all-zero semantic descriptor) or
   `indeterminateMetadata`; count ≥ 1 with PDT 1..63 → `unsupportedDatatype`
   carrying that PDT; count ≥ 1 with PDT 0 (`PDT_CONTROL`) → qualified.
   Legacy bit 6 is preserved but never evaluated.
4. Only when qualified: read PID 6, start/count 1. Require one correlated
   element with exactly one octet. Decode only the six literal Table 95
   property-state values; every other complete octet is `undefinedValue`.

No second index, scan, description retry, alternate path, new session, added
wait/retry, Authorize, Verify Mode, write, restart/reset or memory fallback
is introduced. A failure ends the runtime sequence, never the earlier
successfully collected facts.

## Failure distinctions

Identical to the 0701/0705 contract, plus the exact-0700 description step:

- Other masks: not read, `unsupportedMask`.
- A failed/skipped earlier load read: not read, `loadReadFailed`; no runtime
  frames follow.
- A complete valid PID 1 naming another type: `candidateMismatch`,
  `objectType`. Not proof that no Applicationprogram object exists elsewhere.
- A complete valid PID 13 naming another application: `candidateMismatch`,
  `application`.
- A correlated count zero: `unavailable`, `refused`. Another correlated count
  or wrong width: `unavailable`, `malformed`.
- Wrong object/PID/start echo: ignored by the existing strict matcher; without
  a later matching response the outcome is `noCorrelatedAnswer`.
- Description exchange failures at step `description`: `noCorrelatedAnswer`,
  `connectionLost` or `readFailed` (session causes only; a correlated
  zero-element or wrong-width description answer does not correlate, so it
  ends as `noCorrelatedAnswer`).
- Disconnect or other read failure: `unavailable` with step and a closed cause.
- Exact one-octet state outside `00`–`05`: `undefinedValue`, never bitmasked.

## Persisted, payload-free record

Historical records and actual `0700h`/`0701h`/`0705h` producers are unchanged.
Within a matching record of an exact-`0702h` run, the optional non-null marker
`runStateMask: "0702"` sits adjacent to `runState`, exactly as `"0700"` does
for exact `0700h`:

- New exact-0702 matching producers emit it, including load-read failure or
  candidate failure before description; it does not assert a description ran.
- Actual `0700h`/`0701h`/`0705h` producers omit it; legacy records reserialize
  without inventing a field. Explicit null, other strings, numbers and other
  masks are rejected.
- Presence requires `runState` and forbids `notRead/unsupportedMask`.
- The new `notQualified` reasons and `unavailable` at step `description`
  require that marker; `unsupportedDatatype` alone requires integer `pdt` in
  **1..63**. Without the marker the vocabulary is rejected rather than
  mask-inferred.
- All existing object-index, load-success, nested runtime/state and
  exact-key invariants remain; top-level `runtime` agrees exactly with the
  nested evidence. A parser cannot know a fixture's origin: enforce actual
  producer parity and on-wire invariants, not labels.

Backward **read** compatibility only; older strict readers may refuse new
`0702h` records. No migration, history rewrite or downgrade is claimed.

## Consumers and wording

CLI, server status, reports and activity history use the same shared record;
the EN/DE web presentation identifies the reported property state of the
validated plan-matching Applicationprogram object 3 for a `0702h` plan, with
the same scope wording as the other masks. It does not announce that the
complete device or installation works.

## Required software acceptance

- Literal schedule on exact `0702h`: PID 1 → PID 13 → description → (only
  when qualified) PID 6, all six state values, undefined `06`, `80`, `81`,
  `FF`; every qualification branch and both legacy-bit values;
  independently configured descriptor evidence and the
  one-byte-value/no-description negative case.
- Unchanged `0700h`/`0701h`/`0705h` schedules and serialized bytes;
  unsupported `0702h`-neighbours (`0703h`, `070Fh`) and other families keep
  their outcomes; wrong/relocated candidate, binding mismatch, incorrect
  widths/counts, wrong echoes, refusal, no answer and disconnect at each
  stage; first failure prevents later runtime requests; failed prior load
  reads cause zero runtime frames; differing valid load states still permit
  observation.
- Zero prohibited operations from this continuation, no memory fallback or
  additional session; prior evidence preserved.
- Shared Rust/web strict legacy, positive and contradictory wire cases,
  durable history reopen, actual CLI/server report composition, EN/DE
  presentation.
- Original semantic RED before changed behavior, discriminating controls,
  independent whole-feature review and the maintained integrated package
  gates.

Simulator/isolated loopback acceptance is not field or hardware acceptance.
The memory-state encoding, nondefault object discovery, physical-exemplar
proof, other masks/layouts, broader parameter policies and whole-mutation
recovery remain open.
