# Catalogue updates from supplied files

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-catalogue-updates-from-supplied-files)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## Catalogue updates come only from files you supply (2026-10-12)

KNXBench never queries the KNX Online Catalog: no documented interface or
terms for third-party tools were found ([research](../research/product-database.md)).
`knx products update` and the web catalogue's **Update with preview…** work on
`.knxprod` files you download yourself. Programs whose ID already exists from
another file are counted as ID conflicts but not listed per program. The backup
is a full copy of the product database and is not deleted automatically, neither
by the CLI nor by the web. The web uploads the file twice (preview, then install
with the token) and restores only backups it lists next to the product database;
a backup elsewhere needs `knx products restore`. The plain web install and the
device wizard still install directly without preview or backup
([ADR-0118](../adr/0118-catalogue-updates-through-the-web-server.md)). A restore can
remove programs an open project uses; those devices show no parameters until the
program is installed again. Evidence is Chromium (intercepted build and a
loopback-only real-server smoke with synthetic packages); no WebKitGTK window.
<!-- END retained catalogue record -->

## Update 2026-10-14: every web package is previewed, and backups can go (Goal B5)

The web's product picker no longer installs a `.knxprod` directly: the
catalogue and the device wizard preview every package and install only after
**Back up and install**. A legacy product database, or one renamed to
`.knxprod`, goes to the legacy import instead. `POST /api/catalog/install`
remains for compatibility (scripts, older web builds) and still installs
directly without preview or backup. The web deletes a listed backup only when
asked to and only the file it listed (`POST /api/catalog/backups/delete`, bound
to that file's identity); the CLI has no delete command
([ADR-0118](../adr/0118-catalogue-updates-through-the-web-server.md)).
