# B1 private schema-28 admission

Status: **bounded implementation decision independently accepted**
(21 October 2026), with no CRITICAL, IMPORTANT or MINOR findings. Parent
authentication verifies 54 input bindings and 13 outputs. This authorizes only
the stated private incremental-TDD slice, not runtime or delivery acceptance. Refines [managed binding section 4](managed-binding.md#4-typed-full-current-schema-streaming-witness-v1)
and [staged readiness](design-readiness.md#implementation-order-after-independent-readiness-approval).
The private helper is delivered, still unwired. Corrected-source independent
rereview closes B1-I1 with no remaining findings; exact source identities,
maintained branch/integrated gates and live publication readback are accepted.
See the [definitive delivery record](../../IMPLEMENTATION_STATUS.md#wp-043-b1-schema-admission--source-delivered-21-october-2026)
for actual counts, separate overlapping scopes and retained original evidence.
This is not a usable job, full witness or managed binding. Only independently
observed fresh-chain variants are initially supported; historical layouts are
not implicitly admitted just because their version field is 28.

## Evidence and narrow supported lane

An actual unchanged ProductDB full migration chain at `c012f714` produced a
synthetic fresh catalogue through `open_and_migrate`, then closed and reopened
it through the current-version `open_read_only` path. Independent controller
readback authenticates 10,992 input identities, 4,652 output identities and all
3,354 exported Git blobs/modes. All 869 source-inventory comparisons match.
The actual backend is bundled SQLite 3.53.2 / rusqlite 0.40.2; read-write and
true native read-only descriptors agree exactly, including after a refused RO
write (SQLite code 8). This is a finite fresh-chain oracle, not B1 behavior or
historical-upgrade acceptance. See [research](../../research/2026-10-20-catalogue-job-transactions.md).

The oracle observes 46 application tables, 24 explicit and 50 automatic indexes:
120 stored schema objects, 47 scanned relations and 121 logical objects when
including sqlite_schema. All 70 non-NULL SQL strings differ textually from the
static source seeds; their actual final migration-generated strings, not guessed
normalization, are the accepted fresh-chain variants. The static inventory is
not replaced or silently repinned. Historical byte-different SQL variants refuse
until separately evidenced and reviewed. Schema rowids/rootpages are retained
observations, never fixed global acceptance constants.

## One executable behavior

Add one normally compiled, crate-private ProductDB module that receives a
borrowed `rusqlite::Transaction` and the existing `PublicationGate`. It returns
complete admitted schema descriptors or a typed whole-operation refusal. No
caller-supplied allowlist, path opener, migration, connection ownership,
transaction settlement, full witness, managed owner, file identity, installer,
server/API/UI or production A2 construction is introduced.

The admitted value retains the transaction borrow and has no public constructor.
This prevents use after that borrow, **not mutation through other same-connection
calls**: a shared transaction reference is not an immutable database capability.
Future orchestration must enforce the actual reserved W/no-intervening-DML scope;
B1 alone cannot certify continuous schema identity or grant installation consent.

### Complete inspection and explicit refusal

- Require main user_version 29 and UTF-8 (see the data-only v29 amendment below); reject attachments and application TEMP
  objects, permitting SQLite's implicit empty temp schema. Do not copy/retain
  filesystem paths from database_list or open/repair any database.
- Enumerate every stored main.sqlite_schema row without sqlite-prefix filtering.
  Validate exact object identities and fresh-chain SQL, table_list type/wr/strict,
  all table_xinfo fields, index_list origin/unique/partial, complete index_xinfo
  key/aux/expression/rowid/direction/collation fields, and foreign_key_list tuples.
  Include sqlite_schema explicitly as the special built-in relation, with its
  actual five columns; it is not an application STRICT/PK table or a stored
  self-row. Do not execute a view/trigger to validate it.
- Unknown objects (including stat/sequence tables), old/missing/future schema,
  non-STRICT/WITHOUT ROWID/PK-less application tables, generated/hidden columns,
  case-folded rowid/_rowid_/oid shadows, custom collations, triggers/views and
  virtual/shadow tables refuse. Explicitly compare predicates/expressions and
  constraints through the exact SQL, not an index-name or object-count shortcut.
- Deterministic retained order is table-name byte order, columns by cid, indexes
  by name bytes/xinfo seqno and foreign keys by (id,seq). Preserve all observed
  schema rowid/rootpage/nullable SQL values for the later witness; do not replace
  them with fixture layout values. Retain observed index-list fields without
  using incidental enumeration order to conceal a missing descriptor.
- Errors distinguish unsupported version/shape, WitnessSchemaAdmissionLimit,
  observation/SQL failure with original cause, and typed A1 Stop. No partial
  descriptor result is returned on any failure. Legacy errors/policies do not
  change. SQL corruption/observation failure is not silently turned into success.

### Retention bounds before ownership

Fixed new-path limits remain 256 stored objects, 64 application tables, 256
columns/table, 256 bytes/identifier, 256 KiB/schema SQL, 2 MiB aggregate logical
retained metadata. No accepted unknown shape merely because it fits these caps.
Before copying variable metadata, obtain SQL byte lengths where possible and
inspect borrowed ValueRef bytes, validate storage class/length/checked totals,
then charge and copy. Never fetch an unbounded owned String/Vec and check later.
Check schema-object/table/column counts before retaining their populations.
Use admitted bounded identifiers only, quoting embedded quotes, for introspection
queries. Query construction is separately bounded by those identifier ceilings;
no untrusted caller-supplied SQL or generic query plugin is admitted.

Freeze logical accounting as one tag byte for each retained scalar value plus
8 bytes for INTEGER, and one tag plus an 8-byte length plus actual byte length
for TEXT/BLOB; NULL costs one tag. Charge every retained occurrence, including
repeated names/SQL and fields in distinct descriptors, plus an 8-byte framing
charge per retained record. Use checked arithmetic and charge before each owned
append/copy. Do not retain duplicate schema collections without charging them.
This is a specified logical-retention bound, not a Vec allocator/capacity,
SQLite-native materialization, RSS or native-call deadline guarantee. Fixed
expected constants and borrowed values are not newly retained observation data.

Checkpoint the unchanged A1 gate before/after SQL and between metadata records.
No native interrupt/progress hook, callback framework, COMMIT claim or terminal
cancellation/rollback claim. A cfg(test)-only rendezvous can demonstrate actual
mid-scan stop at a named record boundary; join all workers before assertions.

## Allowed implementation paths

Only these five runtime/test paths, plus parent-owned relevant docs/handover:

1. `crates/knx-productdb/src/lib.rs`: private module declaration only, with a
   narrow unwired dead-code allowance if needed; no public re-export.
2. `crates/knx-productdb/src/catalogue_schema.rs`: actual admission, descriptors,
   typed errors and bounded accounting.
3. `crates/knx-productdb/src/catalogue_schema/schema28.rs`: typed fixed expected
   fresh-chain shapes/SQL, source-bound to the independently checked oracle.
4. `crates/knx-productdb/src/catalogue_schema/tests.rs`: real production-path tests.
5. `crates/knx-productdb/src/catalogue_schema/schema28-fixture.json`: test-only
   portable oracle with exact observed descriptors/provenance; layout values
   remain observations, not admission constants.

No manifest/lock/dependency change, public API, migration/report/master-provenance
change, A1/A2 edit, server wiring, quiet opener/token/fingerprint change or bus.
If module size requires splitting further, stop with the concrete proposed split
instead of widening ownership unilaterally. Keep production constants and test
oracle independently compared to source/runtime evidence, not circularly derived
expectations or a production JSON parser introduced through a dev dependency.

## Executable acceptance

The ten named test groups in the authenticated B1 inventory refine these cases:

- Full fresh-chain fixture/descriptor comparison with real RW and true RO positive
  controls; all object/column/PK/index/FK/SQL identities, not just totals.
- Version/count-preserving substituted index; independently altered column,
  PK/default/CHECK/FK; automatic index/expression/predicate/collation drift.
- Extra internal/view/trigger objects refused without evaluation; unsupported
  rowid/STRICT/hidden/generated shapes, versions, TEMP/attachments/UTF-16 refused
  without repair or any helper DDL/DML/transaction settlement.
- Independently pinned exact/next-reachable/overflow accounting boundaries and
  actual enumeration bound refusals where SQLite can construct the fixture.
  Helper-only seam tests remain labelled helper tests. Competing shape/native
  limits must not masquerade as the intended budget discriminator.
- Pre-cancel and scheduled mid-scan typed Stop; no partial admitted result.
- Caller transaction still active, data/version/schema/total_changes unchanged;
  observed descriptor layout comes from actual transaction, not expected constants.

Use strict incremental TDD: interface absence is distinct from compiled semantic
RED; retain intended failures before the smallest implementation, then GREEN.
Do not label unchanged quiet-opener permissiveness as a reproduced product bug.
Require final nonempty Cargo discovery/execution and warning-denied all-target
Clippy, header/test-layout checks, independent whole-source review, and isolated
compiled wrong-branch controls for count-only admission, prefix filtering,
incomplete index metadata/SQL and charge-after-copy. Each control needs original
GREEN, intended semantic failure and byte-exact restoration/restored GREEN.
Source mutation never occurs in a checkout under acceptance. Maintained scoped
branch/main-first gates, publication/readback and owned cleanup follow separately.

## Amendment: data-only product database v29 (28 October 2026)

Product database v29 (WP-08, `TypeDate` as kind `Date`) re-derives rows only
and changes no DDL, so the pinned fresh-chain shape (`schema28.rs`, the
`schema28-fixture.json` oracle) is unchanged and remains the one v28 introduced.
The admitted `user_version` is now the named constant `ADMITTED_VERSION = 29`,
with a compile-time assertion that it equals `CURRENT_PRODUCTDB_VERSION`: a
later version bump fails to build until its shape is re-established and the
constant deliberately raised. Versions 28 and 30 are now refused like any other
unsupported version. The witness encodes the observed `user_version`, so
witnesses of a v29 catalogue differ from those of the same content at v28; no
witness is persisted across a migration. The oracle file keeps its observed
`user_version` 28 as recorded evidence; tests read only its descriptors.
