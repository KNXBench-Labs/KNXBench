# ADR 0165: Journal writes trust the context and baseline they verified

Date: 2026-11-08
Status: Accepted

## Context

[ADR-0163](0163-journal-writes-keep-unchanged-undo-rows.md) made a journal write
independent of the number of undo states and named what remained: costs that grow
with the project. It was measured on a demo project built from scratch, whose
retained source context (the opaque data of an import, embedded product data
included) is small.

Issue #35, follow-up (tester, Docker build containing 4ca7ca981): up to 40 s per
parameter change with 18 undo states, 76 devices and a 335 MB `.knxdb`. Reproduced
on the dev machine with a release build: a large real ETS project import plus the
UP 258D12 application, saved as a 308 MB file. Its retained context is 141 MB
stored (`project_history_context`) and the root opaque table 134 MB; a model image
is 2.3 MB. Every write took 3.9 to 5.5 s. Phase timing of one write (5.3 s):

- the saved baseline's semantic hash, which builds an in-memory store holding the
  retained context, reads it back and hashes it: about 0.6 s, three times
  (read-only admission, write admission, workspace write);
- decoding and semantically hashing the retained context: about 0.45 s, twice;
- `context::remember`, which encodes the context again and decodes the stored
  one for comparison: about 0.8 s;
- the rest about 1 s, including two model images (working state and the new undo
  state) at about 0.25 s each.

That file had no saved versions. Both admissions also list the saved versions and
decode and semantically hash the retained context of each one, once per distinct
context; the tester's project has 10 named versions. With 10 versions on the
working context the same writes took 0.4 to 1 s longer.

So nearly all of it re-derived a context the server had itself just written or
admitted, and the baseline it had just hashed.

## Decision

The server keeps a `JournalTrust` from its last journal write or native open, in
place of ADR-0163's bare row hashes. Besides those rows it remembers, with full
values:

1. **The retained context**: its value, its semantic identity and the SHA-256 of
   its stored bytes. Admission (read-only and in the write transaction) verifies a
   stored context with that identity and recorded SHA-256 by hashing its bytes,
   instead of decoding it; bytes that no longer have the recorded hash refuse the
   write. `remember` keeps the stored context when the working context equals the
   known value, after the same byte check, or without repeating it when this
   transaction's admission already made it.
2. **The saved baseline**: its model and semantic hash, and its own retained
   context only while that differs from the known one (unsaved plugin data, for
   example). A baseline read from the file that equals the remembered value takes
   the remembered hash; the semantic hash is a function of the value alone. Any
   other baseline is hashed as before, so a root changed by another writer is
   still refused as stale.
3. **The stored working state**: its model and the SHA-256 of its stored image. A
   new stack state equal to that model (the previous working state, which is what
   an edit pushes onto undo and an undo onto redo) is stored with those bytes once
   they still have that hash, instead of being encoded again.
4. **The contexts of saved versions** it decoded and checked, by identity and
   stored-byte SHA-256, without values. Listing the versions inside a write skips
   the working context the admission of the same transaction just checked, checks
   a known one by hashing its bytes and decodes only contexts it does not know;
   the first write after an open learns them.

A default trust knows nothing. It decodes everything as before with one exception:
listing the saved versions in the same transaction as an admission no longer
decodes a version context identical to the working context that admission just
checked, including for ordinary Save and open. A full read captures its trust with
the values it read, so a caller changing the returned history afterwards cannot
turn its own edits into a claim the file does not back. Other
writers (Save, restore, Clear undo and so on) still leave the server's knowledge
stale; that only costs the decode or encode a claim would have saved, because every
claim is checked against the file.

## Alternatives considered

- **Skip validating saved versions during journal writes.** A write would then
  admit a file whose versions no longer read; the byte check keeps that refusal.
- **Skip the read-only pre-admission for a file the server already wrote.** It
  guards the writable open and migration of a file another process may have
  replaced; keeping it costs about 0.3 s now.
- **Trust the recorded column hashes without hashing the bytes.** Cheaper by about
  0.13 s per check of a 141 MB context, but a damaged row would then be kept.
- **Remember fingerprints instead of values.** Saves memory, but equality would
  then rest on importer-supplied per-entry hashes nobody verifies against the
  bytes held in memory.
- **A command journal or a separately stored context.** A file-format change; the
  measured cost was redundant work, not the format.

## Consequences

- Same machine and file, release build, interleaved A/B on identical copies of
  the saved file (previous build against this one, fresh server and copy per
  round) while a release gate loaded the machine, so absolute times vary between
  runs:

  | File | Writes | Before (median, range) | After (median, range) |
  | --- | --- | --- | --- |
  | no saved versions, run 1 | 12 | 5.84 s (5.42 to 6.23) | 2.13 s (1.99 to 2.56) |
  | no saved versions, run 2 | 8 | 4.04 s (3.73 to 4.29) | 1.49 s (1.47 to 1.76) |
  | 10 named versions | 12 | 4.90 s (4.55 to 7.68) | 1.79 s (1.67 to 2.30) |

  Run 1 measured an earlier candidate without item 4; runs 2 and 3 the final one.
  What remains: two admissions of about 0.3 s each (root read, a 141 MB byte hash,
  the working image decode, row hashes, limit checks), one model image of the new
  working state, the version list and the commit. No figures exist for other
  machines or Docker volumes.
- The server additionally holds a copy of the retained context and the baseline
  model, and of the baseline's retained context while it differs: for the measured
  file about 134 MB more resident memory. During a write the trust is moved out of
  the cache rather than copied, but returning the known context still copies it
  once; an open also copies it while it publishes the project. That transient peak
  is unmeasured; it stays below the previous build's in-memory encoding of the same
  context.
- Unit tests count the in-memory stores a write builds (a test-only counter): after
  one verified write or an open, the next edit and an undo build none with a
  retained context and only the new working image. They also refuse a baseline
  changed by another writer as stale, refuse damaged context bytes, re-encode a
  changed context and remember the baseline from then on, and do not copy a
  working image another writer stored, and ignore changes a caller makes to an
  opened history before asking for its trust. The ADR-0163 store harness now runs
  the trusted chain and still equals a full rewrite row for row and after
  reopening; the server cache test requires that both a journal write and an open
  hand more than rows on. The counter counts in-memory store builds, not decodes.
  An independent review with seven run-backed mutants (all killed, comment-only
  control survived) found the trust sound for current callers; its finding that
  `trust()` read the public fields is fixed as above.
- No file format change; older and newer builds read the same files.
