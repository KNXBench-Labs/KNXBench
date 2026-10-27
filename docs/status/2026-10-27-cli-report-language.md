# CLI documentation report language — 27 October 2026

## Delivered source scope

Main-first merge `5679ba3c5d00afb75a454b81d4a1682db683936a` publishes
feature `fe8b57981` and documentation correction `41c7c84cc`. Local, tracking
and live `main` were read back equal after publication. The user checkout was
not synchronized; unrelated local changes and parked worktrees were preserved.

```sh
knx doc-export <store.knxdb> <out.html> --language de
```

The optional switch accepts `en`, `en-US`, `de` and `de-DE`, matching the
existing HTTP language spellings. Without it, reports remain English. German
uses the existing report renderer and product-string locale; this is not a new
renderer or language catalogue. Invalid, missing or repeated switch values are
refused before opening the store or creating/replacing output.

## Executed acceptance

- Independent whole-feature review and documentation correction review: APPROVE.
- Baseline: six built-CLI export regressions. Three new tests first failed on
  the missing language-selection behaviour, then passed after implementation.
- Focused final acceptance: ten built-CLI export tests and two parser unit tests;
  focused Clippy also passed.
- An earlier branch gate refused missing browser-test dependencies and a changed
  retained dossier section. Dependencies were prepared and the historical section
  restored exactly before the accepted rerun.
- Maintained change-scoped package gates on the committed branch and main-first
  merge: **13/13 selected stages green** at each boundary, frozen inputs and no
  touched or unchecked user data.
- Integrated Rust selection: **529 passed, zero failed, 21 ignored**, four test
  binaries. Selected offline corpus: **17/17**. Tooling registered 461 tests,
  with six declared optional skips.
- No application Web/browser, generated-bindings or full-workspace stage was
  selected by this CLI-only package. `browser-lanes` checks spec registration,
  not application browser execution. The web dependencies and bundle were
  prepared for the tooling-stage browser-launch checks.
- An authentication-only push failure was retried with a one-shot credential
  helper after revalidating the unchanged accepted commit and upstream base.
  No gate result was replaced by the retry.

The feature tree, its merged branch and its task cache were retired only after
the final source snapshots and executables passed archive member, SHA-256 and
resolved-byte-count verification. The integration tree remains until its result
note has passed the documentation gate and publication readback.

## Retained boundaries

No third language, report-parity claim, new PDF output, report sections switch,
web-dialog language selector or translation catalogue is added. Stored raw enum
values and detailed diagnostics retain their documented language boundaries.
This was source delivery only: no release, deployment, bus or hardware action.

[Report-language scope](../limitations/KL-048-report-languages.md)
