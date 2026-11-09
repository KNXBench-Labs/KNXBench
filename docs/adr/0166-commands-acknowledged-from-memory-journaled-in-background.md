# ADR 0166: Project commands are acknowledged from memory and journaled in the background

Date: 2026-11-09
Status: Accepted

## Context

[ADR-0100](0100-persistent-native-project-history.md) publishes a native project
command only after its working journal is durable. ADR-0163 and ADR-0165 cut that
write from minutes to about 1.5 to 1.8 s on a 308 MB file from a large real ETS
import (issue #35), but the edit still waits for it while holding the project
lock. The owner decided on 8 November 2026 that a value change must not wait
seconds and chose this design (B) together with dropping the full re-admission of
the server's own file (D); a row-delta journal (C) stays a separate later package.

A spike on the same file (release build, dev machine) measured: cloning the
project 1.6 ms, comparing it 0.3 ms, one durable SQLite row update plus a small
append 2.1 ms (median), and a full normalized model write 208 ms. The time is
spent in re-deriving and re-checking, not in the disk.

## Decision

1. **Acknowledge from memory.** A project command that goes through the
   guarded command path (edits, bulk device creation, undo, redo) is validated
   and applied to the in-memory project and stack as today, published, and
   answered. It no longer waits for the journal. This is the mode of a user's
   server (`AppState::with_user_product_db`, used by `knx-server` and the desktop
   shell); test states keep the synchronous ADR-0100 behaviour unless a test
   switches the writer on.
2. **One background journal writer** per server owns the journal writes of
   project commands. A command hands it the newest project, stack and retained
   context; a newer hand-over replaces a pending one, so rapid edits coalesce into
   one write of the newest state. The undo history stays complete because every
   journal write carries the whole stack (ADR-0163/0165 rules unchanged).
3. **Visible state.** `GET /api/project/journal` answers `saved`, `saving` or
   `failed` with the failure message; `POST /api/project/journal/retry` writes
   the current in-memory state, not the job that failed, because another writer
   may have stored newer content since. The status bar asks after each change and shows "Saving
   changes…" while a write runs and the failure with a retry button; it shows
   nothing while every edit is in the file.
4. **A failed write blocks further edits.** While `failed`, project commands are
   refused with a message that names the failure; nothing already shown is
   discarded. The writer retries on request; Save, Save As, programming
   settlement and catalogue creation also clear it, because each writes the
   newest state synchronously. The in-memory state remains the newest and is what
   Save writes. A writer that panics records a failure instead of staying busy.
5. **Flush before anything that needs the file.** Locking the history
   generation waits until the writer is idle or failed. Every other journal
   writer (Save, Save As, open, new, import, version and undo-history actions,
   notes, instructions, naming recipes, plugin data, selective import,
   device-comparison adoption) reads the generation first, so none of them
   interleaves with a background write or writes over a newer one. Server stop
   and desktop close flush explicitly. The writer never takes the project lock,
   so a caller may wait while holding it. A synchronous write of the newest state
   discards every older pending job by sequence number and clears its failure.
   Replacing the project writes the old one's pending jobs first; a write that
   had already failed holds unsaved edits, so replacing needs the usual discard
   confirmation, and the server log names the discarded write.
6. **Some writes stay synchronous.** A programming result
   (`settle_programming`) is persisted before it is published: it records what a
   device holds, and losing it would misstate the installation. Catalogue device
   creation keeps its synchronous write too. Both flush first.
7. **No full re-admission of the server's own file (D).** The writer keeps its
   connection open between writes. When the path still names the file it has
   open (same device and inode, so nothing was renamed or copied over it), and
   `PRAGMA data_version` and the stored generation are unchanged since its own
   last commit, no other connection has written the file, so it skips the
   read-only pre-admission and the in-transaction re-admission and version
   listing. Identity and `data_version` are read before each write and the
   identity compared again after it, so a commit or swap during the write also
   counts. Any other answer, or a platform without a file identity, runs the
   full ADR-0165 path. `data_version` changes on the connection whenever another
   connection commits to the database (https://sqlite.org/pragma.html#pragma_data_version,
   inspected 2026-11-09).

## Alternatives considered

- **Keep changes in memory until autosave (A).** Up to five minutes of work lost
  on a crash or container restart; autosave runs in the browser and stops with the
  tab; a whole Save still blocks every five minutes.
- **Durable row-delta journal (C).** About 2 to 5 ms durable per edit, but a new
  journal format and migration; kept as its own package.
- **Only D.** Stays synchronous at about 0.4 s on the measured file and grows with
  the project.

## Consequences

- A hard crash or power loss can lose the edits of the last write interval
  (normally one to two seconds). A clean stop, Save and every file-replacing
  action flush first. This replaces ADR-0100's durable-before-acknowledge rule
  for project commands only; bus settlement and every other writer keep it.
- Edits continue while a write runs; only a failure stops them.
- The writer is the only journal writer of project commands, so the generation
  and the ADR-0165 trust have one owner; other writers flush first and then take
  them as before.
- The writer keeps one connection to the file open between its writes.
- Tests (server, writer switched on): an edit is answered while its write is
  held back and is in the file after a flush; edits during a running write
  become one write of the newest state with the full undo history; a failed write
  refuses further edits until Save As succeeds; a retry writes the current state
  (not the failed job) once the file is back; a real foreign save stops the
  writer as stale instead of being overwritten; reading the generation waits for the writer; own
  commits skip the admission and a foreign commit restores it; a file replaced
  under the same path is admitted afresh and written (the previous code failed
  the write: SQLite refuses writing a moved file on unix); a writer panic records
  a failure instead of hanging readers; replacing the project while failed needs
  the discard confirmation; opening another
  project writes the last edits first. Web: the status note stays silent, follows
  a running write, keeps a failure with a retry and tolerates an older server.
- Final flush and exit status: the standalone server exits with status 1 when the
  journal is still failed after the stop's last flush (it prints the failure as
  before), on the ordinary return and on the forced-exit path; a healthy journal
  and every other stop outcome keep their status. A script or container
  supervisor can thus see that acknowledged edits did not reach the file.
