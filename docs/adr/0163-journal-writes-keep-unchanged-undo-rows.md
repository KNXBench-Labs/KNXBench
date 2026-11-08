# ADR 0163: Journal writes keep the undo rows they prove unchanged

Date: 2026-11-08
Status: Accepted

## Context

[ADR-0100](0100-persistent-native-project-history.md) stores every undo and redo
state of a saved native project as a complete model image and published edits
only after the recovery journal is durable. Its consequences named the cost
("no unmeasured large-project speed claim"). Every journal write deleted and
re-encoded the whole stack, and the in-transaction admission plus the read-only
pre-check decoded every stored state twice.

Issue #35 (tester, Docker source build of `main`): after a Restriction value was
picked, its dropdown stayed disabled until "an unrelated edit" released it, while
the information panel still showed the old adopted value. Reproduced on the dev
machine with a release build, a 36-device demo project and the UP 258D12
application (`M-0001_A-9205-04-9270`):

- unsaved project: one parameter write 0.14 s; the row unlocks after it.
- the same project saved as `.knxdb` with 33 to 56 undo states: 6 to 16 s per
  write. Per undo state, re-encoding cost 0.17 to 0.27 s (1.5 MB image); the two
  admission passes cost 1.6 s each at 42 states. At the 256-state ceiling a single
  edit would take over a minute.

The row is disabled exactly while its write is pending, and the write holds the
project lock, so other requests wait too. Server write semantics and the client
row logic were correct; the time went into re-encoding and re-decoding states
that had not changed.

## Decision

A journal write of the working state keeps a stored stack row when both of these
independent conditions hold:

1. **Core proof.** `CommandStack::shared_snapshot_prefix` answers how many bottom
   states per side two histories share: their entries below the boundary are
   equal, and replaying each history over its own live project down to that
   boundary gives equal projects. Core replay uses no clock or randomness, so
   every state below is then equal. A direct project change outside the stack
   reaches the boundary and answers zero; so does any replay failure.
2. **Store check.** Inside the write transaction `save_editor_reusing` keeps a row
   only while its stored image hash equals the hash the server recorded for that
   state when it encoded it or admitted it at open. A row whose hash differs, for
   example because Save, restore, Clear undo or another process stored different
   content, is written again, as is everything above it. The hash compares
   content, not authorship.

Kept rows take the working context hash, exactly as a full rewrite pairs every
state with the current context. Rows the server already holds (encoded by it, or
decoded and admitted at open) are verified by the SHA-256 of their bytes during
write admission instead of being decoded again; every other row is still decoded
and checked. Opening a file still decodes and checks every row, so a row the
decoder would refuse now surfaces at the next open rather than at the next edit.

The server remembers the project, stack and stored row hashes of its last journal
write, and of a native open, whose rows are exactly the recovered stack. A failed
write forgets them; new projects and imports clear them. Other journal writers
(Save, Save As, restore, Clear undo, notes, instructions, naming recipes,
adoptions) leave them stale; that only lowers reuse, because the core proof and
the stored-hash check both still apply. The cache holds full copies of the
project and stack, so the history occupies memory about twice. The row shows
a polite "Saving…" status while its write is pending, so a slow store no longer
looks like a dead control.

## Alternatives considered

- **Acknowledge before persisting.** Faster, but breaks ADR-0100's rule that an
  edit is durable before it is published.
- **A command journal instead of snapshot images.** A new versioned format and
  migration; far larger than the measured bottleneck required.
- **Persisted entry identities.** A schema change; equality plus a replayed
  boundary needs no new stored data.
- **Only a client spinner or timeout.** Hides the cost; a timeout would abandon a
  write that may still commit.

## Consequences

- Measured on the same saved project: 2.1 to 2.5 s per parameter write with 48
  to 56 undo states, independent of the number of undo states; the first edit
  after opening is incremental too. What remains is proportional to the project,
  not to its history: the baseline semantic hash (twice, about 0.27 s each), the
  working image and one new state image. The write still runs synchronously under
  the project lock. Both are recorded in the
  [history limits](../limitations/persistent-native-working-history.md).
- Store tests write the same edit, undo, redo, edit-after-undo, outside-change and
  context-change sequence incrementally and as a full rewrite into two files and
  require equal rows and equal reopened histories; a foreign rewrite and a claim
  beyond the stored rows must be written afresh, and a trusted row whose bytes no
  longer match its hash refuses the write. Core tests prove each kept state equal
  to the full replay; a server test pins the cache after edits, undo, open and a
  new project. An independent review with mutants confirmed the guards.
- No file format change: older and newer builds read the same files. Rows kept
  from an open keep the bytes of the build that wrote them; they decode to the
  same states.
