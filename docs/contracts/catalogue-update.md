# Catalogue update contract

Status: implemented by [ADR-0117](../adr/0117-previewed-recoverable-catalogue-updates.md).

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

## Evidence

`crates/knx-productdb/tests/catalog_update.rs` and
`apps/knx-cli/tests/catalog_update_cli.rs`;
[evidence record](../evidence/catalogue-update-2026-10-12.json).
