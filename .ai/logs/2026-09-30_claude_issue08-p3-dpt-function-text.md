# 2026-09-30 — Claude — ISSUE-08 data half, P3: program DPT, DPT text, function text

## What
- `knx-projection::ComObjectNode`: `program_dpt` (ADR-0027 program default
  beside an empty `DatapointType`, never in it), `dpt_text`, `function_text`.
  All `#[ts(skip)]`, so `apps/knx-web` is untouched (web lock).
- `knx-server::domain`: `apply_dpt_texts` (master-data text per DPT shown,
  memoised per id) and `apply_function_texts` (product `FunctionText` via
  `com_object_views`, in the request language).
- `com_object_activation::apply` substitutes a module object's
  `FunctionText` arguments from the scope of the expansion that activated
  it; `decide` now returns the whole `ActiveRef`, not just its channel.

## Why these, and not more
- Baseline probe (throwaway, outside the repo, aggregates only) showed:
  - names: every corpus object has one, and the T33 overlay already
    translates it, so no generic object name is needed;
  - DPT: 122 of the ETS objects with an empty slot have a program default
    that was never shown; the rest have no DPT in the product at all
    (ETS4 473, ETS 6.3.0 434) or a list of several (22 each);
  - `FunctionText`: in the product for every object, translated for most,
    and in KV every one carries a module placeholder.
- Nothing is picked from a multi-DPT list: neither the project nor the
  product chooses one, so choosing would be guessing (§146).

## Evidence
- Corpus pin extended (`com_object_activation_corpus.rs`): DPT stated/
  from program/none 290/122/495 (ETS4), 289/122/456 (ETS 6.3.0), 75/0/0
  (KV); every shown DPT has a text; every object has a `FunctionText` with
  no `{{…}}` left.
- Unit tests: 12 in `com_object_activation` (new: per-instance substitution,
  and a scopeless object keeps its placeholder); projection 40 (new: program
  DPT beside an empty slot; hidden by a stated one).
- Mutation check: 4 of 4 killed (program DPT over a stated one, program
  DPT written into `dpt`, module scope not substituted, a scopeless object
  borrowing another expansion's scope).
- Gates (serial, fresh `CARGO_TARGET_DIR`): fmt, clippy `-D warnings`,
  workspace test 2,607 passed / 0 failed / 152 ignored (130 suites),
  layering, headers 314/161, anchors 397 links, `tsc --noEmit`,
  `git diff --check`; corpus pin `--ignored` green.

## Open
- Channel `@Name`/`@Number` for untitled channels (§146) — parser, schema
  migration, re-ingest.
- UI half (U12): bindings, grouping, collapsing.
