# 2026-10-01 — Backup directory chains, not only the final directory

[Research index](../commissioning.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="2026-10-01--backup-directory-chains-not-only-the-final-directory"></a>
<!-- BEGIN preserved record-2-004 -->
## 2026-10-01 — Backup directory chains, not only the final directory

- **[D]** Linux [`fsync(2)`](https://man7.org/linux/man-pages/man2/fsync.2.html)
  distinguishes syncing a file's data/metadata from syncing the containing
  directory's entry. The directory must be synced separately. A successful
  call is the OS/storage completion report, not a power-loss experiment.
- **[D]** Rust [`create_dir_all`](https://doc.rust-lang.org/std/fs/fn.create_dir_all.html)
  creates missing parent components and is explicitly non-atomic;
  [`File::sync_all`](https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_all)
  requests content and metadata synchronization. Sources fetched directly
  on 2026-10-01; the configured text-extraction backend refused extraction,
  so the primary HTML was retrieved with the terminal's HTTP client.
- **[V]** Both application backup writers create nested directories, sync the
  new file, verify readback, and sync only the final directory. Missing
  parent directories can therefore acquire entries whose parent sync is
  not explicitly requested by either writer.
- **[A]** Conservatively sync both the absolute supplied path and its resolved
  target directory chains, child-to-parent, after verified file readback,
  deduplicating shared path spellings in traversal order. This applies the
  documented per-directory rule to every possibly new ancestor entry without
  guessing which components existed beforehand or trusting partial mkdir
  progress. Keep symlink and `..` components: canonicalization can erase
  alias-containing directories or intermediate components needed to reopen
  the originally returned path; the resolved target also needs its own
  ancestors. Anchor relative paths to the process CWD; do not synchronize an
  empty relative ancestor. Any sync/path-resolution error must refuse a backup
  receipt. Avoid new dependencies and share the traversal between the writers.
- **[A]** This does not prove filesystem/hardware power-loss behavior, defend
  against concurrent directory replacement, provide process-crash recovery,
  or make plan-scoped backups whole-device recovery evidence. It must not
  open the address-write gates. Offline tests can verify traversal order and
  error propagation; they cannot simulate actual disk durability.
- **[V]** The implemented shared helper covers both directory chains. Four
  initial writer regressions were RED with unchanged leaf-sync behavior;
  the alias-target regression was separately RED with a supplied-path-only
  traversal. Sixteen focused tests pass after the fix. Restored leaf-only and
  canonical-only guard mutations caused eight and two failures respectively.
  Real temporary files/readback/fsync are exercised, with injected parent
  errors; these are request-order/error tests, not a physical crash experiment.
- **[V]** A separate in-session review RED caught accidental empty-path-to-CWD
  normalization. The shared helper now refuses empty paths before any directory
  sync callback, preserving the previous receipt refusal without repairing names.


<!-- END preserved record-2-004 -->
