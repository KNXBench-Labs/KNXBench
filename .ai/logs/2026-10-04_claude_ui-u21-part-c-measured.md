# 2026-10-04 — U21 part C: load measurement and the fixes it forced

Agent: Claude, goal-ui.md owner session; Web lock `de941a9e` (released in this delivery).

- Attempt 1 (dev server): "motion off" ran with motion on, because an init-script
  attribute is lost on parse. Markers shared a group with generated traffic.
  Not published. Consequence: the e2e "with motion off ..." test had passed
  without testing anything, since page.clock also froze rAF. It is split into a
  real-time case (attribute checked, 0 frames under live traffic; the
  motion-always-on mutant makes it fail with 61 frames) and an expiry case.
- Attempt 2 (dev build): plausible, but React development checks inflate it.
  The profile showed motion cost as native painting. Not published.
- Production runs (`vite build` + `vite preview`): 60 fps baseline kept as
  `measurements-before-frame-cap.json`; final `measurements.json` with the 30 fps cap.
- A test written for an assumed nudge hot spot was wrong about the hot spot
  but found a defect instead: `sync` absorbed rate changes without a reheat, so
  distances never adapted. Fixed with activity classes (RED; mutants
  fine-signature and class-change-ignored caught). Frame cap RED; no-cap mutant
  caught.
- Process slips, both recorded:
  - an edit and the dependent study were started in parallel (edit failed, study
    killed);
  - `pkill -f` matched its own shell.
- Review (in-session): IMPORTANT, activity adaptation (fixed). MINOR, the frames
  metric counts callbacks (documented). MINOR, the capped loop still takes a
  cheap 60 Hz callback.
- Full gate (first attempt): web build, fmt, clippy -D warnings, workspace tests 3,196 passed / 0 failed / 177 ignored in 175 blocks with 0 skip markers, five repository gates (headers 521 ok; anchors 453 ok; ledger 186 rows), tsc, `check:flow-study`, Vitest 2,001/116 files, complete Chromium suite 131/131, whitespace including new files; source frozen.
