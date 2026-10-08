# LCARS engineering study

Status: interactive offline study implemented and exercised in system Chromium on 2026-10-07; visually approved by the user. Product integration was subsequently authorized and implemented on 2026-10-08; see [LCARS guide](../../DESIGN_LCARS.md). This standalone study remains a separate memory-only demonstrator, not an importable v1 pack.

The production presentation gained two user-requested ambient loops on
2026-10-08 (header-band pulse and emblem colour shift). The standalone study
below is unchanged and still has no idle loops. Its original receipts describe
that historical study, not the later production animation amendment. The
production verifier now also exercises ambient timing and live cancellation.
Fresh combined wizard/ambient source and build evidence is in
`ambient-integrated-verification.json` (2358 frontend / 140 files, 162 Chromium,
54 production-workbench assertions). `ambient-verification.json` remains the
historical 2332/158/54 local-only source receipt; neither receipt alone proves
live deployment.
Latest both-wizards/achievement/ambient candidate: `ambient-publication-verification.json`
(2373 frontend / 142 files, 170 Chromium, 54 workbench, 27 targeted Rust checks).
Its image/live result is recorded separately after actual activation.

## Design tokens and composition

- Canvas `#0b0c10`, working surface `#14161c`, primary ink `#eeeaf3`.
- Warm frame `#eda970`, lavender selection `#c2b2e3`, subdued ink `#b0acba`.
- Error, warning and success have separate semantic tokens and text/icon labels. Decorative orange is not a warning indicator by itself.
- Space Grotesk headings; Inter work content; JetBrains Mono addresses. Use the project's installed, unmodified Latin WOFF2 files, embedded for offline portability with their OFL notices.
- Large elbow unites header and navigation. Segmented bands mark actual sections. Keep the existing left explorer / central table / right inspector relationship.
- Native tables, independent checkbox selection and name activation. Decoration remains inert, outside table structure. Real overflow, viewport-bounded shell and compact/comfortable row density.
- Motion follows actions and real fixture state changes; no idle loops, fake telemetry or network activity. Off and OS reduced motion cancel running effects.

```text
   navigation elbow ───── segmented header / project actions ──────┐
   navigation       │ workspace title / filter       │ properties │
   project explorer │ native group-address table    │ editable   │
   diagnostics      │ honest fixture progress/error  │ draft      │
   └────────────────── offline / local-only status ────────────────┘
```

Brief review: the warm/lavender multi-segment frame and elbow supply the distinctive LCARS identity. The center deliberately stays quiet and dense; no generic card grid, neon glow, randomized counter or marketing hero. Rounded ends serve navigation/action affordances, not every data cell. Modernization is readable type and restrained, reversible action feedback.

## Planned behaviors and acceptance

Filter/select/edit example group addresses; expand project branches; switch workspace examples; show/hide inspector/navigation; change density, motion and neutral comparison presentation. Demo saving must explicitly say memory-only, retain dirty state on failure/cancellation and retain newer edits when an older snapshot finishes. Form drafts survive selection changes; invalid/duplicate addresses cannot apply.

First browser tracer: assert the initial offline study contains a native address table with a real address and activates its inspector through the row's name button. Further checks follow this implementation, not an imagined production API.

Verify actual animation lifecycle with real timers and OS motion emulation, keyboard focus, overflow at small/zoomed sizes, semantic status feedback, no external requests and no global console errors. Capture and inspect browser screenshots. Browser evidence does not certify native WebKitGTK/Orca or whole-application WCAG compliance.

## Delivery boundary

The standalone HTML will include its fonts and notices; no server, live bus, user project, localStorage or installed application is needed. Reloading resets local demonstration data. Production integration requires separate visual approval and a documented, narrow presentation extension because v1 packs cannot carry layout or animation.

No light variant, sound, workflow redesign, licensed series artwork or new dependency is in scope. The study is not an official Star Trek product.

## Verified delivery

Open `index.html` directly in a browser. It is self-contained, including four unmodified Latin WOFF2 font files and the original OFL notices in “Über die Studie”. No installation or server is required.

`verify-browser.js` is a plain Playwright CLI verification function, not a production component or a Playwright Test suite. Open the study in an isolated CLI browser session, configured with `browser.launchOptions.executablePath: /usr/bin/chromium`, then run `playwright-cli run-code --filename verify-browser.js`. The wrapper can be invoked through Bash if its executable bit is absent. Fetch `window.__lcarsVerification` with CLI `eval` for the structured result; Node-side console output is not necessarily returned by the CLI.

Final candidate: **50 browser assertions passed**, zero external page requests, zero browser console/script errors. Embedded script syntax check passed. Exact HTML/verifier fingerprints and assertion names: `verification.json`. Screenshots from that candidate: `evidence/desktop.png`, `evidence/error.png`, `evidence/narrow.png`, `evidence/zoom-layout.png`, `evidence/small.png`.

Checks cover native table/activation and independent checkbox marking; filtering, room scope and real overflow; invalid/duplicate address refusal; drafts retained across selection; local apply and dirty state; truthful pending/success/error/cancel simulation; protection of newer edits; shortcuts, reset confirmation and focus; actual live motion cancellation, OS reduction and no idle animation; independent density/presentation; and viewport layouts at 1600×1000, 1280×900, 768×1000, 720×620 and 480×900. The 720px layout represents the layout space of a 1440px window at 200% zoom: native browser-chrome zoom itself was not operated.

Visual inspection caught an initially unusable small-screen table that showed only its header; a named RED assertion now ensures usable row space, with minimum workspace geometry and local scrolling. A later verification attempt failed because the harness navigated at its previous 480px viewport before setting the desktop size: initialization correctly collapsed navigation. Setting viewport/motion **before** navigation and asserting actual summary focus fixed the harness; no product workaround was added.

Observed RED→GREEN tracers: initial native table, invalid-address feedback, enabled action animation, pending local save and usable small-screen table. The first invalid-address test timed out on the unhandled native form navigation; the revised native-submit tracer reached the named missing-validation assertion. These setup/nonacceptance attempts are not counted as passing checks.

Limits: fixture interactions are not integration with the real application. The neutral comparison is a study presentation, not a shipped Graphite theme. No native WebKitGTK/Orca, Firefox or complete WCAG audit; no KNX/ETS compatibility, throughput or hardware claim. Input borders were strengthened and representative token contrast was measured, but this is not accessibility certification. No product dependencies or app source changed. No live server, Docker or bus touched.
