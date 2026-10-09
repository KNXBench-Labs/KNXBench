# Pages Node-24, IPv6 and offline demo website links

- Agent: codex; timestamp: 2026-10-09 07:52 CEST.
- Scope: user request “Pages-Actions auf Node 24 heben, IPv6 prüfen, Demos auf der Website verlinken”. Owner explicitly approved green-tests → commit/main push → public verification.
- Base: `0553f43c58c4d136d1eb81b4d82ea9eae9f25509`; isolated task branch/worktree. Root and foreign legacy worktree remain untouched.

## Evidence and review

Two new unit tests observed RED (missing demos section; old action refs), then GREEN. Official action metadata read at resolved commits, including composite upload's pinned Node-24 action. Upload v5 defaults to excluding dotfiles, so the inventoried `.nojekyll` is explicitly included. No forced-runtime override or other workflow/release change.

Static DE/EN/root demo download section reuses existing typography/layout. Direct GitHub downloads verified anonymously: all four ZIPs and SHA256SUMS exact; guide 200. All local package outer checksums verified; no archive bytes edited. Fictional/offline/English-guides/reset-copy/auto-save/candidate-label/older-alpha caveats retained.

Public IPv6 HTTPS GET via Globalping measurement `2iJor2k3SmrFCDtiN00021HcZ`: four finished probes, DE/NL/US/GB, 200 with authorized TLS, two distinct resolved IPv6 endpoints. All four Pages AAAA records resolve locally; no local public IPv6 route. Local `curl -6` fails before connection; IPv4 HTTP redirects 301 to HTTPS.

Final acceptance: website unittest 22; story unittest 67; preview/release builds; Chromium 62 landing + 26 imprint + 48 headlines + 18 default-language + 134 preview demos + 22 release + 134 release demos = 444 named checks. Fresh-target five repository gates and whitespace pass. EN/1440 screenshot: readable three aligned project choices and setup/safety/footer notes, clear focused download. DE/390 screenshot: stacked readable choices and wrapped bundle link, no clipping; top heading and lower safety notes are outside that frame and assessed separately by DOM checks. No broad visual/screen-reader claim.

Failures retained honestly: first Chromium startup died on too-long SingletonSocket path; corrected with a short owned profile-cache path. First mobile demo test exposed 38px inherited grid placement squeezing the download link; diagnostic confirmed it; scoped `.feature-list.demo-downloads article` block selector fixed it. Final full seven-recipe run passed.

Self-review after rereading complete source diff: CRITICAL 0 / IMPORTANT 0 / outstanding MINOR 0. No new secret, storage/parser/networking code, automatic external request or compatibility widening. No subagent, application release, server/container rollout, native/AT/full-Rust/corpus/ETS/hardware validation. Raw temporary outputs are not publication artifacts; the compact public-safe receipt is in `docs/evidence/pages-demos-2026-10-09.json`.

## Publication

Pending authorized commit/push and real GitHub Pages/live-file verification; results appended only after execution.
