# Catalogue update contract

Status: implemented by [ADR-0117](../adr/0117-previewed-recoverable-catalogue-updates.md); web routes by
[ADR-0118](../adr/0118-catalogue-updates-through-the-web-server.md).

## Source

A `.knxprod` file the user obtained from the manufacturer. KNXBench downloads
nothing. Provenance is the file name the user gave and the package SHA-256.

## Preview — `knx products update <file.knxprod>`

Copies the product database into memory (SQLite online backup), runs the
ordinary install there and prints JSON: `applied: false`, the package hash,
`packageAlreadyInstalled`, per application program `status` (`new`,
`newVersion` with `installedVersions` of the same manufacturer and
application number, or `alreadyInstalled`), added products and
Hardware2Program links, ID-conflict count, a catalogue fingerprint and a
`token`. The database file is byte-identical afterwards. Malformed, legacy
(`.vd?`, EX-IM) and oversized packages are refused exactly as `products
ingest` refuses them.

## Adoption — `--confirm <token>`

A fresh preview must yield the same token (catalogue fingerprint, package
hash, source name and program list are bound); otherwise the update is
refused with "preview again". Then a full backup is written next to the
database (`<name>.backup-<unix-seconds>-<sha12>.sqlite`) and the transactional
install runs. A spent token is stale afterwards.

## Recovery — `knx products restore <backup.sqlite>`

Validates `PRAGMA quick_check`, a schema version between 1 and the current
one, and the catalogue tables, then copies the backup into the live database
with the online backup API. Anything else is refused and the live file stays.

## Project bindings

Devices reference a Hardware2Program ID, which names one exact application
program. A newer version is installed beside the old one; no device is
rebound. Changing a device to a newer application version is not offered.

## Web routes ([ADR-0118](../adr/0118-catalogue-updates-through-the-web-server.md))

All behind the existing authentication; JSON is camelCase; errors are the usual
`{error, kind?}` body.

- `POST /api/catalog/update/preview` — multipart, one file field (as
  `/api/catalog/install`). 200: `{applied:false, preview}` where `preview` is the
  `UpdatePreview` above. Package refusals are 400 (a legacy product database is
  422 kind `legacyProductDatabase`, as for install); no product database is 503 on
  every route below; database failures are 500. The database file is unchanged.
- `POST /api/catalog/update/apply` — multipart: the same file field plus a text
  field `token`. 200: `{applied:true, preview, backup, report}`; `backup` is the
  backup's file name only and `report` the install report of `/api/catalog/install`.
  A token that no longer matches is 409 kind `catalogueUpdateStale` and nothing
  is written; a missing token is 400.
- `GET /api/catalog/backups` — 200: `{backups:[{name, bytes, modifiedUnixSeconds, identity}]}`,
  newest first by the time in the name (then the attempt suffix), only regular
  files (no links) with names of the form
  `<database file name>.backup-<seconds>-<12 hex>[-<n>].sqlite` in the database's
  directory. Never a path. `identity` (since Goal B5) is opaque: 32 hex digits of
  a SHA-256 over the file's device, inode, size and nanosecond modification time,
  so it changes when the name is given to another file.
- `POST /api/catalog/restore` — JSON `{name}`. The name must be in a fresh
  listing (no separators, no other file): otherwise 400 kind `unknownBackup`.
  An unusable backup is 422 kind `invalidBackup` and the live file stays. 200:
  `{restored:name}`; the server then reopens and migrates its connection.

- `POST /api/catalog/backups/delete` (Goal B5) — JSON `{name, identity, confirmed}`.
  Without `confirmed: true` 400 kind `confirmationRequired`; a name not in a fresh
  listing (links, other files, separators) 400 kind `unknownBackup`; an identity
  that no longer matches, checked against the listing and again without following
  a link right before removal, 409 kind `catalogueBackupChanged` and the file stays.
  A failed removal is 500 ("the backup could not be deleted: …") and the backup
  stays listed. 200: `{deleted:name}`. The live database is never touched.

Preview, apply, restore and delete hold the product-database mutex for their duration.

**Web use (Goal B5).** The web installs every `.knxprod` through preview and apply
(catalogue and device wizard); a preview refusal of kind `legacyProductDatabase`
sends the file to `/api/catalog/install-legacy`. `/api/catalog/install` is no longer
called by the web and stays unchanged for compatibility.

## Evidence

`crates/knx-productdb/tests/catalog_update.rs`,
`apps/knx-cli/tests/catalog_update_cli.rs` and, for the web routes,
`apps/knx-server/tests/http_catalogue_update.rs`;
[evidence record](../evidence/catalogue-update-2026-10-12.json).
