# Session device comparison (owner P4)

Implementation scope: [ADR-0130](../adr/0130-session-device-comparison.md).
Package 1 implements the table/batch surface; package 2 (parameter bit attribution)
is not included in package 1. Acceptance and publication are recorded separately
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
- Every original read range is also listed as project/device raw bytes. No
  differences-only filter or collapsed raw disclosure hides unattributed bytes.
  Unwritten/masked bytes are outside the read scope, even when the image has a
  desired value there. Another mask is raw-only, not a borrowed table codec.
- Parameter name/bit/value decoding is **pending package 2**. Package 1 shows
  those bytes raw. Existing image admission and unsupported-type limitations
  are unchanged; this is not full ETS/device compatibility.

## Required evidence

Named RED/GREEN for identity, P3 revisions, raw/decoded tables, server binding,
queue/control semantics; targeted restored-source mutants; separate in-session
self-review, EN/DE production screenshots 1440/400; complete branch and actual
fresh-main merged template gates (RUST=1, fifteen EXTRA_CONFIGS); publication
readback and private source-bound evidence. No real bus access is authorized.
