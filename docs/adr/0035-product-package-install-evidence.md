# ADR-0035: Persist product-package install evidence

## Decision

`knx-productdb` persists a versioned install-evidence ledger beside each
standalone package. Counts are recorded from archive encounters and write
outcomes during installation, not reconstructed from final database totals.

`InstallCategory` and `InstallDisposition` are closed Rust enums with stable
SQLite strings. Unknown database values are errors, never coerced. Categories
cover archive members, products, application programs, parameters,
communication objects, dynamic nodes, module definitions, baggage, unknown
constructs, master sections, datapoint declarations, and the `Baggages.xml`
index boundary.

The hardware, application-program, dynamic, and master parsers return their own
declaration and write outcomes through crate-private detailed paths; their
pre-existing public outcome shapes remain constructible. `stored` means that
parser's INSERT wrote the row. `deduplicated` exists only where an identity
lookup or conflict path actually suppressed the declaration: product and
application-program first-winner decisions, and content-addressed
archive/source members. A parent program rejection does not classify its
parameters, communication objects, or dynamic nodes as deduplicated; those
children remain `read` with no write outcome. The dynamic pass independently
tracks same-file application-program declarations in document order, so a
non-first declaration cannot contribute dynamic or module-argument rows even
when the first declaration had no dynamic tree. Datapoint type and subtype
declarations share one install-evidence unit and each is `read` plus exactly
one of `stored` or `dropped`. The legacy `dropped_datapoint_types` API remains
an `INSERT OR IGNORE` collision counter; an orphan subtype increments only the
new dropped evidence.

`ModuleDef` declarations are counted by the application-program parser and are
reported as `read` only. The database has dynamic rows scoped by a module id and
module-argument rows, but no durable row representing the ModuleDef declaration
itself, so no module `stored` or `deduplicated` evidence is claimed.

`Baggages.xml` is neither an opaque payload nor a typed inventory. A bounded
boundary parser counts only `Baggage` declarations at the index's schema path.
Those declarations are `baggage_index/read` and
`baggage_index/unsupported`, with a normalized capability diagnostic: PDB-10
owns typed inventory and index-to-payload resolution. Files under `Baggages/`
remain byte-preserved opaque payloads and are separately reported as `baggage`
with read, stored/deduplicated, and `retained-but-uninterpreted` evidence.
Only that exact disposition spelling is valid.

For unknown constructs, `read` is the number of parser encounters, including
repeated occurrences; `stored` is the number of distinct unknown rows persisted
for this package.
The API also exposes `unknown_occurrences` and per-key occurrence fields.
Unsupported master sections aggregate by (kind, archive path, XML path, reason)
and persist an occurrence count. Diagnostic identities are unique on those
fields within a package. Reload also proves that the archive path belongs to
that package with the required role: `Master` for an unsupported master
section and `Baggages` for an unsupported baggage index. Master paths are
exactly `/KNX/MasterData/<single-local-name>` with detail derived canonically
from that section; baggage paths and capability detail are fixed constants.
Diagnostic kinds are a closed enum. `Languages` is a supported master section
and is never diagnosed as unsupported.

A v12 report row is an explicit `measured` or `unavailable` marker. Migration
creates `unavailable` markers for pre-v12 packages because their encounter/write
history cannot be recovered. A fresh install creates a measured marker and all
required count rows, including truthful zeroes. A retry returns the original
ledger with `skipped: true`; an unavailable marker maps to `facts: None`, while
a missing marker for an already-installed v12 package is corruption.

Reload validates the exact report version/status, the complete legal
category/disposition row set, archive-member totals, unknown header/detail
agreement, diagnostic-to-unsupported agreement, normalized paths, and
category-specific arithmetic. It intentionally does not impose a universal
`read = stored + deduplicated` rule: unknowns use occurrence versus distinct
units, and children skipped after parent rejection have no exclusive write
outcome.

The ledger is core-owned and is projected losslessly through the server HTTP
DTO, the CLI package-install output, and the web catalog browser. Historical
installs remain explicitly unavailable in each surface.

## Consequences

Product database schema v12 adds normalized report, count, unknown-construct,
and diagnostic tables. The v11-to-v12 migration creates them unconditionally;
any collision aborts and rolls back with `user_version` still 11. Unsupported
declarations are reported, not presented as stored rows; ETS parity is not
claimed. Categories without a declaration row have no fabricated write
outcome. Every connection returned by `open_and_migrate` has SQLite foreign-key
enforcement enabled before migration work begins.
