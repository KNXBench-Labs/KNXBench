# Shared server project and undo stack

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-063)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

## Update 2026-10-13 (Goal A2)

Parameter writes and installation renames joined the device/group-address
renames and CSV import as routes with an optional revision binding
([contract](../contracts/revision-bound-writes.md)). They remain
feature-specific guards: one shared project and undo stack, no accounts, no
merge. The retained record below predates every such guard.

<!-- BEGIN retained catalogue record -->

## 63. `knx-server` has no multi-user/concurrent-edit support — one shared project, one shared undo stack, no conflict detection at all

**Limitation.** `apps/knx-server`'s web/Docker deployment target holds
exactly one project in one process-wide `AppState`, constructed once and
shared by every connected browser for the life of the process:
`Arc::new(knx_server::AppState::new(data_dir))`
(`apps/knx-server/src/main.rs:54`), `pub type SharedState = Arc<AppState>`
(`apps/knx-server/src/lib.rs:42`), handed to the router with
`.with_state(state)` (`apps/knx-server/src/lib.rs:153`). There is no
per-session and no per-connection project state. Since 2026-09-20 a
middleware *does* read a session cookie out of a request
([§22](../KNOWN_LIMITATIONS.md#22-knx-server-authenticates-with-one-password-or-refuses-to-leave-loopback),
[ADR-0026](../adr/0026-server-authentication-or-loopback.md)) — but every
valid session was opened with the same single password, so a session
identifies a browser rather than a person, and nothing downstream of the
guard ever sees which session a request arrived on. The server still
cannot tell two operators apart. Verified concrete consequences:

1. **A second client's undo can undo the first client's command.**
   `command_stack: Mutex<knx_core::CommandStack>`
   (`apps/knx-server/src/domain.rs:48`) is one stack for the whole
   process; `undo_impl`/`redo_impl` (`apps/knx-server/src/domain.rs:2008-2027`)
   pop/replay whatever is on top of it without regard to which client
   pushed it there. Nothing associates a stack entry with the client that
   created it.
2. **No write route carries any optimistic-concurrency check.** No ETag,
   `If-Match`, version/revision counter, or "expected current value"
   field exists on any route in `apps/knx-server/src/routes.rs`,
   `domain.rs`, or `fs_routes.rs` — every command-applying function
   (`apply`, `apps/knx-server/src/domain.rs:1126-1142`; `undo_impl`/
   `redo_impl`, `:2008-2027`; `save_project`/`save_project_as`, `:546-588`)
   reads and mutates the shared state unconditionally, with no way for a
   client to say "only if nothing changed since I last looked."
3. **No client is told the project changed underneath it.** There is no
   `WebSocket` or `EventSource` anywhere in `apps/knx-web`; the only
   `setInterval` call in the whole frontend (verified with `grep -rn
   setInterval apps/knx-web/src`, one hit, no test-file matches) is
   inside `BusMonitorPanel.tsx`'s telegram-polling `useEffect`, calling
   `poll()` on `POLL_INTERVAL_MS` — line 386 as of this writing, but the
   line number is not the citation to trust: this exact line has drifted
   twice before while the fact underneath it held, so re-run the grep
   above rather than trust either number. It polls bus telegrams, not
   project state. A browser's view of the project tree only updates
   from the response to its own request — it never learns about another
   client's edit, undo, redo, or save except by the user manually
   reopening the project.
4. **File-level save is plain last-writer-wins, silently.**
   `save_project`/`save_project_as` (`apps/knx-server/src/domain.rs:546-588`)
   both funnel into `knx_store::save_project`
   (`crates/knx-store/src/project.rs:72`), which unconditionally
   `DELETE`s every row of every project table and reinserts the current
   in-memory project inside one transaction (`crates/knx-store/src/project.rs:93-97`)
   — no check against what is currently on disk, no file lock. Two
   clients saving the same `.knxdb` path (via `store_path`,
   `apps/knx-server/src/domain.rs:32`) end with whichever transaction
   commits last silently discarding the other's work; neither client is
   warned.

**What is protected.** `apply`, `undo_impl`, and `redo_impl` each take the
same `state.project`/`state.command_stack` locks for the full duration of
one command (`apps/knx-server/src/domain.rs:1132-1134` for `apply`,
`:2008-2027` for `undo_impl`/`redo_impl`), so
two simultaneous requests cannot interleave into a torn or corrupted
in-memory `Project` — one command always finishes before the next one
starts. That is a real, verified guarantee of memory-level consistency
for a single command. It does not protect a user's mental model of the
project, a browser's now-stale view of the tree, the one shared undo/redo
history, or a `.knxdb` file from last-writer-wins.

This is a limitation of the web/Docker deployment target specifically,
where `main.rs` binds `0.0.0.0` and any number of browsers can reach the
one process. The Tauri desktop shell constructs the identical
`Arc<knx_server::AppState>` type — `state: Arc<knx_server::AppState>`
and `Arc::new(knx_server::AppState::new(data_dir))`
(`apps/knx-desktop/src-tauri/src/lib.rs:31,57`) — so it shares this
limitation's state *shape*, not a different design. What makes it
single-user in practice is deployment, not architecture: its embedded
server binds `127.0.0.1` for exactly one locally-spawned webview window
(`apps/knx-desktop/src-tauri/src/lib.rs:60-73`), so no second, remote
client can ever reach it.

**Cause.** `knx-server`'s state model (one project, one `Mutex`-guarded
`AppState`) was built for a single open project per process, the
assumption the desktop app started from; the web/Docker deployment (see
[§22](../KNOWN_LIMITATIONS.md#22-knx-server-authenticates-with-one-password-or-refuses-to-leave-loopback)'s
original design spec) reused it as-is. Session isolation, locking, or merge
logic were never added. ADR-0026 has since added authentication, which was
the prerequisite named here — but deliberately as one shared password with
no user model, so it supplies a session without supplying an identity, and
this limitation is exactly as true after it as before.

**Impact.** A `knx-server` deployment reached by more than one person at
once has no conflict detection, merge, or locking: one person's undo can
remove someone else's change, one person's save can silently overwrite
another's, and neither browser shows any sign that the other exists or
that a change came from outside its own actions.

**Lifted when.** **T22** (multi-user/concurrent-edit support for
`knx-server`) is designed and implemented. Per its own backlog entry
([GAP_ANALYSIS_ETS.md](https://github.com/KNXBench-Labs/KNXBench/blob/994ac08c0328/docs/GAP_ANALYSIS_ETS.md#f-non-functional--operational-gaps)),
it "needs its own design (locking vs. merge vs. last-writer-wins, and
what 'conflict' even means for a `Command`-based undo model)" — that
design question is unresolved, and this limitation stands until it is
answered and built.
<!-- END retained catalogue record -->
