# Product-column promotion repair

Status: implemented and source-delivered as `8fa0d76b9fd41f81c4e135c29e54f2a98d516b38` on
20 October 2026 after accepted final integration and publication readback.
[Acceptance record](../IMPLEMENTATION_STATUS.md#product-column-promotion-repair--source-delivery).
Owner-authorised sequence: [Alpha 16 successor
work](../goals/alpha16-successor-sequence-2026-10-19.md).

## Problem and evidence boundary

The [WP-02 follow-up](../OPEN_WORK.md#wp-02--suffixtext-end-to-end) records two
classes of discrepancies introduced by the product database's v23–v25 column
promotions: a later declaration of one program id can overwrite values from the
winning first declaration, and overly broad host-name retirement can remove
unsupported evidence which fresh ingest still reports.

Static inspection of the released v26 implementation confirms:

- `migration::backfill_text_parameter_refs` does not track the first program
  declaration and does not skip the static parser's excluded `Dynamic` subtree;
- `migration::backfill_placement_columns` skips `Dynamic`, but does not retain
  first-declaration identity;
- `Retired::retires` matches host suffixes, while promoted attributes depend on
  parser context as well as the last element name. The v23 and v24 dynamic
  hosts are legitimate promotions; excluding every dynamic path is not a fix;
- `backfill_suffix_text` already demonstrates first-declaration selection and
  the static/dynamic boundary for v26's own suffix promotion only.

The independent runtime pass retained 24 passing existing tests and eight
intended semantic failures against unchanged production. A separate read-only
review accepted the bounded test preparation. Neither establishes a repair;
already-upgraded fixtures and the wider acceptance cases below remain required.

## Derived values

1. Append a corrective migration after WP-17's delivered product schema version;
   editing old steps alone cannot repair databases which already passed them.
2. Hash-check every retained source used for reconstruction and preserve its
   `application_program.source_sha256` ownership. Cross-file losing programs do
   not become owners. Within a source, the first declaration remains the winner,
   including its absent attributes and empty identifiers where ingest admits
   them. Match ingest's static/dynamic and module traversal rules.
3. Repair only the named promoted columns: static communication-object-reference
   text references, parameter own/property placement and enumeration presentation
   fields. A wrong non-NULL value must be replaceable; when the winning source
   omitted it, a contaminated value must become NULL. This narrow correction
   needs an explicit ADR amendment before implementation; it does not relax
   unrelated migration ownership or absent-slot rules.
4. No new manufacturer behavior, device admission, parameter writes, native
   project migration or bus route belongs to this repair.

## Unsupported evidence

Reconstruct the targeted constructs through parser-owned evidence rules, keeping
occurrences, first sample, source identity, module paths and qualified-name/scheme
handling. Restore static `ParameterBlock` text-reference/access attributes,
dynamic `ComObjectRef` text references and dynamic `Parameter` offsets where the
current ingest reports them. Preserve unrelated unknowns, conflicts, diagnostics
and retained source bytes. A blanket delete-and-rebuild of the catalogue is not
permitted.

Per-blob unknown evidence is not the same as a package report: shared sources
must be repaired for every affected package even after their global ledger row
has been restored. `backfill_parameter_attribute_unknowns` alone is insufficient:
it excludes `Dynamic` and returns only additions missing from the global ledger.

## Historical reports and counters

- Reconcile targeted unknown evidence only for existing measured reports, using
  original package-member order for aggregation and first samples. Do not infer
  historical encounters, deduplications, writes or translation contributions
  from today's normalized tables.
- Preserve non-unknown report categories, diagnostics, dropped-DPT counters,
  source inventories and normalized winners. Missing reports stay missing;
  unavailable reports stay unavailable. Source replay cannot make unavailable
  history measured.
- Package counters count distinct constructs per member; report rows merge
  matching keys across members. Neither is a global-ledger row-count delta.
  An absent or partial shared aggregate does not identify which program's
  counter units earlier retirement removed. Preserve that package's original
  counter, record ambiguous per-member history and make its report unavailable;
  retained XML cannot manufacture historical member attribution. Complete shared
  aggregates and unambiguous single-member cases remain eligible for repair.
- Dispatch by the stored member role and path, not SHA alone: opaque baggage
  never inherits semantic evidence from a byte-identical program. Authenticate
  semantic non-program target ledgers against the retained source's parser
  result before using them; missing or invented rows require unavailable
  reports rather than rewriting measured counts or samples.
- Admit lexical/evidence work before semantic parsing and owned evidence copies.
  The repair's fixed 16 MiB allowance is a conservative working-set policy,
  not a KNX format limit or a process-RSS guarantee. Commit retained cross-source
  budget consumption only after a source succeeds, matching its savepoint.
  A failed source must not spend the allowance of later healthy sources.
  A repair failure without a source measurement marker is a public diagnostic
  reader error, never absent evidence or invented measured parse statistics.
- Validate before and after reconciliation. If truthful reconstruction fails,
  retain source evidence and derived rows, record an explicit failure and make
  the affected measured report unavailable through the established mechanism.
  A malformed report must not be silently promoted to valid history.

## Atomicity and acceptance

Use a per-source savepoint for reconstructable failures and the migration's
outer transaction for SQLite errors. Do not erase existing evidence on a failed
source repair. Unreadable, corrupt, no-longer-classified or missing source data
must have an explicit conservative disposition, not guessed values. Legacy
EX-IM payloads are not XML and need their existing exclusion.

Acceptance requires:

1. Named RED assertions against unchanged production for each defective
   earlier step, with genuine earlier schema and matching evidence/report setup.
2. Already-upgraded fixtures containing the released steps' wrong non-NULL
   values and deleted rows; a test that only exercises rewound old steps is
   insufficient.
3. Fresh/migrated equivalence for recoverable fixtures, including exact values,
   evidence counts/samples and preserved report categories/diagnostics.
4. Shared-source packages, cross-file losers, winner-absent attributes, repeated
   occurrences, module/qualified paths, legacy exclusion and source corruption.
5. Missing/unavailable history preservation, rollback on SQL failure and a
   no-change reopen. Failure cases are judged by preservation and explicit
   diagnostics, not by an impossible fresh-install equality.
6. Independent review and the maintained change-scoped package gate on the final
   integrated candidate. Private corpus pins may move only after a documented
   prediction and complete output comparison; do not re-pin unexpected changes.

## Ownership

WP-17's v27 delivery and reservation release are published at `dc83e939`.
The successor allocates v28 under
[ADR-0148](../adr/0148-repair-product-column-promotions.md), preserving WP-17's
final report-validation/provenance contracts. No other worktree is taken over.
The first repair is source-delivered at the scope recorded above; the sequence
advances only to WP-04.1. No broader historical reconstruction, release or
WP-04 completion is implied.
