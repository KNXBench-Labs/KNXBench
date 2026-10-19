# ADR-0145: Catalogue cleanup removes placements only

Status: accepted.
Date: 2026-10-19.
Builds on: [ADR-0117](0117-previewed-recoverable-catalogue-updates.md) (backup and
restore), [ADR-0118](0118-catalogue-updates-through-the-web-server.md) (web routes).

## Context

The owner reported that devices appear twice in the Device catalog without any
visible reason, and asked for a way to remove old entries that are no longer
needed.

What the code shows (19 October 2026):

- The catalog lists `catalog_item` rows, which are **placements**: one row per
  catalogue entry a package declared. Several placements can point at the same
  product and the same Hardware2Program link, and a newer application version is
  installed beside the old one on purpose
  ([catalogue-update contract](../contracts/catalogue-update.md#project-bindings)).
- The rows showed only name, order number and description. Placements that differ
  in application version, catalogue section or source package looked identical.
- Re-installing the same package does not add rows: IDs are primary keys and the
  first installed row wins.

A project device stores `product_ref` and `program_ref` (the Hardware2Program ID),
never a placement ID. `query::resolve_program` resolves the application without a
placement. One read does use placements: `query::device_product` finds the
catalogue item name and number for a device by the exact product +
Hardware2Program pair, for the device inspector and the documentation export.

## Decision

1. **Explain first.** Every catalogue row carries additive `details`: application
   program ID, name and version, catalogue section, and the recorded source names of
   the package that supplied the placement (file names only, never a path).
2. **Remove placements only.** Cleanup deletes exactly the selected `catalog_item`
   rows. Products, hardware, Hardware2Program links, application programs,
   parameters, objects, translations, packages, original source bytes and identity
   evidence stay byte-identical. Projects are not scanned and cannot break: every
   device keeps resolving its product and application.
3. **Preview, then confirm.** A preview returns the selected rows, a token and,
   per row, whether the selection leaves the exact product + application pair
   without any placement, and how many devices of the open project use that pair.
   The token binds the sorted selection, the schema, the full `catalog_item`,
   `catalog_section`, `hardware2program` and `application_program` tables, the
   package/source name tables and the database file incarnation (so an exact
   restore of older contents still invalidates an old preview).
4. **Back up, then delete, in one writer reservation.** Apply takes an `IMMEDIATE`
   transaction, recomputes the token, writes a full backup through the existing
   online-backup helper and only then deletes. A refused or failed step writes
   nothing; a failed backup deletes nothing. The backup is listed and restorable
   through the existing backup routes.
5. **People choose, the software does not.** The web dialog starts with nothing
   selected. Its only suggestion selects all but one placement per exact
   manufacturer + product + Hardware2Program triple. No version ordering, name or
   description ever decides that an entry is old.

## Consequences

- Removing the **last** placement of a pair is allowed, after the preview says so:
  devices bound to that pair keep working but no longer show a catalogue item name
  or number, and the documentation falls back to the product text. Saved projects
  that are not open are not inspected.
- No disk space is reclaimed (no `VACUUM`, no blob collection).
- Re-installing an already known package is skipped and does **not** bring removed
  placements back; the backup does. A different package that declares the same
  placements may add them again; there is no suppression marker.
- No schema change, migration or ingest change.

## Alternatives considered

- **Deleting whole applications or products**: would need a scan of every saved
  project to stay safe, which KNXBench cannot do; rejected.
- **A "hidden" flag instead of deletion**: needs a schema migration, which the
  parallel product-database work reserves; it remains a possible follow-up.
- **Choosing the newest version automatically**: `ApplicationVersion` order is not
  proof that one program replaces another; rejected.
