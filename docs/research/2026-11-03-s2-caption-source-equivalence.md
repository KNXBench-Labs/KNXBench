# S2-05: caption-only source change — bounded witness, 3 November 2026

## Result and boundaries

**PARTIAL:** the unchanged pinned-profile premise is invalidated. A separate,
explicitly new current profile proves equal executable plans for this authored
caption-only fixture. This is evidence for a missing settlement-only contract,
not implementation or approval of one, not a defect in the current strict policy,
and not completion of [S2-05](../open-work/S2-05-result-and-settlement.md).

Executed production source:
`e2e6e46831012fa4fdd025a964ed6566685af8ec` (tree
`43864dd372843795039ba8e50d00fc6820634599`). Only a temporary registered
App test append was applied; production, dependencies and all existing test bytes
were unchanged. The [reproduction patch](fixtures/2026-11-03-s2-caption-equivalence.patch)
is retained here as data, **not applied to the maintained test target**.

## What actually changes

The fixture first obtains an observed, original start-admitted configured
format-3 hardware-kind bundle and proves that the public
`current_project_hardware_parts` returns `ALL` on unchanged source.

Its authored package changes only ApplicationProgram `Name="Synthetic"` to
`Name="Synthetic caption revised"`, exactly once in `M-0001/Application.xml`.
Reverse replacement reproduces the original XML bytes; all other unpacked members
are exact. The ZIP is rebuilt; raw container metadata equality is not claimed.

Ordinary installation would leave the previous first winner unchanged. Instead,
public `install_package` creates a fresh catalogue with the changed package first
and the original retained as a loser; public `restore_product_db_backup` replaces
the fixture catalogue, followed by a fresh `open_and_migrate`. The witness reads
back the changed persisted winner and both actual source bytes/hashes. No new
SQL writes or injected owner/type rows implement this replacement.

The same original project and product/program/device-source references are used.
The complete Hardware2Program member descriptor stays exact. The application
member's owner kind/id/occurrence, path and namespace stay exact; changed source,
length and element digest are not described as unchanged ownership descriptors.
Original source/package bytes remain retained, so refusal is not fabricated by
omitting the old inputs.

## Three different profile outcomes

| Profile passed to complete current preparation | Actual outcome |
|---|---|
| Original profile, unchanged | `source ownership binding mismatch`; no fresh plan is produced. |
| New source hash, old package selector | `Query returned no rows`; the current source/old package pair is absent. This precedes planning, not evidence of different plans. |
| Explicitly new actual source **and** package pins | Complete public preparation succeeds; parsed profile fields other than those two provenance pins are exact. |

The old profile therefore refuses **before** `admit_current_project_hardware`'s
final source-preparation-hash/normalized-plan comparison. The old bundle's start
admission and the public per-part helper return that same early refusal. No
unchanged-profile equal-plan admission was observed.

With the explicitly new current profile, request, coverage, configuration policy,
project snapshot and semantic identity remain exact. Full normalized executable
plan values **and serialized bytes**, not merely hashes, are equal: **2,704 bytes,
28 planned steps**, SHA-256
`4e00351eb1c41cda24645a56e06daf881a2ab1cfa2a1786acb831dfc5961eb4f`.
Source and whole source-preparation hashes differ.

The new preview is format 2, not a newly observed hardware bundle. It does not
relabel the original format-3 bundle. Original serialized bundle/profile bytes
remain exact and their internal `plan()`/`preconditions()` validation still passes.
That is internal reconstruction/ownership consistency, not a vendor signature or
proof of historical archive membership.

## Execution and classification

The actual owning target was `knx-app --test it`; its retained executable's real
`--list` reported 672 test names after applying the append, including three new
names and six unchanged nearest controls. Each named case ran with `--exact`:
one executed, zero ignored, 671 filtered out.

- Two new characterizations passed: old pinned-profile refusal and separate new
  current-profile equal-plan proof with old credit still refused.
- Six unchanged controls passed: source missing/changed, parameter edit, reference
  switch, tampered/unbound/simulator bundle, links/flags and unchanged-source edits.
- One separately and explicitly `--ignored`-selected desired-feature test reached
  its final `None` versus `Some(ALL)` assertion **after** every stated new-profile
  premise. This is an intentional missing-feature RED, not a current-policy bug.

All 15 original compile/list/test invocations have direct exits, raw logs and full
before/after source inventories; actual test binaries are retained where an
executable was produced. Inputs stayed frozen per invocation. One initial compile failed (`Cow<str>`/`String`); one early
fixture expected a later selector message but received the earlier lookup refusal.
Both attempts are retained and neither is credited as semantic RED. Earlier source
variants were reconstructed and hash-matched to their recorded inventories, not
misrepresented as new executions. Final original executions share one frozen
source inventory and executable SHA-256
`3e8d696c3e0454e7c4ffa1c455a6cd0a7f8f4b3374300e7b2f95955b30b390c7`.

The parent separately executed that retained binary's listing, both
characterizations, the desired-feature RED and the tampered-bundle control: five
fresh source-frozen invocations, each in an observed network namespace distinct
from the host. Three cases passed; the intended feature case failed at its target
assertion; the listing retained the same nonzero inventory. These are additional
bounded checks, not a new compile or package acceptance. No full suite or
product-package gate was run for the witness.

## Reproduce without changing production policy

Use an isolated disposable checkout at the named base. Apply the linked patch
there, with scratch XDG homes, offline locked dependencies and the shared build
lease prescribed by `AGENTS.md`. The patch appends only to
`crates/knx-app/tests/it/system2_validation_preparation/configured_hardware/per_part.rs`;
its first 279 lines remain byte-exact. The resulting file's SHA-256 is
`ca79dab9027f17cb04dbe8b4c8db9c738e768c97555a8d21aab66c80f7da464b`;
patch SHA-256 is
`969338784a0c3f8e8148010bd2ce4eb3348ddd137140cd8e15a5320cd62b3620`.
The patch preserves the actually executed experimental source, not a new product
implementation or a claimed formatting/package-gate acceptance.

Compile the owning target with `--no-run`, inspect its actual executable `--list`,
and select names under
`system2_validation_preparation::configured_hardware::per_part::`:

- `s2_caption_original_pinned_profile_refuses_before_plan_comparison`
- `s2_caption_explicit_current_profile_equal_plan_still_refuses_original_credit`
- `s2_caption_desired_credit_after_explicit_current_profile_equivalence_is_missing`
  (requires explicit `--ignored`; expected feature RED on this base).

`KNXBENCH_CAPTION_WITNESS_DIR` optionally writes fresh profile/package/source/plan
artifacts under the caller's owned scratch path. The immutable original raw run
records, source variants and executed binaries are retained privately.

## What this does not authorize or prove

No download was executed. The socketless hardware-kind observation fixture is
not a finished journal, terminal worker settlement, recovery, persistence or
physical-device verification. No bus/device, VM, private corpus, real project,
native UI, release or deployment was involved. Other ignored cancellation/recovery
measurements and their original budgets were untouched.

A future change needs a separately reviewed **settlement-only** contract: validate
the original bundle and its hardware/source bindings, derive explicitly new current
provenance from actual winners, retain non-provenance identity/policy invariants,
reprepare the original request fully and compare executable plans exactly before
defining credit/dependency masks. Such a comparison must grant no new start or
restore authority. Mounted worker completion, server persistence/undo/reopen and
adversarial changed/refused-source cases remain unverified by this witness.
