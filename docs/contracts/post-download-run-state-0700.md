# Exact 0700 run state: qualify the optional property before observing it

## Status and scope

PRG-01 implementation `5f075cf99`, begun 29 October 2026; **software accepted
30 October on main-first runtime `0af68438f`**. Independent whole-feature static
review, both maintained 15-stage gates and actual isolated build/smoke passed.
[Executed acceptance and limits](../status/2026-10-30-descriptor-qualified-0700-run-state.md)
keep runtime execution separate from documentation-only delivery/publication.
No hardware acceptance follows. Historical baseline `8d14de9ca` returned
`notRead/unsupportedMask` for exact `0700h`.

The [source investigation](../research/2026-10-29-property-description-run-state-0700.md)
separates original definitions, their unresolved upper-nibble conflict and
KNXBench's conservative qualification policy. The existing
[0701/0705 contract](post-download-run-state.md) remains unchanged for those masks.
No alternative object discovery, memory decoder/fallback, broader programming
family admission, authorization, write, restart, polling, application-level retry,
new wait policy, Secure or DIAG-03 work is admitted.

## Codec and named-description primitive

Add Property Description Read/Response APCIs `0x3D8` / `0x3D9` to the existing
application-service codec. In `decode_management` coordinates, `extra` excludes
both TPCI/APCI header octets:

- Request payload: exactly three octets, object index, PID, property index.
- Response payload: exactly seven octets: those three identifiers, then four
  descriptor octets. The complete application APDU is nine octets; the suffix
  after its first header octet is eight. These are different coordinates.
- Any other request/response payload width remains `Other` with its raw bytes
  intact, not a guessed partial typed response.

Use one canonical representation of the four raw descriptor octets, with derived
getters: write-enable bit 7, legacy bit 6, lower-six-bit PDT, raw upper maximum-
elements nibble, lower-twelve-bit element count, and read/write access nibbles.
Raw decoding is total and lossless. Do not retain independently mutable raw and
decoded copies. A parts constructor is unnecessary; if provided for a real
consumer, it must reject out-of-range parts rather than mask or clamp them.

The named API accepts an existing typed object index and a nonzero PID. It sends
request property index 0 as a deterministic implementation choice, **not a
normative requirement** for named PID requests. Correlate exact object/PID, not
the response property index; return that response index without interpreting it.
A nonzero actual or echoed index is valid. Wrong object, wrong PID, PID zero,
wrong width and unrelated responses do not correlate. If only those or silence
remain, retain `SessionError::NoAnswer`. Do not invent a PID-zero error protocol.
Use the existing exchange/session timing, ACK and TPCI retransmission behavior.
One semantic API call does not promise one wire transmission after a lost ACK.

Codec-only vectors such as `[03 D9] + payload7` are **not** valid connected peer
responses by themselves. A management-session peer uses the existing correctly
sequenced numbered-data transport, whose first octet combines the peer's send
sequence with the APCI bits. Do not inject unnumbered data or blindly copy an
arbitrary request sequence as a connected response.

## Descriptor qualification, in precedence order

1. Raw upper maximum-elements nibble nonzero: `unqualifiedElements`, regardless
   of lower count, type or other fields. Preserve the raw value; do not decode an
   exponent or call the response normatively malformed.
2. Upper nibble zero and lower count zero:
   - write-enable false, PDT zero and access byte zero: `descriptionProblem`;
   - otherwise: `indeterminateMetadata`.
3. Upper nibble zero, lower count at least one and PDT 1..63:
   `unsupportedDatatype`, preserving that PDT number only.
4. Upper nibble zero, lower count at least one and PDT 0: qualified.

`PDT_CONTROL = 0` is valid; zero is not an undefined datatype. Legacy bit 6 is
preserved but never evaluated in any branch. Thus `40 00 00 00` and
`00 00 00 00` have the same description-problem classification. A zero semantic
descriptor reports an unspecified description problem, **not proof of absence,
an authorization cause or faulty hardware**. Mixed zero-count metadata likewise
has no invented normative error meaning. Access and write-enable metadata do
not establish current permission or trigger authorization or writes.

Simulator/peer descriptor evidence must be independently configured. Never
synthesize PDT, maximum elements, access or write-enable from a property's
presence, one-byte value, successful value read or mask. A one-byte PID 6 value
without independent descriptor evidence remains unavailable/unqualified.

## Observer schedule and outcomes

Retain the current family/plan-identity gate, one existing fresh read-only
observer session, identity observations and load-state reads. Every required
load read must succeed; successful values may differ from the plan or from
Loaded. Preserve that differing load verdict and continue observation, as today.
A failed load read prevents the candidate/description continuation and retains
`loadReadFailed` for supported observation masks.

| Mask | Continuation after the existing gates |
| --- | --- |
| `0701h`, `0705h` | Existing strict PID 1 → PID 13 → PID 6, byte-for-byte unchanged |
| exact `0700h` | Strict PID 1 → PID 13 → named PID 6 description → qualified only: strict PID 6 value |
| remaining `070nh` | Existing `unsupportedMask`, no added continuation traffic |
| other families | Existing pre-session unsupported outcome |

