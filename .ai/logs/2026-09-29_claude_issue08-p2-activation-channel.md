# 2026-09-29 — Claude — ISSUE-08 data half, P2: evaluated activation and channel ownership

## What
- `knx-productdb` evaluator: `ActiveRef::channel` (`ChannelOwner`: node id,
  kind, element id, module scope). The owner is carried through `walk`,
  `walk_branch` and `evaluate_choose`, and into module expansions.
- `Diagnostic::may_hide_refs()`: `NoBranchMatched` and
  `UnresolvedTextPlaceholder` return `false`, everything else `true`.
- `query::channel_texts(program, language)`; `substitute_text` made public.
- `knx-projection`: `ComObjectNode.activation` (`ComObjectActivation`) and
  `ComObjectNode.channel` (`ComObjectChannel`). Both are serde-visible but
  `#[ts(skip)]`, because the UI session holds the web lock.
- `knx-server`:
  - `evaluate_device` extracted from the parameter panel;
  - `device_detail` applies the pure `com_object_activation::apply`;
  - `ParameterDiagnosticDto.severity`.
- Docs: ADR-0050, KNOWN_LIMITATIONS §146, IMPLEMENTATION_STATUS,
  IMPORT_EXPORT §9.3.

## Evidence (aggregates only)
- Corpus pin `com_object_activation_corpus` (ignored, corpus-gated):
  - ETS4: 907 objects, all `Active`, all in a channel, 34 channels, every
    channel text empty;
  - ETS 6.3.0: 867, likewise, 33 channels;
  - KV: 75, all `Active`, 32 channels, all texts present and substituted.
- Source check: 29 `Channel` elements across the corpus programs:
  - 24 with empty `@Text` (these state `@Name` and a digit `@Number`);
  - 4 whose text holds a `{{…}}` placeholder;
  - 1 plain text.
- Probe re-run on the current code: panel `NoBranchMatched` 1,016/978/61.
  The P1 doc's KV "64" came from the probe's own evaluation without the
  module-scoped values, and is corrected in IMPLEMENTATION_STATUS.
- Mutation check: 10 of 10 killed.
  - Activation module (7): uncertainty ignored, D40 guard, orphaned scoped
    hit, module-scope match, translation, tree order, severity.
  - Evaluator (3): channel never opened, channel not passed into a module
    expansion, outermost-wins.
- Gates (fresh `CARGO_TARGET_DIR`, serial):
  - fmt and clippy `-D warnings`;
  - workspace 130 suites, 2,604 passed, 0 failed, 152 ignored;
  - layering, headers (161/161), anchors (397), corpus gates;
  - bindings regenerate with no diff; `git diff --check`;
  - corpus pins `com_object_activation_corpus` and `com_object_activity`
    run with `--ignored` and green.

## Decisions
- `Inactive` is only given on a clean evaluation. Uncertain cases are
  `Undetermined`, never rounded.
- `Active` is not downgraded by stale rows, so that it equals what the
  panel shows from the same evaluation (§146 names the exception).
- The channel key is the module node chain plus the node id. Two
  expansions of one module definition are two channels.

## Open
- `@Name`/`@Number` for untitled channels (parser, migration, re-ingest).
- P3: generic names and DPTs (probe: ETS4 497 empty and 120 absent DPTs;
  ETS6 464 and 114).
- The UI half, U12 in goal-ui.md: drop the `#[ts(skip)]`s and regenerate
  the bindings.
