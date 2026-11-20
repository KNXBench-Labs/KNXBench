# ADR 0180: Exact `0702h` joins the descriptor-qualified run-state observation

Date: 2026-11-20
Status: Accepted
Session: PRG-01-N2

## Context

The accepted post-download run-state observation covers exact masks `0701h`
and `0705h` (unqualified PID 6 read) plus descriptor-qualified exact `0700h`
([contract](../contracts/post-download-run-state-0700.md)). PRG-01's open list
names broader runtime-state observation beyond those bounded contracts as the
next gap. The `070nh` family is admitted for the memory download by MP
§3.31.2's low-nibble rule, but Resources Table 7 approves only
`0700h`/`0701h`/`0705h` as BIM M112 masks, and Profiles §4.2 has no `0702h`
column. Footnote 94 of Resources §4.24.2 grants the `PDT_UNSIGNED_CHAR`
legacy exception only to masks `0020h` and `0701h` — not `0702h`. The
[original-source review](../research/2026-11-20-property-run-state-observation-0702.md)
records these facts.

## Decision

1. Exact mask `0702h` gets the same bounded observation shape as exact `0700h`:
   after the unchanged identity/load gates and strict candidate-3 binding
   (PID 1 = `0003h`, strict PID 13 = the plan's five-octet application), one
   PID 6 property-description read must qualify the optional property as
   `PDT_CONTROL` before one strict PID 6 value read. Because footnote 94 does
   not list `0702h`, this is source-backed, not merely conservative policy.
2. The shared matching record carries optional non-null
   `runStateMask: "0702"` beside `runState` for exact-`0702h` producers only;
   all existing producers keep their exact bytes. Older strict readers may
   refuse new `0702h` records; no migration or downgrade is claimed.
3. No other `070nh` mask, object layout, scan, memory fallback, session,
   retry, wait, write, restart or authorization is admitted by this decision.
   The memory-mapped run-state encoding stays
   `MISSING_MASK070NH_MEMORY_RUNSTATE_ENCODING`.

## Alternatives considered

- **Admit `0702h` like `0701h`/`0705h` without a description read.** Rejected:
  the legacy `PDT_UNSIGNED_CHAR` exception is closed by footnote 94 and does
  not cover `0702h`, and a one-byte value is never datatype evidence; the
  description-qualified path is already implemented and reviewed.
- **Admit every `070nh` mask at once.** Rejected: Table 7 approves only three
  BIM M112 masks and PROF profiles those; extending observation to unapproved
  masks would be an unverified compatibility claim. Each further mask needs
  its own bounded contract.
- **Wait for a `0702h` product in the corpus.** Rejected: the corpus census
  counts only `0701`/`0705` System 7 programs; waiting would leave the
  family-boundary question without simulator evidence, and simulator/loopback
  evidence is admitted on the declared list (owner decision 2026-11-20).

## Consequences

- The observer gains one mask branch reusing the exact-0700 qualification
  path; the shared record gains one marker value `"0702"`; Rust and web
  strict readers, CLI/server/report/history consumers and EN/DE messages
  carry the new vocabulary under the same invariants.
- Tests must prove the exact `0702h` schedule, unchanged `0700h`/`0701h`/
  `0705h` bytes, all qualification branches, failure stops, and strict
  record admission/refusal including marker/PDT rules.
- Nothing about hardware, physical function, other layouts or whole-mutation
  recovery follows; PRG-01 stays open.
