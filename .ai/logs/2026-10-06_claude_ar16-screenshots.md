# AR16 slice 1: manual screenshots (Claude, 2026-10-06)

- User policy (2026-10-06): manual on GitHub, with screenshots.
- Fictional project generator `tools/manual_sample_project.py` (from the AR17
  smoke sample; links fixed so actuators receive and sensors send). Tests:
  determinism, members/counts, only `M-7FF0` appears.
- Release knx-server + production dist from `f7e98896`; Playwright spec
  `e2e/manual-screenshots.shots.ts` (2 tests, 2 passed, offline namespace),
  two server instances (open; password from /dev/urandom for the login page,
  never used to sign in).
- Exploration found: catalog opened from the explorer's "+ Add device" stays in
  the sidebar at this width, so the spec opens it from the Topology line "+";
  comm-object and help tabs needed explicit expansion; shared scroll needed a
  reset between views.
- Every picture read before commit; alt texts rewritten against it.
- Stale manual claims fixed: "Discover gateways" (7×) → bus monitor **Search**;
  welcome cards; help 11 topics (note removed); catalog row format; KV demo
  references.
