# PDB-11: package identity and versions (goal.md §2.8)

- **Agent:** Claude (goal.md session), implementation delegated, review
  delegated to an independent reviewer, lead verification and delivery here.
- **Delivered:** merge `7844590` on `main` (pushed).
- **Design:** [ADR-0043](../../docs/adr/0043-package-identity-is-recorded-per-candidate.md).

## What changed

Schema v17 adds four tables: `package_source_name`, `source_identity`,
`source_identity_scan` (with an `IDENTITY_SCANNER` version) and
`source_producer`, plus a `source_sha256` index on each of the six identity
tables.

- **Scan:** a streaming scan mirrors the parsers' dispatch and records a
  canonical element digest per candidate. Each digest begins with the context
  the parser stores from outside the element.
- **Ingest checks:**
  - On every ingest, the recorded candidates are checked against the parser's
    rows and conflicts; a mismatch rolls back.
  - A blob's first parse must match exactly.
- **Backfill:** the v16->v17 migration backfills every parsed blob. It checks
  historical rows and records a disagreement as `unavailable`.
- **Queries:** program families by (manufacturer, `ApplicationNumber`),
  `ReplacesVersions` links, and products by order number, all computed at
  query time.
- **CLI:** `knx products identity|family|order-number`.
- **Winner rule:** unchanged. The first installed row still wins; the
  identity query shows every candidate and whether it matches the winner.

## Review record

The independent review accepted with follow-ups and raised no CRITICAL
finding.

**IMPORTANT, I-1:** the same element bytes under another parent were reported
as the same element, although the stored `section_id`/`hardware_id` differed.
Fixed with the context token.

**MINOR findings:**
- M-1: exact rule on a blob's first parse.
- M-2: backfill agreement check.
- M-3: wording.
- M-4: indexes.
- M-5: scanner version.
- M-6: a measured winner without a candidate is named.
- M-7: evidence recorded.
- M-8: `InstallReport.source_names` accepted unchanged; nothing outside the
  crate constructs it.

## Evidence

Gates on `968c3c3`, run with a fresh `CARGO_TARGET_DIR`:

- fmt and clippy `-D warnings`.
- Workspace tests: 2151 passed, 0 failed, 115 ignored.
- headers, anchors, layering, corpus-gates and diff-check.

Private Gira+MDT matrix: pass. Aggregates and the baseline commitment are
unchanged by the review fixes; the v16 projection equals the PDB-10 pin.

Mutation sweeps:
- 23 of 23 killed on the original guards.
- 12 of 13 killed on the review fixes. Two survivors exposed test gaps, which
  were closed with new tests.
- The remaining one (the no-parent flag) is equivalent.

## Lessons

- A parser-mirroring scan must mirror not only which elements are dispatched
  but also the context a parser carries into the stored row. Otherwise
  "same digest" overclaims.
- A subset agreement check cannot detect an invented candidate. On a first
  parse the parser's decisions are complete, so an exact check is possible
  and cheap.
