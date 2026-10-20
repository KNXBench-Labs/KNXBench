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
   **Dependency satisfied:** WP-17's product schema v27 and reservation release
   are published at `dc83e939`. The repair uses a separate v28 corrective step
   ([ADR-0148](../adr/0148-repair-product-column-promotions.md)); master replay
   and historical encounter evidence remain outside its mutation scope.
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

### Source delivery and current package (20 October 2026)

The first package is source-delivered as `8fa0d76b9fd41f81c4e135c29e54f2a98d516b38` with
[accepted final integration and verified publication](../IMPLEMENTATION_STATUS.md#product-column-promotion-repair--source-delivery).
It advances the dedicated product database to 28, not native/model16 or an
Alpha15 release artifact. Authentic-source reconstruction, missing/unavailable
history, ambiguous member-counter attribution and conservative resource
refusals remain boundaries under the existing repair contract.

**Current ordered package: WP-04.1 — measure large catalogues.** Its output
is a disposable-catalogue growth/peak-memory baseline, not a storage or parser
change. No measurement has been performed by this documentation tail.
WP-04.2, WP-04.3, WP-19 and WP-05 remain later ordered work. WP-04 is open;
no later package is claimed or completed by this update.

### Retained pre-delivery checkpoint (20 October 2026)

The following paragraph preserves the original review/gate checkpoint; its
pending assertions are historical, superseded by the delivery record above.

The first package's v28 implementation and already-upgraded regressions are
present. Original review rejected resource admission, role dispatch and
historical counter attribution; its observations remain retained. A correction
round adds 11 review regressions (nine observed semantic REDs against original
production, all 11 GREEN after correction). Controller updated the one obsolete
shared-aggregate test to require unavailable history rather than guessed member
counters, while retaining exact single-member recovery and unaffected-history
assertions. The final focused selection passes all 46 tests. Independent
correction review accepts the full feature with no remaining findings and four
additional passing reviewer probes. The complete ProductDB recheck passes
459 library, 644 integration and two doc tests (31 ignored). Private corpus
equivalence and the maintained branch/integrated package gates remain pending.
No package delivery is claimed by this planning document.
