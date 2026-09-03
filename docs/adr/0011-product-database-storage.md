# ADR 0011: Product database storage — blobs and parsed tables, content hash as identity

Date: 2026-09-03
Status: Accepted
Session: 4

## Context

ADR-0005 decided that manufacturer data lives in its own shared SQLite
database rather than inside each project file. That decision left open
*how* the database stores what it ingests, and what counts as one entry's
identity.

Measured against the reference project's manufacturer data (spec
§9/§16, `knx-productdb/tests/golden_reference_products.rs`): 24 distinct
source files across 4 manufacturers, one application program alone
carrying 5,919 translation rows and 1,096 enumeration values, and the
whole set totalling 22 MB. The parsers in this crate are deliberately
tolerant (RESEARCH's own finding about manufacturer XML) and do not model
the `Dynamic` tree at all (the `when/@test` grammar is unresearched,
RESEARCH R3) — some of what a file contains will always be unmodelled,
by manufacturer or by construct.

## Decision

Every ingested file is kept **twice**: verbatim as a blob in `source_file`,
keyed by the SHA-256 of its bytes, and as parsed rows in the entity tables
(`hardware`, `application_program`, `com_object`, `parameter`, and so on).

The blob is the integrity guarantee. What the parser does not understand —
the `Dynamic` subtree, a vendor's `Legacy*` option, a construct from a
manufacturer never seen before — is still recoverable byte-for-byte from
the blob, because nothing is ever dropped to make room for the parsed
view. Unknown elements and attributes the parser *does* notice are
additionally counted into `ingest_unknown`, so a gap is visible without
needing to inspect the blob.

**Identity is the content hash, not a name or a manufacturer-assigned
id.** `store_source_file` skips re-parsing a hash already present
(`ingest_is_idempotent` in spirit; measured in
`a_second_ingest_of_the_same_files_stores_nothing_new`), which is what
makes importing a second project that references the same devices cheap.
An application-program *id* can still collide across two different
content hashes — a re-signed or re-exported program keeps the same id but
changes bytes — and that case is a recorded `IdConflict`
(`the_same_program_id_from_two_different_files_keeps_the_first_and_records_the_conflict`,
`crates/knx-productdb/tests/malformed_input.rs`), not an overwrite: the
first ingest's rows win, and the fact that a second hash existed under the
same id stays on record.

## Alternatives considered

**Parsed tables only, no blob.** Rejected: it discards exactly the
information CLAUDE.md forbids discarding — anything the parser does not
yet model (the `Dynamic` tree, an unrecognized vendor extension) would be
gone rather than retained, and export would have no way to reconstruct the
original `<M-xxxx>/*` container entries at all.

**Blob only, no parsed tables.** Rejected: it satisfies the integrity
guarantee but not the reason this database exists in the first place —
resolving `Program`/`ProgramRef` values onto `ComObjectInstance`
(ADR-0012) requires structured rows to query, not a file to re-parse on
every lookup.

**Leave manufacturer bytes in the project's own opaque store
(ADR-0005 half-implemented).** This was Session 3's deliberate, temporary
arrangement. Rejected as the permanent shape because it keeps the 22 MB
duplicated per project and never resolves anything — the state
KNOWN_LIMITATIONS §12 recorded and this session lifts.

## Consequences

Export reconstructs `<M-xxxx>/*` container entries from the database by
content hash rather than from a per-project copy (`knx-app`'s
`export_ets_project`); a project whose manifest names a file the database
cannot supply gets `ExportWarning::MissingManufacturerData` naming exactly
which one, rather than a silently incomplete archive.

A missing product database is a nameable gap (the project's own
`manufacturer_ref` manifest, `knx-store` schema v3), never a reason the
project cannot open — `a_project_opens_and_names_its_gap_when_the_product_database_is_gone`
(`crates/knx-app/tests/product_db.rs`) proves the degradation path stays
tested, not asserted.

The `Dynamic` tree can stay unparsed without data loss, because "unparsed"
never means "gone" — it means the bytes are one `load_source_file` call
away, exactly where they always were.
