# Monitoring CLI: read-only input layout admission

Date: 2026-10-20. Scope: the offline monitoring-export adapter, not a change to
shared project/product openers. [Contract](../contracts/monitor-project-export.md),
[ADR-0151](../adr/0151-monitoring-export-is-a-sparse-snapshot.md).

## Public facts and independent observations

- **[D]** SQLite's database-header table specifies a 16-byte magic string at offset
  0 and file write/read versions at offsets 18/19. Versions `1/1` describe rollback
  journalling and `2/2` WAL. Source: SQLite, *Database File Format*, sections 1.3.1
  and 1.3.3: <https://www.sqlite.org/fileformat.html#the_database_header>.
- **[D]** A read-only WAL database may still require writable directory access to
  create `-shm`/`-wal`; a read-only connection flag alone is not a no-side-effects
  guarantee. Source: SQLite, *Write-Ahead Logging*, section 5:
  <https://www.sqlite.org/wal.html#read_only_databases>.
- **[D]** Hot-journal recovery precedes database reads and can require writes.
  Source: SQLite, *File Locking And Concurrency*, section 4.1:
  <https://www.sqlite.org/lockingv3.html#dealing_with_hot_journals>.
- **[V]** The original monitoring CLI, exercised as an actual executable, created
  WAL/SHM beside a checkpointed WAL input despite preserving the main file. It
  could also publish a ZIP at the target of a dangling optional-catalogue link.
  The witnesses were synthetic native/catalogue stores; this is not an observation
  about a user's installation or all other repository readers.

## Implemented policy

Before either SQLite-backed reader opens a file, admit only a quiescent regular
SQLite file with `1/1` versions and no `-wal`, `-shm` or `-journal` filesystem entry.
Inspect both the supplied name and a live symlink's resolved target. Refuse cold
journals, directories and dangling sidecar links as well; absence of a provably hot
journal is not the acceptance criterion. A missing optional catalogue is allowed
only without sidecar entries. An existing dangling catalogue link is an error.

The exporter does not checkpoint, change journal mode, recover, delete recovery
files or use an immutable/main-only shortcut. Active-WAL fixtures contain committed
changes absent from a main-only copy: ignoring WAL would omit current data. Inputs
must remain quiescent throughout inspection and read; this adapter does not add a
cross-process lock or close a filesystem path-replacement race.

Do not advise users to delete sidecars. Close the owning application normally or
prepare an independently validated standalone copy using SQLite-aware procedures.
For an already-open KNXBench project the HTTP/UI exporter uses the live snapshot,
not this file-reader path.

## Regression boundary

Named original-source semantic failures cover explicit/default dangling catalogue
aliases, checkpointed WAL inputs and six independently present sidecar cases. Final
focused tests contain 2 unit and 27 actual-binary integration cases; 15 additional
retained-binary probes overlap that coverage and are not a second unique total.
Both native and catalogue controls include active WAL, real interrupted
transactions/hot journals and ordinary rollback-layout success. Complete entry
inventories and source/sidecar bytes are compared before and after refusals.

One disposable-copy recovery control originally required all free-page bytes to
return to their pre-transaction values. Legitimate recovery changed a freelist
page while restoring the live schema and marker; that control now checks actual
rollback plus integrity. The exporter's byte-exact preservation oracle was not
weakened. These focused facts do not replace integrated package acceptance,
receiving-tool execution or a concurrent-writer guarantee.
