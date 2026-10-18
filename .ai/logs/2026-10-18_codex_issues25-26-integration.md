# Issues #25 and #26: integration and acceptance

## Implemented scope

- #25: diagram-only filtering of device-empty areas and lines, with preserved
  project/explorer structure, selected-range editing and device-creation targets.
- #26: parameter block and active separator captions reuse the existing exact
  positional text resolver. Source templates and fill provenance remain separate
  from readable display fallbacks; no device values or source metadata change.
- Permanent production browser coverage added to the existing Devices and
  Structure configurations. No base fixture configuration or lane policy change.

Feature commits: `6602ede6` and `74c36dda`. Main-first integration: `b84164d3`
on `3133a077`. The implementation-history merge kept both upstream evidence
entries and the two issue entries.

## Observed focused evidence

The #25 implementation log records the genuine pre-fix RED and the accepted
independent review. Its corrected tests pass 22 component cases, 23 fixture
browser cases and eight production heading/hover cases. The extra queued topology
preview was canceled before any stage executed; its case set is now included in
the maintained production Structure configuration, not claimed as executed there.

The #26 worker retained genuine HTTP/UI REDs and reports 69 HTTP cases, 101 UI
cases, four production Chromium cases, server all-target Clippy, build/fixture
TypeScript and 223 tooling tests (one existing skip). Parent read the receipts,
source call paths and all four wide/narrow EN/DE screenshots. Captions and
fallbacks are readable; no stray template or hash names appear. Narrow navigation
scrolls locally; the preexisting achievement notification overlays lower context
text, not the caption proof.

Controller rerun on the combined tree: 108 focused UI cases in seven files,
production build, fixture and browser-tooling TypeScript, format, documentation
and whitespace checks passed. Added-line security scan found no secret, shell,
eval/exec, pickle or SQL-formatting candidates.

## Still pending

Independent #26 review, any corrections, reconciliation with newer main/UI-05,
full integrated package gate, publication, issue comments/closure and owned
artifact cleanup. Focused evidence is not full acceptance. No bus, hardware,
container, deployment, release, native WebKitGTK or universal compatibility claim.
