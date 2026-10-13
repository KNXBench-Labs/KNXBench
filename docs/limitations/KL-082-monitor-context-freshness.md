# Monitor-context freshness

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-082)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 82. The diagnostics companion's stale lock sees one browser profile's own windows, and nothing else

**Status (2026-10-02 scoped delivery).** The browser-only proof described
below is superseded by the delivered authoritative polling
contract. The server compares the active session's real `GroupAddressContext`
with the current project's interpretation (style, names and resolved DPTs).
`contextStatus`, nullable `projectOpen` and `serverIncarnation` are returned by
the same monitor poll. Busy/poisoned/unavailable evidence is not freshness;
legacy/malformed responses or poll failure lock the new UI's compose form.
Pause performs context-only reads without rows or cursor advancement. Delayed
replies cannot erase a newer invalidation or replacement session. Browser
records cannot establish a verified state. Backend/UI regressions and bounded
mocked browser evidence and twelve complete candidate gates pass; integrated
acceptance repeated and source `8ceacf49` published with exact remote/tree
readback. Scoped comparison is delivered, not the remaining boundaries. See
[owner evidence](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/UI_ALPHA_READINESS.md).

**Remaining boundary.** This is a point-in-time interpretation comparison, not
a complete project identity/version, collaboration/push channel or atomic
write-context token. Existing captured rows keep their old decoded values; a
later edit can occur before another poll or send. Parameter/device changes are
checked only insofar as they change the interpreter's actual DPT/name/style
snapshot. Real native/live-bus verification is not inferred from fixtures.
An older server without this evidence remains readable but cannot enable Send
in the new monitor UI. No hardware permission or source-wide alpha waiver.

**Historical explanation (before this follow-up).** The following describes
the former local-storage-only mechanism and its known cases, not the new
implementation's source of freshness evidence.

**Limitation.** The second-window diagnostics companion (T-UI-06) locks
itself when the project changes under a running bus session. That lock is
decided entirely from two `localStorage` records written by the windows of
one browser profile (`apps/knx-web/src/busContext.ts`). It therefore
detects only edits made in a window that shares that storage. Four cases
it cannot see, and what each one costs:

1. **Another client edits the project.** A second browser, a private
   window, another machine, or `curl` against the same server changes a
   group address's name or DPT without changing its style. No record in
   this profile's `localStorage` moves, so the companion can keep reporting
   `synced` while the running session's `GroupAddressContext`
   (`apps/knx-server/src/bus.rs`) describes an older project. T13 is the
   explicit exception: a successful style change, including Undo/Redo that
   changes the style, refreshes the complete server-side context. That does
   not introduce general cross-client project synchronization.
2. **The project record outlives the server.** `localStorage` survives a
   server restart; the server's in-memory project does not. The companion
   can therefore believe a project is open (`projectContextKnown()`) when
   the server holds none. The only cost is a suppressed hint on the
   compose form: it stops explaining that no DPT will resolve
   automatically. The session half of this self-corrects — the first poll
   after the restart gets a `404` and the panel detaches, clears the
   session record and says the session ended elsewhere.
3. **A project opened in a window that later reloads.** The project record
   is published from the live tree in `App.tsx`; a reloaded window has no
   tree until the user opens a project again, so it publishes nothing and
   the previous record stands until it does. A stale-but-identical
   fingerprint is the harmless case; a project *closed* and a different
   one opened elsewhere is case 1 again.
   **Update 2026-10-06 (AR18 re-check N6):** the same gap shows in the main
   window. After a browser reload, the welcome page appears while the
   server still holds the project, possibly with unsaved edits. A
   following Open or Import is correctly refused, and the unsaved-changes
   dialog appears, but its *Save* and *Discard* then act on a project the
   user cannot see. No data is lost: nothing is replaced without that
   explicit choice. Fetching `GET /api/project` on start is a UI-owner
   change, handed over in `.ai/CURRENT_STATE.md`.
4. **Two sessions in one profile, one of them unrecorded.** If a session
   is started by something that does not write the record — another
   client, or a direct `POST /api/bus/monitor/start` — the companion
   reports `unverified` rather than `synced` or `stale`: it says it cannot
   confirm the decoded values, and leaves sending enabled. That is
   deliberate (nothing observed says the snapshot is wrong), but it is
   weaker than a real answer.

