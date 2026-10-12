# ADR 0117: Previewed, recoverable product-catalogue updates from manufacturer files

Date: 2026-10-12
Status: Accepted
Session: AP7 (manufacturer catalogue updates)

## Context

AP7 asks for catalogue updates with verified sources and terms, version
comparison, preview, provenance and integrity, atomic adoption and recovery,
without silently changing existing project bindings.

Verified sources (KNX support articles read through the public Help Center
API on 2026-10-12; [research](../research/product-database.md)):

- the **Online Catalog** is a KNX Association service used from within ETS;
  its products are managed exclusively by the manufacturers, and ETS checks
  for updates automatically or via *Update now*;
- product data is otherwise obtained **from device manufacturers as
  individual files**; `.knxprod` is the XML format since ETS4.

No documented public API, endpoint or licence lets a third-party tool query
or download the Online Catalog. KNXBench therefore does not contact it.

The repository already installs a `.knxprod` in one SQLite transaction
(`knx_productdb::install_package`), records every source name and the
package hash, refuses legacy containers by content, and reports ID
conflicts. It has no preview, no comparison against what is installed, no
backup and no restore.

## Decision

1. **Source:** a file the user obtained from the manufacturer. KNXBench
   never downloads catalogue data; the user's file name and the package
   SHA-256 are the recorded provenance.
2. **Preview:** `knx_productdb::preview_package_update` copies the product
   database into memory with SQLite's online backup, applies the exact
   installation there, and reports per application program: `new`,
   `newVersion` (with the installed versions of the same manufacturer and
   application number), `alreadyInstalled`, plus ID conflicts, product and
   hardware counts, and the package hash. The file is not touched. A
   confirmation token binds the preview to the package hash and a
   fingerprint of the installed package set.
3. **Adoption:** with the token, the CLI re-runs the preview, refuses any
   mismatch, writes a backup copy of the product database next to it with
   the online backup API, then calls the existing transactional install.
4. **Recovery:** `knx products restore <backup>` validates the backup
   (`PRAGMA quick_check`, a supported schema version, the catalogue tables)
   and copies it into the live database with SQLite's online backup API,
   which replaces the content under the database's own write lock.
5. **Project bindings:** projects reference application programs by exact
   ID. A new version is installed side by side and never rebinds a device;
   the preview states this, and tests resolve an existing device to its old
   program after a newer version is installed.

## Consequences

- No network code is added; the layering rules are unchanged.
- Updating a device to a newer application version remains a separate,
  unsupported operation.
- Backups are full copies of the product database; the user removes them.

## Alternatives considered

- **Query the Online Catalog.** Rejected: no documented interface or terms
  for third-party tools.
- **Dry-run inside a rolled-back transaction on the live file.** Rejected:
  it still writes WAL/journal pages to the user's database and holds its
  lock during parsing.
