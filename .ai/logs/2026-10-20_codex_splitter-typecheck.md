# Workbench splitter typecheck admission — 20 October 2026

## Scope

Close the recorded handover follow-up: existing workbench-splitters browser
spec was not a root in any committed derived TypeScript config. Add one small
config extending the application config; unchanged runner discovers it. No
application code, fixture assertions, browser policy, retry or dependency change.

## Evidence

- Tool regression fails first on missing discovered config (named RED).
- Focused gate-tool suite and new real TypeScript config pass.
- Deliberate spec-local type mismatch: application-only compiler succeeds,
  new config refuses with TS2322 naming workbench-splitters. Compiler file
  inventory includes the spec. Original spec restored byte-exact, then new
  config passes again. Temporary sentinel is not a committed test or runtime
  browser case.

Independent review ACCEPTS with no findings; reviewed file identities match.
Focused gate-tool tests and actual compiler checks pass, including restored
source after the negative control. Merged maintained package gate and publication
remain pending.
G16 code/tail is published separately; its acceptance is not reused for this
package. No schema, bus/device, desktop, release/tag/deployment/vendor action.
Raw command evidence stays private under the active profile.
