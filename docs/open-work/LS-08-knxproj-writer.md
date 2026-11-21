# LS-08 — Unsigned `.knxproj` writer (parked prototype)

**Status:** Later scope. Parked by owner decision on 21 November 2026 ("vermerken
und stehenlassen als later work"). Listed in [OPEN_WORK](../OPEN_WORK.md).
Not on `main`, not in any release, no implementation claim.

## Origin

On 14 and 15 November 2026 an owner-approved feasibility session built a
best-effort, explicitly unsigned `.knxproj` export on the local, unpublished
branch `work/knxproj-writer-20261114` (11 commits ahead of `main`, last commit
15 November 2026). Nobody has worked on it since, and it was never
reviewed, gated or merged.

## What the branch contains

- `knx-etsproj` writer (`write.rs` plus `archive`, `integrity` and `losses`
  submodules) with loss disclosure, provenance, member identity, forest
  validation and Puid emission, and roundtrip tests against the own importer.
- `knx-csv` ETS group-address XML export (`ga-export-xml`), unsigned.
- CLI command, HTTP route (`http_knxproj_export`) and a File-menu entry in the
  web UI, with browser tests.
- A branch-local ADR proposal for this export. It is numbered 0157 or 0175 on
  the branch; both numbers now belong to other ADRs on `main`, so it needs a
  fresh number.

## Verified fact recorded on the branch

The ETS 6.4.1 demo, using a fictional project, refused every archive whose
project signature was missing or no longer matched the content (message
"Projektdatei hat keine gültige Signatur"). An unsigned third-party
`.knxproj` is therefore **not** importable into ETS 6.4.1. Older ETS
versions are untested. The private evidence was not published.

## Why it is parked

- [ADR-0028](../adr/0028-no-knxproj-export.md) ("KNXBench reads `.knxproj` and
  never writes one") is still the accepted decision on `main`. Picking this up
  needs a new ADR that explicitly supersedes it.
- The branch sits on pre-rewrite history (its merge base with `main` is from
  2 September 2026), so its 11 commits must be ported onto `origin/main`
  rather than merged.
- Since ETS refuses the result, the value is limited to schema-valid exchange
  with independent readers. That needs a named consumer before the
  maintenance cost is justified.

## Before work resumes

1. Owner decides whether a `.knxproj` writer returns at all (new ADR
   superseding ADR-0028, own number on `main`).
2. Port the 11 commits onto fresh `origin/main` and resolve drift in the
   export, HTTP and File-menu surfaces.
3. Independent review and change-scoped package gate. Wording stays
   "KNX-compatible, unsigned, not ETS-importable"; never claim ETS
   compatibility.

The local branch is kept as it is. If the local checkout is ever retired,
first secure the branch as a `git bundle` in the backup.
