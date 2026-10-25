# Session device comparison (owner P4)

Implementation scope: [ADR-0130](../adr/0130-session-device-comparison.md).
Package 1 implements the table/batch surface; package 2 captures actual image
parameter bit attribution and adds named values. Acceptance and publication remain separate
in [implementation status](../IMPLEMENTATION_STATUS.md).

## Scope, admission and cancellation

- Devices offers Compare on a row and Compare all shown over its **current
  filtered/sorted rows**, not checkbox selection, hidden rows or another line.
  The review freezes stable IDs, addresses and per-device P3 content revisions.
- Exactly one explicit gateway review starts the queue. No comparison tunnel
  is opened by mounting, filtering, opening review or viewing existing results.
- Each request uses existing gateway guards and a read-only management session.
  No phrase, key or write authorisation is admitted. A monitor-owned tunnel
  returns structured `gatewayBusy`; this is a run-admission refusal, not a
  device outcome. Device failures continue to the next target.
- Cancel stops after the current bounded operation disconnects, never by
  abandoning tunnel ownership or starting a second read. Unvisited targets
  acquire no result; earlier local Not comparable outcomes remain visible.
- Server/project incarnation, ID, address and content revision must match.
  Stale preparation is rejected before connecting; late scope-mismatched
  responses cannot give another device/project a sync claim.

## Tags

Only the current browser/project session owns tags; reload/reopen has no tags
until an explicit attempt. They never mutate loaded flags or project files.

| Tag | Evidence |
| --- | --- |
| Synced | Verified matching application, all plan bytes equal, compared parts Loaded |
| Different | Byte differences or a compared part not Loaded |
| Outdated | Relevant P3 content revision changed after the reading; also changes during the read |
| Wrong application | Complete five-byte application identity differs |
| No answer | Typed session silence/released-connection outcome; not a guessed offline diagnosis |
| Not comparable | Grey, with reason; missing product/plan/address, unsupported or unproven identity/shape, other named refusal |

P3 content semantics, not loaded booleans, decide edit invalidation. Undo/redo
does not restore old evidence. Manual marks, names and DPT labels alone do not
make the memory comparison outdated. A comparison is an observation, not a
subscription: another tool can change a real device without notifying KNXBench.
A gateway refusal does not erase earlier results or fabricate new live-device
tags. A non-contactable row attempted before that refusal can retain its honest
local Not comparable outcome.

## Decoding and integrity

- Application identity uses the existing 070nh partial-download identity path:
  task record or consistent property precondition, then Application Program
  Object PID_PROGRAM_VERSION, exactly five bytes. Conflicting witnesses and
  short/malformed responses are unknown, not guessed identity mismatches.
- Package 1 decodes the verified 0701 group-address table (big-endian words,
  count includes individual-address entry) and association table (TSAP/ASAP).
  Device associations resolve through the **device** address table, not the
  project's. Entry order and unmapped/unread values remain explicit.
- Decoding requires a declared placement/capacity and is bounded by its segment.
  No byte beyond that range is attributed as an entry. Missing compared bytes,
  invalid counts, TSAP zero/out-of-range and incomplete fields are unknown with
  a reason, never silently zero, empty or read from neighboring parameter data.
