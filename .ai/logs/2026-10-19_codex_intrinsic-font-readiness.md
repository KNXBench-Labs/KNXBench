# Intrinsic oracle waits for the mounted table's fonts

## Context

The gate-isolation full gate on `a86cb1f9` failed in the default browser
functional lane: 788 passed, one failed at the independent fit assertion.
The intrinsic floors changed between consecutive measurements (77 to 76 and
77 to 71) while only a rendered column width was changed. The trace records
the JetBrains Mono request after the initial Inter requests. The guard itself
reported no touched user data and no incomplete coverage. The full gate was
not accepted; later stages did not run.

## Reproduction and fix

A new browser regression mounts a table, requests a separate font face and
holds its response. It observes the oracle entering its measurement sandbox
or accessing the font barrier before releasing the response. On the old helper
it fails with `the oracle must not measure fallback-font widths` (received
`true`). After the response is released the result must equal a fresh
measurement using the loaded face.

The first test draft checked a Node promise across one browser round trip;
that did not establish a causal execution barrier and passed without the fix.
A second draft observed all computed-style reads and could catch the production
component's independent remeasurement. Neither draft is counted as proof. The
retained test observes only the oracle's isolated sandbox and font access.

`intrinsicFloors` flushes layout on the source table and awaits its document's
current font readiness before measuring. A font requested when a table mounts
can begin loading after navigation's earlier readiness promise resolved. No
production component, width budget, threshold, retry or timeout changed.

## Evidence

- The retained delayed-font test fails on the old helper.
- With the barrier: delayed-font test and the original failed assertion pass.
- Removing just the font-readiness await makes the delayed-font test fail;
  helper restored byte-for-byte afterwards.
- The complete table-column spec passes 50/50 offline, one worker, zero retries.
- `tsconfig.table-columns-fixtures.json` typechecks successfully.
- Browser lane audit reports zero untagged real-time uses.

## Pending

Independent review of the font-helper delta and a fresh complete package gate.
The previous guard review remains accepted; the failed full gate is retained.
