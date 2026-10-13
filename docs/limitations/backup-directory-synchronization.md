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
