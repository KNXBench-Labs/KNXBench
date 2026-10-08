# ADR 0076: One ledger is the status of record for tracked source IDs

Date: 2026-10-04
Status: Accepted
Session: docs consolidation (alpha-release-goal AR14D D2)

## Context

The alpha queue tracks 180 source IDs from the frozen
[OFFENE_PUNKTE](https://github.com/KNXBench-Labs/KNXBench/blob/6a1ba6ae5d54/docs/archive/OFFENE_PUNKTE.md) inventory plus post-snapshot
findings (`KL-149` to `KL-153`). By 2026-10-04 their status was written in up
to six places: the routing table in `alpha-release-goal.md` §7, two owner
checkpoint tables in the same section, the post-snapshot table in §8, the
per-ID ledger and post-snapshot table in `ALPHA_READINESS.md`, the
disposition column of `COMMISSIONING_ALPHA_LEDGER.md` and the disposition
prose of `UI_ALPHA_READINESS.md`.

The copies had drifted, measured by script on 2026-10-04:

- 23 of 180 rows differed between `alpha-release-goal.md` §7 and
  `ALPHA_READINESS`. 21 were owner updates written into one file only; two
  (`DATA-02`, `KL-42`) still said `TODO` in §7 although package AR04 is
  `DONE` and published as `216c673e`.
- `KL-149`, `KL-150` and `KL-152` were `TODO` in both post-snapshot tables
  although their AR06P checklist items are ticked and their code is
  published (`362fec24`, `1b215d51`, `2b2a267f`).
- The mechanically counted status line in `ALPHA_READINESS` said `TODO=54`;
  its own table had 30.

Every session that changed a status had to find and edit all copies, and none
of them could tell which copy was current.

## Decision

[`docs/status/LEDGER.md`](../status/LEDGER.md) is the only place that records
the status of a tracked source ID. It holds one row per ID with: ID, priority,
owner, route, status, owner disposition, and evidence with the remaining work.

- **Status** uses the vocabulary of `alpha-release-goal.md` §2.2: `TODO`,
  `IN_PROGRESS`, `DONE`, `BLOCKED_EXTERNAL`, `WAITING_OWNER`,
  `WAITING_DECISION`, `ACCEPTED_BOUNDARY`, `LATER`. It is the release-level
  status the alpha decision reads.
- **Owner disposition** carries an owner track's own finer word where that
  track defines one (the commissioning vocabulary such as `BLOCKED_HARDWARE`
  or `VERIFIED_SCOPE`), otherwise `—`. It lives in the same row, so it cannot
  drift from the status.
- **Who edits which row:** the owner named in the row changes its status,
  owner disposition and evidence. The alpha controller changes owner and
  route. A new finding gets a new row in the post-snapshot table with its
  origin; the 180 snapshot rows are never added to or removed.
- **Evidence documents keep evidence, not status.** `ALPHA_READINESS`,
  `COMMISSIONING_ALPHA_LEDGER` and `UI_ALPHA_READINESS` keep their dossiers,
  evidence, fallback and unblock text. They carry no per-ID status or
  disposition column and link to the ledger instead.
- **Earlier checkpoint tables** that carried statuses are moved verbatim into
  the ledger file as dated history and marked superseded. Nobody updates them.

A repository check (`cargo run -p xtask -- check-ledger`, AR14D D3) enforces the format: unique IDs, only
the vocabulary above, exactly 180 snapshot rows, and no per-ID status table
elsewhere in `docs/` or the goal files.

## Alternatives considered

- **Keep the copies and add a consistency check.** It would detect drift but
  keep the work of editing every copy, and it would still not say which copy
  wins.
- **A machine-readable file (TSV/TOML) with generated Markdown views.** It is
  easier to parse, but every reader of the project would then read a generated
  file, and a generator is one more tool to maintain. A single Markdown table
  is readable as is and simple to parse with fixed columns.
- **Keep the ledger inside `alpha-release-goal.md`.** The goal file is a work
  plan of over 1,000 lines that several sessions edit at once. A separate file
  keeps status edits out of plan edits and makes the ledger easy to find.
- **Fold the LIMITATION_TRIAGE criticality (K1–K4) into the ledger.** It rates
  a different set (all numbered limitations, not only alpha IDs) on a different
  axis than the inventory priority (P0–P3). It is not a status, so it stays a
  separate view.

## Consequences

- One edit changes a status. A status in any other file is either history or
  an error the check reports.
- Track evidence stays with its owner, so the dossiers keep their depth while
  the ledger stays a table.
- The goal files and track readiness documents must link to the ledger
  instead of restating statuses. Where those files belong to another session
  (`goal-ui.md`, `goal-commission.md`), their text changes only with that
  owner's agreement (AR14D D5).
- Package-level status lines (for example AR06 `DONE_SCOPED`) stay in the
  goal file. They describe a package, not an ID; a package delivery without a
  per-ID disposition leaves its rows as they are until the owner maps them.
