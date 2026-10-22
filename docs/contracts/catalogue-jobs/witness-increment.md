# WP-04.3 stage B — typed full-current-schema streaming witness (§4)

Delivered 22 October 2026 (source merge `9e57258c7`). Second implementation
increment of WP-04.3 stage B
under [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md) and the
[managed-binding contract](../catalogue-jobs/managed-binding.md) section 4.

## Scope of this increment

The **full typed chunked current-schema witness** — a deterministic, lossless
SHA-256 digest of one reserved catalogue transaction, produced by streaming
the complete schema relation plus every persisted column and accessible hidden
rowid of every admitted main table. It refines §4 into an exact byte grammar
and implements it as a private, unwired ProductDB helper that reuses the B1
schema admission for shape validation and descriptor retention.

- `CatalogueWitness { encoding_version, sha256, table_count,
  schema_object_count, row_count, encoded_bytes }`.
- `witness_current(tx, gate)` taking a borrowed `rusqlite::Transaction` and the
  existing `PublicationGate`, returning the witness or a typed refusal.
- The `CatalogueWitness/1` canonical encoding below, with checked arithmetic,
  chunked TEXT/BLOB streaming (`substr`, never a whole Rust `String`/`Vec`) and
  a typed per-cell admission bound of 1,000,000,000 bytes.

## Encoding (`CatalogueWitness/1`, version 1)

All integers big-endian; every byte is hashed and counted. A *cell* carries a
storage-class tag and typed payload. A *tuple* is `u32 field count` followed by
that many cells. A *byte string* is `u64 length` followed by exactly those
bytes (no terminator). Record tags are the §4-frozen `0x10..0x16`; cell tags
are `0x00..0x04` and appear only at cell positions.

Cell payloads: `0x00` NULL; `0x01` INTEGER + i64 BE; `0x02` REAL + f64
`to_bits` u64 BE; `0x03` TEXT + framed bytes; `0x04` BLOB + framed bytes.

Stream order:

1. **Header `0x10`** — raw domain `KNXBench/CatalogueWitness/1`, NUL `0x00`,
   `u32` encoding version `1`, `i64` `PRAGMA user_version`, `i64`
   `PRAGMA schema_version`, `i64` `PRAGMA application_id`, framed
   `PRAGMA encoding` (validated `UTF-8`). Journal mode/cache/locking stay bound
   separately in the managed configuration receipt, not here.
2. **Descriptor `0x11`** — one per admitted relation (the built-in
   `sqlite_schema` plus the 46 application tables, 47 in all), in
   deterministic table-name byte order: framed table name, `table_list` tuple
   (6 cells), `u32` column count then each `table_xinfo` tuple (7 cells, by
   cid), `u32` index count then per index the `index_list` tuple (5 cells) plus
   `u32` xinfo count and each `index_xinfo` tuple (6 cells, by seqno), `u32`
   foreign-key count then each `foreign_key_list` tuple (8 cells, by id,seq).
   `sqlite_schema` has its five fixed columns, no indexes and no foreign keys;
   its complete 120-row content is streamed as data below.
3. **Table data** — all 47 relations in the same table-name byte order, each
   with rows ordered by `_rowid_` (for `sqlite_schema`, its 120 stored objects
   ordered by schema rowid). Table-begin `0x12` = framed name + `u32`
   stored-column count; row-begin `0x13` = i64 BE rowid + each stored column in
   cid order as a cell; row-end `0x14`; table-end `0x15` = `u64` row count.
4. **Stream-end `0x16`** — `u32` scanned-relation count (47), `u32`
   stored-schema-object count (120), `u64` total row count. Then finalize
   SHA-256. `encoded_bytes` is the checked external stream-byte count.

`_rowid_` is selected only because B1 admission already proved `table_list.wr=0`
and rejected WITHOUT ROWID and case-folded rowid/`_rowid_`/`oid` shadows; names
are quoted by doubling quotes and size-bounded. `ingest_unknown` emits its
rowid and its aliased `id` column (redundant intentionally).

## Explicitly out of scope (later increments)

- The `CopyContentWitness/1` copy witness and W/S/D recipe (§6).
- `CatalogueStamp`, the `CatalogueOwner` server wrapper and route adapters.
- Any production route, public API, schema, trigger, `user_version` or quiet
  API change.

## Done when

- The private helper compiles and passes test-first coverage on schema-28
  synthetic data: complete counts (47 relations / 120 stored objects),
  determinism, mutation sensitivity (insert/update/delete/outside-set values),
  exact cell-codec bytes, storage-class distinctions (NULL vs empty TEXT/BLOB,
  INT vs REAL, embedded NUL), multi-chunk BLOB streaming, admission-refusal
  propagation and typed cancellation Stop.
- No production route becomes reachable; quiet public APIs and schema unchanged.
- Focused tests green; owning ProductDB suite, Clippy `-D warnings`, fmt and the
  one-sentence header check pass.

## References

- [managed-binding.md §4](../catalogue-jobs/managed-binding.md#4-typed-full-current-schema-streaming-witness-v1) (authoritative)
- [schema-admission.md](../catalogue-jobs/schema-admission.md) (B1, reused admission)
- [managed-owner-increment.md](../catalogue-jobs/managed-owner-increment.md) (B owner, delivered)
- Existing B1 admission: `crates/knx-productdb/src/catalogue_schema.rs`
