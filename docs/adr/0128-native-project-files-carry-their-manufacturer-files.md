# ADR 0128: A native project file carries its manufacturer files

Date: 2026-10-14
Status: Accepted
Session: GitHub #21 (owner decision: full fix)

## Context

An ETS import installs every manufacturer file of the `.knxproj` (application
programs, `Hardware.xml`, `Catalog.xml`) into the shared, user-level product
database (`$XDG_DATA_HOME/knx/products.sqlite`, ADR-0005). Until now the `.knxdb`
kept only a manifest of those files (`manufacturer_ref`: path, SHA-256, length,
kind, store schema v3). That is enough to name the gap, but not to close it.

Issue #21 (alpha.11, Docker): after a container rebuild, every device of a reopened
`.knxdb` said "no resolvable application program". Verified in the source:

- `knx_productdb::query::resolve_program` joins `hardware2program` with
  `application_program` in the product database. An empty database resolves
  nothing (reproduced by `http_knxdb_product_data.rs`: the import resolves
  `M-0001_A-1`, the same file reopened next to an empty database answers `null`).
- The Docker image set `KNX_DATA_DIR=/data` (the volume), but the product
  database followed `$HOME` (`/root/.local/share/…`) and lived in the container
  layer. A rebuild deleted it while the projects on the volume survived. Copying a
  `.knxdb` to another computer has the same effect.

A project imported **without** a product database already kept its manufacturer
files whole, as opaque entries (`manufacturer_to_stored`). So the format already
treated "the file is complete on its own" as a goal; the product-database path
was the one that lost it.

Measured on the private corpus (6 projects): manufacturer data is about 20 MiB
uncompressed for typical projects, 186 MiB at most; the `.knxproj` stores it in
1.5 MiB and 18 MiB respectively (deflate).

## Decision

1. **Store schema v16** adds `manufacturer_file` (`sha256` primary key,
   `source_path`, `len`, `codec = 'zlib'`, `bytes`). Rows are keyed by the
   manifest's SHA-256, compressed with zlib and re-checked for length and hash
   on every read (`knx_store::manufacturer_files`). At most 64 MiB per file, the
   same bound the `.knxproj` reader puts on one archive member.
2. The rows belong to the **file**, not to the editor history. A native snapshot
   never contains them: the history image check refuses an image with rows, the
   semantic snapshot hash skips the table, and undo, redo and versions never copy
   them. A v15 snapshot image stays admissible.
3. **After every Save, Save As, browser download and CLI `import --store`**, the
   file is made to carry exactly what its saved manifest names
   (`knx_app::manufacturer_files::embed_into`). The bytes come from the product
   database or, failing that, from the project's previous file. Whole-file opaque
   entries with the same hash count as carried. Rows the manifest no longer names
   are removed. A file nobody can supply is logged as a warning with its path. The
   save itself has already succeeded and stays successful.
4. **After every open**, every manifest entry the product database lacks is
   installed from the bytes the file carries (`reinstall_from_file`, existing
   `ingest_file`, manifest order, one file's failure does not stop the others,
   every outcome in the session log).
5. **Files saved before v16** cannot carry what they never stored. Their gap is
   named: the parameter panel's `unresolvedProgram` (program reference, whether a
   product database exists, how many of the project's files are missing). It
   offers **Restore from original .knxproj**, which is
   `POST /api/project/manufacturer-files/restore`. Only outer archive members
   are read (`Container::open_outer`), so a password-protected project needs no
   password (verified on three real protected exports, see
   [project-format research §2.3](../research/project-format.md)). Only members
   whose SHA-256 equals the manifest's are installed.
   Members with the right path but other bytes are reported, not installed. The
   project, its file and its history are not touched; the next Save stores the
   restored files in the file.
6. The **Docker image** sets `XDG_DATA_HOME=/data/.knxbench-share`, so the product
   database lives on the volume too.

## Alternatives considered

- **Docker fix and an error message only.** It fixes the reporter's container, but
  not a `.knxdb` copied to another computer. A project file would still depend
  on machine state it does not name in full.
- **Put the files into the snapshot (opaque entries).** This is the existing
  no-product-database path. Every edit's working journal and every retained
  history context would hash and copy up to 186 MiB. The uncompressed opaque
  table would also make the file several times larger than its `.knxproj`.
- **Store the bytes uncompressed.** Simpler, but a 186 MiB project file for an
  18 MiB `.knxproj`.
- **Write the restored files into the `.knxdb` immediately.** A restore would then
  write the user's file outside Save. Instead, the panel tells the user to save.

## Consequences

- A `.knxdb` saved with this version opens with its application programs next to
  any product database, including an empty one. It grows by roughly the
  compressed size of its manufacturer data, close to the `.knxproj`'s share.
- The first save of a large project compresses its manufacturer data once (zlib,
  default level). Later saves skip every row that is already present.
- Selective import (model v15 audit) was re-audited for v16: no `Project` field
  was added, and a source's manufacturer files arrive as whole-file opaque
  entries, which already count as carried.
- Limits, recorded in KNOWN_LIMITATIONS: `knx_master.xml` is not reinstalled on
  open (it stays carried as an opaque entry). CLI readers open `.knxdb` files
  read-only and do not reinstall. A retained history version whose manifest
  differs from the saved one does not keep its own carried files.
- Tests: `apps/knx-server/tests/http_knxdb_product_data.rs` (reopen with an empty
  database, old file → named gap → restore → save, mismatched archive,
  password-protected original, Save As and plain Save without a product
  database, download, refusals, login guard), `knx_store::manufacturer_files` unit tests (compression, damage,
  retain, v15→v16), CLI `import --store`, Docker image check in
  `tools/tests/test_server_deployment_defaults.py`, web panel tests.

## Verification beyond the tests (2026-10-14)

A temporary probe (not committed) drove the server end to end: import with a
product database, Save As, reopen next to an empty one, then remove the carried
rows to imitate an older file and restore from the original archive.

| Source | Devices resolved: import / reopen / after restore | Files carried and restored |
|---|---|---|
| xknxproject `xknx_test_project` (ETS 5.7, ZipCrypto, restored without password) | 4 / 4 / 4 | 32 / 32 |
| xknxproject `test_project-ets4` (ETS4, ZipCrypto, restored without password) | 2 / 2 / 2 | 13 / 13 |
| Largest private corpus project (110.5 MiB archive, release build) | 76 / 76 / 76 | 248 / 248 |

Before the restore, every device of the imitated older file was unresolved, and
nothing mismatched. The largest project's manufacturer members are 185.8 MiB raw
and 18.0 MiB with zlib at the default level, which is what the file carries.
The whole probe (import, save, two opens, restore, 228 panel reads) took 230 s;
the stages were not timed separately.
