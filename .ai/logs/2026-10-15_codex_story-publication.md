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

Normal main push and direct remote readback verify commit
`022d85a0ad3bdfe0e9862ea190e22898cc3a2f87`. Its Pages run `38701307431`
completed successfully. Actual public HTTP200 HTML is byte-identical to the
accepted build, SHA256 `8b9251fb64a3220b294657c95c3e0f673a98fb7565e1b1c1ad88860d5a9f9cb4`,
with the approved edition and story digest. These are observed delivery outcomes,
not inferred from approval or a local build. Final docs-only closure does not
alter any consumed story/site input or trigger another Pages build.
Public acceptance is recorded without private paths or source transcript locators
in `docs/evidence/story-publication-2026-10-15.json`.
