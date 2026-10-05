# ADR 0081: `Parameter` and `ParameterRef` report every attribute they do not store

Date: 2026-10-05
Status: Accepted — KL-156 package; the evidence is listed below.
Session: 4 (manufacturer semantics), alpha follow-up to AR07
Amends: none. Applies the static parser's existing rule (an attribute that
reaches no column is reported, e.g. `ComObjectTable`/`ModuleDef`,
KNOWN_LIMITATIONS §7) to the two elements that skipped it.

## Context

The static program parser stores a fixed attribute set of `Parameter` (`Id`,
`Name`, `Text`, `ParameterType`, `Access`, `Value`, `Suffix`) and of
`ParameterRef` (`Id`, `RefId`, `DisplayOrder`, `Tag`, `Text`, `Value`; `Access`
since [ADR-0080](0080-parameter-write-authority.md)). Unlike `ComObject`,
`ComObjectRef` or `ApplicationProgram`, these two arms never called
`report_unknown_attrs`, so everything else on them silently stayed out of
`ingest_unknown` and the install report (KNOWN_LIMITATIONS §156, found by an
AR07 probe). The bytes stayed retained, so nothing was lost, but the report
under-stated what was not interpreted.

Corpus census (AR07, 332 `OriginalData` programs, names only): `Parameter` —
`SuffixText`, `DefaultUnionParameter`, `InitialValue`, `LegacyPatchAlways`,
`BaseValue`, `InternalDescription`, and a union member's own
`Offset`/`BitOffset`; `ParameterRef` — `Name`, `SuffixText`, `InitialValue`,
`InternalDescription`, `ForbidGrantingUseByCustomer`.

## Decision

1. **Report, do not store.** Both arms report every attribute outside their
   stored set, unconditionally (also for a program another blob won, as the
   `ComObject` arm does), through one helper shared with the backfill. No new
   column: interpreting `InitialValue`, `SuffixText` or union placement is
   separate work. A union member's `Offset`/`BitOffset` *is* read later by the
   download image builder from the retained bytes; it is still reported,
   because the report describes what the product database stored.
2. **Schema v21 re-derives existing databases** (no DDL). Per stored blob
   that classifies as an `ApplicationProgram` and whose bytes match their
   key, the same walk plus the same scheme-evidence reconciliation that
   ingest runs adds the rows the blob lacks, once per package member that
   re-parsed it or once for a standalone blob (the v16 rule; a blob both
   ingested standalone and carried by a package gets one copy too few,
   accepted as in v16).
3. **Install reports are re-derived, not left historical.** v15 kept reports
   historical (ADR-0041); v16 and v18 rewrote them. This follows v16/v18: the
   rows are pure functions of the retained bytes and the old report
   under-states what the parser met, which is the integrity defect being
   fixed. Each measured report gains its members' new rows merged as install
   merges them, and `package.unknown_count` the distinct rows per program
   member. A key already present changes nothing, so the migration is
   idempotent on a database a v21 parser wrote.
4. **Fail visibly.** A blob that no longer re-reads gets a
   `ParameterAttributeBackfillError`; any report counting it is downgraded to
   `unavailable` with an `InstallReportBackfillError`, never an invented row.

## Consequences

- Unknown-row totals rise for every corpus that uses these attributes; the
  corpus matrix commitment is re-pinned with the measured delta.
- The parameter panel is unchanged; `SuffixText` display remains the
  product-language gap described in KNOWN_LIMITATIONS.

## Evidence

`crates/knx-productdb/tests/parameter_attribute_unknowns.rs` (install,
standalone ingest, genuine v20 rewind, fresh-install reproduction,
idempotence, damaged blob), the full workspace gate, and a corpus probe that
installs every `OriginalData` product package, rewinds to v20, migrates and
compares all unknown/report tables with the fresh install.
