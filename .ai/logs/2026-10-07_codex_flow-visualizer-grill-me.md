# Flow Visualizer usability follow-up — grill-me decisions

Date: 2026-10-07. Agent: codex (Hermes).
Status: interview decisions recorded; awaiting final scope confirmation before implementation.

## Context and inspected implementation

The user supplied a screenshot of their live house bus and requested a grill-me interview, not immediate coding. Main concerns: small flow area, crossing edges/crowded nodes, and missing project links in the otherwise useful right-hand Inspector.

Inspected repository context/handover, flow design/architecture/status/limitations, and these implementation surfaces:
- `apps/knx-web/src/TelegramFlowView.tsx`
- `apps/knx-web/src/flowLayout.ts`
- `apps/knx-web/src/flowDynamics.ts`
- `apps/knx-web/src/flowModel.ts`
- `apps/knx-web/src/flowFeed.ts`
- `apps/knx-web/src/BusMonitorPanel.tsx`
- `apps/knx-web/src/diagnosticsWindow.ts`
- `apps/knx-web/src/DiagnosticsCompanion.tsx`
- `apps/knx-web/src/main.tsx`, `App.tsx`, `styles.css`, package manifest.

Verified constraints: fixed 960 x 520 solver/view box; SVG CSS height capped at 32.5rem; Inspector grid column reserved even before selection; activity springs shorten busy pair distances; no flow Inspector project-navigation actions. Existing diagnostics window supports browser and Tauri but individual monitor mounts own distinct in-memory flow graphs. Sharing the backend monitor session does not alone share all previously accumulated graph membership.

## Confirmed user choices

1. Embedded view uses available width/height; Inspector appears on selection and is closable; add in-app maximization hiding competing workbench panels temporarily.
2. Dedicated Flow window in browser and Linux desktop, with Flow toolbar/Inspector. Repeated opening focuses the existing window; embedded Flow remains available. Opening/closing the view neither starts nor stops the monitor connection.
3. Fewer edge crossings and better readability take priority over stable node positions. Stronger rearrangement is explicitly permitted. Do not reduce this request to merely increasing spacing.
4. Device project link and all uniquely mapped GA entries in Inspector values/connections navigate to/reveal the relevant entity in the main editor. From a separate Flow window, focus the main window and leave Flow open. Uncertain, missing or incompatible-current-project targets have explained unavailable states, never guessed links.
5. Automatically rearrange in bounded/batched updates with settling, not per telegram; also provide an explicit rearrange action. Freeze prevents movement of existing nodes while facts/values/pulses continue.
6. New window immediately adopts the same previously collected session graph and continues live updates. Viewport/layout/selection may be independent per window (round-2 recommendation accepted with 'ja'). No additional bus connection or persistent traffic history.
7. Auto-zoom is wanted and must be switchable off. This overrides the interviewer's recommendation to keep manual zoom as the default after initial fit.

## Proposed synthesis details awaiting final confirmation

Auto-fit enabled by default, with an explicit per-view toggle. Update fit on relevant graph/viewport changes with controlled transitions rather than chase every solver frame. Turning it off preserves the current camera; manual pan/zoom turns auto-fit off so user input is not immediately undone; 'Show all' remains a one-shot action. These interaction details are proposed defaults, not separately answered interview questions.

## Implementation and verification obligations

- No guarantee of crossing-free arbitrary graphs; explicitly cover node/label collisions and edges passing through unrelated nodes as well as edge crossings.
- Preserve session identity, historical participant evidence, expiry and bounded/refusal diagnostics. Cross-window clock origins require care: do not directly reuse another document's `performance.now()` deadlines.
- Validate navigation against current project/entity identity, including project replacement and ambiguous addresses. Do not create a second editor in the Flow window.
- Preserve graph history across view switches and navigation within the current session; monitor restart keeps the existing session reset semantics.
- Test responsive sizing/Inspector toggles, automatic and frozen layout, auto-fit and manual override, late window adoption, blocked popup/focus behavior and entity navigation.
- Use synthetic/offline deterministic graph fixtures for verification. The screenshot is observational input, not new live bus or hardware authorization.
- Update relevant maintained flow documentation/status/limitations when implementing. No product edits or tests/builds were performed during the interview.

## Worktree awareness

At 09:52 CEST, root checkout remained `main`; `.ai/CURRENT_STATE.md` was this interview's change. `apps/knx-web/e2e/readme-hero.shots.ts` also became modified during the interview and is not owned by this task. Leave it untouched.
