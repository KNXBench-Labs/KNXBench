# Descriptor-pinned pre-write JSON backups

## Admitted requirement — 5 November 2026

LS-07 is split into three finite implementation tasks in one coherent package:

1. Acquire and retain directory handles for JSON backup creation and directory
   synchronization, anchoring a relative path to one initially opened CWD.
2. Use those handles and one retained read/write file in the device-memory
   backup writer, including exclusive creation, readback and failure refusal.
3. Apply the same contract to the service-control backup writer, preserving
   its semantic validation before filesystem mutation.

This is Linux-first offline storage hardening. Catalogue SQLite backups,
listing/deletion/restore, backup formats, public backup readers, device-write
admission, protocol policy, hardware, release and deployment are excluded.

## Path and identity policy

- Preserve supported static directory symlinks, `.` and `..`; do not silently
  narrow these existing callers to a no-symlink path policy.
- Reject an empty directory before filesystem mutation. Open a relative path's
  CWD once and verify its captured absolute spelling identifies that opened CWD.
  Resolve/create components relative to held directory handles; subsequent CWD
  changes cannot redirect creation, verification or sync. A relative input returns
  an absolute receipt anchored to that initial CWD, deliberately replacing the
  earlier CWD-dependent relative result. Absolute inputs retain their spelling.
- Retain handles for traversed supplied directories and resolved target
  ancestors, including alias-containing directories. Sync the containing
  directory before its parents. Propagate every acquisition/sync error.
- Create the final basename exclusively, owner-only on Unix, read/write and
  close-on-exec. Never overwrite or follow an existing final symlink.
- Write, file-sync, rewind and parse/compare using the same open file. Do not
  close and reopen a pathname as proof of the created file's contents.
- Before returning a receipt, verify that the originally anchored pathname
  still identifies the held target directory and created file, including one
  ordinary full-path open used only for identity observation. A pathname that
  exceeds normal lookup limits cannot become a receipt even when component-wise
  descriptor traversal succeeded. Detected
  replacement or disappearance refuses a receipt. This is a bounded observation,
  not an atomic namespace lock. All mutating and sync operations stay bound to
  retained objects even if that final observation fails.
- Concurrent namespace mutation during initial traversal does not produce an
  atomic snapshot or establish path confinement. There is no permanent guarantee
  that a returned pathname still resolves after external rename/unlink, and no
  protection against another writer modifying the same inode. Failed attempts
  may leave owner-only files; do not delete by a potentially substituted path.
- Unsupported platforms must refuse explicitly rather than silently revert to
  a path-based implementation advertised as descriptor-pinned.

## Required executable acceptance

Use deterministic hooks/barriers, not probabilistic racing loops. Cover both
real JSON writers: stable aliases and `..`, relative paths, exclusive collisions
and leaf symlinks, Unix modes, parent rename/alias replacement after acquisition
and after write, same-descriptor readback, final identity refusal and untouched
replacement trees. Cover isolated-subprocess CWD changes and injected read/seek,
file-sync and directory-sync failures. Keep existing semantic/format roundtrips
and service-control pre-validation refusal. Every failed stage returns no receipt.

Independent review and the maintained change-scoped package gate must pass on
frozen inputs before publication. No actual power-loss or device recovery proof
is claimed.

## Primary filesystem sources

Inspected on 5 November 2026:

- Linux `open(2)`, especially open-file descriptions, `O_EXCL` and the
  `openat()` rationale: <https://man7.org/linux/man-pages/man2/open.2.html>.
  A held descriptor remains tied to its opened object after pathname changes.
- Linux `fsync(2)`: <https://man7.org/linux/man-pages/man2/fsync.2.html>.
  File synchronization does not replace synchronization of its directory entry.
- Rust `std::fs::File`: <https://doc.rust-lang.org/std/fs/struct.File.html>.
  Reads and writes use the opened object; seek changes its retained cursor.
- The workspace's existing `rustix` dependency supplies safe descriptor-relative
  filesystem calls. Use its locked API, not an invented newer version or raw
  unsafe syscall wrapper.
