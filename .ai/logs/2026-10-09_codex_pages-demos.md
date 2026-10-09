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

Verified at 2026-10-09 08:01 CEST: first push was refused because concurrent Docker delivery advanced main to `8dc28a5f`. Local feature `f1d1061c` rebased to published `77ab6e97818ebd079292048dd9ffc1570089dd33`; both documentation conflicts preserve the exact upstream handover/status tail. Website/workflow source bytes unchanged; five repository gates, documentation checker and website 22 rerun green. Author/committer KNXBench <github@knxbench.com>, no co-author; exact local/tracking/live main readback succeeded.

Pages run `37890928052` build/deploy success. CI executed website 22/story 67 and the same resolved action commits. `.nojekyll` is in the uploaded archive. All 34 live files plus manifest exact; live Chromium 134 named checks, no errors/automatic external requests. Post-deploy IPv6 measurement `2GyiZtgGE1dNfEEeH00021Hct` has four 200/authorized-TLS results and the new Demos navigation; bodies are truncated by the service, not admitted as full-file equality. The official deploy action emits upstream DEP0040 punycode; no Node-20 warning observed. A nonessential check-suite lookup used an invalid run endpoint (404), corrected to commit/check-runs (build/deploy both success); it is not an accepted gate result.

Closure metadata changes only maintained documentation, evidence and handover; no deployed source change. Owned temporary browser/preview processes closed. Final metadata publication/readback and owned scratch/worktree cleanup follow this factual receipt; no foreign cleanup or root synchronization inferred.
