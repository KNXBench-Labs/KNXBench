# Resolved nested-module storage

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-150)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## §150 A product with nested `ModuleDef`s crashes the install with a database constraint error

**Scoped storage fix delivered 2026-10-03 23:05 CEST.** Actual integrated
cbe9952f passed fresh public16 and the same authorized private baseline RED/
candidate GREEN; code published as1b215d51, fetched/live refs equal. An incoming
foreign stats-only commit was retained byte-exact, source706 unchanged, five
fresh publication doc gates pass. Full853 measured installed687→688 with one
constraint refusal→installed and unchanged other categories; all original
hashes independently checked. Fourteen owned build/snapshot/browser directories
removed; closing metadata and final checkout hygiene pending. Lift only this
fresh-install storage/key-scoping bug. R-MODULE-04 runtime/allocation/parameter
semantics, new limits/grammar, full vendor/ETS compatibility and automatic repair
of already-admitted mis-scoped catalog rows remain outside this fix.

**Full offline/public candidate evidence accepted 2026-10-03 22:35 CEST;
integrated acceptance/publication/cleanup pending, not yet lifted.** Commit
a2aa4b7 has fresh complete public16: Rust153 blocks/3009-0-166, ignored
inventory166, Web1702, Chromium82 and shadow bindings17 token-exact. The first
binding helper API rejection is retained separately, not a product/test failure.
Fresh same-profile Release pair12 confirms old-parser named storage RED,
candidate7 GREEN and CLI5 controls GREEN on both sides. Full853 original-name
CLI pair independently reconciled: baseline687 installed/candidate688, exactly
one database-constraint refusal becomes installed;147 namespace,15 ZIP-limit,
two evidence-item-limit and one invalid-ZIP refusals unchanged. No installed-
to-refused regression. Original manifest/all853 files independently rehashed,
successful package bytes/source-blob hashes verified, four catalog/package
tables empty after refusal, temporary inputs/databases gone; aggregate-only
receipts, no private item/raw output. No new limits or runtime semantics.

**Storage candidate verified 2026-10-03 20:56 CEST; not yet delivered/lifted.**
Synthetic named UNIQUE-constraint RED confirms that a single mutable definition
identity loses its enclosing scope at an inner End; Empty also clears it.
The candidate stores lexical definition identity and argument position on a
stack, preserving existing keys, source ownership and atomic transactions.
Nine public focused tests and ProductDB625/0/25 pass with strict Clippy;
three compiled mutants have five named failures, plus an exact committed
pre-fix baseline RED. One authorized private nested-package pair is baseline
RED/candidate GREEN, with independent lexical/stored scope-count agreement,
original-ZIP/retained-member equality, successful retry and unchanged originals.
No private raw/item records survive. Full853 CLI measurement, broad/current-
upstream gates, publication/readback and cleanup remain pending. This does not
claim R-MODULE-04 runtime/allocation semantics, new nested diagnostic XPath
fidelity or automatic repair of previously admitted mis-scoped catalog rows.
The existing idempotence policy still skips stored trees; eligible retained-
source replay is synthetically verified, not a new database migration.

**Observed 2026-10-03.** MDT `RF-TAL55Bx0x-01S_MDT_KP_V12.knxprod`
(scheme 20, program `M-0083_A-00F2-12-05C4`) fails in a shared and in a
fresh database with `UNIQUE constraint failed: dynamic_node.program_id,
dynamic_node.module_def_id, dynamic_node.node_id`. The whole package is
rolled back, so no partial rows are written, but nothing of it can be used.

- The program declares 19 `ModuleDef`s, 11 of them nested inside another
  `ModuleDef` (depth 2). It is the only one of 97 crawled MDT packages with
  nested `ModuleDef`s, and every other MDT package installs.
- **Hypothesis (unverified, no code changed):** `dynamic/parse.rs` clears
  `module_def_id` at the end of a `ModuleDef` (around line 317). After an
  inner definition ends, the outer one's remaining `Dynamic` content lands
  under the wrong `(program_id, module_def_id)` key, where the per-key
  monotonic `node_id` then collides.
- This contradicts the earlier note that "the installed corpus measures
  zero products that actually nest" (GAP_ANALYSIS_ETS A3). That note
  remains true for `OriginalData`, but real downloadable products do nest. A full scan of the 852 crawled ZIP
 packages found this one package as the *only* nested example, which makes
 it the sole real R-MODULE-04 sample so far.
 - Planned as alpha package AR06P (`alpha-release-goal.md`), P1.
- **Lifted when** nested `ModuleDef`s parse into correctly scoped
  `dynamic_node` rows, proven by a synthetic nested fixture plus this
  package.
<!-- END retained catalogue record -->
