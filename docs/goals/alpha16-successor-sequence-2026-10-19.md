# Alpha 16 successor work sequence

Owner-authorised on 19 October 2026: work the recommended sequence until the
owner redirects or pauses it. This is an implementation goal, not authority to
cut a release, deploy, control the desktop or operate a real KNX bus/device.
Each package is separately reviewed, tested, integrated and published before
continuing. Do not turn accepted boundaries into speculative implementations.

Canonical remaining details: [OPEN_WORK](../OPEN_WORK.md). Existing WP numbers
keep their scope; this goal orders slices rather than renumbering the queue.

## Order and acceptance

1. **Earlier product-database promotions (WP-02 follow-up).** Repair the
   v23–v25 discrepancies between migrated and freshly installed databases:
   first program declarations must retain ownership of promoted columns, and
   evidence still reported by ingest must not disappear during migration.
   Begin with synthetic regressions per affected step; repair databases that
   already passed those steps in a new schema version. Hash-checked retained
   sources supply the facts, never guessed values or changed historical
   install outcomes. Prove fresh/migrated equivalence for admitted sources and
   explicit, non-destructive diagnostics for sources that cannot be repaired.
   **Dependency:** WP-17 owns v27 and shared migration/report surfaces. Its
   publication and release of that reservation precede this package's schema
   implementation. Until then, only independent analysis and regression
   preparation are allowed; never edit the other worktree.
2. **WP-04.1 — measure large catalogues.** Record per-table/index/source-byte
   growth and peak memory using disposable catalogues. Account for retained
   bytes and unknown-evidence growth before proposing optimisation. Preserve
   all private source material and detailed measurements outside Git.
3. **WP-04.2 — bounded procedure parsing.** Inventory resolver
   byte/node/depth/expansion bounds. Change parsing only where the measurements
   justify it, preserving source order, refusal and malformed-input behaviour.
   A documented decision not to optimise is valid when evidence does not
   justify a change; invented procedure semantics are not.
4. **WP-04.3 — web large-profile workflow.** Reuse the CLI's install service;
   supply truthful progress, cancellation, shared budgets, bound preview and
   atomic installation/rollback. Prove cancellation cannot publish a partial
   catalogue. A larger upload limit alone is not completion.
5. **WP-19 — sync from a comparison.** Amend the comparison contract and
   ADR-0130 first. Adopt only verified, unambiguously attributed decoded values
   from a current comparison as one undoable project change. Reject stale or
   ambiguous adoption atomically; retain unknown/raw-only evidence. Prove
   undo and simulator re-comparison. Project-to-device only hands over to the
   existing full-download path, preserving all admission and confirmation
   rules. Any real write acceptance requires operation-specific owner consent;
   this goal does not grant it or permit a differential write route.
6. **WP-05 — parameter layout fidelity.** Build the feature/evidence table,
   then deliver one source-backed feature at a time with an explicit contract,
   unsupported fallback and browser regressions. Reuse existing caption and
   layout machinery. Undefined manufacturer hints stay unsupported.

**Small independent follow-up:** check the AppImage build's native Rolldown
binding immediately after dependency installation, with a missing-binding
regression and unchanged tag-only publication rules. This does not establish
why the retained Alpha 15 attempt lacked the binding.

## Execution and continuity

- Check fetched main's handover and current owners before each package and
  immediately before integration/publication; claim only the package being
  started, not every later surface in this sequence.
- Keep the owner's selected dirty checkout and all foreign changes untouched;
  work in owned worktrees. Keep product data and test homes disposable and
  isolated, including tests that spawn binaries.
- Test first and retain actual RED/GREEN evidence. Independent review precedes
  the maintained change-scoped package gate. Releases alone require the full
  release gate; none is authorised here.
- Publish each accepted package at a sensible intermediate boundary, verify
  remote readback, update handover/status and clean only task-owned scratch.
  Continue directly into the next available ordered slice without asking for
  another go. An evidence or ownership blocker limits only its dependent
  scope; report it honestly without guessing a compatibility claim.
- Do not contact manufacturers or KNX Association. No quota checks or quota
  pauses. An explicit owner pause overrides this standing goal.

## Current state

The first package has a proposed repair contract, eight observed semantic RED
regressions, 24 passing existing baseline tests and an accepted read-only review
of the bounded test preparation. WP-17 still retains its migration reservation.
No repair implementation, accepted package gate or package delivery is claimed
by this planning document.
