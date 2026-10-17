# Live website flicker investigation — 15 October 2026

## Owner observation

Linux, Chromium152.0. Flickering while scrolling both directions between
“Don’t just read about it. See it.” and “What is your building saying?”.
Scrolling remains possible; this is visual flicker, not a scroll-position lock.
Persists with heading animations manually paused. Input method and whether the
first video was playing were not established. Do not misreport absent scroll
position jumps as proof that this visual symptom is absent or fixed.

Additional owner observation: smartphone Chrome scrolls normally with no flicker.
Phone OS/model/browser version not supplied; this is one real-device observation,
not universal mobile compatibility evidence. Linux desktop Chromium152 remains
reported affected. This narrows the observed environment but does not establish
GPU/video/compositor causation. The desktop original-versus-poster A/B result is
still pending; no product patch, new probe or deployment was performed for this
observation.

## Published source and completed parent package

Source CRT publication merge d90b3d4b; docs closure3a9fd92f is on origin/main.
Website runs38789580428 and38790235057 both build/deploy success. Live38/38bytes
match accepted public artifact;124deployed Chromium checks passed before the owner
reported this new symptom. These earlier checks did not test GPU visual flicker.
Parent publication evidence archived separately with256individually verified
members. No feature rollback, speculative CSS change or additional push performed.

## Investigation measurements

- Dedicated clean-source worktree fix/site-scroll-20261015 on3a9fd92f.
- Actual installed `/usr/bin/chromium` is152.0.7977.82 Arch Linux, matching the
  reported major version; tests use a private headless context, not the owner's
  GPU/window/session/extensions. No DISPLAY/WAYLAND_DISPLAY in this agent runtime.
- Live Chromium four warm-media traces: natural/up/down/motion-off, document
  heights stable, each1800px of wheel input moves1800px with no reverse frames.
- Six cold-media/partially visible heading traces at390/1440 and top-65/-15/+150:
  actual heading glyph animation recorded, all1280px input moves1280px, no reverse
  frames or idle movement in Chromium. Thus the exact section was exercised.
- Same six traces in installed Playwright Firefox also complete1280px wheel
  progress, with at most0.5px idle shift/1px document rounding. This is diagnostic
  scope only, not general Firefox certification.
- Three additional traces with motion manually paused/reduced and pointer over
  heading, native-video centre and native controls completed. No rendering fix
  or visual-symptom reproduction follows from their scroll-position measurements.
- Existing runtime has no scroll/wheel/preventDefault handler; site.js controls
  video mutual exclusion and mobile menu only. Heading owner never calls scrollTo.

## Human-in-the-loop visual A/B (ready, not a production fix)

A loopback-only4202diagnostic server serves generated copies, never source/root:

- `/original/en/#watch`: original approved native-video page plus diagnostic label.
- `/poster/en/#watch`: same page and label; only first native video is replaced by
  its own genuine existing poster. Caption, surrounding content and remaining
  videos unchanged. The generated inventory explicitly says private-diagnostic.

Actual Chromium integrity recipe8checks passes: paths/labels/native-video counts,
real poster decoding, original headline/palette and renderer screenshots. No
fabricated media, no public deployment, no modification of product source or CSP.
This comparison distinguishes the first video-rendering surface from surrounding
layout; a positive result is still evidence to investigate, not by itself a
proven root cause or authorization for a permanent UX redesign.

Server proc_e14d40d40b08/PID649466. Evidence and private diagnostic copies are under
the profile task scratch `site-scroll-20261015`; retain while awaiting owner result.
The original live symptom remains OPEN/UNREPRODUCED in headless measurements.
Next: owner checks whether A flickers and whether B still does, then reproduce the
matching visual condition, write a discriminating regression and fix at source.
No subagents/vendor/hardware/root synchronization or speculative product patch.

## Closed (17 October 2026)

Owner verdict: the flicker was a problem of the owner's local desktop browser,
not of the website. No product change, regression test or deployment follows.
The A/B diagnostic server, the investigation worktree `fix/site-scroll-20261015`
and its private scratch are retired; this log is the remaining record.
