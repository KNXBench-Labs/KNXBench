# ADR 0185: Download refusal classifiers are exhaustive and pinned

Date: 2026-11-21
Status: Accepted
Session: PRG-01-N7

## Context

`UnsupportedCategory` is the stable reason code that CLI, server, web, reports
and the coverage corpus read. `plan_category` and `image_category` mapped
refusals with a `_ =>` wildcard, so a newly added refusal (for example the
ADR-0167/0168 property-write forms) silently inherited `procedure-contents` or
`image-structure` without anyone choosing it. Several variants had no test.

## Decision

Both classifiers list every variant (no wildcard), so a new refusal fails to
compile until someone classifies it. Behaviour is unchanged: every variant keeps
the category it already had. A unit test pins every `DownloadPlanError` variant
to its category and code; a second pins the cheaply constructible `ImageError`
variants. See the [contract](../contracts/download-refusal-reason-codes.md).

## Consequences

No admitted behaviour, wire format or corpus count changes. Adding a refusal now
needs a deliberate category choice and a test row. Finer reason codes (for
example separating declared-size-without-count) would change consumer output and
corpus attribution, so they are not made here.
