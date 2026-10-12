# ADR 0118: Catalogue updates through the web server

Date: 2026-10-12
Status: Accepted
Session: web integration package 4 (catalogue updates in the web)

## Context

[ADR-0117](0117-previewed-recoverable-catalogue-updates.md) added previewed,
token-confirmed, backed-up catalogue updates and restores to `knx_productdb`
and the CLI. The web workbench only has `/api/catalog/install`, which installs
a `.knxprod` directly: no preview, no comparison with installed versions, no
backup and no restore. The browser must not be given filesystem paths to
choose a restore file, and the server owns one shared product-database
connection behind a mutex.

## Decision

1. **Routes.** `POST /api/catalog/update/preview` and
   `POST /api/catalog/update/apply` take the same multipart file field as
   `/api/catalog/install` (same 256 MiB body limit, `PackageLimits::STANDARD`;
   no large-package override from the browser). `GET /api/catalog/backups`
   lists backups, `POST /api/catalog/restore` restores one. All sit behind the
   existing authentication middleware.
2. **Stateless confirmation.** The server keeps no upload between preview and
   apply. Apply receives the file again plus the preview's `token`;
   `apply_package_update` re-runs the preview and refuses a mismatch (HTTP 409,
   kind `catalogueUpdateStale`). This is exactly the CLI's contract.
3. **One lock.** Preview, apply and restore hold the server's product-database
   mutex for their whole duration, so they are serialised with installs and
   catalogue reads. The project mutex is never taken at the same time.
4. **Backups by name only.** The server lists files next to the product
   database whose names are `<database file name>.backup-<seconds>-<12 hex>`
   with an optional `-<n>`, ending in `.sqlite`, the names `apply_package_update`
   writes. Responses carry names, sizes and modification times, never paths.
   Restore accepts only a name present in a fresh listing; anything else is 400.
5. **Reopen after restore.** A backup may have an older supported schema
   version, so after a successful restore the server opens and migrates the
   database again and replaces its shared connection. A failed reopen is a 500
   and is reported as such; the restored file stays as restored.
6. **Bindings.** Unchanged from ADR-0117: devices keep their exact
   Hardware2Program/application references, a newer version is installed beside
   the old one, nothing is rebound. A restore can remove programs an open
   project references; the UI says so before restoring.
7. **Existing install route unchanged.** `/api/catalog/install` and the device
   wizard keep installing directly. The catalogue view adds the previewed update
   beside it rather than silently changing what the old button does.

## Consequences

- No network access is added; KNXBench still downloads no catalogue data.
- A large package is uploaded twice (preview, apply). Accepted for a stateless,
  token-bound server; measured cost is bounded by the 256 MiB limit.
- Backups accumulate next to the product database until the user removes them;
  the web lists but does not delete them.

## Alternatives considered

- **Keep the uploaded bytes on the server between preview and apply.** Rejected:
  adds server-side upload state, expiry and cleanup for no integrity gain over a
  token-checked re-upload.
- **Let the browser name a backup path.** Rejected: it turns restore into an
  arbitrary-file read primitive on the host.
- **Route `/api/catalog/install` through the update path.** Rejected for now: it
  would change an existing, tested behaviour (and the device wizard) without the
  user asking; a later decision can retire the direct install.
