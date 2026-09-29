# U8 shared overlay geometry — branch review (2026-09-29)

Branch `ui-overlay-geometry` began at `c248e37` under the U8 web lock. The root checkout is owned by the other session; its unrelated untracked `docs/paperclip-shutdown/` was not touched. No KNX tunnel, bus, hardware or device write was used.

## Change and architectural boundary

- The existing `Overlay` gained one opt-in width/height contract, a CSS-native corner grip and a labelled arrow-key resize control. Dimensions are capped at the viewport; long content scrolls inside. Focus containment, Escape and restoration remain in the shared component, while a pointer drag ending on the backdrop does not masquerade as a dismissal. Unopted overlays retain their geometry.
- Settings starts at 860×680 px and lays out two columns when space permits, one on narrow windows. Full-width inputs fit grid cells; overlay checkboxes override the generic form minimum height. Debug report starts at 820×640 px. The Settings toolbar SVG now depicts a gear while preserving the localized button name.
- Real Chromium revealed an integration defect that a standalone dialog test missed: File closes its `<details>` after selecting Debug report; the fixed overlay inside that hidden subtree collapsed to menu width and lost focus. Following the existing Documentation dialog precedent, Debug report now portals the shared Overlay to `document.body` and returns focus to File's visible summary after close. A File-menu integration regression covers the path.

## Evidence and limits

- TDD RED/GREEN for keyboard bounds, focus/Escape, settings layout, debug-report contents and File-menu portal. The pointer-origin and width-clamp mutants were each rejected by focused tests; the width mutation initially survived until its test asserted the intermediate ArrowRight result, then failed and was restored.
- After rebasing onto concurrent commissioning changes, branch Web: `npm run build` and 77 Vitest files / 1170 tests passed. Branch Rust: fmt, Clippy and 124 workspace suites / 2453 passed / 0 failed / 138 ignored. Fresh U8-specific Cargo target built `xtask`; layering, headers (287/161), 397 links across 214 Markdown files and corpus gate passed before rebase and are rerun after the new docs. `git diff --check` clean; added-line scan found no credential assignment, shell eval or unsafe HTML.
- A separate Chromium probe at 150% application CSS zoom exposed a unit mismatch: `getBoundingClientRect()` returned visual pixels while the inline width and viewport cap expected layout pixels. ArrowLeft changed the inline style without changing the clipped 592 px dialog. A new RED/GREEN `Overlay.test.tsx` zoom case converts rendered dimensions and viewport bounds using the existing root `--app-ui-scale`; Chromium then measured a real shrink from 592 to 556 px, with focus retained and no page overflow.
- Local Vite/Chromium at 1280×800 and 640×700 verified one/two columns, 640 px document width at the narrow viewport, 16×16 px checkbox, bounded pointer and arrow resize, internal scrolling to lower Settings fields, Escape closing to Settings, portalled Debug report with reachable Save action and Escape returning to File summary. Local API errors were unrelated to this offline UI check. Native WebKitGTK, a real screen reader and hardware were not exercised.

## Integration pending

Rebase onto the then-current `origin/main` while preserving concurrent docs and handover entries; rerun full merged-result gates, push the reviewed package, release the U8 web lock, and remove only U8-owned browser, worktree, target and scratch artifacts. The overall `goal-ui.md` continues with U9–U13.