Candidate index 3 remains a bounded existing interpretation, **not a universal
layout** or a conversion of object-type code 3 into an index. Require PID 1
count 1 / exactly `00 03`, then PID 13 count 1 / exactly the plan's five application
identity octets. Candidate mismatch or unavailable reads stop before description.
No second index, scan or alternate path is tried.

A qualified descriptor permits the existing value read: object 3, PID 6,
start 1, count 1, exactly one value octet. Values `00`–`05` keep their existing
state meanings; every other complete octet remains `undefinedValue`. This is
only the reported property of the validated plan-matching hosting object.

Add a description step and closed `notQualified` reasons listed above. For the
one description exchange: no correlated answer → `noCorrelatedAnswer`, existing
connection loss/release → `connectionLost`, other session error → `readFailed`,
all at step `description`. No PID 6 value read follows any unavailable or
non-qualified description. Preserve earlier observations, transfer/restart,
settlement, cleanup, and existing error categories; add no retry, reconnect or
additional observer session.

## Shared record and compatibility

The current matching record does not carry observed mask. Add optional,
non-null `runStateMask: "0700"` adjacent to the existing matching record's
`runState`. It is reported provenance, not cryptographic authenticity or
physical-exemplar proof.

- New exact-0700 matching producers emit it, including load-read failure or
  candidate failure before description; it does not assert a description ran.
- Actual 0701/0705 producers omit it, preserving their serialized bytes and
  schedules. Legacy records remain readable and reserialize without inference.
- The only admitted marker value is exactly `"0700"`; reject explicit null,
  other strings, numbers or masks. Presence requires `runState` and forbids
  `notRead/unsupportedMask`.
- New `notQualified` reasons and `unavailable` at step `description` require
  that marker. Without it, reject the new vocabulary rather than infer a mask.
- `unsupportedDatatype` alone requires integer `pdt` in **1..63**; reject zero,
  64, fractions, strings, null, missing PDT and PDT on any other reason. Do not
  persist raw descriptors, access bits, response index or arbitrary error text.
- Existing object-index, load-success, nested runtime/state and exact-key
  invariants remain effective. Top-level runtime stays unknown unless the
  nested property was actually observed, and must agree with it exactly.
- A parser cannot know a fixture's origin. A legacy-shaped observed record with
  an otherwise valid 0700 marker is indistinguishable from a legitimate 0700
  record; enforce actual producer parity and on-wire invariants, not labels.

Keep the existing report format with explicitly extended vocabulary. New
readers admit old records; **older strict readers may reject new 0700 records**.
No migration, rewriting of history, downgrade or forward-read compatibility is
claimed. Do not reinterpret an old unsupported-mask record as a new observation.

## Consumers and acceptance

Carry the actual new result through Net presentation, Rust shared admission and
formatting, CLI output, HTTP finished status, durable activity-history reopen,
report/tester JSON, web strict parsing/rendering and EN/DE messages. A protocol
API without these consumers is not completion. Describe an optional property or
unavailable description, not a missing device, failed transfer, healthy whole
installation or successful recovery. Keep 0701/0705 wording semantics intact.

Use vertical test-first increments. New API-absence/compiler/setup failures are
not semantic RED. Codec/session primitives may first gain compiling interfaces,
then their own executed semantic RED/GREEN. Before changing the observer itself,
execute a real high-level transfer/check witness with independent descriptor
metadata, valid candidate/load reads and PID6=`01`: expected reported Running,
actual original Unknown/unsupportedMask. Do not emit the new marker beside the
old unsupported-mask result merely to make a reader reject it. The old 0700
no-continuation characterization is explicitly superseded by this new bounded
capability; preserve original evidence and all unaffected-mask assertions.

Acceptance must include:

- Independently derived request/response vectors, lossless roundtrip, widths
  other than 3/7 retained as Other, reserved/legacy bits and nonzero response
  indices; real connected encoded-transport correlation and wrong-only/later-
  correct replies, not only a typed simulator shortcut.
- Every qualification branch, both legacy-bit values, independently configured
  descriptor evidence, and the discriminating one-byte-value/no-description
  negative case. All six states and undefined `06`, `80`, `81`, `FF`.
- Exact schedule and zero forbidden later reads after each refusal, candidate
  mismatch, load failure or description error. Successful differing loads still
  permit observation. No new authorize/write/restart/scan/fallback/session.
- Actual 0701/0705 traffic and serialized-byte parity; unchanged 0702/070F and
  other-family outcomes. Existing TPCI retransmission remains unchanged.
- Strict new/legacy Rust and web records, null/missing/extra fields, impossible
  PDT values, wrong indices and runtime/load disagreements; durable reopen,
  report/CLI/HTTP consumers and actual EN/DE rendering. Browser success does
  not replace owning fixture-tsconfig typechecks.
- Independent whole-feature/correction review, maintained branch/main-first
  gates, actual integrated build and isolated server smoke before publication.

Only offline, simulator and loopback software work is authorized. No hardware,
whole-device runtime/function, full mutation/recovery or overall PRG-01 closure
follows from this increment.
