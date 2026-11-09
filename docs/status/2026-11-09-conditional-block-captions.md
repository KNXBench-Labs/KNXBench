# Conditional block captions: software acceptance

Date: 9 November 2026. Accepted source on `main`:
`da7d667d1ca7d24319e83932353c7743912d10c3`, integrated against
`5d5436a523ea37eb96cc645c34b33572941e4129`.

## Delivered slice

A literal `ParameterBlockRename` or `Rename` leaf can retitle its own enclosing
block in an application/channel tree while its condition is active. Navigation,
heading, context and search share the derived caption and the leaf's own
translation. With no active rename, the immutable base caption returns. Parameter
values still use the existing validated command and undo/redo path; a caption is
not new project state.

The [contract](../contracts/conditional-parameter-block-rename.md) defines the
supported shape, fail-closed competition, shared work/text budgets, language
fallback and diagnostics. ProductDB version 33 re-parses retained program sources
without DDL changes; the v33 report rewrite validates existing historical facts
before promotion and leaves an invalid report unavailable rather than repairing
its counters into a new measured claim.

## Acceptance on the integrated source

The ordinary change-scoped package gate completed **15 of 15** selected stages.
Every selected stage exited zero with valid proof; source inputs stayed frozen.
Its user-data checks recorded neither touched nor unchecked user-data scopes.
This was not the release/full-workspace gate.

| Producer | Actual result |
| --- | --- |
| Rust, seven selected packages and reverse dependants | 5,667 passed, 0 failed, 180 ignored in the ordinary run |
| Private corpus runner, six selected targets | 97 of 97 selected tests passed |
| Tool tests | 560 tests, no failures, 16 optional skips |
| Nested web gate | 37 of 37 selected stages, production build included |
| Vitest | 5,484 passed, none failed or pending |
| Default Playwright lanes | 1,144 cases: 1,067 functional and 77 exclusive |
| Production `parameters` lane | 44 functional cases passed |

The Rust selection was `knx-app`, `knx-cli`, `knx-desktop`, `knx-mcp`,
`knx-productdb`, `knx-server` and `xtask`. Its raw log has 19 native test-target
runs and five doctest suites: 24 result summaries are **not** 24 executables.
The corpus selection executes its named ignored cases separately; it is not a
claim that every ignored case ran. Default and production browser cases overlap,
as do earlier focused proofs, so these figures must not be added into a unique
coverage total. The built web bundle had the same fingerprint before and after
the web gate.

## Review and retained chronology

The corrected candidate received independent code and confidentiality approval;
its focused proofs cover ambiguous/undecided competition, text and resolver
budgets, historical-report validation and diagnostic fallback. Test captions use
invented aliases with a targeted private equality check against the held witness.
That check is not a general confidentiality guarantee.

The original rejected candidate and reviews remain private. The first integrated
gate stopped at formatting. The second stopped at the historical-catalogue
preservation check and the production-spec inventory check. Corrections restored
the original retained history verbatim and require both exactly 39 production
spec files and explicit membership of the new caption spec. Neither correction
changed production behaviour. Both failed runs remain separate from the third,
successful run; their later unexecuted stages receive no retrospective credit.
The source-bound native receipts and raw logs are retained privately.

## Boundaries and follow-up

The [reference observation](../research/2026-11-09-conditional-parameter-block-rename.md)
is one product and five settled offline states, not a general ETS compatibility
or repaint-timing claim. No bus, device write, deployment or new release is
claimed by this package. The next release must account for ProductDB version 33.

[WP-05](../open-work/WP-05-parameter-layout-fidelity.md) stays open. Module-scoped
or foreign-target renames, competing or undecided candidates, text-parameter or
template replacements and other layout features remain outside this slice.
Unsupported selected translation templates retain their diagnostic source-text
fallback; they do not become a new supported template mechanism.
