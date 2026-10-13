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