- Every original read range is also listed as project/device raw bytes, never
  in a collapsed disclosure. No filter may hide a differing or unread byte, and
  the *Show equal rows* toggle always reveals every original read range (see
  [Result display](#result-display-wp-18-owner-amendment-19-october-2026)).
  Unwritten/masked bytes are outside the read scope, even when the image has a
  desired value there. Another mask is raw-only, not a borrowed table codec.
- Parameter attribution is captured only after an actual successful active image
  write, including the winning union field. Each row carries the source name
  (parameter ID fallback), reference and exact segment/offset/bit/width; no
  ownership is inferred from differing bytes or inactive references.
- Named numeric values use the existing MSB-first writer's bit order, excluding
  neighboring bits even across byte boundaries. Text is the exact inverse of
  the existing ASCII/declared Latin-1/Latin-9 writer, with zero padding only.
  The project value must still match the final plan; later layout overwrites,
  missing/masked bytes, unsupported encodings and negative signed patterns
  remain Unknown with an EN/DE reason. Negative signed project values remain
  refused by existing image admission, not newly encoded here.
- Parameter decoding needs matching supported 070nh masks, exact field coverage
  and the declared segment bound. Every raw range remains visible even when a
  named row is unknown. Manufacturer names, stable references and technical bit
  metadata are not invented translations. No schema migration, new device type
  admission, enum/DPT unit inference or full ETS/device compatibility claim.

## Result display (WP-18, owner amendment 19 October 2026)

The owner decision of 19 October 2026
([WP-18](../history/OPEN_WORK_2026-10-25.md#wp-18--device-comparison-shows-what-differs))
supersedes the earlier "no differences-only filter" sentence. Display only:
the response, the session tags and the Synced claim are unchanged, and the
filter never edits or reorders the response rows.

- **Rule:** no filter may hide a differing or unread byte, and the toggle
  always reveals every original read range.
- The result opens with only differing rows. Hidden by default are only rows
  proven equal: a known decoded kind (address/association table entries and
  counts, named parameters) whose two sides are known, equal and carry no
  reason, and a raw range whose bytes were all read on both sides, parse as
  bytes, have equal length and are all equal. Unknown or not-read rows,
  not-decoded rows, rows with a reason, unknown row kinds and any raw range
  with a differing or unread byte stay visible.
- A *Show equal rows (N)* checkbox shows every row in response order; while
  rows are hidden a note states how many. When every row is proven equal, the
  table says so instead of appearing silently empty.
- Raw rows mark each byte on both sides. A byte is *differing* when the two
  displayed sides differ at that position (equal-length byte comparison of the
  server's hex text) **or** when the server's byte-level `changes` (run
  address plus project and device bytes, computed per plan write) cover its
  address. The two server sources are computed separately (the raw row from an
  address map of all plan writes, `changes` per write step); where they
  disagree the byte is marked differing, never equal. A byte without a known
  counterpart (side unknown, unequal length, malformed token) is *not read or
  not comparable*, marked differently from a differing byte. No KNX codec is
  added in the UI.
- Marks do not rely on colour: differing bytes are bold with a solid
  underline, unread bytes italic with a dotted underline, both carry a
  tooltip, and each raw row counts its differing and unread bytes in text.
- The shared table also serves the device-check comparison
  (`DeviceInspectionPanel`). Its response is the same `/api/device-compare`
  shape, so it passes the same byte-level `changes` and follows the same
  differences-only rule. Its separate *Differing ranges* table is unchanged.

## Device → project adoption (Alpha 16 amendment, 23 October 2026)

The separate [atomic adoption contract](alpha16-comparison-adoption.md) admits
only service-held, identity-checked observations and exact attributed named
parameters in this increment. Requested unsupported/raw/unknown/table rows
refuse the whole selection; unselected unknown rows remain visible and intact.
Incarnation, P3 revision, complete project/history and rebuilt source authority
are checked again before one undoable edit. Comparison itself still cannot
write, and project → device reuses the existing validated download workflow.
Its [association amendment](alpha16-comparison-adoption.md#association-link-adoption-wp-19-amendment-24-october-2026)
(24 October 2026) adds object ↔ group-address link adoption from fully decoded
`0701h` association tables with unambiguous project GA/object mapping; address
and count rows stay unsupported and nothing is created.

## Required evidence

Named RED/GREEN for identity, P3 revisions, raw/decoded tables, server binding,
queue/control semantics; targeted restored-source mutants; separate in-session
self-review, EN/DE production screenshots 1440/400; complete branch and actual
fresh-main merged template gates (RUST=1, fifteen EXTRA_CONFIGS); publication
readback and private source-bound evidence. No real bus access is authorized.
