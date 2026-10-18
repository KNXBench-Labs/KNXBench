# Reversible website headline entrances — 18 October 2026

## Scope

Owner request: reuse the SessionRadar headline entrance on the KNXBench marketing
website, preserve rolling letters and reverse the entrance on viewport exit.
Only the static marketing companion changes. App, core, formats, approved Story,
legal/privacy pages and media stay unchanged.

## Implementation

- Inspected the local SessionRadar presentation: 650ms character animation,
  24ms stagger, translateY(.3em), rotateX(-45deg), opacity 0 → 1.
- Extended the existing headline owner rather than adding a second scheduler.
  Outer character boxes own the entrance; ambient roll/glitch transforms stay
  on the existing inner glyph/twin layers. Total stagger is capped at 600ms.
- An inset viewport observer drives a shared-duration per-heading timeline;
  exit reverses both motion and stagger, and an interrupted reversal preserves
  native current times. Settling releases animation objects.
- Manual pause, reduced motion and hidden/pagehide cleanup cancel both layers
  and restore readable static text. Missing APIs/no JavaScript/animation failure
  preserve the readable fallback; no dependency, storage or network action.

## Evidence

- RED: the new actual-browser recipe timed out waiting for the absent
  `headline-reveal` animation on the unchanged runtime.
- GREEN: preview Chromium 654 checks in nine recipes, including 66 new entrance
  checks; release Chromium 116 checks in three recipes. Zero reported page
  errors or foreign requests. The interruption midpoint is deliberately paused;
  pagehide/pageshow and document-hidden cleanup are synthetic lifecycle probes.
- Website unittest 29 passed; Story unittest 76, one existing skip.
- Repeated release builds byte-identical; JavaScript syntax, documentation and
  local-info checks passed. Visual inspection of the actual built hero midpoint:
  first line settled, second line visibly staggered/fading, unchanged screenshot
  hierarchy and no visible clipping or horizontal overflow.
- Old headline recipe initially failed its immediate "no offscreen animations"
  assertion: a finite reverse entrance is now intentional. It now waits at most
  3s for all motion to settle, then requires zero animation objects.
- First Story test invocation used the wrong cwd and failed imports; corrected
  invocation matches Pages (`cd story && python3 -m unittest discover -s tests`).
  The failed setup is not counted as runtime acceptance.

## Delivery boundary

Independent review completed: the only blocking finding concerned print/PDF
rendering of offscreen headings (23 of 24 transparent in the reviewer's probe).
On 18 October the owner explicitly authorized commit/push without that fix:
printing/PDF is not important for this package. This limitation is recorded in
`docs/WEBSITE.md`, not counted as a passing print test.

Published merge: `3f76c161` on `main`, exact remote equality read back. Website
and Story tests, documentation/header/anchor checks and the release build passed
on the merge; 116 merged-release browser checks passed (66 entrance, 28 direction,
22 public-build). Pages run `39153331822` completed successfully. Live headline JS
and CSS returned HTTP 200 and matched the accepted source bytes exactly.
The selected root checkout and other sessions' work were not synchronized or edited.
Local Chromium acceptance is not a deployment receipt, Firefox/WebKit test,
screen-reader certification or proof of native back-forward-cache behaviour.
