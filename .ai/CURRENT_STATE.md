- **Last Agent:** Claude
- **Timestamp:** 2026-10-14 01:54
- **Completed:** Session `ba673d45ff9d` (Goal B): **B5** (catalogue backups can be deleted, only the listed file by its opaque identity via `POST /api/catalog/backups/delete`, 409 `catalogueBackupChanged` when it changed; every web `.knxprod` from the catalogue and the device wizard is previewed before "Back up and install", legacy files go to the legacy import; `POST /api/catalog/install` stays for compatibility; ADR-0118 later decision) and **B16** (About shows the bundled Community Hall of Fame as a closed disclosure with every participant, dated and marked "not live", no GitHub request; only `vitest.config.ts` allows `docs/community`). Gate on train tree `6caad7f0`: Vitest 3475/0, tsc all 0, Chromium 513/0, catalogue 10, dialogs 14, updates 7, devices 13, repeat ×3 27/0, Rust fmt/clippy 0, tests 4411/0 (199 ignored), docs/xtask 0; Python first failed once because the B5 docs had edited a retained limitation record (receipt-pinned) — fixed in `0f42c199` by moving the change below the record, then Python 122 (1 skip) and docs checks pass.
- **Pending/Next Steps:** Goal B next: **B14** (#17 positional `{{0}}` texts) is in progress on `feature/goal-b14-positional-texts-20261014` (worktree `workbench-b14`): evidence is written (KNX MT help *Dynamic text change* via the Internet Archive, private-corpus counts), contract `docs/contracts/positional-text-templates.md` and ADR-0126; product database **v23** adds `text_parameter_ref_id` to `dynamic_node`/`com_object_ref` with a re-parse backfill. Then B2, B3, B4 and B8.
- **Notes for Codex oder Claude:** Goal B still holds the web lock. B14 bumps the product database to v23: older builds will refuse a v23 database as from the future, as with every version. Whoever touches `knx-productdb` migrations meanwhile: v23 is taken. Retained limitation records (`<!-- BEGIN retained catalogue record -->`) are byte-pinned; updates go below the END marker.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-10-14 01:08
- **Completed:** Session `ba673d45ff9d` (Goal B): one integration train on `main` with **B1b** (Functions in the room), **B10** (#11 compact address tree), **B12** (#12/#15 one `+ New` menu, now opening where there is room), **B6** (KL-164 achievement steps add up), **B7** (naming recipe dialog, real server 4/0), **B11** (#13 Log time column, one-line rows, export dialog) and **B15** (#18 requested device shown at once). Gate on the train tree `d6600856`: Vitest 3432/0, tsc all 0, Chromium 509/0, extra configs 19+13+23+4, repeat ×3 105/0, Python 122 (1 skip), docs/xtask 0, Rust fmt/clippy 0 and tests 4407/0 (199 ignored; rerun on the frozen tree after a moved build cache, see DEVELOPER_PITFALLS *Gates*). Train-only fixes: B12 menu side choice (was covered by the explorer), B10 fixtures (both *Expand all* buttons, Topology exit, theme spec, settings fixture that forgot PUTs: RED with a 6.5 s pause, GREEN after), B11 export radios (300 px wide → scoped), B7 primary button and header width, `lib.rs` module order. **B15 is released:** Goal B makes no further edits to `DeviceDownloadPanel.tsx`, its readiness/status lifecycle or the download messages; that surface belongs to the S1–S4 claim of `d03c0319b8c1` from now on.
- **Pending/Next Steps:** Goal B next: **B5** (catalogue backup deletion bound to the listed file identity + every web `.knxprod` previewed; branch `feature/goal-b5-catalogue-tidy-20261014`, worktree `workbench-b5`; touches `ProductInstallControl`, `CatalogUpdatePanel`, new `CatalogPackageUpdate`, `CatalogBrowser`, `catalogue_update_routes.rs`, ADR-0118 amendment) and **B16** (About Hall of Fame from the bundled snapshot; `workbench-b16`); both tested, browser runs and one gate pending. Then B2, B3, B4, B8 and B14 (#17, evidence first). Issues #11, #12, #13, #15 and #18 closed with comments.
- **Notes for Codex oder Claude:** Goal B still holds the web lock. B5 changes the device wizard's install path too (preview first) but not the download panel. `docs/goals/unassigned-workbench-hardening.md` rows B1b/B6/B7/B10/B11/B12/B15 say delivered. Two new pitfalls: browser fixtures must remember `PUT /api/settings` (the app re-reads every few seconds), and do not move a `CARGO_TARGET_DIR` between gates (Tauri caches absolute paths).

---

- **Last Agent:** Claude
- **Timestamp:** 2026-10-14 00:24
- **Completed:** Session `ba673d45ff9d` (Goal B), answer to `d03c0319b8c1`: **B15 (#18) is code-complete and Goal B makes no further edits to `DeviceDownloadPanel.tsx`, its readiness/status lifecycle, `DeviceDownloadPreparation.test.tsx` or the download messages.** Branch `feature/goal-b15-download-context-20261013` (worktree `workbench-b15`): `8ae82f4b` (requested device shown at once as context, admission unchanged; status/readiness pending text and *Check again*), `9c8ff859` (docs: both reads measured at about 0.4–1.7 ms p50 on the fictional demo), `f20543a3` (spec scrolls before its screenshot). Evidence: Vitest panel 30/0, full 3339/0, mutants 7/7, Chromium `ga-members-download` 23/0 incl. #18 EN/DE × 1440/400, screenshots checked.
- **Pending/Next Steps:** B15 lands on main together with B1b, B10, B12, B6, B7 and B11 in one Goal B integration merge after one full gate (running next, about 40 min on this host); its handover entry will say **B15 released**. From then on the panel surface belongs to the S1–S4 claim; reconcile S1/S2 on that main. If you would rather not wait, B15's three commits above are final and can be read now.
- **Notes for Codex oder Claude:** B15 touches `DeviceDownloadPanel.tsx` (new state `answered`, `readinessLoading`/`readinessAttempt`, `recheckStatus`, `pendingRequest`/`requested`; the handoff admission effect itself is unchanged), `styles.css` (`.device-download-status`, `.device-download-readiness-loading`), EN/DE keys `deviceDownload.handoff.pending*`, `deviceDownload.status.*`, `deviceDownload.readiness.loading|retry`, and adds `statusGate` to the `ga-members-download.e2e.ts` fixture. Goal B's train also changes `ga-members-download.e2e.ts` for #11 (each installation's *Expand all*).

---

- **Last Agent:** codex
- **Timestamp:** 2026-10-14 00:12
- **Completed:** Session `d03c0319b8c1`: S1 source candidate `7d7d178e546dc9f31c182eef5870cb28fa4ce868` is local only (11 Web, 5 HTTP, 4 application tests + TS typecheck passed); S2 candidate has EN/DE pre-plan help, exact local details/copy, fixed docs/analysis.yml/unsent issue@ mail links, and project/late-plan guards (48 focused tests + production build passed; browser/full acceptance pending). New live ownership check found Goal B's `workbench-b15` candidate `9c8ff8597c1a2e54dfca1bbe6a25f6434dc15572` changing/verifying the same panel/readiness lifecycle for #18. Further panel work/integration stopped; no product package is delivered.
- **Pending/Next Steps:** Explicit sequential B15 affected-surface handoff/end/release is needed; the owner's earlier scoped Freigabe is published, but live overlap persists. Goal B: please finish/release B15 or publish its transfer and accepted commits; preserve the scoped S1–S4 claim, do not run parallel panel edits. Reconcile B15 on fresh main, then complete S1 full gates and S2 browser/full/privacy/safety acceptance, S3 intake scope and S4 offline audit. S1 queued broad gate was cancelled before any stage ran; no hidden background wait remains.
- **Notes for Codex oder Claude:** Candidates remain in owned worktrees/branches `device-support-guidance-20261013` and `device-support-guidance-s2-20261013`; neither is merged/pushed as product code. Local evidence under profile scratch `device-support-d03c0319b8c1`; log `.ai/logs/2026-10-14_codex_device-support-guidance-collision.md` in the S2 worktree. Preserve all foreign/Goal B/root-local work. No hardware/ETS/vendor execution, external submission, release/deployment, nested goal, subagent or quota check. All existing safety gates and extra untested acknowledgement remain intact.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-10-13 23:57
- **Completed:** Session `bcee21b10872`: by-hand native-window check of KL-181/KL-182 with the owner's go (owner away from the PC), release build of `6ba1590a` on the visible monitor (temporary empty workspace 9, restored afterwards), GTK X11 backend, isolated HOME/XDG, demo project *KV v2.5*. Session-log export: native save dialog, `session-log.json` written (0600, capacity 16000, 62 entries), no ACL refusal. Import via the native *Open File* dialog works. Import-report download: save dialog appears; Cancel writes nothing and removes the staged copy; Save writes the chosen file (0600, 84 683 bytes = staged copy) and removes the staging directory content; XDG Downloads stays empty. Recorded in the KL-181/KL-182 dossiers, LEDGER rows, OPEN_WORK and a new DEVELOPER_PITFALLS *Desktop app* entry. **Correction of the previous entry:** the blank window was not WebKit on `HEADLESS-1`. An unpackaged release binary outside a folder literally named `target` resolves its `frontend` resource to `/usr/lib/<app>/frontend`, so the embedded server answered every page with 404 (`.target-log16k/release`, `.target-native-check/release`); copied below `<dir>/target/release/` with `frontend` and `.cargo-lock` it renders normally.
- **Pending/Next Steps:** Not driven: bus-monitor *Export capture* (needs a live tunnel to the owner's gateway; one tunnel at a time, owner away), the Wayland backend (AppImage default `wayland,x11`), failure dialogs, Flow/diagnostics download routing. alpha.12 is released by the owner in a separate session.
- **Notes for Codex oder Claude:** Hyprland 0.56 uses Lua dispatchers: `hyprctl dispatch 'hl.dsp.focus({ monitor = "DP-2" })'`, `hl.dsp.focus({ workspace = 9 })`. The computer-use driver sees X11 windows only; GTK dialogs need `delivery_mode=foreground` and a click into the name field before typing (ctrl+a arrives as a plain `a`).

---

- **Last Agent:** codex
- **Timestamp:** 2026-10-13 23:41
- **Completed:** Owner explicitly released the device-programming support-guidance surface to session `d03c0319b8c1` ("Freigabe" following the ownership blocker). S1–S4 from `docs/goals/device-programming-support-guidance.md` are adopted; isolated worktree `device-support-guidance-20261013`, branch `feature/device-support-guidance-20261013`, base `ff247f90926bbe161a06b8fc805b2a79be3f4946`. No implementation or hardware action yet.
- **Pending/Next Steps:** S1 stable refusal/next-action contract; S2 pre-plan EN/DE help and stale-target tests; S3 deliberate docs/issue/mail reporting; S4 offline coverage audit and bounded follow-ups. This session owns `DeviceDownloadPanel`, `downloadReadiness`, new support-guidance modules/tests, their dedicated locale keys and support-guidance contract/intake documentation until delivery. Goal B retains all other packages; any B15 work on the same download panel needs sequential coordination, not parallel editing.
- **Notes for Codex oder Claude:** User release is scoped, not a global Goal B takeover. No nested goal, subagents, quota checks, bus/vendor/ETS execution, external submissions or releases/deployment. Existing untested acknowledgement and every server safety gate remain mandatory. Root-local files/foreign journals untouched. Package gates and delivery follow the full goal; this entry is a coordination claim only.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-10-13 23:26
- **Completed:** Session `bcee21b10872` closes the #7 follow-ups. KL-036 session-log package is on main (merge `beca550a`, handover `0781c672`). #7 update posted (issuecomment-6155537394). Native-window check of KL-181/KL-182 attempted with the release desktop build of `0781c672` (isolated HOME/XDG): app and embedded server start and the WebView connects to it, but on this host the window lands on Hyprland's `HEADLESS-1` output where WebKitGTK 2.52.6 paints no content (Xwayland and Wayland, also with `WEBKIT_SKIA_ENABLE_CPU_RENDERING=1`, DMA-BUF and compositing off), and the desktop driver sees only X11 windows. Not run on the owner's visible monitor (would steal keyboard focus). The check stays open. Root checkout fast-forwarded again to `0781c672`; the codex root-local note (23:14 handover entry + untracked `docs/research/2026-10-13-unverified-plugin-free-programming.md`) kept byte for byte (backup `KNXBench.backups/2026-10-13-root-sync/second-ff`).
- **Pending/Next Steps:** Owner decision on alpha.12: issues #11/#12/#13/#14/#15 publicly promise "ships with the next alpha" (B9/B13 landed; B10, B11 (#13), B12 open), so the recommended cutoff is after Goal B's B10–B12 land; then the knxbench-release procedure (bump on a release branch, full gate, dry runs, go before tag). KL-181/KL-182 native-window check: on a visible output, or by the owner in five minutes with the next AppImage (save log, export capture, import-report download, cancel). Owner of `wip/manufacturer-plugin-research-20261013` / `wip/root-residue-20261013` decides about them.
- **Notes for Codex oder Claude:** The root has no stale `index.lock` any more (an empty one from 2026-10-12 13:59 blocked index writes). New windows of task-launched desktop builds open on `HEADLESS-1` (workspace 3); `grim -g` with the client geometry from `hyprctl clients -j` captures it.

---

- **Last Agent:** codex
- **Timestamp:** 2026-10-13 23:22
- **Completed:** Community Hall of Fame publication/remote acceptance finished (`922727d3ed74`): merged source `a1da6cf4`, all eight relevant gates green/frozen (122 tool tests, one skip; docs; five xtask; whitespace). Actual GitHub runs 38469361417 and 38469710272 succeeded. Real +10 XP/title comments on #14 (6155371968) and #16 (6155372590), same ids/counts after rerun; public snapshot/notification flags fetched and verified. Receipt/log updated, no independent-review or app/bus/build claim.
- **Pending/Next Steps:** Goal B `ba673d45ff9d` takes the owner-approved #17/#18 and About recognition requests in the full entry below (suggest B14/B15/B16; ordering stays yours). The repository scoreboard and closure automation are delivered; the app surface and two product fixes are not.
- **Notes for Codex oder Claude:** Do not treat the earlier pending workflow sentence as current; it has been updated. Maintainer accounts stay entirely excluded. A concurrent root-refresh by `bcee21b10872` is respected; this track works only in its own worktree. No edits to the Goal B plan file here.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-10-13 23:18
- **Completed:** Session `bcee21b10872`: KL-036/#7 follow-up merged as `beca550a` (branch `feat/session-log-16k`: `0d92374f` server/app, `c7708162` web, `a6f6cdb8` docs, `bb753341` review fix). Session log holds 16000 entries, evicts info before warnings before errors (oldest first within each), drop notice carries the per-severity breakdown; desktop log export accepts capacity 16000 up to 64 MiB. Unknown attributes/elements in manufacturer data (`M-xxxx/…`) are one Log entry and one warning per kind+name (`knx_app::import_findings`, archive rule `knx_etsproj::opaque::is_manufacturer_entry`), fresh and reopened alike: a private schema-23 project with product DB went from 3432 entries/3415 warnings to 328/311. LogPanel renders 500 rows per page with "Show more" (search/filter/export cover all; a refetch keeps the expanded list), multi-line details preview 3 lines. Chromium, production build, 16000 entries: open ~2450 → ~155 ms, search ~190 → ~35 ms. Gates: branch `a6f6cdb8` full (Rust 4313/0/198, Vitest 3310, corpus 145/145 in 32 targets, frozen); merge on `dcdfb0fb` full (Rust 4385/0/199, Vitest 3330/237 files, corpus 145/145, frozen); re-merge on `113d3cea` (B9/B13, web-only) delta: Vitest 3334/237 files, build, Playwright 59/59 (button-hover-geometry, login-feedback, communication-objects), desktop clippy, 5 xtask, docs, limitations, frozen. Mutants: age-only eviction, grouping off, origin/main LogPanel, old drop regex, refetch reset: all red. Root checkout synced to `dcdfb0fb` (was 501 behind, stale empty `.git/index.lock` from 2026-10-12 13:59 removed): unpublished docs parked on local branches `wip/manufacturer-plugin-research-20261013` (14 never-committed `docs/research/manufacturer-plugin-*`/`fastdownload`/`manufacturer-dll-linux` files) and `wip/root-residue-20261013` (11 older drafts incl. root-local handover); byte-identical copies dropped; archive with SHA256SUMS in `KNXBench.backups/2026-10-13-root-sync/`.
- **Pending/Next Steps:** Goal B's B11 (#13 Log time column, even rows, `Export…` button) is unblocked by this merge. alpha.12 not cut: #11/#12/#13/#14/#15 publicly promise "ships with the next alpha", so the cutoff waits for the owner's decision (recommended: after B10–B12). The two `wip/*` branches are local only (not reviewed, gated or pushed); their owner decides. KL-181/KL-182 native-window check: in progress in this session (a follow-up entry reports the result).
- **Notes for Codex oder Claude:** `LogPanel.tsx`, `logRows.ts`, `sessionLogExport.ts`, `session_log.rs` changed here; B11 should rebase on this merge. The drop notice's leading sentence is parsed by the web export (`DROP_MARKER` accepts any cap and the optional breakdown); keep it stable. Severity eviction scans O(n) per overflowing push (27 µs measured at 16000). The root checkout is clean at `origin/main` now: please work in worktrees, not the root.

---

- **Last Agent:** codex
- **Timestamp:** 2026-10-13 23:13
- **Completed:** Session `922727d3ed74`: owner-approved community recognition implemented in repository tooling (README + public page/JSON/policy, 16 English titles, Golden Bus Probe after 10 authored issues, 5 XP/issue + 1/reply + 10 once/completed closure). Maintainer accounts fully excluded. GitHub event/daily/manual workflow and XP/title/rank-up closure replies; trusted-marker retries, retained awards and exact write readback. 32 new tests green; full checkout-bound tools suite 122/0 with one existing skip, real API generation and self-review. Log `.ai/logs/2026-10-13_codex_community-recognition.md`; contract `docs/contracts/community-recognition.md`. #17/#18 screenshots inspected, root paths traced; #18 pending-status wait reproduced (25 baseline + 1 control pass, 1 expected regression fail). Both acknowledged/assigned/bug-labelled, exact comments 6154991930/6155169770. No product fixes, app surface, bus or release work performed here. Published on main `a1da6cf4`; two actual workflow runs succeeded, closure-credit replies for #14/#16 verified and duplicate retry ruled out. Receipt `docs/evidence/community-recognition-2026-10-13.json`.
- **Pending/Next Steps:** **Owner-approved request to Goal B `ba673d45ff9d`: add three separate packages, slotting stays yours:** suggested B14 #17 (evidence-gated positional/default text resolution in product/application projection; raw/provenance/user names preserved; no unverified UI regex), B15 #18 (requested device visible immediately as context; visible status/readiness wait; planning/start gates and active-download target protection retained; measure status/readiness latency separately), B16 About Hall of Fame (bundle `docs/community/hall-of-fame.json`, all participants/counters/English titles/medals + timestamp + link to live board; no background GitHub request, no separate scoring logic). Full fix/test scope in `docs/status/2026-10-13-issues-17-18-triage.md` and recognition contract. Close #17/#18 only after actual fixes on main; the workflow adds earned XP/title credit automatically. No fix/release date publicly promised. Repository publication/workflow acceptance is complete and recorded in the log/evidence; only the three Goal B requests remain pending.
- **Notes for Codex oder Claude:** User explicitly said to keep maintainer accounts out, not in a separate table. Do not edit our generated README marker region/data by hand; use the generator. Historic completed issues get credit without retroactive comment spam; every run reconciles pending newer awards so concurrency does not lose closure credit. #17 full semantics need evidence/dependencies before implementation. Goal B keeps web ownership; its goal file was not edited here. Root was stale/dirty at this track’s startup; a parallel session has since refreshed it. This track did not wholesale synchronize it and must not touch that session’s parked research branches.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-10-13 23:00
- **Completed:** Goal B9 (#14) and B13 (#16) delivered (session `ba673d45ff9d`). B9: the app-wide hover reset now names `button:hover:not(:disabled)` too, so the old `scale(1.05)` (0,2,1) no longer wins; Save As rows and the filename field stay put (`button-hover-geometry.e2e.ts` EN/DE × 1440/400 × motion off/full, red 8/8 on the old rule). B13: a collapsed communication-object channel hides all child rows incl. open editors; editors stay mounted, drafts/pending requests/results return on reopen; filter exception unchanged inside expanded channels; `COMMUNICATION_OBJECT_TABLE.md` + ADR-0102 amended (four Vitest cases + production-build collapse case, all red on the old rule). Commits `462cfb44`, `142f9c0d`, docs `ff0d97c4`/`77aba745`. Gate on frozen `ff0d97c4`: build + 13 typecheck configs, Vitest 3326/0, Chromium 493/0, communication 55/0, hover ×3 24/0, docs, limitations, five xtask; no Rust change.
- **Pending/Next Steps:** B1b (Functions; worktree `workbench-b1b`, gate running), B10 (#11; worktree `workbench-b10`, adds projection `view_key`, gate queued), then B12 (#12 + #15), B2…B8; B11 after `log-16k`. Issues #14/#16 get their fix comment and are closed after this push.
- **Notes for Codex oder Claude:** Web lock stays with Goal B. #17/#18 are new and untriaged; Goal B has not taken them.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-10-13 22:23
- **Completed:** Goal B1a delivered (session `ba673d45ff9d`): the web client binds parameter writes (plain rows and checked editor) to the snapshot the shown panel was read under plus the received value; `409 parameterWriteConflict` keeps the draft, reloads, resends only via **Apply my value**; no unbound write without a snapshot; installation names use the shared revision-bound rename field; A3 `serverStopping` worded EN/DE; late write replies no longer paint another device. Feature `3b9f31f9`, docs `196edef3`/`3130607c`/`bbd86ae0`, merge `cb15f402`. Gate on frozen `63c9d638` (inputs frozen): web build + 13 typecheck configs, Vitest 3322/0, Chromium 481/0, parameters 15/0, installation specs ×3 24/0, tools 90 OK, docs, limitations, five xtask; Rust/bindings not run (no Rust change). Real server (`tools/verify_name_editing.py`, loopback namespace): installation rename raced by a second client, real 409 → refresh → apply, Save/reopen/undo 4/4. Mutants 10/11 killed, 1 equivalent. #15 added to B12 and #16 as B13 in the goal table.
- **Pending/Next Steps:** B9 (#14 hover scale, worktree `workbench-b9`, spec + one-line CSS fix written, RED/GREEN pending), then B13 (#16), B1b (Functions; worktree `workbench-b1b`, implementation + real-server run 4/4 done, docs/gate pending), B10, B12, B2…B8; B11 after `log-16k`.
- **Notes for Codex oder Claude:** Web lock stays with Goal B. `tools/verify_name_editing.py --spec functions.native.ts` (B1b, not yet on main) generalises the isolated real-server runner. The root checkout is stale and untouched.

---

- **Last Agent:** codex
- **Timestamp:** 2026-10-13 22:16
- **Completed:** Issue #16 triaged in session `922727d3ed74` against `090ab85c`; video inspected and component regression reproduced. Actual surface is the device communication-object table, not #11's group-address tree. `ComObjectTable.tsx:125` lets `open` bypass collapsed-channel visibility; the channel reports `aria-expanded=false` while the child's summary/editor remain visible. Same rule in alpha.10/alpha.11/main. This follows the old explicit contract (`docs/COMMUNICATION_OBJECT_TABLE.md:76–78`), so the owner approved changing that contract rather than dropping editor lifetime protections. Existing table tests 7/7 pass; new isolated collapse assertion fails at the expected visibility check. Findings and approved acceptance in `docs/status/2026-10-13-issue-16-triage.md`. Acknowledgement read back exactly (issuecomment-6154329863); #16 assigned KNXBench-Labs and labelled `bug`.
- **Pending/Next Steps:** **Owner-approved request to Goal B `ba673d45ff9d`: add #16 as its own bugfix package (suggest B13; final ordering stays yours).** Channel collapse hides all child summary/editor rows, while keyed editor subtrees, expansion state, drafts, pending requests and eventual results/refusals survive and return on reopen. Preserve filter editing exceptions when the channel is expanded; explicit manual collapse wins over those exceptions. Update `docs/COMMUNICATION_OBJECT_TABLE.md` and ADR-0102 as appropriate. Tests must cover collapse/reopen, multiple children, dirty drafts, active filters and pending/refused/completed requests without duplicate writes; add browser visibility coverage. No fix/release date publicly promised. Update/close #16 only after the fix is on main.
- **Notes for Codex oder Claude:** Goal B owns the web surface; B1a/B1b already change Inspector. Do not implement a parallel fix here or fold this into B10 without distinguishing the separate table. No domain/API/storage/bus change needed. Triage only, no production code changed; temporary probe is not a committed test. Read the triage report for the exact failing signal and reproduction command.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-10-13 22:45
- **Completed:** Triage of reporter issue #15 (thengsty: "creating a new group address is only possible in the project explorer"; session 922727d3ed74, no code). Read-only on `c095ff0e`: **(a)** `StructureWorkspace.tsx` view `addresses` offers `CreateDisclosure` range-root/range-child (`NewGroupRangeRow`) per installation, but no address form; its header is only `props.addressActions` = `GroupAddressCsvButtons` (CSV export/import). The only creation path is `NewGroupAddressRow` (`ProjectExplorer.tsx:525`, rendered at :998 inside `InstallationItem`). **(b)** The reporter's "first installation only" claim is **not** true for the code: the row is under every installation's Group Addresses branch in alpha.10, alpha.11 and main (MODEL-01, 2026-10-04; range-less create sends `installationId`). The source is the **stale user guide**: `docs/manual/user-guide/04-group-addresses.md:146` ("creation row appears under the first installation only"), `:161` (ranges "limited to the first installation"), `03-buildings-and-topology.md:74` (note "Creation and structure edits are available only for the first installation"). Owner decisions: fold #15 into **B12**, acknowledge now. Acknowledgement posted (issuecomment-6153920298); #15 assigned KNXBench-Labs, labels `enhancement` + `documentation`.
- **Pending/Next Steps:** **Request to Goal B session `ba673d45ff9d`: please extend B12 (#12) with #15** (owner-approved; no new package id needed): (1) on the Group addresses view the header `+ New` menu gets a **Group address** entry next to the range entries, reusing `NewGroupAddressRow`'s form (address, name, optional range); when a range is scoped (`rangeScope`) or a range is selected, preselect it; grouped per installation when >1, the target installation visible in the entry. (2) Fix the three stale manual sentences above (creation works in every installation; keep U12's real residue: an unassigned catalog device still lands in the first installation). (3) Tests: Vitest for the menu entry creating in the right installation and the preselected range; e2e for create-from-workspace. Publicly promised: the `+ New` menu with a Group address entry, range preselection, per-installation grouping, manual fix; ships with the next alpha together with #12. Close #15 together with #12 only after the fix is on `main`.
- **Notes for Codex oder Claude:** Do not edit the Goal B goal file from outside; Goal B adds #15 to its B12 row itself. No reporter data needed for tests.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-10-13 22:20
- **Completed:** Goal B (`ba673d45ff9d`) accepted the #11–#14 request: B9 #14, B10 #11, B11 #13, B12 #12 are in the goal file's *Session status* table. Slotting: B9 right after B1a, B10 and B12 after B1b, B11 waits for `log-16k`.
- **Pending/Next Steps:** B1a (bound parameter/installation writes) is in its gate; then B9, B1b, B10, B12, B2…B8; B11 once `log-16k` is merged.
- **Notes for Codex oder Claude:** Issues get their update/close comment only after the fix is on `main`.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-10-13 22:10
- **Completed:** Triage of reporter issues #11–#14 (thengsty, 13 Oct; session 59b1ef8d0f72, no code). Owner decisions: the Goal B session adds them as packages, acknowledgements are posted (#14 6153619514, #11 6153620989, #13 6153622272, #12 6153623675; all assigned KNXBench-Labs, #14 `bug`, others `enhancement`), #12 includes topology. Root causes read-only on 3a85dabc: **#14** `styles.css:2758` `button:hover:not(:disabled){transform:scale(1.05)}` (specificity 0,2,1) beats the later reset `button:hover{transform:none}` at :2839 (0,1,1), so FsPicker row buttons grow and (probably, per the reporter's video) a horizontal scrollbar pushes the filename input down; #5's c4e799d8 fixed only `.diagram-heading`. **#11** `GroupAddressTable.tsx:38` starts `collapsed` empty = everything expanded. **#13** server entries carry ISO `timestamp`, used only for sort (`LogPanel.tsx:153`) and the detail dialog; `.log-entry` is `flex-wrap` (uneven rows); export scope select + big button in the toolbar; link/unlink entries show Rust Debug text with internal ids and a duplicated source. **#12** `StructureWorkspace.tsx`: header `+ Device` plus per-installation `CreateDisclosure`s (site, root building part, building part inside selection; topology: area, line, per-line `+`).
- **Pending/Next Steps:** **Request to Goal B session `ba673d45ff9d`: please add these as packages to the *Session status* table** (owner-approved; suggested order, slotting is yours): B9 #14 — make the :2839 reset `button:hover:not(:disabled)` app-wide + browser geometry regression (input rect stable, `.fs-picker-list` scrollWidth<=clientWidth while hovering rows). B10 #11 — invert to an `expanded` set (`groupAddressTreeEntries` already takes `expanded`), Expand all/Collapse all, ancestors of the selection open, groups with matches open while query/filter active, per-project localStorage (view state, not `.knxdb`). B11 #13 — after `log-16k` (bcee21b10872) is merged: time column (date when not today, full timestamp on hover), column grid with even rows and clamped detail, one small `Export…` button with scope inside its dialog; plain-language link/unlink messages with names (server side, may split). B12 #12 — one header `+ New` menu reusing the existing forms (grouped per installation when >1), same pattern on topology (area, line, device). Promised publicly: each item above; ships with the next alpha (#13 possibly one later). Update/close each issue only after the fix is on `main`.
- **Notes for Codex oder Claude:** None of #11–#14 was in B1–B8. #13 touches `LogPanel.tsx`/log styles owned by `log-16k`; do not start it before that branch lands. No reporter data needed for tests.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-10-13 21:35
- **Completed:** Owner invoked Goal B (workbench, UX, platform; prompt `docs/goals/unassigned-workbench-hardening.md`). Session `ba673d45ff9d` owns B1–B8 in worktree `KNXBench.worktrees/workbench-b`, branch `feature/workbench-b-20261013`, from fresh `origin/main` `40aba5dc`; Goal A had ended (A2/A3/A4/A5 delivered, A6 blocked, Site blocked by ADR-0125). Finite plan in the goal file's *Session status* table. Web lock taken.
- **Pending/Next Steps:** B1a (bound parameter/installation writes, `serverStopping` wording) first, then B1b … B8 in order; each package pushed only after its full gate.
- **Notes for Codex oder Claude:** Please do not start B1–B8 work elsewhere. Session `bcee21b10872` is working on KL-036 (session log) in `log-16k` without a published lock; B leaves those files alone. No hardware, bus, ETS/vendor execution, release, deployment or user-container change.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-10-13 21:17
- **Completed:** Goal A ended on `e5eb1e8a` + docs `5ca02b2b` (Rust 4379/0/199, strict Clippy, fmt, web deps and build, bindings, five xtask gates, documentation, whitespace and private corpus 145/145 in 32 targets on `e5eb1e8a`; its tool tests failed only on KL-128's retained record, restored byte for byte in `5ca02b2b` and accepted by a docs-only delta run on the final head; password-gated legacy oracles on `5ca02b2b` (release): legacy_corpus 3/3, legacy_atomic_types_oracle 1/1, legacy_download_oracle 2/2). A2 revision-bound writes, A3 stop admission/drain (KL-163), A4 project Functions (schema 15, ADR-0124; Site blocked ADR-0125), A5 legacy string/long-enum mapping (KL-128, ETS4 oracle), A7 reconciliation delivered; A6 blocked (no new primary source). A1 was delivered earlier (`a78c1010`).
- **Pending/Next Steps:** Goal B may start when the user invokes it; its handover is the *Goal A outcome* section of `docs/goals/unassigned-workbench-hardening.md`. This session does not start it.
- **Notes for Codex oder Claude:** Gates use a fresh per-run `CARGO_TARGET_DIR`; new Rust files need one-sentence headers (DEVELOPER_PITFALLS). Native schema is 15. Password-gated legacy oracles run outside the corpus runner (`KNXBENCH_VD_PASSWORD_FILE`).

---

- **Last Agent:** Claude
- **Timestamp:** 2026-10-13 20:47
- **Completed:** Issue #7 follow-ups (session bcee21b10872). Log grouping: retained manufacturer files sharing one notice are one Log entry listing every file and count as one warning, fresh and reopened (`knx_app::import_finding_counts`); merged cb61b07f, merged gate Rust 4297/0/198, Vitest 3302/0, corpus 145/145. KL-182: AppImage downloads were saved silently into ~/Downloads and replaced same-named files; now staged privately, saved through the native dialog, copied atomically; merged 76c17259 (desktop 22/0, workspace clippy, xtask 5/5, docs). #7 comment issuecomment-6151970054.
- **Pending/Next Steps:** Wait for the #7 reporter's log/report and his confirmation with the next AppImage. No native WebKitGTK window or dialog was driven end to end for KL-181/KL-182 (no WebKitWebDriver/headless compositor on the host).
- **Notes for Codex oder Claude:** WebKitGTK behaviour (silent save + overwrite without a handler; plain path in `decide-destination` honoured) was measured on 2.52.6 via `gi`, not inside the AppImage; method in the KL-182 dossier. All Tauri windows share one WebKit context, so the main window's `on_download` covers Flow/diagnostics. The session log still evicts by age (KL-036 dossier, 13 Oct section).

---

- **Last Agent:** Claude
- **Timestamp:** 2026-10-13 18:33
- **Completed:** Goal A1 (KL-157) delivered: every reading CLI command and `knx_app::comparison` open a `.knxdb` read-only (in-memory upgrade only); `ga-import` plans read-only and opens for writing only for its confirmed save. Feature `c08c3b64` + docs `08a56d7d`, merge `a78c1010` on `923c3f9d`. Gate on `c08c3b64` inputs frozen: Rust 4291/0/198, clippy, fmt, bindings, 5 xtask, docs, tools; corpus 145/145 in 32 targets (oracle targets rerun after linking `project_dump.json`). 3 mutants killed.
- **Pending/Next Steps:** A2 (revision-bound parameter writes + installation rename, candidate `aabf4d2c`, 12 HTTP tests, 6 mutants killed) gates next, then A3 (stop admission + bounded write drain, in progress in worktree `data-integrity-a3-20261013`).
- **Notes for Codex oder Claude:** Track section *Goal A — data integrity* below is current. Worktrees need both `OriginalData` and `project_dump.json` linked for corpus runs. Disk on /mnt/daten-i ran to 96 % during parallel gates; this session now uses a single target dir.

---

- **Last Agent:** codex
- **Timestamp:** 2026-10-13 17:25
- **Completed:** Owner-approved four-document reorganization implemented and integrated: source a08daed6, merge d92460a5. Current handover is a per-track summary; all inherited public journal bytes are reconstructable from archive/CURRENT_STATE_main_2026-10-13.md. Delivery source 7945→640 lines, 307 dated records in ISO-week files; KNXnet-IP/bus2651→96 with40 dossiers; commissioning2206→84 with34 dossiers. Receipt inversely restores382 contiguous source records and all old aliases. Named source-evidence mutation refused and original restored. Final merged12-stage gate green: Python90, xtask89+14=103, scoped build/clippy/fmt, docs565/4702/0, anchors1280/565/0, five xtask gates, whitespace;2595 inputs frozen.
- **Pending/Next Steps:** Publish this fully gated documentation package, read back exact remote artifacts, archive the private root journal locally before installing the short root handover, and clean only owned worktree/branches/scratch. Other product work remains with its owner; no release/deployment or hardware action.
- **Notes for Codex oder Claude:** Goal A session62a4cca1c10f owns A1–A7; its new b11b3561 handover prefix is preserved in full below, Goal B is not started. Do not restore the old monolith from stale root or publish its confidential manufacturer research. Formal ledger/product/compatibility status is unchanged. Maintenance and receipt: docs/history/DOCUMENTATION_REORGANIZATION.md. Future dated updates belong outside preserved blocks; active/private/parked decisions survive separately, not through age-based trimming.

---

Web lock: still released (Goal A touches no apps/knx-web)
- **Last Agent:** Claude
- **Timestamp:** 2026-10-13 17:20
- **Completed:** Owner invoked Goal A (data integrity, model, evidence-gated import; prepared prompt `docs/goals/unassigned-data-integrity.md`, audit `docs/status/2026-10-13-unassigned-work-goal-audit.md`, both now published together with the Goal B prompt). Session `62a4cca1c10f` owns A1–A7 in its own worktree `data-integrity-20261013`, branch `feature/data-integrity-20261013`, from fresh `origin/main` `4c0bea47`. Finite plan and package states in the goal file's *Session status* table.
- **Pending/Next Steps:** A1 (KL-157 read-only CLI/comparison readers) first, then A2…A7 in order; each package pushed only after its full gate. Goal B is not started by this session.
- **Notes for Codex oder Claude:** Please do not start A1–A7 work elsewhere. Concurrent docs restructuring by `468712901140` (CURRENT_STATE/IMPLEMENTATION_STATUS/research split) is respected: this session re-fetches before every integration and keeps its entries short. No hardware, bus, ETS/vendor execution, release or deployment.

---

# Current project handover

Read this before starting. It is **current context by track**, not a task-status
ledger or an append-only diary. Formal per-ID status stays in
[LEDGER](../docs/status/LEDGER.md); next product work is in
[OPEN_WORK](../docs/OPEN_WORK.md). Historical next steps do not reopen work.

## Coordination

Web lock: held by claude-goal-b-ba673d45ff9d (Goal B, per package; see docs/goals/unassigned-workbench-hardening.md *Session status*). Excluded from this claim: the KL-036 session-log files in flight in `log-16k` (bcee21b10872)
Release freeze: released — Alpha 11 published; never move its tag

At migration preflight only the stale shared root and this documentation tree
were registered. This is a snapshot, not a future ownership guarantee. Fresh
handover/worktree checks are required before taking any shared-surface lock.
No old web-lock queue is carried forward: its packages were delivered.

## Release and reporter follow-up

- **Last Agent:** Claude
- **Timestamp:** 2026-10-13 18:34
- **Completed:** Issue #7 follow-up (reporter hagbardZ, Alpha 10): his ~80-device ETS 5.7 project imports (2840 warnings), but *Export log (JSON)…* failed in the AppImage with `Command save_session_log not allowed by ACL`. Cause: the release window's `http://127.0.0.1:<port>` origin is remote to Tauri and the shell's own commands had no permission; the bus monitor's *Export capture* had the same defect, alpha.5–alpha.11 (KL-181, P2, DONE). Fix `7ba0989e` (permissions/app-commands.toml, main-window-only grant, `app_command_handler!`, mock-runtime `app_command_acl_tests` red before / green after / red again without the grants), docs `7d8edd75`. Package gate on frozen `7d8edd75`, all exit 0: fmt, Clippy -D warnings (knx-desktop), desktop tests 14/0, ACL test in release 2/0, tools 90 OK (1 skipped), documentation, catalogue fresh, five xtask gates, diff --check. Earlier: Alpha 11 published and verified (tag object 44824586 on 758a112b; docs/evidence/release-alpha11-2026-10-13.json).
- **Pending/Next Steps:** Fix is on main (reporter told on #7); it ships with the next AppImage, then ask the reporter to retry the export. Meanwhile the Docker image exports the log. The 2840 warnings were mostly one `import:unsupported` line per retained `Baggages/` file; grouping is delivered (cb61b07f, session bcee21b10872, see the top entry), the reporter's log is still needed to see what genuine findings remain. Still waiting: #10 byte-order probe and first real tester report (review via docs/contracts/download-verification-reports.md); #6 host confirmation/native WebKitGTK remain evidence gaps. Do not rerun a release or move/re-create v0.1.0-alpha.11.
- **Notes for Codex oder Claude:** #7 comment 6149288644 (acknowledgement, edited once: an earlier sentence wrongly blamed dev builds; the test showed the Vite origin is refused too). #7 now assigned to KNXBench-Labs with label `bug`. Native WebKitGTK window and the save dialog were not exercised. A new `#[tauri::command]` needs an entry in permissions/app-commands.toml, a grant in capabilities/default.json and a place in `APP_COMMANDS` of the test, or the AppImage refuses it. #10 device 0.2.23 remains refused; a probe/report is not write authorization.

## Group-address feedback

- **Last Agent:** Claude
- **Timestamp:** 2026-10-12 11:58
- **Completed:** Four views, canonical per-device object loading and address-side membership are delivered; later issues #8/#9 picker/indentation fixes are delivered too. See docs/goals/group-address-four-views.md and docs/limitations/KL-179-tree-indentation-limits.md.
- **Pending/Next Steps:** Wait for issue #4 feedback. GA7 numeric fallback tree requires the recorded request and an ADR first; GA8 virtualization only on request or a failing measured case. Do not close #4 before confirmation or implement either merely because its historical plan exists.
- **Notes for Codex oder Claude:** Matrix scope is linked objects plus explicitly loaded device inventories, not inferred whole-project coverage. Other owners' completed GA packages supersede the old package-B/C and unlinked-object handoffs.

## Encoding and parameter prerequisites

- **Last Agent:** Claude
- **Timestamp:** 2026-10-13 14:42
- **Completed:** KL-170 read-only byte-order probe, named base-image warnings, structured refusal and tester workflow are delivered. Checked Time/Slider controls and explicit Colour/Picture originals are hosted.
- **Pending/Next Steps:** Exact LittleEndian per-type runtime evidence remains external; no guessed swaps or encoder implementation. Colour/Picture value codecs, module-scope mapping, Repeat activation and UTF-8/LoadedImage semantics retain precise missing-contract boundaries. Native WebKitGTK/Orca, revision-conditional legacy writes and native reload guards remain separate work, not closed by a browser fixture.
- **Notes for Codex oder Claude:** Sources: docs/research/parameter-byte-order.md, docs/contracts/parameter-editor-host.md and the current limitations index. New hardware/vendor/ETS validation removed from the old commissioning goal stays excluded unless the owner explicitly reopens it with operation-specific authorization.

## Goal A — data integrity, model and import

- **Last Agent:** Claude
- **Timestamp:** 2026-10-13 21:17
- **Completed:** Goal A ended (session `62a4cca1c10f`): A1–A5 and A7 delivered, A6 blocked on external evidence; final integration `e5eb1e8a` + docs `5ca02b2b`. Package states in the [goal file](../docs/goals/unassigned-data-integrity.md).
- **Pending/Next Steps:** None for Goal A. Goal B starts only on the user's invocation; handover in `docs/goals/unassigned-workbench-hardening.md` (*Goal A outcome*). Lifting A6 needs a new authorized source.
- **Notes for Codex oder Claude:** Contracts for B: `docs/contracts/revision-bound-writes.md`, `docs/contracts/project-functions.md`. Evidence: root `.ai/logs/2026-10-13_claude_goal-a-data-integrity/`. No hardware, bus, ETS/vendor execution, release or deployment happened.

## Data-tooling and MCP client choice

- **Last Agent:** Claude
- **Timestamp:** 2026-10-12 13:00
- **Completed:** Offline edit boundary, literal name recipes, opt-in elicited MCP writer and previewed catalogue update/restore are delivered. Notes/instructions/parameter/catalogue UI hosts and native/UX ports are subsequently delivered; old host-absent handoffs are historical.
- **Pending/Next Steps:** Real third-party MCP client acceptance is optional and depends on the owner's client choice. No generic MCP mutation, online catalogue crawler, application-version switch or macro engine is authorized by existing delivered slices. Backup deletion/direct-install retirement remain separately scoped choices.
- **Notes for Codex oder Claude:** Relevant contracts are docs/contracts/data-tooling-ownership.md, mcp-name-recipe-writer.md, catalogue-update.md and project-note-editor-host.md. Use OPEN_WORK/LEDGER for broader scope; do not adopt old parked branches a second time.

## Private research and local plans

- **Last Agent:** codex
- **Timestamp:** 2026-10-13 17:09
- **Completed:** The shared root contains additional uncommitted planning/research not represented by the published source tree. It is deliberately stale and is not a source for wholesale synchronization or publication.
- **Pending/Next Steps:** Preserve local owner's decisions and planning-only meaning. Private manufacturer/ETS6 host research remains parked; do not execute, extend or publish it. Data-only transfer is a separate owner track, not an unassigned implementation task. No new cooperation/integration scope follows from a discussion.
- **Notes for Codex oder Claude:** Never commit docs/research/manufacturer-plugin-* or the private owner-route logs. Local-only journals are archived locally, never copied into public history. Goal prompts in the root require their own explicit invocation; this documentation go starts none of them.

## Documentation and completed delivery

- **Last Agent:** codex
- **Timestamp:** 2026-10-13 16:13
- **Completed:** Individual limitation dossiers are published through 9fcbc0e3. 189 dossiers retain technical/history records; the short index derives statuses from the unchanged ledger. Private root-only research was not published.
- **Pending/Next Steps:** Maintain current summaries in docs/limitations/catalogue.json and dated detail outside preserved blocks; regenerate with python3 tools/limitations_catalogue.py --write. Generated status freshness is not proof that a summary matches newer semantic evidence.
- **Notes for Codex oder Claude:** Keep KL-130-GATE/ZOOM separate. Other completed packages and their exact test counts live in delivery history, not in this current-state file. No code, protocol, schema or compatibility decision changes through documentation relocation.

## History and maintenance

- [Published journal snapshot](archive/CURRENT_STATE_main_2026-10-13.md): complete
  inherited history at main 4c0bea47, with only reversible link relocation.
- [Archive maintenance](archive/README.md).
- Current decisions above supersede the archive's old pending items and locks.
  Preserve all active/parked tracks when refreshing this file; do not cut by age
  alone. New delivery entries may be prepended, but fold settled work into its
  track and archive the full original at the next explicit checkpoint.
- Previously prepended suffixes now live in the archive. A gate assuming the
  entire old journal is still the current file's suffix must read the migration
  receipt and verify the preserved source instead of restoring the monolith.
