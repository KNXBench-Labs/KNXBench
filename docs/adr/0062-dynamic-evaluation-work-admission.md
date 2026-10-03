# ADR 0062: Dynamic evaluation admits repeated work before performing it

Date: 2026-10-03
Status: Accepted for the bounded work-admission contract — actual7ae116a1 merged gates verified; publication/readback pending, broader AR07 open.
Session: 4 (manufacturer semantics), AR07
Amends: ADR-0041's bounded diagnostic/refusal coverage and Dynamic evaluation budgets.

## Context

The existing expansion and activation quotas count expanded Modules and
recorded references/labels/skipped-reference reports. They do not count
ordinary diagnostics, duplicate-reference visits, inert nodes or argument
binding work. A shallow, public synthetic fan-out on unchanged source
`be88e7b9` produced 2,000,896 ordinary diagnoses, no activated references or
labels, below the expansion ceiling. Its proposed diagnostic ceiling of twice
the activation quota failed with Rust exit 101, one assertion failure, zero
passes/ignored, in 1.89 seconds. Compilation succeeded. The run used finite
wall/CPU/address-space limits; neither timeout nor allocation failure is the
finding. Receipt: `.ai/logs/2026-10-03_codex_alpha-parameter-budget.md`.

The source explains that result: `activations_recorded` excludes ordinary
warnings and `diagnose` pushes them without admission. Merely clipping that
vector would leave repeated traversal, binding and condition work intact.
Similarly, searching a growing diagnostic vector on each refused activation is
not constant-time just because only one matching budget marker exists.

This is an application resource policy. It does not establish manufacturer
Dynamic grammar, execute Assign/Repeat/calculation code or change KNX meaning.
Repository source and the synthetic regression are its evidence; no proprietary
format assumption or new protocol fact is needed.

## Proposed decision

- Retain the existing depth, expansion and activation caps. Add a per-call,
  shared `MAX_EVALUATION_WORK` of four times `MAX_MODULE_ACTIVATIONS`
  (4,000,000 admitted work units). This leaves room for the existing
  million-reference synthetic boundary while ordinary diagnoses consume both
  a triggering visit/token and a diagnostic unit. This is generous engineering
  headroom, not a normative KNX limit or a claim of exhaustive corpus acceptance.
- Admit ordinary visits, choose-child inspection, skipped-subtree visits,
  module-argument bindings and diagnostics before their repeated work. Charge
  condition parsing for its input length and placeholder lookup for its
  binding-search width. Deduplicated refs and childless inert nodes still
  consume work; having no recorded result does not make a visit free.
- At the first refused admission, emit exactly one
  `EvaluationWorkBudgetExhausted` diagnostic with the source node, scope and
  configured limit. Stop the entire evaluation; do not keep walking siblings,
  create a partially bound module scope, select a fallback branch after an
  incomplete choice scan or publish a partially substituted label.
- Preserve the deterministic, admitted result prefix for inspection and all
  original trees/source bytes. The marker means truncation, not a complete
  inventory of omitted descendants. It may hide references; projection must
  retain activation uncertainty rather than infer Inactive from absence.
- Cache the activation-budget marker flag instead of repeatedly scanning the
  diagnostic vector. Do not replace one amplification path with another.
- A parameter panel whose evaluation hit any resource cap is read-only,
  including fields in its admitted prefix. A later unseen scope may contain
  duplicate authority; a truncated traversal cannot certify a write target.
  The backend clears write targets and refuses writes atomically. Preserve
  existing editable behavior for complete evaluations and unrelated ordinary
  unsupported-node diagnostics.
- Expose the new backend warning token and stable English fallback. Manual
  Web kind/localization adoption stays with the UI owner; no Web or generated
  binding edits, native schema, migration or source normalization change.

## Alternatives

- Count only stored activations: reproduces the ordinary-diagnostic gap.
- Cap only warnings: does not stop inert traversal, binding, dedup or choice work.
- Silently truncate: hides unsupported information and can authorize writes
  from incomplete evidence.
- A second evaluator or guessed manufacturer semantics: unrelated and rejected.

## Acceptance and remaining limits

The candidate is implemented but not yet accepted for delivery. Nine public
ProductDB stages passed: Library 354/0/0 and DynamicTree 62/0/6; eight small
work-boundary units and all four public fan-out families pass. Strict Clippy,
formatting, whitespace and the frozen source scope pass. The four separately
targeted fan-out runs are subsets, not additional unique tests.

A compiled HTTP RED then returned 200 instead of 400 for an unscoped write
after truncation (Rust 101, 0/1/0). The backend now clears every write target
after work, expansion or activation truncation. Six public backend stages
pass: named regression 1/0/0, full parameter HTTP 35/0/0, server library
204/0/2, strict ProductDB/server Clippy, formatting/whitespace and source freeze.
The HTTP fixture checks all four retained fields are read-only, both ordinary
and module-qualified writes refuse, the nonempty project is unchanged, and
original source bytes are retained. Complete-evaluation sibling writes remain
covered by the full HTTP suite. These are public synthetic fixtures, not
manufacturer-grammar or private-corpus acceptance.

Separate in-session bounded source/security review found no blocking issue;
this is not an independent-model approval. `proc_43b041004d63` independently
accepted five compiled behavioral guard mutations: walk, skipped-descendant,
binding and ordinary-diagnostic admission, plus prefix write authority. Each
compiled successfully and produced its intended Rust101/0-1-0 assertion failure;
canonical evaluator/server bytes and full source hashes were restored. The first
attempt's shared-lock timeout occurred before source/compiler start and remains
rejected separately, not behavioral evidence.

Actual candidate `proc_cd67fb854490` exited0 and all20 stages independently pass:
workspace2944/0/164, Web1559, intercepted Chromium61, selected Dynamic6/0/0 and
offline SimTunnel HTTP13/0/0 with no genuine skips,103/108 unchanged originals,
616 frozen code/config inputs and17 equal shadow bindings. Strict build/lints
and nonempty intended-root audits pass. Projection42 is a workspace subset.
This is restored candidate GREEN, not latest-upstream integration. Preserve
fresh0889c102 U16/U17 and gate the actual merged tree before acceptance/delivery;
do not reuse the prior Float gate. Publication readback remains pending.

Actual merged7ae116a1 (budget8f47c13b plus published0889c102 U16/U17) then passed
proc_096e63a3429e, independently20/20: workspace2953/0/164 over148 blocks,
Web1665, intercepted Chromium61, selected Dynamic6/0/0 and offline SimTunnel
HTTP13/0/0 without genuine skips or private raw logs. All103 archives/108 original
fixtures unchanged;624 frozen code/config hashes equal exact committed blobs,
17 shadow bindings equal; strict build/lints and nonempty root audits pass.
Both complete owner records and source artifacts retained. Five compiled mutants
remain restored. This accepts only the bounded policy above, not full AR07,
new-token UI, byte/RSS/latency/general-depth/native/ETS semantics or delivery.

Private corpus runs remain opt-in, offline, aggregate-only and separate from
public synthetic coverage. No live bus or vendor code is authorized.

The work-unit cap is not a complete byte/RSS/latency guarantee. Variable-sized
labels/arguments/metadata, external `substitute_text` consumers and general
non-module tree depth need their own verified admission boundaries; do not
claim that every possible allocation or outside-evaluation loop is bounded.
RepeatIndex, allocator/download placement, scripts and complete ETS parity
remain unsupported or independently evidence-gated. Broader AR07 stays open.
