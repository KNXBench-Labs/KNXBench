# knxbench.com Alpha 14 refresh — 18 October 2026

Owner request: update knxbench.com with Alpha 14 information, refresh the
screenshots "and so on", and keep "(no bus, no hardware …)" tags off the site.

## What changed

- Release section: four Alpha 14 cards (parameter workspace, vendor ZIPs,
  note decision before a delete, read-only diagnostic export), release link
  `v0.1.0-alpha.14`, new headline and intro in DE/EN.
- Media: hero, CRT/LCARS gallery and four card stills recaptured from the tagged
  source `8d104e2f` with the real server in a loopback-only namespace; all three
  videos re-recorded on Alpha 14 (group-details and project-work on the real
  server, bus-flow with the flow recorder's simulated telegrams). New poster
  `group-details.webp`; `alpha12-*.webp` removed from the site (the Alpha 12 PNGs
  in `docs/assets/releases/alpha12/` stay, the README still uses them).
- Copy: captions, alt texts, transcripts and the Docker hint without negative
  disclaimer tags; regression `test_site_copy_and_captions_carry_no_negative_disclaimer_tags`
  (observed RED on the previous content). Upgrade note: native 16, product DB 24,
  Alpha 13's code refuses the newer product DB.
- Recorders: `release-media.shots.ts` rewritten for the Alpha 14 scenes;
  `readme-hero.shots.ts` follows the Alpha 14 UI (folded navigation, card name,
  object editor summary, searchable address picker).

## How it was captured

Detached worktree at the tag, `cargo build -p knx-server` and `npm ci && npm run
build` there; captures under `flock` on the three web-gate lease files and in
`unshare --user --map-root-user --net` with `lo` as the only interface. The spec
files committed on `main` are byte-identical to the captured ones. Final runs:
release-media 1 passed (7 PNGs + clip), readme-hero 1 passed (57.7 s GIF, MP4 at
2x), readme-flow 1 passed. Failed predecessors were harness/locator issues of
the specs (folded navigation, a correct delete refusal for linked entities,
catalogue region name, dialog close), not application faults.

## Findings worth keeping

- Every group address and device of the manual sample house is linked, so a
  delete is correctly refused; a note-decision scene needs a newly created,
  unlinked entity (the in-range create action works well for that).
- `/sys/class/net` inside `unshare --net` still lists host interfaces (sysfs is
  not remounted); check offline-ness with `ip -o link | wc -l` = 1.
- A cargo target dir reused between the tag tree and `main` replayed a stale
  `knx-core` (missing `limits` module); build `main` tools in a separate target.

## Acceptance (self-review, Chromium only)

Website unittest 29, story unittest 76, docs checker 0 errors, anchors 1,512
none dead, 21 web tsconfigs, `git diff --check`, deterministic release build.
Offline browser recipes: preview 588 checks in 8 recipes, release 350 checks in
3 recipes, zero page errors and foreign requests. All stills and sampled video
frames inspected.
