# Download preparation and settlement receipt

Run-scoped display evidence inside the durable commissioning activity history
([history contract](COMMISSIONING_ACTIVITY_HISTORY.md)). Change this file in the
same commit as the behaviour it describes. Implementation and bounded source
reviews passed the integrated software gate; the [acceptance record](../status/2026-10-25-download-settlement-receipt.md)
distinguishes runtime acceptance, final documentation integration and remaining limits.

## What it is, and what it is not

Two optional objects on a regular `deviceDownload` row's `downloadEvidence`:

- `preparation`: what the run was planned to do.
- `settlement`: how the project's programming belief was settled when the run
  ended ([source freshness](download-settlement-source-freshness.md)).

It is **not** a per-device baseline, a delta or restore authority, an observed
device prestate or flag provenance, and it does not cover the whole project or
every download path. It changes no project flag, no schema, no write
admission and no bus behaviour. CLI and System2 downloads record neither
object.

## Shape

Each object carries its own `format: 1`, the integer 1 exactly. The stored
page format stays 2.

`preparation`, one of:

- `{format, state: "available", scope, program, plannedMask, plannedManufacturer, steps, dataOctets}`
- `{format, state: "unavailable", scope}`

`scope` is always kept: `complete`, `partial-parameters`,
`partial-group-addresses` or `partial-both`. `program` is
`{state: "identified", id, applicationVersion (string or null), sourceSha256}`
or `{state: "unavailable"}`. `id` must equal the request's program id, be 1–128
and `applicationVersion` 1–32 ASCII graphic characters without `"` and `\`, and
`sourceSha256` 64 lowercase hex characters; anything else stores `unavailable`
and never refuses the run or changes admission. The identity is read in the
same catalogue lease that built the request, image and plan, not in the later
lease of the download report. Mask and manufacturer are *planned* four-digit
upper-case hex values, not read from the device. `steps` and `dataOctets` are
`u32`; a value beyond that stores the `unavailable` form with the scope. A
partial scope that selects nothing cannot occur (the plan is refused); such a
plan would simply start without a receipt.

`settlement`, keyed by `project`, only on `finished` or `failed` rows whose
`written` is not null, and only together with a `preparation`:

- `notAttempted`: exactly when `written` is `no`.
- `noProject`, `otherProject`, `deviceMissing`, `unavailable`: after a write,
  no parts, no time. "Project" means the project open at the start of the run;
  its identity is not stored.
- `persisted`, `sessionOnly`: `source`, `currentRequest`, `applied`
  (`programmed`, `pending`, `touched`, each exactly the four booleans
  `address`, `application`, `parameters`, `groupAddresses`) and `settledAt`
  (the instant the settlement was applied, canonical UTC with milliseconds and
  `Z`, 24 characters). `sessionOnly` means the project has no file to write;
  it is not a durable save.
- `persistenceFailed`: `source`, `currentRequest` and `computed` (same parts),
  no time. Computed but **not applied**: flags, undo stack and revision are left
  as they were.

`source` is `unchanged`, `changedOrUnknown` or `notChecked`. `unchanged` means
the same request planned again yields the plan that ran; a replaced product file
can plan the same bytes, so it does not say the file is the same and no source
hash is taken at settlement. `notChecked` is the only value for a run that did
not complete. `currentRequest` says whether the project still yielded a request
for the device at the end.

## Invariants (both readers refuse the whole page otherwise)

- `programmed` and `pending` are disjoint and together equal `touched`.
- A completed run (`finished`, `written: yes`): `touched` is all four parts for a
  complete scope, exactly the scope's memory parts for a partial one;
  `changedOrUnknown` credits at most the address; an unavailable request credits
  nothing.
- An interrupted run: `source: notChecked`, nothing programmed, `touched`
  within the scope's memory parts.
- Unknown format, explicit `null`, unknown keys or enum values, a settlement on a
  `running` or `unknown` row or without a preparation, a bad time, hash or id:
  the page fails and the stored bytes are untouched.

Absence of either object means "no receipt recorded by this history". It is not
"imported", not "unchanged" and not "no change".

## Order, failure and downgrade

The row is written at start (with the preparation), then the result, then the
settlement, then the read-only check, then cleanup. The project store and the
history store are two databases and the sequence is **not atomic**: a crash
after the project persisted but before the settlement is recorded leaves the
flags changed and the receipt absent; a latched history failure never gains a
later rewrite. Recording failures never change the transfer result, the check or
the cleanup. A run whose task ends without a result leaves the preparation only.

The outer format stays 2 following the `postDownloadCheck` precedent. A build
that does not know these objects refuses a page containing them and latches
its history unavailable for that process; it may have written before it
encountered such a row, so this is not a categorical block on opening. Nothing
is rewritten or dropped. Read such a history with this or a newer build.

## Size

The executed Rust size regression enumerates valid receipt/row combinations and
selected four-machine post-check variants. Its largest enumerated row was 1950
bytes against the 4096-byte ceiling; this is not a claim that every possible
post-check variant was enumerated. Fixed field bounds and the finite four-machine
vocabulary constrain the full record. Id, version and hash bounds above keep every
allowed source-text byte one serialized byte.

## Display

The history view shows the receipt per row under the existing heading, in English and German:
planned values labelled as planned, the plan verdict as plan-only, applied
versus *computed, not applied*, memory-only versus saved, and the post-download
check as a separate section. It offers no write or recovery action.
