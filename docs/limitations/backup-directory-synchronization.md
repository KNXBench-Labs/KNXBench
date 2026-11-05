# Backup directory synchronization

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-backup-directory-synchronization)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## Backup directory synchronization is not a disk-loss or confinement proof

The device-memory and service-control backup writers sync the supplied and
resolved directory chains after file synchronization and readback. Errors from
any ancestor refuse a receipt, including when a filesystem or parent directory
does not support the requested synchronization; do not silently ignore those
errors. Failed attempts can leave owner-only diagnostic files on disk and
must not be mistaken for successful recovery receipts.

Directory paths must remain stable during the operation. This path-based
implementation does not pin directory descriptors, defend against concurrent
symlink replacement/renaming or changes to process CWD, or prove actual
power-loss behavior of the filesystem, controller and drive. It does not widen
plan-scoped memory coverage or establish recovery for address programming,
serial address writes, address reset or master reset. No hardware gate has
been opened by this offline storage change.
<!-- END retained catalogue record -->

## Current JSON-writer implementation — 5 November 2026

The [descriptor-pinned contract](../contracts/descriptor-pinned-json-backups.md)
replaces repeated pathname opens in the device-memory and service-control JSON
backup writers. Their supplied/alias and resolved ancestor directories are held
and synchronized child-first; file creation, file sync, rewind and semantic
readback use the same retained file. A final no-follow/nonblocking full-path
identity open also refuses paths that normal backup-reader lookup cannot use;
it does not read file contents again. Detected final target/leaf identity changes
refuse a receipt without path-based cleanup. Static symlinks and `..` remain
supported. Relative inputs now return an absolute receipt anchored to the
initially opened and spelling-checked CWD.

This does not lock the namespace, make initial traversal atomic, establish path
confinement, defend against another writer modifying the same inode, or keep a
returned pathname valid after external rename/unlink. Failed attempts can leave
owner-only diagnostic files and are not receipts. Unsupported non-Unix writers
explicitly refuse before mutation. OS synchronization is not an actual power-loss
or whole-device recovery proof.

Catalogue SQLite backups/list/delete/restore, the public backup reader and the
legacy property-validation recorder's path-based synchronization remain outside
this package. The older stable-path requirement still applies to those unchanged
paths. Source-bound [execution and acceptance](../status/2026-11-05-descriptor-pinned-json-backups.md)
state the actual gate/publication boundary; focused tests alone are not delivery.