**Cause.** There is no project-change push channel and the companion does
not poll current project state. [§63 point 3](../KNOWN_LIMITATIONS.md#63-knx-server-has-no-multi-userconcurrent-edit-support--one-shared-project-one-shared-undo-stack-no-conflict-detection-at-all)
documents the missing cross-client synchronization. Since T12,
`GET /api/project` does return the current authoritative tree without a
mutation; it is used for explicit recovery and save refresh, not continuous
companion synchronization. The older claim that no such GET exists is
obsolete. The companion still relies on its sibling window's local record.

**Impact.** The lock is a guard against the common case — one user, one
browser, editing in one window while watching in another — not a
guarantee. In the multi-client situations of §63 it is silent, and a
silent lock looks the same as a verified-fresh one. The consequence is the
one the feature exists to prevent: a decoded column, and a write's
resolved DPT, describing a project the server has since changed. Writes
land on real hardware and project Undo cannot reverse them (`busCompose.liveAction`).

**Lifted when.** A verified push or polling contract compares current
project state with the actual session context across clients and restarts.
The existing read-only project GET is a prerequisite, not proof that this
contract exists; no continuous companion refresh is implemented by T13.

**Platform note.** Both platforms were exercised on 2026-09-13: the
companion route renders in headless Chromium against the Vite dev server,
and the Tauri desktop shell opens it as a real second native window,
focuses rather than duplicates it on a second invocation, and returns
focus to the main window. What was *not* exercised anywhere is a live bus
session — no KNX hardware was touched, so the lock's behaviour is proven
by tests (`busContext.test.ts`, `BusMonitorPanel.test.tsx`), not by a
running gateway.

**What the fingerprint cannot distinguish, even for edits it does see.**
The four cases above are all "the lock never hears about the edit". These
three are the other axis: the edit happens in this very window, and the
fingerprint still does not move.

5. **A parameter edit's fingerprint never moves — by design, not by gap.**
   `publishProjectContext` runs from an effect on `App.tsx`'s `tree` state,
   so it fires only when something hands the client a fresh `ProjectTree`.
   Until T3 (2026-09-13), `api.setParameterValue` never did: it answers
   with a `ParameterPanelDto`, and `ParameterPanel` was mounted with no
   channel back to `tree` at all, so the effect never fired and nothing
   republished — a real publish hole, not just a fingerprint quirk.
   `DeviceWorkspace` (`Inspector.tsx`) now closes that channel: once a
   field commits, it overlays `can_undo: true, can_redo: false` onto the
   `tree` it already holds — exact, not a guess, since
   `CommandStack::do_command` always pushes onto `undo` and clears `redo`
   — and hands that to `onApplied`, so `setTree` runs and the effect fires
   on every parameter edit, same as any other command.
   What still does not move, and never will without a further change, is
   the *fingerprint value itself*: `fingerprintProjectContext`
   (`busContext.ts`) deliberately excludes parameters, so the republish
   above writes the same fingerprint under a fresh `at` — indistinguishable
   from renaming a device, which has always behaved exactly this way. This
   is harmless **only** because no parameter value reaches a decode today:
   `Command::SetParameterValue` writes `installation.parameters` and
   nothing else, while `resolve_group_address_dpt` reads com-object links
   and resolved DPTs and nothing else, and `GroupAddressNode.dpts` — the
   third fingerprint input — is produced by the same `group_address_dpt_from`
   rule over the same com objects. The two sets do not intersect. The day a
   parameter can influence a com object's DPT, links or activity, this turns
   into a silent false `synced`; `resolve_group_address_dpt`'s doc comment
   carries that warning at the place that would have to change. **[V]**
6. **The digest is 32 bits.** `fnv1a` in `busContext.ts` returns a 32-bit
   FNV-1a value, so two genuinely different projects collide by accident with
   probability about 2^-32 per comparison, and `synced` means "almost
   certainly unchanged", never "provably unchanged" **[D]**. FNV-1a is also
   not collision-resistant, so a *deliberately* crafted project could be made
   to collide **[D]**. Neither is defended against: the lock is a
   decoding-staleness hint, not a security boundary, and the cost of a miss
   is a mislabelled telegram rather than a bad write.
7. **The field separators are non-printing, and not impossible in a name.**
   The pre-hash string separates the three per-address fields with U+0001 and
   successive addresses with U+0002 **[V]**. That is what stops the obvious
   ambiguity — address `1/1/1` named `0Foo` against address `1/1/10` named
   `Foo`, which without a separator flatten to the same bytes; both that pair
   and the record-boundary equivalent are pinned in `busContext.test.ts`.
   What survives is a group address *name* that itself contains U+0001 or
   U+0002. No supported import can produce one: `.knxproj` is XML, and XML 1.0
   section 2.2's `Char` production admits no C0 control except tab, LF and
   CR **[D]**. Nothing else in the product writes such a name today, and no
   keyboard types one **[A]**. It is recorded rather than encoded away
   because a length-prefixed alternative would invalidate every stored
   fingerprint — every live session would read `unverified` once — to close
   a case nothing can currently reach.

   *Historical note, because it cost two reviews.* Items 5 and 6 were found
   by review; a third finding from the same round — "the fingerprint
   concatenates without a separator, so an ordinary rename produces a
   constructible false `synced`" — was **wrong**. The separators were
   already there and had been since the feature landed, but they were
   written as literal U+0001/U+0002 bytes, which no terminal and no diff
   renders, so two successive readers saw a bare concatenation. They are now
   written as escape sequences instead: same bytes, same fingerprints,
   visible to the next reader. The check that settles it is a search of
   `apps/knx-web/src/busContext.ts` for literal C0 bytes, which should find
   none. **[V]**
<!-- END retained catalogue record -->
