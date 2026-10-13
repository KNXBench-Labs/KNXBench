# Pre-schema-6 module identities

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-071)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->


## 71. A project imported before store schema 6 has no module-instance ids to write with

**Limitation.** `migrate_v5_to_v6` (design D38) cannot invent a
`ModuleInstance`'s own `@Id` for rows that predate the migration; every
such row's `instance_ets_id` becomes `''` ([DATA_MODEL.md §11](../DATA_MODEL.md#11-versioning-and-migration)).
A user opening a `.knxdb` last saved before this slice therefore sees
every module-scoped section read-only, with the same "no imported
`ModuleInstance` matches" family of diagnostics a genuinely missing
authority produces — nothing in the diagnostic text distinguishes "this
project predates schema 6" from "this project has no matching instance at
all."

**Cause.** The id can only come from the project file's own
`<ModuleInstance Id="…">` attribute (D38). Before this slice, the import
path read that attribute (`installation_v21.rs:764-769`) but discarded it
after using it only as a local wiring key (`map.rs:1038-1049`, design E5)
— so a project imported under an earlier binary never had the id to carry
into the store, and the migration has nothing to backfill it from.

**Impact.** User-visible, and easy to mistake for a bug: a user who has
not re-imported since upgrading sees read-only channels with no reason
that names "schema version" or "re-import" specifically — only the
generic no-authority diagnostic.

**Task 12 (2026-09-14): unaffected, and a different database.** This
limitation is about the *project* store (`.knxdb`, schema 6); task 12
migrated the *product* database (`products.sqlite`, v10 -> v11), which is
a separate file with a separate version chain. The product-db migration
re-derives everything it needs from stored `source_file` bytes and so
needs no re-install; this one still needs a re-import, for the reason
below.

**Lifted when.** Automatically, the moment the project's source
`.knxproj` is re-imported (not merely re-opened) — re-import re-parses
`ModuleInstance/@Id` from the file and repopulates the column for every
row.
<!-- END retained catalogue record -->
