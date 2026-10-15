# Exact-edition story publication — 15 October 2026

The owner explicitly chose to publish edition `2026-10-15.4` and digest
`081d2c4c42b03a0b97ae803ac7f78375e4ca5593f3687453872200aab8398cf8`
after the preview, changes, full digest, real test results and limitations were
presented. This is a new exact approval, not a copy of the previous edition's.

The cutover adds the approval and changes the website pin, then retires the
previous candidate/preview/approval from the current tree. Their original bytes
remain in Git. The immutable new candidate remains byte-identical. Only this
edition and the approved publication route are authorized; no product release,
hardware action, manufacturer contact, history rewrite or root synchronization.

Fresh upstream at cutover includes unrelated P4 and community-recognition work.
The story cutoff remains the earlier explicitly approved source snapshot. Integrate
upstream additively in a separate owned tree, retain its runtime and handovers,
and gate the actual story/site release build. Do not relabel source-only work
beyond the cutoff as covered by this edition.

Publication acceptance now passes on actual merged `bb2961aa`: story76,
website24, tools163/one inherited skip, Chromium46/46 and Firefox46/46,
44 public-variant frames inspected across desktop/mobile engines. All eight
release nodes and editorial boundaries verified; zero overflow/outgoing requests.
Reference validation, matching approval, 38-file public build, five explicit-root
repository checks and docs pass. Full consumed-input manifest unchanged.
Fresh upstream remains `fbdfbddd`; runtime remains byte-identical to that source.

Push, Pages and exact public artifact readback are still separate pending steps.
Public acceptance is recorded without private paths or source transcript locators
in `docs/evidence/story-publication-2026-10-15.json`.
