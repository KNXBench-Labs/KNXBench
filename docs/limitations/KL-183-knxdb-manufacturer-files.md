# Project files without their manufacturer files

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-183)

## Catalogue-authored saves become portable (Alpha 16, 24 October 2026)

The catalogue-authoring gap characterised below is repaired as Alpha 16 row
A16-06 (`b1703d61d`, follow-up `5763745fa`), per the
[portability contract](../contracts/alpha16-catalogue-portability.md) and the
[release note](../releases/alpha16.md). Save and Save As acquire a
catalogue-added device's product, hardware and application sources from the
installed **winning source hashes** and exactly one recorded installed package,
and carry that package's exact original bytes as a `CataloguePackage` entry.
On open, the existing validated installer replays it, also into an empty
product database. The owning HTTP regression
`a_catalogue_created_device_reopens_without_external_products_after_save`
replaces the earlier characterization's empty-manifest expectation; refusal,
plain-Save, unresolved-import and offline-fallback cases sit beside it in
[`http_knxdb_product_data.rs`](../../apps/knx-server/tests/it/http_knxdb_product_data.rs).

**Still bounded, so the ledger stays `ACCEPTED_BOUNDARY`:** files saved before
store schema v16 or without a carried original, missing or corrupt source bytes,
ambiguous or unrecorded package membership, legacy/unmeasured ownership and
saves without a product database cannot acquire new sources; they are refused or
reported, never invented. Older readers' handling of the new carried originals
is unverified. Hash equality is not manufacturer authenticity. The dated
sections below remain the historical record of the earlier state.

## Catalogue-authoring runtime characterization (20 October 2026)

The previously source-traced case below is now reproduced through the real HTTP
routes with a fictional product package. New → package install → catalogue device
create resolves the application's parameters. Save As and subsequent Save both
succeed, but the saved file has no manufacturer manifest entries or carried files
even though the authoring product database retains the hardware, catalogue and
program source bytes. Reopening with the original product database resolves the
program; reopening next to an empty one preserves the complete project model but
returns `programId: null`.

The unresolved-program response reports `productDatabase: true` and
`missingFiles: 0`: that zero counts missing **manifest entries**, not every source
needed by the project's devices. It is not a portability certificate. Installing
the same fictional package into the empty database makes the program resolve
again without changing the model or saved file bytes; the manifest remains empty.

The [exact scope and evidence](../status/2026-10-20-catalogue-authoring-portability.md)
separate the desired-portability assertion's observed failure from the committed
passing characterization. Twelve owning HTTP tests pass, including existing
imported-project portability positives. This adds regression evidence, **not a
production repair**, a complete dependency closure or universal package/recovery
support. Future manifest acquisition must define exact source/dependency identity,
transaction/history behavior and unavailable-source reporting before implementation.

## Manifest-scoped portability (20 October 2026 reconciliation)

"Fixed for new saves" is too broad. Schema v16 embeds available, hash-checked
files named by the saved manifest; it does not discover every source needed
by every device just because Save succeeds. A manifest entry whose bytes are
missing from both the product database and carried fallbacks remains
unavailable and is logged. The existing
[`a_save_that_cannot_carry_a_file_says_so` regression](../../apps/knx-server/tests/it/http_knxdb_product_data.rs)
expects a successful Save with zero carried files. That Rust test was
source-inspected, not newly executed in the reassessment.

**Additional source-traced case, not runtime-reproduced:** New starts with an
empty manufacturer manifest; the catalogue-create path changes the project
and command stack without extending it. The saved snapshot copies that
manifest, and embedding visits its entries only. A catalogue-added application
absent from the manifest may therefore still need the external product
database after a new Save. Before treating this scenario as a runtime-confirmed
regression or claiming it repaired, exercise **catalogue device create → Save
→ reopen with an empty product database** through the real service. No missing
bytes are invented and no source-derived capability is inferred here.

Evidence: [project replacement, catalogue creation and save consumer](../../apps/knx-server/src/domain.rs),
[native snapshot](../../apps/knx-server/src/project_history.rs),
[manifest-only embedding and unavailable report](../../crates/knx-app/src/manufacturer_files.rs).
The original fix and older-file/master/CLI/history restrictions below remain
valid within that manifest scope. No Docker rebuild or new portability test
was run for this documentation correction.

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
