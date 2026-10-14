# Project files without their manufacturer files

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-183)

## 183. A `.knxdb` saved before store schema v16 does not carry its application programs

**Reported 2026-10-14 as GitHub issue #21** (thengsty, alpha.11, Docker). After a
container rebuild, every device of a reopened `.knxdb` showed "This device has no
resolvable application program" on its Parameters tab.

- **Cause.** An import with a product database installed the manufacturer files
  into the user-level product database and kept only a manifest (path, SHA-256)
  in the `.knxdb`. The Docker image kept that database in the container layer
  (`$HOME`), not on the `/data` volume. A rebuild, or copying the file to another
  computer, left a manifest that no installed program answered.
- **Fixed in source** ([ADR-0128](../adr/0128-native-project-files-carry-their-manufacturer-files.md)):
  store schema v16 carries the files themselves, compressed and hash-checked.
  Save, Save As, the browser download and `knx import --store` fill them in, and
  every open reinstalls whatever the product database lacks. The Docker image puts
  the product database on the volume (`XDG_DATA_HOME=/data/.knxbench-share`).
- **What stays a limitation.**
  - A file saved **before** v16 cannot carry what it never stored. Its devices
    name the gap on the Parameters tab (`unresolvedProgram`), and **Restore from
    original .knxproj** installs every manufacturer file whose SHA-256 matches the
    manifest. Files with the same path but other bytes are reported and not
    installed, so another export of the project may not help. The next Save stores
    the restored files in the project file.
  - `knx_master.xml` is not reinstalled on open. It stays carried as an opaque
    entry and is only ingested by an import.
  - CLI readers open a `.knxdb` read-only and do not reinstall missing files. Use
    the server/desktop app, or re-import, to fill an empty product database.
  - The file carries exactly what its saved manifest names. A retained history
    version whose manifest differed relies on the product database for the
    difference.
  - Existing Docker containers lost their old product database with the rebuild
    that triggered this report. Nothing can bring it back except the restore above
    or a re-import.
- **Evidence.** `apps/knx-server/tests/http_knxdb_product_data.rs` (9 tests, RED
  before the fix: the reopened program was `null`), `knx_store::manufacturer_files`
  unit tests, `cli_import::import_with_a_product_db_stores_the_manufacturer_files_in_the_project`,
  `tools/tests/test_server_deployment_defaults.py::test_product_database_lives_on_the_data_volume`
  and `ParameterPanel.unresolved.test.tsx`. No real Docker rebuild was driven for
  this record.
