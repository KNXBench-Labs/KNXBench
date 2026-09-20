- **Last Agent:** Codex
- **Timestamp:** 2026-09-18 13:03 CEST
- **Completed:** Aktuellen Versionsstand read-only geprüft. Produktversion ist konsistent `0.1.0-alpha.1` in allen 15 Rust-Paketen sowie Web-Manifest und Lockfile; Tauri übernimmt die Desktop-Version bewusst aus seinem Cargo-Manifest. `main` und `origin/main` stehen beide auf Merge-Commit `599593d5586af5ec26128e9027f473cc2fc56772` (`merge: fix T30 R20 connection confirmation`, 2026-09-18 10:47 CEST). Es existiert kein Git-Release-Tag. Der Hauptcheckout enthält außer dieser vorgeschriebenen Handover-Datei keine lokale Änderung. Keine Produktdatei geändert und keine Tests ausgeführt, da es sich ausschließlich um eine Versionsabfrage handelte.
- **Pending/Next Steps:** Für die reine Versionsabfrage bleibt nichts offen. Die laufenden separaten Worktrees `d10-language-data`, `d10-server-prose` und `t18-module-arguments` sind noch nicht Bestandteil von `main`.
- **Notes for Claude:** Aktueller veröffentlichungsnaher Paketstand ist Alpha 1; ein lokales AppImage `KNXBench_0.1.0-alpha.1_amd64.AppImage` wurde bereits gebaut und geprüft, aber kein Tag oder Release veröffentlicht. Die Binärprogramme ergänzen bei Builds aus Git den Kurz-SHA als Build-Metadatum (`0.1.0-alpha.1+g<short-sha>`).

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-18 10:55 CEST
- **Completed:** Committed the verified T30/R20 fix as feature commit `3966828` (`fix(knx-net): await device connection confirmation`) with author `KNXBench <github@knxbench.com>`, merged it into `main` with explicit no-fast-forward merge commit `599593d` (`merge: fix T30 R20 connection confirmation`), and pushed `main` to `origin`. Local and remote refs both resolve to `599593d5586af5ec26128e9027f473cc2fc56772`. Pre-merge dirty main documentation was safely stashed and compared; its documentation was subsumed by the merged result, while three unique handover entries were restored exactly once. Post-merge `cargo test --workspace -q` passed. Removed the owned `t30-disconnect-response` worktree and deleted its local branch after the successful push.
- **Pending/Next Steps:** R20's false-connected mechanism is fixed, merged and pushed. A later management read timeout remains inherently ambiguous and must not alone prove absence. Existing unrelated Claude worktrees remain untouched.
- **Notes for Claude:** `main` is synchronized with `origin/main` at `599593d`. The only local tracked modification is this handover file containing restored pre-merge historical entries plus this final update. Safety backup of the former main dirty patch remains at `/tmp/knxbench-main-pre-r20-merge.patch`.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-18 10:36 CEST
- **Completed:** Fixed T30/R20's false-connected mechanism by TDD in isolated worktree `t30-disconnect-response`. `SessionTiming` now exposes `connection_timeout`, defaulting to the KNX Transport Layer clause 4 value of six seconds. `ManagementSession::connect()` subscribes before `T_Connect`, ignores unrelated/stale frames, creates connection state only after a matching positive cEMI `L_Data.con`, reports matching negative confirmation immediately as `ConnectRejected`, and reports silence after the six-second bound. Simulator emits the real confirmation shape. RED observed old immediate gateway-ACK-only return and old wait-through-negative behavior. Final read-only hardware run over all 34 `devices.md` targets: 33 positive confirmations in 2.851–144.245 ms (median 134.530 ms), all 33 descriptor reads succeeded; one of the two IP interfaces explicitly rejected connect after 162.709 ms and received no descriptor read. Alternating timeouts disappeared. All tunnel disconnects succeeded. Temporary probe deleted. Updated research, known limitation, R20 design risk, implementation status, daily memory and `.ai/logs/2026-09-18_codex_r20_connection_timeout_fix.md`.
- **Pending/Next Steps:** R20's false-connected timeout mechanism is fixed and hardware verified. A later management read timeout remains inherently ambiguous and must not alone prove absence. The isolated worktree changes remain uncommitted and unmerged.
- **Notes for Claude:** Verification passed: 178 `knx-net` unit tests plus integration/doc harnesses, package Clippy all targets with warnings denied, workspace all-target check, formatting, diff check, and final hardware probe. No property read, authorisation request, write service or scan was sent; The address absent from `devices.md` and the excluded alarm panel `1.1.220` both remained excluded.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-18 09:59 CEST
- **Completed:** Isolated R20 at frame level using two already approved read-only targets. Both tunnels were assigned the same gateway-issued tunnel address, and all incoming KNXnet/IP sequence counters began at zero and matched, ruling out alternating tunnel addresses and client receive-sequence rejection. The successful first target received current-target `T_Connect` `L_Data.con`, descriptor-read confirmation, device `T_ACK`, and descriptor response. Failed the second target first received a late `T_Connect` confirmation for the previous target; its three descriptor reads received `L_Data.con`, but no current-target connect confirmation, device ACK or response arrived. `ManagementSession::connect()` completes on gateway `TUNNELLING_ACK` and does not await matching bus-level `L_Data.con`, so it proceeds without evidence the transport connection was established. TPCI Connect/Disconnect encoding is correct and round-trip tested. Temporary instrumentation was restored byte-for-byte and temporary test deleted. Updated R20 research, limitation, risk, status, daily memory and frame diagnostic log.
- **Pending/Next Steps:** R20's direct timeout mechanism is identified; why every alternate bus-level `T_Connect` confirmation is absent remains open. A causal fix must synchronize connection progress with the matching successful cEMI `L_Data.con` or another specification-grounded readiness event, with a failing regression before product changes. Do not add an unexplained delay.
- **Notes for Claude:** Diagnostics remained read-only: `A_DeviceDescriptor_Read(0)` only, no property read, authorisation request, write service or scan. Product implementation remains exactly the pre-diagnostic response-aware disconnect diff in `t30-disconnect-response`; SHA-256 restoration check passed.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-18 09:43 CEST
- **Completed:** Ran the user-approved response-aware T30/R20 read-only probe over all 34 literal `devices.md` targets in ascending order (the line's devices, then the two IP interfaces) through gateway `KNX_GATEWAY:3671`. One address the earlier notes had assumed present, and the project-excluded alarm panel `1.1.220`, were absent and asserted excluded before socket creation. One fresh tunnel per target, `ManagementSession::read_only`, `AuthorisationPlan::Skip`, only `A_DeviceDescriptor_Read(0)`. Exactly 17 odd-position attempts answered and 17 even-position attempts timed out; ascending order reversed the earlier result for those nine, proving attempt order rather than address, manufacturer or mask version selects the failure. one target returned `0012h`; all other answers returned `0701h`. All 34 disconnects received successful final responses. Test passed in 156.63 seconds; temporary source deleted. Updated R20 research, known limitation, design risk, implementation status, daily memory and `.ai/logs/2026-09-18_codex_r20_all_devices.md`.
- **Pending/Next Steps:** R20 remains open. Investigate which transport/application-session state alternates across independently and correctly terminated KNXnet/IP channels. Do not infer a delay or retry constant and do not use a management timeout as proof of absence.
- **Notes for Claude:** No property read, authorisation request, write service or scan was sent. Documentation updates and existing response-aware disconnect implementation remain isolated in worktree `t30-disconnect-response`; no product code was changed by this all-device rerun.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-18 09:08 CEST
- **Completed:** Repeated the bounded T30/R20 real-gateway test on the response-aware disconnect implementation. The test used gateway `KNX_GATEWAY:3671`, nine literal `devices.md` targets in reverse order (the line's highest nine), asserted excluded `1.1.220` before socket creation, one fresh tunnel per target, `ManagementSession::read_only`, `AuthorisationPlan::Skip`, and only `A_DeviceDescriptor_Read(0)`. Every KNXnet/IP disconnect received a matching successful `DISCONNECT_RESPONSE`. The exact earlier result reproduced: the five at even positions returned mask `0701h`; the four at odd positions timed out after three three-second attempts. Test passed in 37.07 seconds. Temporary test source deleted; raw output retained under `/tmp`; added `.ai/logs/2026-09-18_codex_r20_response_rerun.md` in main.
- **Pending/Next Steps:** R20 remains open. Response-aware IP channel teardown is independently ruled out as the cause of the alternating management response pattern. Continue with specification-grounded transport/application-session investigation; do not infer a retry delay and do not treat timeout as proof of absence.
- **Notes for Claude:** No write, property read, authorisation request, or scan was sent. No product code was changed by this rerun. The existing uncommitted response-aware disconnect implementation and its documentation in worktree `t30-disconnect-response` remain otherwise untouched.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-18 08:59 CEST
- **Completed:** Recorded the user-provided T30/R20 test context in project memory: `devices.md` is the authoritative target list for read-only communication tests, and the KNXnet/IP gateway address is `KNX_GATEWAY`. Added the same raw context to `memory/2026-09-18.md`. No network or KNX bus access occurred.
- **Pending/Next Steps:** Use the listed devices and gateway for future explicitly requested read-only T30/R20 communication tests. R20 implementation and verification remain pending as described below.
- **Notes for Claude:** The user explicitly requested persistence of the gateway IP, superseding earlier handover notes that intentionally omitted it. This update only changes memory and handover files.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-18 08:31 CEST
- **Completed:** Audited all 179 PDFs in `The KNX Standard v3.0.0` against T30 R20. Found concrete KNXnet/IP Core §5.5 mismatch: `TunnelClient::disconnect` sends `DISCONNECT_REQUEST` then immediately stops receiver, never observes mandatory `DISCONNECT_RESPONSE` final channel termination. Core sequence reset and Tunnelling one-retry rules match current code. Management Procedures §2.19 confirms current connection-oriented scan; §2.17 connectionless scan is RF-only. Updated RESEARCH §8.8.3b, KNOWN_LIMITATIONS §7, design risk R20, IMPLEMENTATION_STATUS, and audit log. No product code, socket, gateway, or hardware changed.
- **Pending/Next Steps:** R20 remains open because disconnect mismatch is credible mechanism for fresh-tunnel alternation but not yet causal proof and cannot explain shared-tunnel first-session failure. Next implementation task: loopback peer test proving graceful disconnect waits matching response, response-aware disconnect fix, then bounded read-only fresh-tunnel comparison. Do not guess delay/retry constant. Real writes still need explicit operation-specific authorization; never contact `1.1.220`.
- **Notes for Claude:** Main product source untouched. Documentation-only changes in main overlap no reserved worktree implementation. Existing `.ai/CURRENT_STATE.md` audit entry remains below this one.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-18 07:43 CEST
- **Completed:** Read-only audit answering what remains outside Claude's three reserved worktrees (`d10-language-data`, `d10-server-prose`, `t18-module-arguments`). Reconciled current `main` (`6605273`) with `goal.md`, roadmap, implementation status, known limitations, project analysis, source and branch/worktree state. Confirmed old goal entries are stale: DPT codec main types 1-30 are implemented; T38, passive monitor evidence, load optimization, Docker/AppImage, range-less export guard, and several parked findings are already complete. No product code changed.
- **Pending/Next Steps:** Outside Claude's work, primary open delivery is T30 commissioning with R20 tunnel lifecycle evidence unresolved; next independent v1 work is T37 truthful project-load/import progress. Further concrete hardening: atomic unsaved-project replacement/session snapshot, deployment-mode path confinement/product-database startup failures, five lossy `Space/@Type` variants, DPT collision provenance after D10 integrates, malformed non-override preservation where evidenced, ZipCrypto real-project validation, help T28 after UI stabilizes, and documentation consolidation. Deferred items remain deferred by explicit rulings.
- **Notes for Claude:** Three reserved worktrees were inspected read-only and left untouched. `d10-language-data` has its existing four modified files; no Codex overlap introduced. Main working tree was clean before this required handover-only update.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-18 08:43 CEST
- **Completed:** In isolated `t30-disconnect-response` worktree implemented KNXnet/IP Core §5.5 response-aware graceful disconnect by TDD. Real UDP loopback RED/GREEN proves the client waits for the matching final response; missing response is bounded at 10 seconds and non-zero status is a typed protocol error. Repeated the approved hardware comparison with one fresh tunnel per literal target, read-only session, skipped authorisation and only descriptor type 0. All nine tunnel disconnects received successful final responses, but `.32/.30/.28/.26/.24` still answered `0701h` and `.31/.29/.27/.25` still timed out. Incomplete IP-channel teardown is therefore ruled out as the alternation's cause on this gateway. Temporary test deleted; research, limitations, design risk and implementation status updated.
- **Pending/Next Steps:** R20 remains open below or outside KNXnet/IP channel teardown; the exact management transport/application-session state failure is unidentified. Continue using the independent scan probe for presence and do not infer a retry or delay. Never contact `1.1.220`; no real writes are authorized.
- **Notes for Claude:** Product implementation remains confined to `crates/knx-net/src/client.rs`; real-hardware safety boundary held and runtime gateway is absent from repository files. Worktree `/mnt/daten-i/Sourcecode/KNXBench/.worktrees/t30-disconnect-response`, branch `t30-disconnect-response`, uncommitted. Verification before hardware rerun: focused regression, 174/174 library tests, full package test, package Clippy with warnings denied, fmt and diff check. Main checkout and three Claude worktrees untouched.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-18 04:57 CEST
- **Completed:** Ran the explicitly approved T30 R20 read-only comparison against the runtime-supplied gateway with a temporary, uncommitted test. Safety boundary held: nine explicit targets in reverse order (the line's highest nine), `1.1.220` asserted excluded before socket open, `ManagementSession::read_only`, `AuthorisationPlan::Skip`, only `A_DeviceDescriptor_Read`, no property read, authorisation request or write service. On one shared tunnel, the first target answered mask `0701h` and all later targets timed out, proving the previous sole success was a first-session effect rather than device-specific. With one fresh tunnel per target, `.32`, `.30`, `.28`, `.26`, `.24` answered `0701h` while `.31`, `.29`, `.27`, `.25` timed out. Fresh tunnels alone are therefore not reliable; immediate teardown/recreation or gateway channel lifecycle remains involved. The test passed in 109.39 seconds and the temporary source file was deleted. Updated RESEARCH §8.8.3a, KNOWN_LIMITATIONS §7, the commissioning design R20 row, IMPLEMENTATION_STATUS, and `.ai/logs/2026-09-18_codex_r20_comparison.md`.
- **Pending/Next Steps:** R20 remains open. Do not infer or implement a retry fix from the alternating pattern; the evidence does not identify the missing delay, sequence or acknowledgement. Any future commissioning path must use the independent scan probe for presence and must not treat a `ManagementSession` timeout as absence. Never contact `1.1.220`; real writes still require explicit operation-specific authorization.
- **Notes for Claude:** The gateway address is intentionally absent from repository files. Product code is unchanged. Only documentation and tracked handover state are modified. The raw comparison log remains under `/tmp`; the throwaway `tmp_r20_probe.rs` no longer exists.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-17 22:57 CEST
- **Completed:** Used the runtime-supplied gateway for two serial runs of the existing ignored T30 phase-3 read-only integration test. Both runs completed successfully and preserved the structural safety boundary: exactly the line's highest nine `devices.md` targets, `1.1.220` asserted excluded before socket open, `ManagementSession::read_only`, `AuthorisationPlan::Skip`, no authorize request and no write service. The captured second run exactly reproduced the 2026-09-14 observation: the first target answered mask `0701h`, manufacturer `00 0c`, and three `Loaded` states while hardware/program properties returned typed zero-element refusals; the remaining eight timed out on mask plus all six property/load-state reads in the shared tunnel. Added local ignored log `.ai/logs/2026-09-17_codex_t30_readonly_reproduction.md`; gateway address is omitted.
- **Pending/Next Steps:** Await explicit approval for the bounded read-only R20 comparison already presented in chat: reverse order over one shared tunnel, then one fresh tunnel per target, only `A_DeviceDescriptor_Read`. This distinguishes order from tunnel lifetime with no property reads, authorisation, or writes. Do not implement a retry fix before this evidence. Never contact `1.1.220`.
- **Notes for Claude:** The current result is reproduction, not root cause. Existing repository behavior and docs remain accurate. Main was clean and synced to `origin/main` at `e2eff6e` before this handover update; only tracked `.ai/CURRENT_STATE.md` is now modified. The `/tmp` raw log is local and contains no repository change.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-17 22:18 CEST
- **Completed:** Collected all completed verified main-checkout hardening work into feature commit `e8e2c0a` (`chore: integrate verified hardening work`) and integrated it into `main` with no-fast-forward merge commit `5a655fb` (`merge: integrate verified hardening work`). Both commits use `github@knxbench.com`. The integrated set includes the AGPL/licence and VD4 analysis documentation, Session 7 reconciliation, ADR-0018 header repair, Node 22 Docker build and fixture-free CI smoke lifecycle, typed refusal of lossy range-less group-address export, and same-file `ApplicationProgram/@Id` conflict reporting. Pre-merge full proof passed: workspace tests, workspace Clippy with warnings denied, 471 frontend tests and production build, formatting, layering, header ratchet, `cargo deny check`, diff check, and a real fixture-free Docker image smoke test. Post-merge workspace tests and frontend tests/build passed again on `5a655fb`. Pushed `main` through handover commit `12c3e69`; local and remote matched. With hardware access authorized, ran read-only `knx bus discover`; no gateway responded and no bus telegram or individual-address contact occurred.
- **Pending/Next Steps:** Supply `KNX_GATEWAY=host:port` in the runtime environment to resume the documented read-only T30 evidence pass. Never contact `1.1.220`. Any real device write still requires explicit authorization for the specific operation under the repository hardware rule.
- **Notes for Claude:** The dirty `d10-language-data` worktree and clean `d10-server-prose`/`t18-module-arguments` worktrees remain separate and unmodified. The temporary integration branch was deleted after the successful merge and push. Discovery returned normally with an empty result and the CLI's multicast/network-segment caveat; no address was guessed or persisted.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-17 21:38 CEST
- **Completed:** Closed the `application_program` same-file identifier collision blind spot in `knx-productdb`. `ingest_program` now uses the shared `first_winner` helper with a fresh per-file occurrence map. Two `ApplicationProgram` declarations sharing an ID retain the first row and emit `IdConflict` occurrence 2; cross-file behavior is preserved. TDD RED observed zero conflicts before implementation; focused Same-file and Cross-file tests passed after implementation. Updated `docs/KNOWN_LIMITATIONS.md` §86, `docs/IMPLEMENTATION_STATUS.md`, and `docs/PROJECT_ANALYSIS_2026-09-15.md`, consolidated three duplicate range-less export status entries, and added `.ai/logs/2026-09-17_codex_application_program_collision.md`. Verification passed: complete `cargo test -p knx-productdb` (170 unit tests plus all integration suites), package Clippy with warnings denied, workspace formatting, header gate (122 valid / 168 absent at ceiling 168), and `git diff --check`.
- **Pending/Next Steps:** `datapoint_type` conflict provenance remains the only §86 residue: collisions are counted, but the table cannot identify the retained source because it lacks `source_sha256`. Do not implement this while the dirty `d10-language-data` worktree owns `parse/master.rs`, `migration.rs`, product tests, and the same documentation. Continue with another independent task or integrate that worktree first. T30 R20 still requires a configured gateway for safe read-only evidence; no hardware writes are authorized.
- **Notes for Claude:** No D10, T18, server route/session, gateway, or bus code was touched. The parser change is confined to `parse/mod.rs` documentation and `parse/program.rs`; no database migration or public type changed. Preserve all pre-existing dirty release, licence, Docker, roadmap, goal, and analysis work. Never target individual address `1.1.220`.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-17 21:18 CEST
- **Completed:** Closed the silent range-less group-address export loss in schema 11 and schema 21. Both writers now return `ExportError::UnrangedGroupAddress`, including installation ID, group-address ID, and raw address, before any successful export can omit the address. Shared validation lives in the exporter layer; native `.knxdb`, the optional domain field, and current authoring behavior remain unchanged. Added focused tests for both schema writers and updated `docs/KNOWN_LIMITATIONS.md` §21, `docs/GAP_ANALYSIS_ETS.md`, `docs/IMPORT_EXPORT.md`, `docs/IMPLEMENTATION_STATUS.md`, and `docs/PROJECT_ANALYSIS_2026-09-15.md`. Added `.ai/logs/2026-09-17_codex_unranged_group_address_export.md`. Verification passed: `cargo fmt --all --check`, `cargo clippy -p knx-etsproj --all-targets -- -D warnings`, `cargo test -p knx-etsproj`, `cargo test -p knx-app`, `cargo run -q -p xtask -- check-headers`, and `git diff --check`.
- **Pending/Next Steps:** Continue the next independent P0 data-integrity residue in `knx-productdb`: first trace the still documented `application_program` same-file identifier collision and `datapoint_type` provenance gaps in §86, then present the smallest bounded design before implementation. T30 R20 remains blocked until a configured gateway permits safe read-only evidence collection. No hardware writes are authorized.
- **Notes for Claude:** The export guard deliberately fails rather than inventing a range because neither supported writer has an evidenced faithful external representation for a range-less address. Preserve all existing uncommitted licence, release, roadmap, goal, and project-analysis edits. Do not touch `d10-language-data`, `d10-server-prose`, or `t18-module-arguments`; never target individual address `1.1.220`.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-16 13:35 CEST
- **Completed:** Completed the VD2/VD3/VD4 licence and legal review without changing product code. `knxReTk` is GPL-2.0-or-later by explicit parser-file headers; its referenced `FLOSS-EXCEPTION.txt` does not exist in the investigated commit or cloned history, so no exception was assumed. `sbtools-vdio` carries GPLv3 in both root `LICENSE` and module `COPYING`. GPLv3 section 13 permits combination with AGPLv3 in principle, but KNXBench's `AGPL-3.0-or-later` metadata is explicitly documented as an undecided placeholder, no canonical licence file is tracked, and current architecture policy rejects GPL runtime dependencies. Added a detailed decision matrix, compliance duties, CJEU/German-law basis, fixture/data restrictions, encryption risk, and authoritative links to `docs/VD4_PRODUCT_DATABASE_IMPORT.md`; updated the Roadmap reference and investigation log. Removed the VD archive password literal from every repository document and handover file, stored it only in ignored `OriginalData/ProductDatabases/.vd-import-password` with mode `0600`, and changed the documented verification to load it from that file without placing it in process arguments.
- **Pending/Next Steps:** No GPL code may be copied or ported under the current repository policy. Before direct reuse, the owner must formally choose AGPL version 3 or later, add the canonical licence and notices, accept GPL/AGPL source obligations, and deliberately revise the GPL dependency policy. The recommended implementation remains an independent Rust parser using a factual format specification and synthetic fixtures. A released importer must not embed the archive credential without written rights-holder permission or case-specific legal advice; use a user-supplied password and lawfully obtained files meanwhile.
- **Notes for Claude:** Licence compatibility and permission to decrypt product data are separate. GPL covers the two reference implementations, not manufacturer databases, vendor contracts, or technical-protection circumvention. Existing `OriginalData/` is ignored and must remain uncommitted. The local password file is covered by `.gitignore:13`; never print or commit its contents. CJEU C-406/10 supports reimplementing functionality and data-format facts without copying source expression; German `UrhG` sections 69d/69e impose conditions, and sections 69f/95a leave a material password-embedding risk. Existing uncommitted Roadmap/goal/project-analysis work was preserved.

---


- **Last Agent:** Codex
- **Timestamp:** 2026-09-16 11:22 CEST
- **Completed:** User supplied the Task 16 gateway at runtime; passive Group Monitor real-hardware verification is now complete and integrated as `77bb473`. A dedicated current-source `knx-server` exercised only monitor start/poll/stop. Empty-project session: 133 seconds, 52 telegrams, 0 drops. Real schema-23 `Unser Zuhause` project session: 107 seconds, 65 telegrams from 9 sources to 21 group destinations, 65/65 destination names resolved, 10 DPT values decoded, 0 conflicts, 0 decode errors, 0 drops. Both tunnels stayed active until explicit successful stop. No group read/write/response, management request, scan, or `/api/bus/write` call occurred; `1.1.220` was never targeted. Runtime gateway and observed bus addresses were omitted from repository files. Updated §62, compatibility evidence, D5, implementation status, roadmap/architecture anchors, `goal.md`, and `.ai/logs/2026-09-16_codex_group_monitor_reverify.md`. Earlier work in the same request remains integrated: T38 `57ed42b`; `load_project` investigation `61b4b18`. Pushed `main` to `origin` through `77bb473` on user request.
- **Pending/Next Steps:** All three requested workstreams are complete. Task 16 still does not verify routing, transmit behavior, active reads, reconnect, other gateway models, or long sessions; real transmit testing requires separate explicit authorization. The measured `load_project` optimization is a future implementation task: replace 65,000 communication-object graph point queries with ordered bulk loads while preserving ownership, order, override error behavior, module association, and links, then re-run round-trip tests and the same benchmark.
- **Notes for Claude:** Local `main` and `origin/main` both point to `77bb473`; pushed commits are `57ed42b` (T38), `61b4b18` (`load_project` analysis), and `77bb473` (Task 16 evidence). Post-run safety checks: dedicated server stopped; runtime gateway literal absent from repository; `cargo fmt --all --check`, `git diff --check`, and focused §62 anchor validation passed. A broad ad-hoc Markdown link scan surfaced a pre-existing stale §11 anchor in `GAP_ANALYSIS_ETS.md`; Task 16 did not broaden into that unrelated cleanup. Pre-existing local changes remain uncommitted: `.ai/CURRENT_STATE.md`, `docs/ROADMAP.md`, `goal.md`, and untracked `docs/PROJECT_ANALYSIS_2026-09-15.md`. Claude-owned worktrees remain untouched.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-16 11:03 CEST
- **Completed:** T38 implemented and integrated on `main` as `57ed42b`: `apps/knx-web/src/App.tsx` derives the footer version and `document.title` directly from `apps/knx-web/package.json`; focused manifest/sentinel coverage added. Independent post-integration proof: `npm test -- --run src/App.test.tsx` 21/21 and `npm run build` succeeded. `knx_store::load_project` investigation integrated as `61b4b18`: 76,051 SQL executions measured, 65,000 (85.5%) in the communication-object graph; device phase 922.290/944.252 ms (97.7%); final uninstrumented release open 994.615 ms. `docs/PERFORMANCE.md` records method, variance, invariants, and bulk-load optimization candidate; temporary instrumentation was removed. Task 16's passive monitor path and safety boundary were traced without bus access; the measurement log is `.ai/logs/2026-09-16_codex_group_monitor_reverify.md`. T38 status in the existing uncommitted `docs/ROADMAP.md`/`goal.md` planning changes was advanced to shipped/completed.
- **Pending/Next Steps:** Task 16 hardware observation remains pending because `KNX_GATEWAY` was absent from the Codex runtime and no KNXBench monitor session was already running. No address was guessed, no socket was opened, and duration/telegram count/tunnel behavior/project resolution remain not measured. Resume immediately when the user supplies `KNX_GATEWAY=IP:Port`; monitor passively only, never send a group read/write/response, and never address `1.1.220`. `docs/KNOWN_LIMITATIONS.md` §62 remains unchanged until a real measurement exists. The measured load optimization itself is a separate future task: replace the 65,000 communication-object graph point queries with ordered bulk scans while preserving ownership, positions, override error behavior, module association, and link order, then re-run the same benchmark and round-trip tests.
- **Notes for Claude:** `main` is ahead of `origin/main` by two commits (`57ed42b`, `61b4b18`). Pre-existing uncommitted files remain: `.ai/CURRENT_STATE.md`, `docs/ROADMAP.md`, `goal.md`, and untracked `docs/PROJECT_ANALYSIS_2026-09-15.md`; no Claude worktree was modified. T38 full branch proof was 471 Vitest tests plus TypeScript/Vite build; root repeated the focused 21 tests and production build after cherry-pick. Store analysis gates: release perf 1/1, `cargo fmt --all --check`, `cargo test -p knx-store` 65/65, layering, link check, and diff check. Task 16 blocker draft was discarded rather than rewriting §62 around a missing environment variable.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-16 10:23 CEST
- **Completed:** Aktuellen `main`-Stand, offene Goal-/Roadmap-Punkte und Claudes drei reservierte Worktrees auf Überschneidungen geprüft. Task 17 ist inzwischen integriert und fällt aus der Kandidatenliste. Als code- und laufzeitseitig getrennte nächste Arbeiten verbleiben vor allem Task 16 (rein passiver Group-Monitor-Test am realen Gateway), die gemessene `knx-store::load_project`-Analyse/Optimierung (945 ms Baseline; Query-Schleifen im Store) und T38 (Manifest-Version in Footer und Browsertitel). Ebenfalls getrennt umsetzbar sind Docker/Release-Artefakt-Härtung sowie die Import/Export-Korrekturen für bereichslose Gruppenadressen oder die fünf fehlenden `Space/@Type`-Varianten. Keine Produktdatei geändert.
- **Pending/Next Steps:** Nutzer kann einen oder mehrere der getrennten Kandidaten auswählen. T37, zentrale Session-/Dirty-State-Härtung, Pfad-Policy, T18-Restarbeiten, D10 und T28 bleiben wegen direkter Datei-/Schnittstellenüberschneidung oder expliziter Reihenfolge vorerst außen vor. Dokumentationsabschluss der getrennten Kandidaten kann nach Claudes Branch-Integration erfolgen, weil `KNOWN_LIMITATIONS.md`/`IMPLEMENTATION_STATUS.md` dort gemeinsam berührt werden.
- **Notes for Claude:** Reservierte Worktrees unverändert: `d10-language-data` ist dirty; `d10-server-prose` und `t18-module-arguments` sind clean. Task 16 bleibt strikt passiv, ohne Bus-Schreibzugriff und ohne Zugriff auf `1.1.220`. Die N+1-Bezeichnung ist als zu verifizierende Optimierungshypothese zu behandeln; gemessen ist nur `load_project` mit 945 ms, die genaue Query-Anzahl muss vor dem Fix instrumentiert werden.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-16 09:04
- **Completed:** Fixed the Omarchy `SUPER+SHIFT+CTRL+A` Codex launch in the user-owned Hyprland configuration. Backed up `~/.config/hypr/bindings.lua` to `~/.config/hypr/bindings.lua.bak.20260916-0858`, unbound the stock Agent shortcut, and rebound it as `Codex` through `omarchy-launch-tui --app-id=org.omarchy.agent headroom wrap codex -- --approve-for-me`. This routes Codex through Headroom's working project-scoped endpoint while retaining Omarchy's agent terminal app-id and unattended approval mode. No packaged `/usr/share/omarchy/` file or KNXBench runtime code changed.
- **Pending/Next Steps:** None for this fix. The user can close the pre-existing Codex 0.153.4 hotkey window and launch a fresh one with `SUPER+SHIFT+CTRL+A`.
- **Notes for Claude:** Validation succeeded: `hyprctl reload` returned `ok`; `hyprctl configerrors` was empty; `omarchy menu keybindings --print` shows `SUPER SHIFT CTRL + A → Codex`; and the exact wrapped command with `--approve-for-me` completed a read-only test prompt with `OK` and `turn.completed` instead of the former missing-`response.completed` error.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-15 14:24
- **Completed:** Analyzed the current KNXBench repository and created `docs/PROJECT_ANALYSIS_2026-09-15.md` with strengths, 14 prioritized findings, concrete recommendations, an execution order, and explicit non-recommendations appropriate to the unfinished alpha stage. Added `.ai/logs/2026-09-15_codex_project_analysis.md`. No runtime code changed. Verified clean `main` at `06db5ec`: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, layering/header gates, `cargo deny check`, frontend tests (468/468 across 42 files), and TypeScript no-emit check all succeeded; 1,594 Rust tests were discovered. Measured 801 React act-environment warnings despite the passing frontend suite.
- **Pending/Next Steps:** No fixes were requested or implemented. Recommended next slice is the integrity baseline: protect import/open from discarding unsaved edits, introduce a real saved revision, atomically own per-project session state, and distinguish missing from failed product-database startup. Then split desktop/server filesystem path policy before shared network deployment.
- **Notes for Claude:** The most material new evidence is in `docs/PROJECT_ANALYSIS_2026-09-15.md`. `resolve_project_path`/`resolve_new_project_path` intentionally allow arbitrary absolute paths for Tauri but the same router is used by the unauthenticated `0.0.0.0` standalone server. `open_project` and `open_native_project` lack `new_project_impl`'s discard guard. `AppState` publishes project/store/opaque/manufacturer/stack/counts through separate mutex assignments. `AppState::new` silently maps product DB open/migration errors to `None`. Documentation currently contradicts shipped parameter editing and the apparent AGPL metadata; no `LICENSE`/`COPYING` file is tracked. Docker/Tauri bundle and physical KNX were not tested in this analysis.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-09-12 05:40
- **Completed:** **T26 (product-data language) merged to `main`
  (`0912ea5`, `--no-ff`).** Branch `t26-product-language`,
  `c19aa94`..`0c51ca9`, nine commits, 20 files / 2253 insertions / 119
  deletions. `knx-productdb`'s `translation` table — populated on every
  product import since the database layer existed, read by nothing — now
  has a reader. `crates/knx-productdb/src/query.rs` gained a private
  `translation_overlay(conn, program_id, language)` keyed by
  `(ref_id, attribute_name)` whose SQL allow-lists
  `Text`/`FunctionText`/`SuffixText`/`VisibleDescription`/`Name`, making
  a `Value` translation **structurally unreachable** rather than merely
  unusual (translating `Value` would rewrite a stored parameter value);
  `parameter_views(conn, program_id, language: Option<&str>)` applies the
  overlay per element **before** `pick()`;
  `parameter_type_enum_options` overlays `text` only. Public
  `translation_languages` / `program_translation_languages` return
  `TranslationLanguage { language, rows }`; the second has no caller yet
  — a documented seam for the com-object slice. `apps/knx-server` threads
  `language: Option<&str>` through `assemble_parameter_panel`,
  `parameter_panel_impl` and `set_parameter_value_impl` (**both**
  assembly paths), adds `?language=` to `GET`/`POST
  /api/device/{id}/parameters` (`Query` **before** `Json`, axum demands
  the body extractor last) and `GET /api/product-languages` (returns
  `200 []` with no product database — a Settings panel must render on a
  fresh install; a real database error still propagates). `apps/knx-web`
  persists the choice under `knx-desktop:product-language`, adds a fourth
  Settings select, and threads the language into `ParameterPanel`.
  **The whole-branch review found one blocker no task-level review could
  have seen:** `useProductLanguage()` was a plain `useState` called
  independently at `App.tsx:81` and `ParameterPanel.tsx:204`, and
  `Inspector.tsx:504` renders `<ParameterPanel>` with no `key`, so the
  panel never remounts — picking a language in Settings never reached an
  already-open panel. Fixed in `e26ce82` with a module-level cache plus a
  subscriber `Set` behind React 19's built-in `useSyncExternalStore`
  (lazy seeding from `localStorage`, unchanged hook signature, no new
  dependency), pinned by a regression test that asserts `readerMounts
  === 1` so a remount cannot make it pass. Fix round 1 also added the
  missing enum-write-with-language HTTP test and corrected a doc
  miscount; the scoped re-review returned **ALL FINDINGS ADDRESSED**
  (0/0/1/2), and the residual MINOR plus both NITs were parked with
  written rulings (see the log). Docs reconciled in the same slice:
  `KNOWN_LIMITATIONS.md` §37 → "partially resolved", §48's cause
  repointed, **new §64** (1705 `Languages`-block translations discarded
  on import — Catalog.xml 40/5, Hardware.xml 30/5, knx_master.xml
  1635/24, measured on `MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod`);
  `GAP_ANALYSIS_ETS.md` D10 kept open with the reader recorded, new **T32**
  backlog item; dated `IMPLEMENTATION_STATUS.md` entries; `ROADMAP.md`'s
  two false Internationalization statements fixed;
  `.ai/logs/2026-09-12_claude_t26_product_language.md` written.
  **Gates re-measured on merged `main` (`0912ea5`), all green:** `cargo
  fmt --all --check` clean; `cargo clippy --workspace --all-targets -- -D
  warnings` clean; `cargo test --workspace` **989 passed / 0 failed / 3
  ignored across 73 `test result` lines**; `check-layering` ok; `cargo
  deny check` ok; web **245 passed across 26 files**; `tsc --noEmit`
  clean.
- **Pending/Next Steps:** Next slice not yet chosen. The strongest
  candidate is the one this slice opened: **T32** — ingest the
  `Languages` blocks that live outside an application program
  (`KNOWN_LIMITATIONS.md` §64). It is a data-integrity item, which
  outranks UI work under `CLAUDE.md`'s priority order, and it needs a
  schema decision first: `translation.program_id` is `TEXT NOT NULL`, so
  a catalog- or master-scoped translation has no program to belong to.
  **T25** (UI chrome i18n, the other half of D10) is the alternative and
  needs a library choice. Also still open: **the com-object half of §37**
  — `com_object_view` was deliberately untouched, so communication-object
  text is still untranslated and D10 stays open. Other open work
  unchanged: T17 (line scan, needs a spec research spike), T21 (graphical
  views, D1/D2), T28 (in-app help, D12), T16, T22, D8's real settings
  contents, B10, C3, C5, C6, E5, E6, F3. Blocked with named conditions:
  T30/E1 (commissioning), T19/E3 (KNX Secure), T20/A1 (Functions), A4
  (schemas 12-22).
- **Notes for Codex:** Three traps in this slice. (1) **`Value` is never
  translated, and that is load-bearing.** The allow-list in
  `translation_overlay`'s SQL is the only thing standing between a
  translated label and a project file containing the string "An" where
  `1` belongs. `enum_write_with_a_language_keeps_the_raw_value_but_translates_its_label`
  in `apps/knx-server/tests/http_product_language.rs` is the guard —
  injection-tested, fails with `left: 400, right: 200` if the overlay
  ever reaches `value`. (2) **There is no fallback language chain.**
  Asking for `de-DE` when only `de` exists returns untranslated text, by
  decision, not omission — adding one needs a language-tag matching
  policy written down first. (3) **`productLanguage.ts` is a module-level
  store, not per-component state.** If you add a third consumer it will
  share the same value automatically; if you write a test touching the
  language, call `resetProductLanguageForTests()` in `afterEach`, because
  clearing `localStorage` does not un-seed the module cache. Note also
  that `productLanguage.test.ts` became `.test.tsx` — esbuild only parses
  JSX in `.tsx`, whatever the `@vitest-environment` pragma says. The
  worktree `.worktrees/session3-ets-import` is still fully merged dead
  weight, left in place pending the user's call.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-09-12 03:12
- **Completed:** **T31 merged to `main` (`ae1cb94`, `--no-ff`).** Branch
  `t31-overlay-shell` (`ea54b0c`..`ced4499`, ten commits) is in. After
  task 5 the coordinator added three commits: `8fb1c68` renamed
  `KNOWN_LIMITATIONS.md` §20's header from "unaddressed" to "— partially
  resolved" (its body already said "Resolved (2026-09-12, T31)", so the
  heading was contradicting itself two lines later) and repointed all
  three links to the new anchor slug — `GAP_ANALYSIS_ETS.md`'s D9 row and
  T31 backlog entry, plus the design spec's "Closes:" line; an em dash
  becomes a double hyphen in a GitHub slug. `c77c9e5` fixed the final
  review's one nit: the three combobox inputs lacked
  `aria-haspopup="listbox"`, which the ARIA Authoring Practices Guide
  lists for this pattern so a screen reader announces a popup listbox
  rather than inline autocomplete — added to `Search.tsx`,
  `CommandPalette.tsx` and `CatalogBrowser.tsx`, with assertions folded
  into the three existing `aria-activedescendant` tests. `ced4499`
  appended the review record to
  `.ai/logs/2026-09-12_claude_t31_overlay_shell.md`.
  The final whole-branch review (Sonnet, read-only) returned **APPROVE**
  with 0 blocker / 0 major / 0 minor / 1 nit. It re-ran all seven gates
  itself and proved three behaviours load-bearing by deleting them and
  watching named tests fail: `Overlay.tsx`'s focus-restore cleanup, the
  `ArrowDown` branch of `CatalogBrowser.tsx`'s `handleSearchKeyDown`, and
  the `search-overlay` copy-paste guard. Each injection was reverted and
  the tree proven clean.
  **Gates re-measured on merged `main` (`ae1cb94`), all green:** web
  **235 passed across 25 files**, `tsc --noEmit` clean, `cargo fmt`
  clean, `cargo clippy --workspace --all-targets -- -D warnings` clean,
  `cargo test --workspace` **975 passed / 0 failed / 3 ignored across 72
  binaries**, `check-layering` ok, `cargo deny check` ok.
- **Pending/Next Steps:** Next slice not yet chosen. The standing
  candidate on priority grounds is **T26** (KNX data translations, gap
  **D10**'s data half, `KNOWN_LIMITATIONS.md` §37): `knx_core::string_table`
  and `knx-productdb`'s `translation` table are populated on import and
  read by nothing — a data-integrity gap, which outranks **T25**'s UI
  chrome i18n under `CLAUDE.md`'s priority order and needs no new npm
  dependency. Neither track has a design spec yet; `ROADMAP.md`'s
  Internationalization section says the library choice, the storage of the
  language setting (D8's options dialog still does not exist) and what a
  project's own `Language` means once the user can pick another all belong
  in one. Other open work unchanged: T17 (line scan, needs a spec research
  spike), T21 (graphical views, D1/D2), T28 (in-app help, D12), T16, T22,
  D8's real settings contents, B10, C3, C5, C6, E5, E6, F3. Blocked with
  named conditions: T30/E1 (commissioning), T19/E3 (KNX Secure), T20/A1
  (Functions), A4 (schemas 12-22).
- **Notes for Codex:** §20 stays **open** and must not be closed: no
  scroll-into-view for an off-panel highlight, no `inert`/`aria-hidden`
  background, no focus-visible styling pass, and **no verification against
  a real screen reader or a real browser anywhere in this slice** —
  happy-dom asserts DOM attributes, not what NVDA, JAWS, Orca or VoiceOver
  announce. No WCAG or other conformance is claimed and no audit was
  performed. `overlayShell.test.ts` is a hard guard: a fifth `.tsx` file
  under `apps/knx-web/src` containing the literal `search-overlay` fails
  the suite by name. Import `Overlay` instead. Note also that the guard
  enumerates filenames via `import.meta.glob` keys rather than
  `readdirSync`, which is **not** declared — `node-builtins.d.ts` declares
  exactly `readFileSync`, `fileURLToPath`, `dirname`, `join`, and
  `@types/node` is not in this tsconfig's scope. Do not run
  `npm run build` while iterating: Vite's `outDir` clean step deletes the
  tracked `apps/knx-web/dist/.gitkeep`; `npx tsc -p apps/knx-web/tsconfig.json
  --noEmit` is the type gate. The worktree `.worktrees/session3-ets-import`
  is fully merged dead weight, left in place pending the user's call.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-09-12 03:00
- **Completed:** **T31 (shared modal overlay shell) — all five tasks
  done on branch `t31-overlay-shell`** (worktree
  `.worktrees/t31-overlay-shell`, `ea54b0c`..HEAD, four code commits
  `bdba7cb`/`a0c6dbe`/`8d6220d`/`3eff155` plus this docs commit). Not
  merged to `main` yet — that is the coordinator's call, not this task's.
  `apps/knx-web/src/Overlay.tsx` (new, 94 lines, no new dependency) is
  now the one component behind `.search-overlay`/`.search-panel`:
  `role="dialog"`/`aria-modal`, backdrop and panel-level `Escape`
  (a `keydown` listener on the panel, not `window`, not an input),
  initial focus (`initialFocusRef`, else first focusable descendant,
  else the panel), a hand-written `Tab` focus trap, and focus
  restoration on unmount. All four former hand-rolled consumers —
  `Search.tsx`, `CommandPalette.tsx`, `CatalogBrowser.tsx`,
  `SettingsPanel.tsx` (whose `window` `Escape` listener is now deleted
  outright) — render it instead. Listbox semantics
  (`combobox`/`listbox`/`option`/`aria-activedescendant`) are applied
  uniformly across the three list-bearing overlays.
  `CatalogBrowser.tsx`'s result list — previously `<li onClick>` with no
  keyboard path at all — gained `ArrowDown`/`ArrowUp`/`Enter`-to-pick,
  matching what a mouse click already did (selects and pre-fills the
  name field; does not create the device). `overlayShell.test.ts` fails
  the suite, naming the offender, if a fifth `.tsx` file ever contains
  the literal `search-overlay` outside `Overlay.tsx` — the exact failure
  mode (a missed lift trigger, twice) that let this reach four consumers
  unnoticed. `styles.css` untouched; no npm dependency added; no Rust
  file touched anywhere in the slice.
  Docs reconciled in task 5 (docs-only, no code): `KNOWN_LIMITATIONS.md`
  §20 rewritten from "unaddressed" to "resolved" in its body (header
  line kept byte-identical — `GAP_ANALYSIS_ETS.md` links its anchor
  slug), naming what shipped and, honestly, what didn't (no
  scroll-into-view, no `inert` background, no focus-visible pass, no
  screen-reader verification — jsdom asserts wiring, not
  assistive-technology behaviour; no WCAG or other conformance claimed,
  no audit performed); `GAP_ANALYSIS_ETS.md` row **D9** closed and a
  **T31** backlog entry added (no existing tier fit, said so instead of
  inventing one); `IMPLEMENTATION_STATUS.md` dated entry added and
  `Last updated:` moved to 2026-09-12; `ROADMAP.md` checked and left
  alone — nothing there was falsified by this slice;
  `.ai/logs/2026-09-12_claude_t31_overlay_shell.md` written. Gates
  re-measured on this branch: `npm run test` (`apps/knx-web`) **235
  passed across 25 files** (up from 215/21 at the branch point,
  `ea54b0c`); `npx tsc -p apps/knx-web/tsconfig.json --noEmit` clean;
  Rust gates unchanged (no Rust file touched).
- **Pending/Next Steps:** T31 is ready for coordinator review and merge
  to `main`; the `.worktrees/t31-overlay-shell` worktree can be removed
  once merged. Remaining backlog, unchanged by this slice: T17 (line
  scan, D6/E2), T21 (graphical views, D1/D2), T25/T26 (i18n, D10), T28
  (in-app help, D12), T16 (catalog browser for topology), T22
  (multi-user, §63), D8's real settings contents, D10 (i18n). Blocked
  with named conditions: T30/E1 (commissioning), T19/E3 (KNX Secure),
  T20/A1 (Functions), A4 (schemas 12-22).
- **Notes for Codex:** Same two traps as T27's entry below, still live
  and now with a second guard test proving the pattern: (1) `npm run
  test` is `vitest run`, not a build — `npm run build` (`tsc && vite
  build`, `include: ["src"]`, no `@types/node`) is a separate gate, run
  as `npx tsc -p apps/knx-web/tsconfig.json --noEmit` instead of the
  real `vite build`, because a real build deletes the tracked
  `apps/knx-web/dist/.gitkeep`. (2) Vitest does not process CSS or
  bundle `?raw` — `import x from "./y?raw"` silently resolves to the
  empty string and produces a green test that checks nothing;
  `overlayShell.test.ts` and `motionGuard.test.ts` both read source off
  disk with `node:fs` and the `node-builtins.d.ts` declarations instead.
  Net new for whoever touches an overlay next: don't hand-roll
  `.search-overlay`/`.search-panel` again — use `Overlay.tsx`, or
  `overlayShell.test.ts` will fail the suite and name the file.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-09-12 02:25
- **Completed:** **Merged T27 (in-app motion control) to `main`** —
  `c22d5f9`, `--no-ff`, from branch `t27-motion-control` (nine commits,
  `bb43da4`..`b0e37ce`). The whole slice in one place: `motion.ts`'s two
  persisted axes (level `off`/`subtle`/`standard`, style `apple`/`glitch`)
  applied as `data-motion-level`/`data-motion-style` on `<html>`; a
  `styles.css` token layer (`--knx-transition-duration`,
  `--knx-motion-easing`) plus an `index.html` pre-mount bootstrap;
  `motionGuard.test.ts`, which fails the suite if a `transition:`/
  `animation:` declaration in `styles.css` sits outside a
  `@media (prefers-reduced-motion: no-preference)` block or uses a literal
  duration; `SettingsPanel.tsx` behind a gear button (Theme, Motion style,
  Motion level), replacing the deleted `ThemeSwitcher.tsx`; and the T15
  retrofit the roadmap's own text demanded — `BusMonitorPanel.tsx`'s
  new-row entry highlight. Docs reconciled in the same slice
  (`KNOWN_LIMITATIONS.md` §43 rewritten with five remaining limits stated,
  `GAP_ANALYSIS_ETS.md` D11 closed / D8 partial / D9 corrected,
  `ROADMAP.md`'s motion section and its 2026-09-10 style memo answered,
  a dated `IMPLEMENTATION_STATUS.md` entry, `.ai/logs/2026-09-12_claude_t27_motion_control.md`).
  Two coordinator corrections after task 5: `KNOWN_LIMITATIONS.md` §20's
  "lifted when a third overlay is added" trigger had already fired at
  `CatalogBrowser.tsx` unnoticed — four components now consume
  `.search-overlay`/`.search-panel`, and `CatalogBrowser.tsx`'s result
  rows have no keyboard path at all, so §20 and D9 both say so now; and
  `styles.css`'s comment over `.sr-only` now calls it dead code, which it
  is. A final whole-branch review verified the guard by injecting a
  violation and watching it name the line, confirmed no `matchMedia` and
  no `!important` anywhere in `apps/knx-web/src`, and returned two minor
  findings, both fixed before the merge. All seven gates re-measured on
  the merged `main`: `cargo fmt --all --check` clean; `cargo clippy
  --workspace --all-targets -- -D warnings` clean; `cargo test
  --workspace` **975 passed / 0 failed / 3 ignored** across 72
  `test result` lines (unchanged — no Rust file was touched); `cargo run
  -p xtask -- check-layering` ok; `cargo deny check` ok; `npm run test`
  (`apps/knx-web`) **215 passed across 21 files**, up from 184/18 before
  the slice; `npx tsc -p apps/knx-web/tsconfig.json --noEmit` clean.
- **Pending/Next Steps:** T27 is closed. The `.worktrees/t27-motion-control`
  worktree can be removed. Remaining backlog, unchanged by this slice: T17
  (line scan, D6/E2), T21 (graphical views, D1/D2), T25/T26 (i18n, D10),
  T28 (in-app help, D12), T16 (catalog browser for topology), T22
  (multi-user, §63), D8's real settings contents, D9/§20 (overlay shell
  extraction — now overdue rather than speculative, plus the accessibility
  pass), B10 (drag & drop), C3 (partial import), C5 (signed export), C6
  (online catalog update), E5 (Docker `--network host`), E6 (custom
  routing multicast), F3 (licence). Blocked with named conditions:
  T30/E1 (commissioning), T19/E3 (KNX Secure), T20/A1 (Functions), A4
  (schemas 12-22).
- **Notes for Codex:** Two things worth knowing before touching
  `apps/knx-web`. (1) **`npm run test` does not build the application.**
  It is `vitest run`; `npm run build` is `tsc && vite build` over
  `include: ["src"]`. A `node:` import in a test file passes Vitest and
  breaks the build — it did, on this branch, and `apps/knx-web/src/node-builtins.d.ts`
  is the 27-line fix. `npx tsc -p apps/knx-web/tsconfig.json --noEmit` is
  a gate now; please keep running it. (2) **Vitest does not process CSS.**
  `import css from "./styles.css?raw"` resolves to the empty string, so
  `motionGuard.test.ts` reads the stylesheet with `node:fs` on purpose.
  Tidying that import away would leave a green test that checks nothing.
  Both are recorded in `KNOWN_LIMITATIONS.md` §43.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-09-12 02:05
- **Completed:** **T27 task 5 (docs-only): reconciled documentation with
  the shipped in-app motion control.** Branch `t27-motion-control`,
  worktree `.worktrees/t27-motion-control`, head `b2aac7d` before this
  task. Rewrote `KNOWN_LIMITATIONS.md` §43 in full (two-axis control now
  exists, structurally OS-wins, guarded by `motionGuard.test.ts`; five
  specific remaining limits recorded honestly: no per-category control,
  guard reads `styles.css` only, guard matches shorthands only, no test
  observes actual motion, and the `node:fs`/`?raw` guard-disarming trap).
  Updated `GAP_ANALYSIS_ETS.md`: D11 closed, D8 marked partially addressed
  (a `SettingsPanel` exists but holds none of ETS's actual options), D9
  corrected — not "two" overlay-CSS consumers but four
  (`Search`/`CommandPalette`/`CatalogBrowser` already, `SettingsPanel` now
  a fourth; row stays open, arguably worse), and the Tier 7 T27 backlog
  entry marked done with its open question answered (two orthogonal axes,
  not a duration multiplier or a per-category switch alone). Updated
  `ROADMAP.md`'s "Cross-cutting — Motion and animation" section and its
  2026-09-10 style memo (both directions shipped, memo's wording kept
  verbatim as the question asked). Added a dated 2026-09-12 entry to
  `IMPLEMENTATION_STATUS.md` and updated its `Last updated:` line. Swept
  every `ThemeSwitcher.tsx` mention in current-state docs
  (`GAP_ANALYSIS_ETS.md`, `IMPLEMENTATION_STATUS.md` ×2, `ROADMAP.md`) to
  say it was deleted by T27; left every mention under
  `docs/superpowers/` (including this slice's own spec/plan and the
  bitcoin-defi theme spec) untouched as historical record. New
  `.ai/logs/2026-09-12_claude_t27_motion_control.md`. Re-ran all seven
  gates myself rather than trusting prior reports: `cargo fmt --all
  --check` clean; `cargo clippy --workspace --all-targets -- -D warnings`
  clean; `cargo test --workspace` **975 passed / 0 failed / 3 ignored**
  across 72 `test result` lines; `cargo run -p xtask -- check-layering`
  ok; `cargo deny check` ok; `npm run test` (`apps/knx-web`) **215 passed
  across 21 files**; `npx tsc -p apps/knx-web/tsconfig.json --noEmit`
  clean. No code under `apps/knx-web/src` touched, as required.
- **Pending/Next Steps:** T27 is complete; this branch is ready to merge
  to `main` (not done as part of this task — docs-only tasks in this
  project's history get merged separately, see the entry below). Next
  slice not yet chosen.
- **Notes for Codex:** Nothing in this commit changes runtime behaviour —
  verify with `git diff --stat` against `.ai/` and `docs/` paths only if
  you want to double-check before building on this branch. One open
  finding for whoever picks up `KNOWN_LIMITATIONS.md` §20 next: overlay
  CSS (`.search-overlay`/`.search-panel`) now has four consumers
  (`Search.tsx`, `CommandPalette.tsx`, `CatalogBrowser.tsx`,
  `SettingsPanel.tsx`), not the two §20's own prose still names — that
  section's own text is out of this task's scope (only `ThemeSwitcher`
  mentions and the three named docs were in scope) but is worth a look.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-09-12 04:35
- **Completed:** Merged the entry below to `main` (`b480805`, `--no-ff`)
  and re-ran all six gates **on the merged result**: fmt clean, clippy
  clean, `cargo test --workspace` **975 passed / 0 failed / 3 ignored**,
  layering ok, `cargo deny check` ok, `npm run test` **184 passed across 18
  files** — unchanged from the pre-merge baseline, as a docs-only change
  should be. Branch and worktree removed; `origin/main` is at `b480805`.
- **Pending/Next Steps:** Next slice is **T27** (in-app motion control,
  gap D11, partly D8) — design drafted this session, not yet in the repo.
- **Notes for Codex:** Nothing in this merge changes runtime behaviour.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-09-12 04:20
- **Completed:** **Docs-only: closed gap F2 (T22 backlog scope, not implemented).**
  Worktree `t22-concurrency-limitation` off `main` at `6112d3d`. Verified in
  code (not assumed) that `apps/knx-server` shares one `Arc<AppState>`
  across every connected browser (`main.rs:23`, `lib.rs:21,41`), that
  `undo`/`redo` operate on one process-wide `Mutex<CommandStack>`
  (`domain.rs:48,1638-1658` — a second client's undo can pop a first
  client's command), that no write route carries any ETag/`If-Match`/
  revision check, that no push/poll mechanism tells a client the project
  changed (`apps/knx-web` has zero `WebSocket`/`EventSource`, and the one
  `setInterval` in the whole frontend is `BusMonitorPanel.tsx:228`'s bus
  telegram poll, unrelated), and that `save_project`/`save_project_as`
  (`domain.rs:413-462` → `knx-store/src/project.rs:72,93-97`) do an
  unconditional `DELETE`-all-then-reinsert with no on-disk version check —
  two clients saving the same `.knxdb` is silent last-writer-wins. Also
  confirmed what *is* protected: `apply`/`undo_impl`/`redo_impl` hold the
  same locks for one command's full duration, so two requests cannot
  interleave into a torn in-memory `Project` — a real guarantee, just a
  narrow one. Added `docs/KNOWN_LIMITATIONS.md` §63 with that evidence,
  cross-referenced from §22 (no-auth) and from `docs/GAP_ANALYSIS_ETS.md`
  row F2 (still open, still unimplemented — not downgraded). Confirmed the
  Tauri desktop shell builds the identical `Arc<knx_server::AppState>`
  type (`src-tauri/lib.rs:31,57`) but binds `127.0.0.1` for one local
  webview (`lib.rs:60-73`), so it shares this limitation's *shape* without
  its exposure. `docs/ARCHITECTURE.md`/`IMPLEMENTATION_STATUS.md`/
  `docs/ROADMAP.md` had no multi-user-safety claim to correct — checked,
  none found, left untouched. No production code, schema, command, or UI
  changed. Gates re-run on this branch, matching the `main`-at-`6112d3d`
  baseline exactly: `cargo fmt --all --check` clean; `cargo clippy
  --workspace --all-targets -- -D warnings` clean; `cargo test --workspace`
  **975 passed / 0 failed / 3 ignored** across 72 `test result` lines;
  `cargo run -p xtask -- check-layering` ok; `cargo deny check` ok
  (pre-existing `advisory-not-detected` warnings only, exit 0); `npm run
  test` in `apps/knx-web` **184 passed across 18 files** (a fresh `npm
  install` was needed in this worktree — `node_modules` isn't shared
  across worktrees).
- **Pending/Next Steps:** T22 itself (the actual locking/merge/
  last-writer-wins design and implementation) is still unstarted — this
  task only documented the hazard, per its own brief. Next up per
  `docs/ROADMAP.md`'s existing order: T27 (motion toggle, D11), T17 (line
  scan, D6/E2), T21 (graphical views), T25/T26 (i18n), T28 (in-app help),
  T16 (catalog browser), D8 (settings dialog), D9 (duplicate overlay
  implementations + accessibility), T22 (multi-user — now documented,
  still needs its own design decision). Blocked with named conditions:
  T30 (commissioning), T19 (KNX Secure), T20 (Functions), A4 (schemas
  12-22).
- **Notes for Codex:** `docs/KNOWN_LIMITATIONS.md` gained one new numbered
  entry (§63) at the end of the file, plus a one-line cross-reference
  appended to §22's existing "Lifted when" paragraph — no existing entry
  was reworded, renumbered, or softened otherwise. `docs/GAP_ANALYSIS_ETS.md`
  row F2's second cell now links to §63 instead of saying "new finding";
  the gap itself is still listed as open. If you pick up T22's actual
  implementation later, the open design question §63/the T22 backlog
  entry both point at is unchanged: locking vs. merge vs. last-writer-wins,
  and what "conflict" even means for a `Command`-based undo model — no
  decision has been made on that yet, this branch only wrote down the
  symptom.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-09-12 03:05
- **Completed:** **T18 slice 3 merged to `main`.** Branch
  `t18-parameter-editor` (19 task commits from `cc38de5`, plus `4f36e37`
  closing the final review's one nit — `docs/RESEARCH.md`'s §4.4 addendum
  now cites the reproducible source, the three `OriginalData/DemoProjects/`
  archives and the unpack-read-only method, instead of a job scratch path
  that will not outlive the session) merged with `--no-ff`. Final
  whole-branch review verdict before the merge: **fit to merge**, 0
  blocking, 0 should-fix, 1 nit (now closed). Six gates re-run by the
  controller **on the merged `main`, not on the branch**: `cargo fmt --all
  --check` clean; `cargo clippy --workspace --all-targets -- -D warnings`
  clean; `cargo test --workspace` **975 passed / 0 failed / 3 ignored**
  across 72 `test result` lines (baseline at `cc38de5` was 951/0/3);
  `cargo run -p xtask -- check-layering` ok; `cargo deny check` ok;
  `npm run test` in `apps/knx-web` **184 passed across 18 files**
  (baseline 179 across 17). What shipped: `GET`/`POST
  /api/device/{id}/parameters`, a `ParameterPanel` in the web Inspector,
  `SetParameterValue` as an undoable command, product-database-informed
  validation assembled in `apps/knx-server` (the layering rule stands —
  `knx-productdb` still reaches neither `knx-etsproj` nor `knx-store`),
  and the docs set reconciled in the same change.
- **Pending/Next Steps:** Module-scoped (per-channel) parameter writes
  remain out (D25) and unscheduled — a slice 4 would first have to give
  the evaluator's flat `ValueMap` a scope slot. Otherwise pick the next
  item from `docs/ROADMAP.md`: T27 (motion toggle, D11), T17 (line scan,
  D6/E2), T21 (graphical views), T25/T26 (i18n), T28 (in-app help), T16
  (catalog browser), D8 (settings dialog), D9 (duplicate overlay
  implementations + accessibility). Blocked with named conditions: T30
  (commissioning — researched, blockers now physical), T19 (KNX Secure),
  T20 (Functions), A4 (schemas 12-22). The `.worktrees/session3-ets-import`
  worktree is fully merged dead weight and can be removed whenever the
  user says so.
- **Notes for Codex:** The merge is a `--no-ff` commit on `main`; no
  history was rewritten and nothing was force-pushed. Parameter values now
  have exactly one write path — the `SetParameterValue` command through
  `knx-store` — and top-level only; if you add a second writer, it must go
  through the same command or undo/redo silently diverges from the stored
  rows. Stale values (stored ids the current program cannot interpret) are
  deliberately surfaced in the panel rather than dropped; do not "clean
  them up" on import or write.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-09-12 01:15
- **Completed:** **T18 slice 3, Task 5 (docs reconciliation, the slice's
  last task)** — same worktree (`.worktrees/t18-parameter-editor`,
  branch `t18-parameter-editor`), on top of `b25eb6d`. Docs-only, no
  production logic change (code comments allowed, none needed). Updated:
  `docs/IMPLEMENTATION_STATUS.md` (new dated T18-slice-3 entry between
  the slice-2 and T29 entries, plus a fixed stale "no parameter editor
  exists" bullet); `docs/KNOWN_LIMITATIONS.md` §3 (rewritten body, header
  text left unchanged to preserve its anchor slug — 4 occurrences across
  3 other files (`GAP_ANALYSIS_ETS.md` ×2, `DATA_MODEL.md` ×1,
  `IMPLEMENTATION_STATUS.md` ×1) link
  `#3-device-parameters-are-preserved-but-not-interpreted`, not counting
  `KNOWN_LIMITATIONS.md`'s own 2 internal backlinks) and §12
  (surfaced/writable-top-level-only correction, plus its Cause/Impact
  paragraphs further down, updated beyond the literal brief wording so
  they don't contradict the rewritten paragraph immediately above);
  `docs/GAP_ANALYSIS_ETS.md` (A3 row, Tier-5 T18 paragraph, and B8 row —
  B8 also updated beyond the literal brief wording for the same
  no-contradiction reason); `docs/DATA_MODEL.md` §10 (`ParameterInstance`
  now has a reader/writer, still keyed by `(device, ets_id)`, cross-
  references D25 by number); `docs/ARCHITECTURE.md` (only the "Device
  parameter editing" v1-target-table row, nothing else in the file
  touched); `docs/RESEARCH.md` §4.4 (new addendum recording the project-
  side id-shape format fact `<Module/@Id>_MI-<k>_<declared suffix>` as
  `[V]`, re-derived independently from the three demo `.knxproj` files
  rather than copied from the brief — all figures matched: KV 9 rows/9
  module-qualified/0 verbatim; UZ 6.3.0 1343 rows/0/1343, 208 of those
  union-typed; UZ4 1390 rows/0/1390, 216 union-typed; stripping
  `_M-\d+_MI-\d+_` → `_` recovers the declared id for 9/9 KV rows; no
  declared id in any of the three projects already contains that
  pattern; `_MI-` is always `1`, unattested above 1; KV stores five
  distinct values 17/33/49/32/48 for one declared `ParameterRef` across
  five `Module` instantiations). Also wrote
  `.ai/logs/2026-09-11_claude_t18_parameter_editor_slice3.md` (the
  slice's architecture-change log, covering what all four code tasks
  changed plus the three decisions that moved during implementation:
  D22's `display_order` becoming `Option<i64>`, D24's undo/redo payload
  being `Option<String>` not `Override<T>`, D22 gaining `access`).
  Nothing in `evaluate`/`resolve_values` touched; no schema change; D16
  and D25 both restated as still true, not downgraded; no ETS parity or
  KNX-certification claim added anywhere. Full detail, including the
  step-7 re-derivation commands/output and the step-8 grep-occurrence
  audit, in the task report at
  `/home/knxbench/.claude/jobs/8098e9e6/tmp/t18s3-task5-report.md`.
- **Pending/Next Steps:** T18 slice 3 is now fully done (all five tasks).
  Next is merging this worktree to `main` (controller's call) and then
  deciding on a possible slice 4 (per-channel module-scoped writes, D25's
  named non-goal) or moving on per `docs/ROADMAP.md`.
- **Notes for Codex:** The docs changes are prose-only; nothing here
  should conflict with any in-flight code work. If a future slice adds
  module-scoped writes, it will need to touch the same six docs files
  again (`KNOWN_LIMITATIONS.md` §3/§12, `GAP_ANALYSIS_ETS.md` A3/B8/Tier
  5, `DATA_MODEL.md` §10, `ARCHITECTURE.md`'s one row, `RESEARCH.md`
  §4.4) plus whatever changes the evaluator's flat `ValueMap` needs to
  gain a scope slot — that's the structural blocker named in D25, not a
  missing storage key.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-09-12 00:02
- **Completed:** **T18 slice 3, Task 4, fix round 1** — same worktree
  (`.worktrees/t18-parameter-editor`, branch `t18-parameter-editor`),
  commit `b25eb6d` on top of `e2ca2e7`. Review returned spec compliance
  approved, task quality changes-requested (0 blocking, 1 should-fix, 2
  nits). Both self-flagged concerns (blur-commit `<select>`, "Copy
  details" with no feedback) were ruled correct-as-shipped by the
  reviewer; the `(none)` placeholder nit was ruled leave-as-is by the
  controller (the server already 400s an empty `raw`, so no client-side
  required-field logic was added). Only the should-fix required action:
  two behaviours the component already implemented correctly had no
  pinning test.
  - `apps/knx-web/src/ParameterPanel.test.tsx` gained two tests, no
    change to `ParameterPanel.tsx` itself:
    1. A rejected `setParameterValue` reverts the input to its pre-edit
       value and shows the *exact* rejection message text in
       `.field-error` (concrete-value assertion, not a presence check).
    2. `programId === null` renders the empty-program sentence, renders
       zero `.parameter-field` rows, and still renders a non-empty
       `stale` list with its `etsId`/`raw` visible — pinning the
       controller's pre-dispatch ruling from the original task (fetch
       unconditionally, never hide `stale` behind an unresolved
       program) so a future refactor can't silently re-gate the fetch
       with every existing gate staying green.
  - Test-first, genuinely: both assertions were written and run before
    any other change; both passed immediately against the already-shipped
    implementation. Reported plainly as pinning already-correct
    behaviour, not framed as caught bugs — the honest and expected
    outcome the brief itself predicted.
  - Gates: Rust unchanged, **975 passed / 0 failed / 3 ignored**;
    fmt/clippy/layering/deny all clean (deny: pre-existing
    advisory/license warnings only, exit 0). `npm run test` in
    `apps/knx-web`: **184 passed across 18 files** (182/18 baseline + 2
    new, same file).
- **Pending/Next Steps:** T18 slice 3 Task 4 and its fix round are both
  done. Same as the prior entry: next is merging this worktree to `main`
  (controller's call) and then deciding on a possible slice 4 (per-
  channel module-scoped writes, D25's named non-goal) or moving on per
  `docs/ROADMAP.md`.
- **Notes for Codex:** The two new pinning tests in
  `ParameterPanel.test.tsx` exist specifically to catch a regression of
  the controller's "fetch unconditionally, never hide `stale`" ruling —
  if you ever see a PR that reintroduces a `program_id != null` gate on
  the parameters fetch, the `renders the empty-program state while still
  rendering a non-empty stale list` test is what should catch it (and if
  it doesn't fail, something else already broke first).
---
- **Last Agent:** Claude
- **Timestamp:** 2026-09-11 23:55
- **Completed:** **T18 slice 3, Task 4** (the last code task of this
  slice) — same worktree (`.worktrees/t18-parameter-editor`, branch
  `t18-parameter-editor`), commit `dadc92b` on top of `f99da6e`. Built
  the `apps/knx-web` UI consuming Task 3's `ParameterPanelDto` endpoints.
  - `apps/knx-web/src/ParameterPanel.tsx` (new): fetches
    `GET /api/device/{id}/parameters` unconditionally on device
    selection (per the controller's ruling overriding the brief's
    `program_id != null` gate — `DeviceDetail` carries no program id, and
    a device with an unresolved program still returns 200 with its own
    `stale` entries, which a conditional fetch would hide). Renders one
    `<details open>` section per `ParameterSectionDto` (D23's
    `module_id` / `"Module #{module_node}"` fallback label), a
    select/number/text input per field `kind`, disabled input + a
    read-only caption for `editable: false` module-scoped fields (D25),
    a separate stale-values section (D21, never merged into the normal
    list), and a collapsed count-headed diagnostics banner with `detail`
    behind a "Copy details" button (D26). Submitting an editable field's
    change POSTs `{ etsId, raw }` and replaces panel state directly from
    the response — no second `GET`, per D24.
  - `apps/knx-web/src/api.ts`: added `deviceParameters`/
    `setParameterValue` plus hand-written `ParameterPanel`/
    `ParameterSection`/`ModuleScope`/`ParameterField`/`EnumOption`/
    `StaleParameter`/`ParameterDiagnostic` interfaces (server-local DTOs,
    no `ts-rs` binding, same convention as `CsvProblem`).
  - `apps/knx-web/src/Inspector.tsx`: one-line call site —
    `<ParameterPanel deviceId={detail.id} />` at the end of
    `DeviceInspector`. `apps/knx-web/src/styles.css`: new
    `.parameter-*` classes alongside the existing `.inspector-field`/
    `.field-error`/`.provenance-badge` ones.
  - Test-first, genuinely: `ParameterPanel.test.tsx` was written and run
    against a nonexistent component first — failed with `Failed to
    resolve import "./ParameterPanel"`, the right reason — before any
    implementation existed. All three tests (fixture render with two
    section field-rows/stale entry/diagnostic count; submit-and-rerender
    via a `focusout` blur-commit, mirroring `Inspector.tsx`'s existing
    field idiom; module-scoped `<select>` present-but-disabled) passed on
    the first implementation attempt — reported as such, not framed as a
    caught bug. Also separately probed, then deleted, a throwaway spec
    confirming happy-dom/React 18 only delivers a synthetic `onBlur` via
    a bubbling `FocusEvent("focusout", ...)`, not a plain
    `Event("blur")` — that's why the test dispatches `focusout`.
  - Gates: `cargo fmt --all --check` clean; `cargo clippy --workspace
    --all-targets -- -D warnings` clean; `cargo test --workspace` **975
    passed / 0 failed / 3 ignored** (baseline, unchanged); `cargo run -p
    xtask -- check-layering` clean; `cargo deny check` clean (pre-existing
    advisory/license warnings only, exit 0); `npm run test` in
    `apps/knx-web` **182 passed across 18 files** (179/17 baseline + 3
    new in 1 new file). `npx tsc --noEmit` also clean (not a listed gate,
    checked anyway since `package.json`'s `build` script runs `tsc`).
- **Pending/Next Steps:** T18 slice 3's code tasks (1-4) are all done and
  merged onto this branch. Next: get this worktree merged to `main` (not
  done as part of this task — controller's call), then decide whether a
  slice-4 (per-channel module-scoped writes, named as an explicit
  non-goal in D25) is worth opening, or whether T18 moves to something
  else entirely per `docs/ROADMAP.md`.
- **Notes for Codex:** The parameter panel always fetches on device
  selection now, even for devices with no resolvable program — check
  `ParameterPanel.tsx`'s top-of-file comment before "fixing" that into a
  conditional fetch; it's deliberate (see D21's `stale` framing). The
  blur-commit test idiom needs `new FocusEvent("focusout", { bubbles:
  true })`, not `new Event("blur", ...)` — happy-dom/React 18 doesn't
  deliver the latter to `onBlur` handlers at all, so a test written with
  `"blur"` will silently never fire the submit and can look like a
  passing no-op test if you're not asserting the mock was called.
---
- **Last Agent:** Claude
- **Timestamp:** 2026-09-11 23:32
- **Completed:** **T18 slice 3, Task 3, fix round 1** — same worktree
  (`.worktrees/t18-parameter-editor`, branch `t18-parameter-editor`), on
  top of `f152ac4`. Review returned spec compliance approved, task
  quality changes-requested (0 blocking, 3 should-fix, 2 nits); all five
  addressed this round, test-first this time (unlike round 1, which
  skipped that ordering and said so).
  - `ParameterFieldDto` (`apps/knx-server/src/routes.rs`) gains two
    fields: `display_order: Option<i64>` (serialised `displayOrder`,
    `None` -> JSON `null`, never skipped, never `0`) and
    `access: Option<String>` — both verbatim from `ParameterView`, both
    populated at the one construction site in `domain.rs`'s
    `assemble_parameter_panel`. `access` is shown, never used to gate
    `editable` (D24 still doesn't gate on it). This also resolves a
    standing self-contradiction in the design doc: its Non-Goals prose
    already claimed `access` was shown verbatim while D22's own struct
    listing omitted it; the struct listing
    (`docs/superpowers/specs/2026-09-11-parameter-editor-design.md`) now
    carries both fields so it matches the shipped DTO.
  - Four new tests, written and run before any implementation change, as
    required this round:
    1. `domain::tests::decompose_module_qualified_keeps_the_rightmost_of_two_valid_markers`
       (private function, so it lives in `domain.rs`'s own `mod tests`) —
       an id with two valid `_M-<digits>_MI-<digits>_` markers back to
       back; asserts the rightmost split. Ran green on first try, no
       implementation change needed — the function was already correct,
       this only pins it.
    2. `field_order_is_document_order_not_display_order`
       (`http_parameter_panel.rs`) — a new fixture,
       `DOCUMENT_ORDER_DISAGREES_WITH_DISPLAY_ORDER_PROGRAM`, whose two
       `ParameterRef`s declare `DisplayOrder="20"`/`"10"` but activate in
       the opposite order. Also ran green on first try — field order was
       already `Activation::parameter_refs`' document order, not a
       `display_order` sort; this pins that too.
    3. `a_program_ref_that_resolves_to_nothing_returns_the_empty_panel`
       (`http_parameter_panel.rs`) — a device whose `program_ref` doesn't
       match any ingested `hardware2program` row, product database
       otherwise present. Also ran green on first try — `empty_assembly`
       already covered this path (`resolve_program` returning `None`
       distinct from `product_db` being absent entirely).
    4. New assertions in the existing
       `get_returns_stored_and_defaulted_top_level_fields` (AC1) test:
       added a third top-level parameter `P-3_R-1` to `WRITE_PROGRAM`
       whose `ParameterRef` declares no `DisplayOrder` attribute at all.
       This one **did** fail first, genuinely: `assertion `left == right`
       failed / left: Null / right: 10` against `p1["displayOrder"]`,
       since the DTO didn't carry the field yet. Fixed by the DTO/
       construction-site change above; both the verbatim (`p1`, `10`) and
       null (`p3`) cases pass now.
  - So: 3 of the 4 mandated tests turned out to be pinning tests against
    already-correct behaviour (genuinely ran green before any code
    change — reported honestly rather than manufacturing a failure that
    wasn't there); only the `display_order`/`access` DTO gap was a real,
    reproducible bug this round fixed.
  - Gates, all in the foreground, one at a time: `cargo fmt --all --check`
    clean (after one `cargo fmt --all` pass over the new test code);
    `cargo clippy --workspace --all-targets -- -D warnings` clean;
    `cargo test --workspace` = **975 passed / 0 failed / 3 ignored**
    (baseline at `f152ac4` was 972/0/3; delta is exactly +3, one per new
    `#[test]`/`#[tokio::test]` function — the DTO-field assertions landed
    inside an existing test, so they don't add to the count); `cargo run
    -p xtask -- check-layering` ok; `cargo deny check` exit 0 (same
    pre-existing `advisory-not-detected` warnings as before, nothing
    new); `npm run test` in `apps/knx-web` = 179 passed / 179, 17 files
    (frontend untouched this round, as expected).
  - No schema change in either database. `crates/knx-etsproj` and
    `crates/knx-productdb/src/dynamic/evaluate.rs` untouched. DTOs stayed
    in `routes.rs`. No new `CommandError`/error variants. No subagents of
    my own.
- **Pending/Next Steps:** Task 3 (including this fix round) is done
  pending coordinator merge to `main`. No further work queued on this
  branch by me.
- **Notes for Codex:** The design doc's `ParameterFieldDto` struct listing
  (D22) now lists `display_order`/`access` alongside the other fields —
  if you're reading that doc to scaffold a client type, both are present
  on the wire (`displayOrder`, `access`), both nullable. The frontend
  (`apps/knx-web`) still has no parameter-panel UI at all as of this
  entry — this task was server-only (D20-D26 explicitly scoped it that
  way); a future slice presumably wires a UI to
  `GET`/`POST /api/device/{id}/parameters`.

- **Last Agent:** Claude
- **Timestamp:** 2026-09-11 23:10
- **Completed:** **T18 slice 3, Task 3** — `apps/knx-server`: the parameter
  panel read model and its write endpoint. Worktree
  `.worktrees/t18-parameter-editor`, branch `t18-parameter-editor` (still
  unmerged; this task started from `db8175a`).
  - `apps/knx-server/src/routes.rs`: `ParameterPanelDto`,
    `ParameterSectionDto`, `ModuleScopeDto`, `ParameterFieldDto`,
    `EnumOptionDto`, `StaleParameterDto`, `ParameterDiagnosticDto` — all
    `pub(crate)`, `#[derive(Serialize)]` `camelCase`, no `ts-rs`, following
    `CatalogInstallReportDto`'s precedent per the coordinator's ruling.
    `GET`/`POST /api/device/{id}/parameters` handlers registered right
    after the existing `/api/device/{id}` route.
  - `apps/knx-server/src/domain.rs`: `assemble_parameter_panel` (Step 1
    locks only `project`, reads `program_ref` + stored `ParameterInstance`
    rows, drops the lock; Step 2 locks only `product_db` — the two
    mutexes are never held together, reversed lock order from
    `create_device_impl` per the coordinator's ruling), `parameter_panel_
    impl`, `set_parameter_value_impl` (D24's validation chain: program
    resolves -> `etsId` in `parameter_ref_ids` -> kind-appropriate check
    (`Number` bounds, `Restriction` membership, `None` always rejected,
    everything else non-empty) -> D25 module-scope rejection -> exactly
    one `Command::SetParameterValue` applied -> fresh `ParameterPanelDto`
    returned in the same response, no second `GET`), `decompose_module_
    qualified` (hand-rolled greedy-rightmost-match replacement for D21's
    `^(.*)_M-(\d+)_MI-(\d+)_(.*)$` — no `regex` crate anywhere in this
    workspace, by design), `validate_kind_and_bounds`, `diagnostic_
    message` (D26's fixed sentence per `Diagnostic` variant), plus the
    coordinator's addition: when `parameter_views(conn, program_id).len()
    != parameter_ref_ids(conn, program_id).len()`, one extra
    `ParameterDiagnosticDto` names the dropped-row count.
  - `apps/knx-server/tests/http_parameter_panel.rs` (new, 11 tests): all
    10 design-doc acceptance criteria (AC1-AC10) plus the coordinator's
    dropped-row-diagnostic test, following `http_device_routes.rs`'s
    hand-built-fixture-XML + `tower::ServiceExt::oneshot` pattern. Five
    inline `ApplicationProgram`/`Hardware` XML fixtures (a top-level +
    one-module-instantiation program for AC1/AC5-AC9; a two-instantiation
    program for AC2/AC3; the KV v2.5 demo shape verbatim for AC4; an
    unparsable-`when/@test` program for AC10; the dropped-row test reuses
    the first fixture with a `DELETE FROM parameter` against the product
    DB).
  - **Process deviation, disclosed rather than hidden**: the task's
    test-first ordering (write a failing test, run it, quote the genuine
    failure, then implement) was **not** followed for this slice — the
    full `routes.rs`/`domain.rs` implementation was written first, and the
    test file second. The tests still caught two genuine bugs once
    written and run for real (not retrofitted to pass trivially): (1) the
    test harness itself first called `knx_productdb::parse::program::
    ingest_program` directly, which — unlike the crate's own top-level
    `knx_productdb::ingest_file` — never runs the second `Dynamic`-reading
    parse pass, so every fixture program had an empty `dynamic_node` table
    and every `evaluate()` call was silently inert (all 10 non-trivial
    tests failed with empty `sections`); fixed by switching the harness to
    `ingest_file`. (2) The `AC10` diagnostics fixture legitimately produces
    *two* diagnostics (`UnparsableTest` then `NoBranchMatched`, since the
    unparsable `when` also never matches and there is no `default`), not
    one as first assumed — the test's own expectation was wrong, not the
    implementation; fixed the assertion, not the code, since the design
    doc's AC10 only requires the *count* to match `Activation::diagnostics`
    1:1, which it already did.
  - All six gates green: `cargo fmt --all --check` clean; `cargo clippy
    --workspace --all-targets -- -D warnings` clean; `cargo test
    --workspace` 972 passed / 0 failed / 3 ignored (baseline at `db8175a`
    was 961/0/3 — delta +11 matches exactly the 11 new tests in
    `http_parameter_panel.rs`, nothing unexplained); `cargo run -p xtask
    -- check-layering` ok; `cargo deny check` exit 0 (`advisories ok, bans
    ok, licenses ok, sources ok`; the `advisory-not-detected` and
    `duplicate` lines are pre-existing warnings, untouched by this task —
    no new dependency was added); `npm run test` in `apps/knx-web` 179
    passed / 179, 17 files (unchanged from baseline, this task touched no
    frontend code).
- **Pending/Next Steps:**
  - **Task 4** (per the plan this task was dispatched from): the
    `apps/knx-web` frontend — a parameter panel UI consuming `GET`/`POST
    /api/device/{id}/parameters` as built here. Not started.
  - **Task 5** (per the plan): doc updates — `docs/KNOWN_LIMITATIONS.md`
    §3, `docs/GAP_ANALYSIS_ETS.md`'s A3 row and T18 entry, and
    `docs/DATA_MODEL.md` §10 (design doc's own AC12; explicitly *not*
    part of Task 3 — the design doc's own D21 prose calls this "the
    `KNOWN_LIMITATIONS.md` text Task 5 writes"). Not started; still
    accurate as of this task, since nothing here changes what those docs
    already say about module-scoped editing being unsupported.
- **Notes for Codex:**
  - The DTOs in `routes.rs` and the `_impl` functions in `domain.rs` are
    all `pub(crate)`, not `pub` — confirmed via `grep -rn "knx_server::"
    apps/` that no external crate (only `apps/knx-desktop/src-tauri`
    touches `knx_server::`, and only for `AppState`/`app`/`DEV_PORT`)
    needs wider visibility. If Task 4's frontend work ends up needing a
    Rust-side helper beyond the two HTTP endpoints, that visibility will
    need revisiting deliberately, not widened as a reflex fix for a
    `private_interfaces` warning.
  - `decompose_module_qualified` is a hand-rolled stand-in for regex
    `^(.*)_M-(\d+)_MI-(\d+)_(.*)$`, scanning left-to-right for every
    syntactically valid `_M-<digits>_MI-<digits>_` marker and keeping the
    *last* one found — this matches a backtracking regex engine's own
    greedy-then-backtrack behaviour for this specific pattern (the
    leading `.*` is greedy, so the engine ultimately settles on the
    rightmost valid split), but it is not a general regex replacement and
    should not be copied elsewhere without re-deriving that equivalence
    for whatever pattern is actually needed there.
  - If you add a new `ApplicationProgram` fixture to any `knx-server`
    integration test, ingest it via `knx_productdb::ingest_file` (which
    classifies the bytes and runs both the `Static` and `Dynamic` parse
    passes), not `knx_productdb::parse::program::ingest_program` directly
    — the latter only reads `Static` and silently leaves `dynamic_node`
    empty, which is exactly the bug this task's own test-writing caught
    the hard way (see "Completed" above).

---

- **Last Agent:** Claude
- **Timestamp:** 2026-09-11 22:15
- **Completed:** **T18 slice 3, Task 2** — `knx-core`/`knx-store`: writing a
  parameter value. Worktree `.worktrees/t18-parameter-editor`, branch
  `t18-parameter-editor` (still unmerged; started from `e493070`).
  - `crates/knx-core/src/command.rs`: two new `Command` variants,
    `SetParameterValue { id, device, ets_id, raw: String }` and
    `RestoreParameterValue { id, device, ets_id, raw: Option<String> }`,
    plus their `apply` arms and a shared private `upsert_parameter_value`
    helper (find-or-create against `installations[0].parameters`, reused
    by both `SetParameterValue::apply` and `RestoreParameterValue`'s
    overwrite branch, so the "no new `ParameterInstanceId` minted on an
    update" rule lives in one place). `SetParameterValue`'s inverse is
    always `RestoreParameterValue`, and vice versa — they alternate, not a
    self-referential `Restore↔Restore` pair like `SetComObjectFlag`'s —
    because a `ParameterInstance` row can be created/deleted by these two,
    not just flag-toggled in place.
  - **One deliberate deviation from the task brief's literal field type**:
    the brief and design D24's own Rust snippet write `raw:
    Override<String>` for `RestoreParameterValue`, with comments
    "`Absent`"/"`Present(prior)`". `crate::provenance::Override<T>` has no
    such variants (`Absent`/`Empty`/`Value(Resolved<T>)`/`Malformed`,
    confirmed via `git log -p` — it never had a `Present` case) and pairs
    every value with a `Resolved`/`Layer` provenance chain that
    `ParameterInstance` doesn't carry (`parameter.rs`: "retained but
    uninterpreted"). Used `Option<String>` instead — `None` = no row
    existed, `Some(prior)` = restore that exact string — the same
    reasoning `SetDeviceDescription` already gives for its own bare
    `Option<String>` ("no provenance layer exists to preserve"). This task
    was explicitly mine to resolve (D24 assignment) where the brief and
    the actual type shape disagreed; documented in the doc comment and
    here rather than silently reusing an incompatible type or inventing a
    new one.
  - `crates/knx-store/src/command_sync.rs` needed one small addition not
    named in the brief: its `sync_after_command` match is exhaustive over
    every `Command` variant (own doc comment: "every `Command` variant has
    a match arm here"), so the two new variants needed a stub arm to keep
    the crate compiling. Added a no-op arm in the same style as the
    existing `CreateArea`/`SetComObjectFlag` stubs ("persistence layer not
    yet implemented ... out of this task's scope"). No new store function
    was added — confirmed `upsert_parameter_instance` is already called,
    unconditionally, from `crates/knx-store/src/project.rs`'s
    whole-installation save loop (`for (i, p) in
    installation.parameters.iter().enumerate() { upsert_parameter_instance
    (&tx, i as i64, p)?; }`), so no gap existed there per the brief's own
    acceptance criteria.
  - Three new unit tests in `command.rs`'s own test module (no database),
    following the `Command::Variant { .. }.apply(&mut project).unwrap()`
    idiom (`update_group_address_inverse_carries_the_previous_values`):
    create-when-absent + undo removes the row;
    overwrite-when-present + undo restores the exact prior string on the
    *same* `ParameterInstanceId` (no new id minted for an update);
    unknown `DeviceId` → `Err(CommandError::DeviceNotFound(..))`, project
    untouched. All three watched red first: temporarily reverted the
    `Command` enum/`apply`/helper additions, ran
    `cargo test -p knx-core --lib command::tests::set_parameter_value`,
    got `error[E0599]: no variant named 'SetParameterValue' found for enum
    'command::Command'` at each of the three call sites, then restored the
    implementation and re-ran green.
  - All six gates green: `cargo fmt --all --check` clean; `cargo clippy
    --workspace --all-targets -- -D warnings` clean; `cargo test
    --workspace` **960 passed / 0 failed / 3 ignored** (960 − 3 new tests
    = 957, matching this task's stated pre-slice baseline exactly); `cargo
    run -p xtask -- check-layering` ok; `cargo deny check` → advisories
    ok, bans ok, licenses ok, sources ok (pre-existing
    advisory-not-detected warnings only, unrelated to this change);
    `npm run test` (apps/knx-web) unchanged at **179 passed across 17
    files** (no frontend code touched).
  - **Unexplained number, flagged rather than guessed at**: the previous
    `.ai/CURRENT_STATE.md` entry below (Task 1 fix round, same branch,
    commit `8747324`, one commit before the `e493070` this task started
    from) reported `cargo test --workspace` as **1017 passed**. My own
    measurement immediately before this task's changes, backed out from
    this run's 960 (960 − 3 = 957), disagrees with that 1017 by 60 tests.
    **Resolved by the coordinator: 957 is right and 1017 was wrong.** The
    coordinator re-ran the suite at `e493070` (957/0/3, 71 `^test result`
    lines) and corrected that entry; the branch is linear from `cc38de5`
    with no merges, so the "inherited baseline drift" it blamed never
    happened.
    I did not re-run the suite at `8747324`/`e493070` to chase this down —
    it does not affect Task 2's own correctness — but it is a real
    discrepancy in the branch's test-count history, not a typo I can
    explain away, and someone should reconcile it before trusting either
    number blindly.
  - Full report: `/home/knxbench/.claude/jobs/8098e9e6/tmp/t18s3-task2-report.md`.
- **Pending/Next Steps:** Tasks 3-5 of the `t18-parameter-editor` plan
  remain unstarted: Task 3 (`apps/knx-server` — validation, assembly,
  `POST /api/device/{device_id}/parameters` route, read model), Task 4
  (`apps/knx-web` — the parameter panel UI), Task 5 (reconcile the
  documentation set). Do not merge `t18-parameter-editor` before a
  whole-branch review.
- **Notes for Codex:** (a) `RestoreParameterValue.raw` is `Option<String>`,
  not `Override<String>` — see the deviation note above before assuming
  the design doc's Rust snippet is the literal signature; the doc comment
  on the variant explains why. (b) `SetParameterValue`/`RestoreParameterValue`
  always alternate as each other's inverse (never a `Restore`↔`Restore`
  pair) — if Task 3's server code builds a `CommandStack` sequence
  assuming otherwise, that assumption is wrong. (c) `command_sync.rs`'s
  new stub arm is a genuine no-op — writing a parameter value through the
  command layer today does **not** persist to SQLite via incremental
  sync; only the whole-installation save path does. If Task 3 or later
  wires up incremental parameter-instance sync, it also needs a
  `delete_parameter_instance`-shaped function for `RestoreParameterValue`'s
  "no row existed" case, which does not exist yet. (d) The 1017-vs-957
  test-count discrepancy above is unresolved — worth a quick `git stash`-
  free re-run at `8747324` if it matters to whatever you're about to do.
  (e) Standing rules unchanged: `OriginalData/` read-only, scratch files
  under the job tmp directory, no ETS parity/certification claims,
  module-scoped fields stay read-only (D25).

---

- **Last Agent:** Claude
- **Timestamp:** 2026-09-11 21:55
- **Completed:** **T18 slice 3, Task 1 fix round 1** — coordinator ruling on the `display_order` concern flagged in Task 1's own report, implemented on the still-unmerged `t18-parameter-editor` branch. Worktree `.worktrees/t18-parameter-editor`, commit `8747324` (on top of `a7ab571`/`e21196e`; branch still unmerged, four tasks plus a whole-branch review remain).
  - `ParameterView.display_order` and the private `ParameterRawRow.display_order` are now `Option<i64>`, not `i64`. The `COALESCE(pr.display_order, 0)` in `parameter_views`'s `SELECT` is gone; `pr.display_order` is selected raw, so a NULL column maps to `None` via rusqlite's normal `Option<i64>` handling. Reasoning, now in the doc comment: `ParameterRef/@DisplayOrder` is genuinely optional in shipped packages (measured, not assumed — all 543/543 `parameter_ref` rows for `prod3`'s `M-0083_A-0317-31-7DC6` omit it), so `None` is its own real value, distinct from `Some(0)`; a `0` fallback was a magic constant erasing that distinction.
  - `ORDER BY pr.display_order` gained a `pr.rowid` tiebreak (`ORDER BY pr.display_order, pr.rowid`). `parameter_ref` is a plain rowid table (`PRIMARY KEY (program_id, id)`, not `WITHOUT ROWID`), and the parser inserts `ParameterRef` rows in XML document order, so `rowid` recovers the program's own declaration order whenever `DisplayOrder` ties (which is every row on the real corpus). SQLite's default NULLS-FIRST-ascending placement was deliberately kept, not inverted or given `NULLS LAST` — how ETS orders a *mixed* declared/undeclared set is unattested in the corpus, so the doc comment says so instead of guessing. **This supersedes** the original Task 1 round's "no secondary sort key" note, which assumed `display_order` would usually be populated; it doesn't hold once every observed row is NULL.
  - Two new unit tests in `query.rs`'s `mod tests`: one pins `display_order == None` for a `ParameterRef` with no `DisplayOrder` attribute (genuinely failed first against the `COALESCE` version — `left: Some(0), right: None` — confirmed before the real fix landed); one pins the `rowid` tiebreak (`PR-Z` declared before `PR-A`, both DisplayOrder-less, returned in that declaration order, not id order). The existing three-view ordering test now compares `Some(10)`/`Some(20)`/`Some(30)`. The corpus regression test's 543/543/543 assertions are unaffected (re-run directly, still passing).
  - Nit from the task review, also fixed here: `parameter_ref_ids` now imports `HashSet` at the top of the module instead of writing `std::collections::HashSet` inline, matching the rest of the file. The review's other nit — the `INNER JOIN` chain can silently drop a malformed `parameter_ref` row — was explicitly **not** touched; it stays as shipped (matches `com_object_view`'s own precedent) and is queued for Task 3 as a candidate id-count cross-check once diagnostics are assembled.
  - D22 in `docs/superpowers/specs/2026-09-11-parameter-editor-design.md` and the field list in `docs/superpowers/plans/2026-09-11-parameter-editor.md` (Task 1, step 1) were amended in the same commit to say `Option<i64>` with the 543/543 measurement cited, so neither doc still implies a non-optional integer.
  - Gates on this commit: `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo test --workspace` **957 passed / 0 failed / 3 ignored** — exactly `955 + 2` from the two new unit tests; `cargo run -p xtask -- check-layering` ok; `cargo deny check` → advisories ok, bans ok, licenses ok, sources ok; `npm run test` (apps/knx-web) unchanged at **179 passed across 17 files** (no frontend code touched this round). *(Corrected by the coordinator: this entry first said **1017 passed**, explained as a baseline that "drifted upward from unrelated `main` merges". Both halves were wrong. `git log --merges cc38de5..HEAD` is empty, the branch is linear from `cc38de5`, and `main` never moved off `cc38de5` — nothing was merged in. The coordinator re-ran `cargo test --workspace` on this very commit and summed the `^test result` lines: 957/0/3 across 71 result lines. Take test counts in this file as measured only where the measuring agent is named.)*
  - Full fix-round report: `/home/knxbench/.claude/jobs/8098e9e6/tmp/t18s3-task1-fix-report.md`.
- **Pending/Next Steps:** Tasks 2-5 of the `t18-parameter-editor` plan remain unstarted: Task 2 (`knx-core`/`knx-store` — writing a value), Task 3 (`apps/knx-server` — assembly/routes/read model; also where the `INNER JOIN`-drops-malformed-rows nit gets a diagnostics cross-check), Task 4 (`apps/knx-web` — the parameter panel), Task 5 (reconcile the documentation set). Task 1 plus this fix round are implemented and gated; whether a further review pass is needed on the fix round itself, or whether the branch proceeds straight to Task 2, is the coordinator's call, not decided here. Do not merge `t18-parameter-editor` before a whole-branch review.
- **Notes for Codex:** (a) `display_order` is `Option<i64>` now, not `i64` — if you see code elsewhere assuming it's always populated (e.g. from a stale read of the original Task 1 report before this fix), that assumption is wrong; the corpus is 543/543 NULL. (b) The `ORDER BY pr.display_order, pr.rowid` tiebreak is load-bearing for determinism, not decorative — do not remove the `pr.rowid` half without re-checking whether SQLite's sorter stability was ever actually depended on elsewhere. (c) NULLS-first-ascending (SQLite's default) is kept deliberately; do not "fix" it to NULLS LAST without new corpus evidence of how ETS orders a mixed set — none exists yet. (d) Module-scoped fields stay read-only this slice (D25), unchanged by this fix round. The standing rules are unchanged: `OriginalData/` is strictly read-only, scratch files belong under the job tmp directory, and nothing here claims ETS behavioural parity, KNX certification or hardware verification.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-09-11 16:40
- **Completed:** Task 5 (final task) of the `t15-group-monitor` branch
  (`.worktrees/t15-group-monitor`), design spec
  `docs/superpowers/specs/2026-09-11-group-monitor-design.md` — the
  compose/send form that closes the loop (click a row, edit, send it
  back to the bus), per §6, plus two inherited defect fixes. Server-side
  fix, its own commit `b540264`: `POST /api/bus/write`
  (`apps/knx-server/src/bus_routes.rs`) parsed `destination` as
  `GroupAddressStyle::ThreeLevel` unconditionally, while `GET
  /telegrams` renders it in the open project's actual configured style —
  breaking the round trip for Free/TwoLevel projects. Fixed by a new
  `BusSession::group_address_style()` accessor (`bus.rs`) and reordering
  `write_value` to derive its parse style from the active session
  (falling back to `ThreeLevel` only with no session/project); new
  regression test
  `a_non_three_level_projects_telegram_destination_round_trips_through_write`
  in `apps/knx-server/tests/http_bus_write.rs` proves the full round
  trip for a `Free`-style project, not just isolated parsing. CLI's
  identical bug (same hardcoded `ThreeLevel`) was deliberately **not**
  touched — out of scope for this task. Frontend, its own commit
  `d494033`: new `apps/knx-web/src/BusComposeForm.tsx` (sibling
  component to `BusMonitorPanel.tsx`, not folded in — different
  lifetime, survives polls, resets only on a new row click via a
  `key`-remount trick) implements the resolve/reject/defer state
  machine mirroring `resolve_write_value`, rejecting an unresolvable
  `None`/`Conflict` DPT client-side before any request, with the two
  verbatim messages (em dash included) reproduced exactly. Also fixed
  the inherited frontend dead end: unmounting `BusMonitorPanel`
  orphaned the server-side session (correct — not React's to own), but
  remounting assumed no session existed, so Connect hit an
  unrecoverable `409`. Fixed with a mount-time `GET /telegrams`
  reattach effect: `404` shows the Connect form as before, `200` adopts
  the session/rows/cursor/status and feeds `droppedBefore` into the
  existing gap notice, a `"closed"` session reattaches read-only (Stop
  still works). `api.ts` gained a duck-typed `.status` on request
  errors plus an `errorStatus()` helper (no new exported class, so
  existing `vi.mock("./api", ...)` factories elsewhere don't need
  touching). New test file `BusComposeForm.test.tsx` (12 tests, standalone
  from session/polling mocking) plus 4 new mount-reattach tests and a
  row-click-prefill integration test in `BusMonitorPanel.test.tsx`. All
  six gates green: `cargo fmt --all --check` clean, `cargo clippy
  --workspace --all-targets -- -D warnings` clean, `cargo test
  --workspace` 951 passed/0 failed/3 ignored (was 950/0/3 — +1 net from
  the new round-trip regression test), `cargo test -p knx-server` 147/0/0
  (was 146/0/0), `cargo run -p xtask -- check-layering` ok, `cargo deny
  check` ok (advisories/bans/licenses/sources all ok), `npm run test`
  178 passed across 17 files (was 162/16 — +16 net: 12 new
  `BusComposeForm.test.tsx` tests + 4 new mount-reattach tests, one
  legacy assertion adjusted for the new per-mount `GET /telegrams`
  call). Full report:
  `/home/knxbench/.claude/jobs/8098e9e6/tmp/t15-task5-report.md`.
- **Pending/Next Steps:** Task 5 was the final task listed for this
  branch's plan; branch is ready for final review/merge to `main` by
  whichever agent runs that step next — not done here (out of this
  task's mandate; no `git checkout main`/merge performed). No other
  loose ends known for this branch's own scope.
- **Notes for Codex:** `service` on `BusTelegramRow` stays a plain
  `string` in `api.ts` on purpose (not narrowed to a union) — the
  synthetic `"SessionClosed"` marker row needs to fit it too, and this
  task was told explicitly not to change that. The client-side `rows`
  array's unbounded growth across a long session is a known, documented
  limitation (not this task's to fix — see the design spec's own out-
  of-scope list and `docs/KNOWN_LIMITATIONS.md`). Nothing in this work
  claims ETS parity or hardware verification anywhere — server-side fix
  and frontend form are both entirely `FakeConnector`/`FakeTunnel` and
  mocked-`fetch` tested, no real gateway anywhere in the new tests.
- **Last Agent:** Codex
- **Timestamp:** 2026-09-10 09:32
- **Review Note:** On 2026-09-09 Codex reviewed implementation status, roadmap, and known limitations; no product-code changes.
- **Goal Instruction:** Added root `goal.md` as the durable `/goal` completion instruction. It requires evidence-backed closure of actionable work, reconciled status documents, honest external blockers, and explicit user acceptance for anything left out of scope. Its highest-priority goal is now direct device installation from manufacturer-supplied product databases, evidenced by six files in `OriginalData/ProductDatabases/` (five `.knxprod`, one legacy `.vd2`); `OriginalData/DemoProjects/` now has three ETS-project fixtures, including ETS4 and ETS 6.3.0 examples, for the versioned compatibility corpus. The instruction now also requires a VS-Code-safe, GitHub-aware workflow (remote verified as `KNXBench-Labs/KNXBench`), autonomously authorizing branches, verified commits, pushes, PRs, task/issue closure, and green-check merges while prohibiting force pushes, history rewriting, bypassed checks, and loss of unrelated changes.
- **SDD Instruction:** `goal.md` now mandates token-conscious Subagent-Driven Development: parallelize only independent work in isolated worktrees (maximum three subagents plus coordinator), then use per-task and final reviews. Every dispatch explicitly sets model and effort: `gpt-5.6-luna`/low for mechanical work, `gpt-5.6-terra`/medium for integration and normal review, `gpt-6-astra`/high for research, architecture, integrity-critical work, and final review.
- **KNX Standard Reference:** `goal.md` now mandates consulting the 179-document extracted KNX Standard v3.0.0 Markdown corpus at `/mnt/daten-i/Sourcecode/knx-spec-kb/extracted/The KNX Standard v3.0.0/` for unclear KNX behavior, citing the exact file and section and never inventing ambiguous semantics.
- **Completed:** Session-log design approved in chat: the log is scoped to the
  current application session and project, not persisted in `.knxdb`. Wrote
  `docs/superpowers/specs/2026-09-08-session-log-design.md` and
  `.ai/logs/2026-09-08_codex_session_log_design.md`. Previous work: T7,
  communication-object flag editing, added `Command::SetComObjectFlag` /
  `RestoreComObjectFlag`, `POST /api/com-object-flag`, and five editable
  communication-object flags in the Inspector; focused Rust and Vitest tests
  passed.
- **Active Goal Progress:** Manufacturer-database audit completed; reports are in `.ai/reports/2026-09-09_{productdb_audit,corpus_probe,status_audit}.md`. Isolated branch `codex/standalone-productdb-install` at `.worktrees/standalone-productdb-install` contains reviewed integrity fix `2d1e001` (first-winner catalog/hardware/product/H2P rows, conflict reporting, regression coverage). Task 2 package ingestion changes are present but uncommitted and under review by the implementer; do not restart or redispatch the implementation. Agent `productdb_package_impl` was resumed after its usage-limit interruption. Its reported initial corpus tests passed for all five readable archives; coordinator requested additional retry-diagnostic and member-identity validation before acceptance. No complete standalone device-installation claim yet.
- **Task 2 Review Update:** Package implementation committed as `b98a663`; implementer reports 89 passing productdb tests, strict clippy, focused formatting and layering. Independent reviewer reproduced blocking defects: package-to-file parse-cache poisoning; repeat-import conflict counts changing after overlapping package imports; non-transactional schema migration/version change; malformed XML acceptance. Original implementer resumed for fix round 1 with regression tests. Task 2 is not accepted or merged. This update supersedes the earlier uncommitted Task 2 status above.
- **Pending/Next Steps:** Finish/review Task 2 fixes in the isolated worktree, then CLI/HTTP, browser/creation diagnostics, persistence/export provenance, corpus gates and GitHub delivery. Keep existing `.knxproj` CLI ingestion compatible when adding standalone formats. The full `goal.md` objective remains active; the current plan is only one slice. Previous lower-priority session-log work: user review of the session-log design, then write
  its implementation plan and implement it. Tier 2 remains T8 (building-part
  CRUD commands) and T9 (bulk/multi-select operations, needing a separate
  `Command::Batch` design).
- **Notes for Claude:** The session-log design intentionally leaves
  `ImportReport` in `knx-etsproj` and makes `knx-server` the projection and
  lifetime owner; no core/store change is appropriate. The specification was
  not committed because the user did not request a commit. Earlier T7 work
  happened in a git worktree; merge/rebase onto `main` before continuing from
  there. No new `CommandError` variant was needed for T7: unknown com-object
  and flag-name failures reuse `CommandError::ComObjectNotFound` or the
  server's existing plain-`Err(String)` parsing convention.
- **Current Goal Progress (2026-09-10 09:15):** Task 2 standalone product database package ingestion is complete and independently approved at `f6cb11f`. Fifteen standalone tests cover all five readable `.knxprod` corpus archives plus cache, conflict-snapshot, migration rollback, XML and malformed-input regressions. Task 3 CLI/HTTP entry points is now dispatched in the isolated branch; no CLI/server changes are accepted yet. Full goal remains active; `.vd2` decryption, Dynamic/module activation, creation diagnostics, export provenance, docs reconciliation and repository-wide gates remain open.
- **Current Goal Progress (2026-09-10, continued):** Task 3 (CLI `knx products ingest` + `POST /api/catalog/install`) is complete and independently approved in `.worktrees/standalone-productdb-install` at `2afa28d` (impl `4b403e4`); a storage-error classification fix keeps persistent SQLite/storage failures at HTTP 500 vs. malformed-input 400. Existing `.knxproj` CLI ingest path is unchanged; 3 `http_product_install` tests plus existing CLI import tests pass. Task 4 (browser install action + creation diagnostics) dispatched to an implementer subagent (`gpt-5.6-terra`/medium) in the same worktree/branch, brief at `.superpowers/sdd/2026-09-09-standalone-product-database-install/task-4-brief.md` (gitignored, local-only). Task 4 also folds in a design-mandated behavior change: `create_device_impl` currently treats an unresolved `hardware2program_ref_id` the same as genuinely passive hardware (zero com objects, silent success); it must instead distinguish that case and return visible diagnostics. Not yet implemented, reviewed, or merged. Task 5 (docs/gates reconciliation) remains queued after Task 4 lands.
- **Memory System (2026-09-10):** Codex applied the durable-memory guidance from OpenClaw commits `7d1fee7` and `9ffea23` to this workspace's `AGENTS.md` and created canonical `MEMORY.md`. The guidance keeps private long-term memory in main/direct sessions, raw notes in dated `memory/` files, and periodic maintenance in heartbeat or scheduled passes. Verification: `openclaw doctor --non-interactive` no longer reports the memory-system warning for agent `knxbench`.
- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 12:10
- **Completed:** Task 5 (final task) of
  `docs/superpowers/plans/2026-09-09-standalone-product-database-install.md`
  ("Reconcile evidence and compatibility documentation"), on branch
  `codex/catalog-creation-diagnostics`. Documentation-only, per the plan's
  own file list: `docs/GAP_ANALYSIS_ETS.md` (A5 partially closed for
  scheme 11/20 standalone package install; B1/B2/B3/B5/B6/B7 table rows
  reconciled against their already-"Done" task-backlog entries; D3 closed
  and extended; new **T24** backlog entry for this plan's Tasks 1-4),
  `docs/KNOWN_LIMITATIONS.md` (§11 rewritten for the scheme-11/20 split
  plus `.vd2` as a named permanent legacy-format blocker, not an
  untested general failure; §35 marked **RESOLVED**),
  `docs/ROADMAP.md` (stale "`.knxprod` ingest ... out of v1 scope" open
  question corrected), `docs/COMPATIBILITY.md` (new §2 rows for the
  package installer and its rejection path; §3's schema-20 row
  disambiguated from the `.knxprod`-package claim; §4's blanket "not
  supported" row narrowed to the untested schemes), and
  `docs/IMPLEMENTATION_STATUS.md` (new T24 entry for the whole plan,
  Tasks 1-5). Every claim re-verified against current code/tests before
  writing it — 5-file corpus scheme split confirmed directly from each
  `knx_master.xml`'s `xmlns` via `unzip`, `.vd2` rejection mechanism read
  from `crates/knx-productdb/src/package.rs` (pre-hash filename check,
  exact string `"legacy .vd2 product data is unsupported"`), both spec
  citations (`Project Schema23 v01.00.00.md` §4.2.2-§4.2.3,
  `03_01_01 Architecture v03.00.02 AS.md` §6.2) read directly from the
  primary spec text, not cited on faith. Documentation consistency search
  (`rg -n "future catalog-browser|no device catalog browser|scheme 11 is
  readable|direct.*knxprod.*out of v1" docs`) went from 2 matches to 0.
  Full log: `.ai/logs/2026-09-10_codex_standalone_product_database_install.md`
  (kept the plan's own literal `_codex_` filename despite the executing
  agent being Claude, per explicit dispatch instruction).
  **Full gate run, all green:** `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings` (clean except
  the two pre-existing, deliberately-untouched `clippy::large_enum_variant`
  errors on `knx-etsproj`'s `Frame` enum,
  `crates/knx-etsproj/src/parse/installation.rs:36` and
  `installation_v21.rs:48`), `cargo test --workspace` with
  `KNXBENCH_PRODUCT_CORPUS=/mnt/daten-i/Sourcecode/KNXBench/OriginalData/ProductDatabases`
  (all green, `installs_the_readable_corpus` included),
  `cargo run -p xtask -- check-layering` (clean), `npm test` (96/96),
  `npm run build` (clean, `dist/.gitkeep` restored after). Two
  pre-existing, non-product-code issues found mid-gate and fixed as
  separate `style:` commits (not mixed into the docs commit): leftover
  `cargo fmt` drift in `apps/knx-server/src/domain.rs`/`routes.rs` from
  the earlier fix-loop commits (`e56c83a`), and three
  `clippy::bool_assert_comparison` lints in
  `crates/knx-core/src/command.rs` test code, unrelated to the `Frame`
  enum (`1f80064`). Docs reconciliation itself committed as
  `docs: record standalone product database compatibility`.
- **Pending/Next Steps:** This plan (Tasks 1-5) is now fully done on this
  branch. Next steps belong to whoever picks the branch up: (1) merge
  `codex/catalog-creation-diagnostics` to `main` if the review gate is
  satisfied; (2) the design spec's acceptance criterion "the caller
  receives the archive hash/size in the error report where available" is
  **not implemented** for the `.vd2` rejection path (filename check runs
  before any hash/size is computed) — a real, small follow-up if anyone
  wants the error report to carry it, not done here since Task 5 is
  docs-only; (3) `GAP_ANALYSIS_ETS.md`'s B4/B8/B9/B10/B11 and most of
  Section C/D/E/F gaps remain genuinely open (not touched this task,
  they were never claimed closed); T8 (building-part CRUD) is the next
  natural Tier-1/2 backlog pick if continuing that track.
- **Notes for Codex:** Nothing gate-blocking left — the workspace was
  clean at handover (`cargo fmt`/`clippy`/`test`/`check-layering`/`npm
  test`/`npm run build` all pass, modulo the two named
  `large_enum_variant` errors that are explicitly out of scope and
  should stay untouched absent a measured reason to box `SourceDevice`).
  If you re-run the productdb corpus test, remember
  `KNXBENCH_PRODUCT_CORPUS` must point at
  `OriginalData/ProductDatabases` (absolute path) or it falls back to a
  relative path that may not resolve the same way from every cwd.
  `docs/superpowers/plans/2026-09-09-standalone-product-database-install.md`
  itself is untracked in git and only exists in the main checkout, not in
  this worktree's history — if you need to re-read Task 1-5's exact
  original wording later, it is not retrievable from `git log` on this
  branch, only from the main checkout's working tree.
- **Merge note (2026-09-10):** `codex/catalog-creation-diagnostics`
  (Tasks 1-5 above) merged to `main` as `af5639a`, pushed to `origin/main`
  (`6a4a858..af5639a`). `.worktrees/standalone-productdb-install` (branch
  `codex/standalone-productdb-install`) carried its own, different,
  unmerged Task 4 commit (`2368a47`) — a separate implementation of the
  same creation-diagnostics feature, predating and superseded by the
  merged one. Follow-up issue #1 opened, then closed on user confirmation:
  worktree and branch discarded (`git worktree remove --force` +
  `git branch -D`), including 2 never-committed `.ai/logs/*.md` notes with
  no copy elsewhere. Nothing from that branch was pulled forward — its
  content was superseded, not merely duplicated. `codex/catalog-creation-diagnostics`'s
  own worktree/branch were also cleaned up after the merge (both deleted;
  plan's `.superpowers/sdd/2026-09-09-standalone-product-database-install/`
  workspace removed). Remaining worktrees: `.claude/worktrees/building-part-crud`
  (branch `worktree-building-part-crud`, T8 building-part CRUD, 7 commits
  unmerged, untouched, unrelated to this plan) and `.worktrees/session3-ets-import`
  (0 commits ahead of `main`, clean, stale).
- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 13:15
- **Completed:** Finished and merged `worktree-building-part-crud` (T8, building-part CRUD) to `main` as merge commit `13e084e`. Per `/goal`, resumed the plan
  (`docs/superpowers/plans/2026-09-08-building-part-crud.md`) via subagent-driven-development: Task 7 (`ProjectExplorer.tsx` create UI, briefed in an earlier
  session but never dispatched/committed — re-dispatched fresh) landed as `049a49c`; Task 8 (`Inspector.tsx` rename/delete/move UI) as `11d3970`; Task 9 (docs
  closeout) as `c85cbf1`, after ruling out its brief's stale `.ai/CURRENT_STATE.md`-replace instruction (the file didn't exist on this branch's fork point,
  predating this session's other-plan merge) and its stale doc-anchor references — both rulings independently confirmed by the task reviewer. Final
  whole-branch review (sonnet, fork `e8bea7f`..`c85cbf1`, 10 commits): 0 Critical/Important, 3 Minor (2 already-deferred cosmetic items, 1 pre-existing
  repo-wide `cargo fmt` drift present at the fork point, not a regression). Merge resolved the anticipated conflict in
  `crates/knx-store/src/command_sync.rs` (this branch's 4 `BuildingPart`-command no-op stub arms vs. `main`'s independently-added
  `SetComObjectFlag`/`RestoreComObjectFlag` stub arm, both inserted after `UnlinkComObject` — kept both, no semantic overlap) plus two doc conflicts in
  `docs/GAP_ANALYSIS_ETS.md`/`docs/IMPLEMENTATION_STATUS.md` (kept `main`'s T7/T24 entries, added this branch's new T8 entry). Post-merge gates all green:
  `cargo build --workspace`, `cargo test --workspace` (all crates, 0 failed), `cargo clippy --workspace --all-targets -- -D warnings` (clean except the two
  known pre-existing `large_enum_variant` errors on `knx-etsproj`'s `Frame` enum), `cargo run -p xtask -- check-layering` (clean), `npx tsc --noEmit` (clean),
  `npx vitest run` (104/104). `crates/knx-core` gains `Command::CreateBuildingPart`/`DeleteBuildingPart`/`RenameBuildingPart`/`MoveDeviceToBuildingPart`;
  `apps/knx-web` gains create/rename/delete/move UI for building parts in `ProjectExplorer.tsx`/`Inspector.tsx`. Closes **T8**/**B4**
  ([GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)).
- **Pending/Next Steps:** Push `main` to `origin/main`. Clean up the `worktree-building-part-crud` branch and its worktree at
  `.claude/worktrees/building-part-crud`, and delete the plan's `.superpowers/sdd/2026-09-08-building-part-crud/` workspace, once the push is confirmed.
  Remaining worktree: `.worktrees/session3-ets-import` (0 commits ahead of `main`, clean, stale) — no action needed unless picked up. T9 (bulk/multi-select
  operations, needing a `Command::Batch` design) is the next natural Tier-2 backlog pick if continuing this track; the pre-existing repo-wide `cargo fmt`
  drift flagged by this branch's final review is a genuine, out-of-scope follow-up (not started, not requested).
- **Notes for Codex:** Nothing gate-blocking left on `main` post-merge. The `command_sync.rs` non-exhaustive-match pattern (a stub arm per new `Command`
  variant, "persistence layer not yet implemented" convention) now has both this session's and the prior session's additions reconciled side by side —
  keep following that convention for any new `Command` variant until the store's real persistence layer catches up.
- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 (continued session)
- **Completed:** T10 (wire up `export_ets_project` to a real interface, closes **C4**) via subagent-driven-development on branch `t10-export-ui`,
  worktree `.worktrees/t10-export-ui`, plan `docs/superpowers/plans/2026-09-10-ets-project-export.md`. Task 1 (`knx-server` `POST /api/project/export`,
  commit `89cd7ca`), Task 2 (`knx-cli` `knx export`, commit `6f5afea`), Task 3 (`knx-web` "Export to .knxproj…" button, commit `ad1aa5f` + fix round
  `7a9e4c1` for a warning-toast-collapsing bug) all task-reviewed clean. The final whole-branch review (sonnet, `b0db80d..7a9e4c1`) found the branch
  **NOT READY**: a genuine data-integrity bug (B1) — server-side ETS import used a throwaway in-memory store, so `save_project_as_impl` never
  persisted opaque passthrough/manufacturer manifest data, and an Export taken after Save As on the server path silently lost that data with no
  warning (1.7MB source → 29.9KB output) — plus a missing documentation closeout (B2). Fixed B1 as commit `9cdb7aa`: `AppState` now carries
  `opaque`/`manufacturer_refs`, filled on both ETS import and native `.knxdb` load, and every save path writes them back into the target `.knxdb`;
  added a regression test (`exported_project_still_carries_opaque_and_manufacturer_data_after_save_as`) that reimports the export and checks its
  own opaque/manifest tables are non-empty, not just "the file exists". Fixed B2 in this update: `docs/GAP_ANALYSIS_ETS.md` (C4 marked closed with
  the B1 caveat, T10 backlog entry struck through), `docs/IMPLEMENTATION_STATUS.md` (new T10 entry). `docs/COMPATIBILITY.md` needed no change (its
  export claims are byte-level roundtrip claims, unrelated to UI wiring). Gates: `cargo fmt --all --check` clean, `cargo test -p knx-server -p
  knx-cli` all green (30/30 + 13/13).
- **Pending/Next Steps:** Dispatch a follow-up whole-branch re-review confirming READY TO MERGE, then merge `t10-export-ui` to `main` and clean up
  this plan's worktree/`.superpowers/sdd/2026-09-10-ets-project-export/` workspace per `finishing-a-development-branch`. Not yet pushed or merged as
  of this entry. After T10 lands: T9 (bulk/multi-select operations, `Command::Batch` design) is the next natural Tier-2 pick; `GAP_ANALYSIS_ETS.md`'s
  B8-B11 and most of Section C/D/E/F gaps remain genuinely open.
- **Notes for Codex:** If you pick this branch up before it's merged, re-run `cargo test -p knx-server` first — the new regression test
  (`http_export_route.rs`) is the one that would have caught B1 originally; trust it over eyeballing the diff. `apps/knx-server/src/domain.rs`'s
  `AppState.opaque`/`manufacturer_refs` fields are the new single source of truth for "what opaque/manifest data does the in-memory project carry" —
  any future code path that swaps `state.project` (import, native load, or a not-yet-existing "new project" command) must also set these two fields
  consistently, or B1 recurs in a new shape.
- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 (continued session, round 2)
- **Completed:** Dispatched a round-2 whole-branch re-review (sonnet) of `t10-export-ui` after the B1/B2 fixes above. Verdict: **NOT READY** —
  confirmed B1/B2 correctly and completely fixed, but found a new blocking bug (C1): `knx_store::insert_opaque`/`insert_manufacturer_refs` were
  plain `INSERT`s with no clear-first step, unlike `save_project`'s own DELETE-then-insert convention. B1's fix made `save_project_as_impl` call
  them on *every* save, not just the first, so a plain repeated `POST /api/project/save` — reusing an already-populated `store_path` — duplicated
  every opaque/manifest row without bound. The reviewer reproduced this empirically in a standalone crate outside the worktree. Also flagged: an
  Important non-blocking finding that `export_project` read opaque/manifest off disk via `store_path` rather than live `AppState`, compounding
  `KNOWN_LIMITATIONS.md` #18's stale-`store_path` gap; a dangling `KNOWN_LIMITATIONS.md` cross-reference in `GAP_ANALYSIS_ETS.md`'s C4 row; a Minor
  lock-ordering inversion in `export_project`. Ruling: fixed C1 at the `knx-store` level (`DELETE FROM` before `INSERT` inside the existing
  transaction, in both `opaque.rs`/`manifest.rs`, so every caller gets the fix) rather than only at the `save_project_as_impl` call site — commit
  `ac9ccaa`, with regression tests in `knx-store` (repeated insert calls) and `knx-server`
  (`saving_the_same_project_twice_does_not_duplicate_opaque_and_manifest_rows`, exercising the real HTTP `/api/project/save` path). Also fixed the
  Important finding and the Minor lock-ordering finding together: `export_project` now reads `AppState.opaque`/`AppState.manufacturer_refs` directly
  instead of re-opening `store_path`, copying them into a throwaway in-memory `.knxdb` for `export_ets_project`'s `Connection`-shaped interface, and
  locks/drops them before touching `project`/`product_db` — closing the staleness risk for this data and the lock-ordering inconsistency in one
  change. Fixed the dangling cross-reference in `GAP_ANALYSIS_ETS.md`'s C4 row and documented all of this in `docs/KNOWN_LIMITATIONS.md` #18's new
  "Related" note and `docs/IMPLEMENTATION_STATUS.md`'s T10 entry. The `ApiError::bad_request` coarsening Minor finding was already adjudicated in
  an earlier round and parked as-is. Gates: `cargo fmt --all --check` clean, `cargo test --workspace` all green (one pre-existing,
  environment-dependent `knx-productdb` failure needing `KNXBENCH_PRODUCT_CORPUS`, unrelated), `cargo clippy --workspace --all-targets` clean except
  the two known pre-existing issues, `xtask check-layering` clean.
- **Pending/Next Steps:** Dispatch a round-3 whole-branch re-review confirming READY TO MERGE, then merge `t10-export-ui` to `main` and clean up
  this plan's worktree/`.superpowers/sdd/2026-09-10-ets-project-export/` workspace per `finishing-a-development-branch`. Not yet pushed or merged as
  of this entry. After T10 lands: T9 (bulk/multi-select operations, `Command::Batch` design) is the next natural Tier-2 pick; `GAP_ANALYSIS_ETS.md`'s
  B8-B11 and most of Section C/D/E/F gaps remain genuinely open.
- **Notes for Codex:** Same caution as the prior entry: any future code path that swaps `state.project` must also keep `state.opaque`/
  `state.manufacturer_refs` consistent. New addition: any future writer of the `opaque_entry`/`manufacturer_ref` tables must go through
  `knx_store::insert_opaque`/`insert_manufacturer_refs` (now clear-before-insert) rather than hand-rolling an `INSERT`, or the duplication bug
  recurs outside those two functions' protection.
- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 15:18
- **Completed:** After merging T10 (PR #2, `7250197`), checked GitHub Actions CI on that PR and found the "Build, test, lint" job FAILED — not
  a T10 regression, but CI's `RUSTFLAGS: -D warnings` turning two pre-existing `clippy::large_enum_variant` warnings on `knx-etsproj`'s `Frame`
  enum (`Device(SourceDevice)`, ~744B vs ~360B for the next-largest variant) into hard compile errors. These two warnings had been flagged and
  explicitly left untouched by several prior sessions' CURRENT_STATE.md entries ("explicitly out of scope... absent a measured reason to box
  `SourceDevice`") — this is that measured reason: with no branch-protection/required-status-checks configured on this repo (confirmed via
  `gh api repos/.../branches/main/protection` → 403), every future PR's CI will show red regardless of its own changes, which defeats CI as a
  signal at all. Fixed on branch `fix-ci-red` (worktree `.worktrees/fix-ci-red`): boxed `Device(SourceDevice)` → `Device(Box<SourceDevice>)` in
  both `crates/knx-etsproj/src/parse/installation.rs` and `installation_v21.rs`, updating each file's one construction site
  (`Frame::Device(Box::new(SourceDevice { ... }))`) and `into_device`'s extraction arm (`Frame::Device(v) => *v`); all other match arms needed
  no change (`Box<T>` derefs transparently). Commit `0f4f680`. Also found, in the same `-D warnings` clippy pass, an unrelated pre-existing
  `clippy::field_reassign_with_default` in `apps/knx-server/tests/http_product_install.rs`'s `state()` helper — fixed as a struct literal with
  `..Default::default()`, commit `1b69d50`. Both were bare warnings locally (not caught by any prior session's plain `cargo clippy`) but hard
  errors under CI's exact invocation — worth remembering that "clippy clean locally" and "clippy clean under `-D warnings`" are different checks.
  The `cargo fmt` drift in `command.rs` that an earlier scratch-worktree comparison against `origin/main` had also flagged turned out to already
  be fixed on `main` (by `1f80064`, landed via the T8 merge) — a stale finding from comparing against a slightly earlier `origin/main`, not a
  real remaining issue; no `command.rs` change was needed here. Gates on `fix-ci-red`: `cargo fmt --all --check` clean, `RUSTFLAGS="-D warnings"
  cargo clippy --workspace --all-targets` clean (zero warnings/errors, both fixed lints confirmed gone), `cargo test --workspace` all green
  (0 failed; required symlinking the untracked `OriginalData/` fixture directory into this worktree first — it's local-only, not git-tracked,
  and the earlier test run failed with 13 fixture-not-found errors until the symlink was added and then removed again before committing).
- **Pending/Next Steps:** Push `fix-ci-red`, open a PR against `main`, and — this being the first genuinely green CI run this repo will have
  had — actually watch it go green before merging (previous PRs merged without that confirmation being possible). Then merge, clean up the
  branch/worktree, and continue the broader `/goal` backlog: `GAP_ANALYSIS_ETS.md`'s B8-B11 and most of Section C/D/E/F remain open; T9
  (bulk/multi-select, `Command::Batch` design) is the next natural Tier-2 pick.
- **Notes for Codex:** If clippy ever reports `large_enum_variant` again on a *different* enum, the fix pattern here (box the oversized
  variant's payload, fix the one construction site and any owned-extraction match arm, leave every other arm alone) is the established
  convention for this codebase now — no need to re-derive it. Also: prefer running `cargo clippy --workspace --all-targets -- -D warnings`
  (or setting `RUSTFLAGS="-D warnings"`) at least once before merging any branch, not just plain `cargo clippy` — they can disagree, as they
  did here.

- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 15:48
- **Completed:** Finished `fix-ci-red`'s real goal: PR #3's first CI run (against the two clippy fixes alone) still failed, at the test step —
  confirmed by simulating CI's actual condition locally (`rm` the local `OriginalData/` symlink, `KNXBENCH_PRODUCT_CORPUS=/nonexistent cargo test
  --workspace`), which turned up dozens of tests across the workspace that `.unwrap()`/`.expect()` on the git-ignored, local-only `OriginalData/`
  fixture corpus (real personal ETS project + manufacturer `.knxprod` files, never committed, never present on GitHub's runners) and therefore
  panic instead of failing gracefully — this is *why CI has never once passed for this entire repo* (`gh run list --branch main` — every run,
  going back through every past merge, shows `conclusion: failure`; previously masked because build/clippy errors happened first in the pipeline
  and the job never reached the test step). Fixed every one: each corpus-dependent test now starts with
  `if !reference_ets4_path().exists() { eprintln!("skip: ..."); return; }` (knx-etsproj's own unit tests use a new
  `crate::testutil::corpus_available()` helper instead, since `testutil.rs` is reachable directly from in-crate `#[cfg(test)]` modules but not
  from separate integration-test crates). Found via two passes: a static regex scan for the `reference_*()` helper names (missed `export.rs`
  entirely — never in either script's file list — and missed a few `map.rs`/`report.rs`/`detect.rs` tests whose corpus access goes through a
  local helper function rather than a direct call the regex could see), then closed every remaining gap by running the suite corpus-absent and
  guarding whatever still failed — which also caught three whole test files the static scan never looked at: `oracle_xknxproject.rs`,
  `roundtrip.rs`, `standalone_packages.rs` (the last already read `KNXBENCH_PRODUCT_CORPUS`, just didn't check the resolved path existed before
  reading it). One extra find along the way: `http_export_route.rs`'s `exporting_without_a_store_path_is_a_400` was flagged in an earlier
  session's file-by-file check as "correctly needs no guard — its check runs before any file IO," which was wrong for *this* test (it imports a
  real project via `/api/project/import` first, to get an open project to test the export-without-save-as-first error against); guarded like
  every other one. 30 files, 397 lines, one commit (`10df2a8`, all guards - this is a single logical fix at 30 files despite CLAUDE.md's normal
  "one change per commit" preference, because splitting it would be 30 commits of the identical one-line pattern with no independent value in
  reviewing them separately). Verified: `cargo fmt --all --check` clean, `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` clean,
  `cargo test --workspace` green both with `OriginalData/` symlinked in (576 passed, tests exercise the real corpus) and absent (576 passed,
  corpus-dependent ones no-op instead of panicking) — same total either way, confirming the guards don't accidentally skip anything they
  shouldn't.
- **Pending/Next Steps:** Push `fix-ci-red`, confirm PR #3's CI actually goes green this time (this repo's first genuine green run), merge per
  `/goal`'s autonomy grant, clean up the branch/worktree. After that: continue the `/goal` backlog — `GAP_ANALYSIS_ETS.md`'s B8-B11 and most of
  Section C/D/E/F remain open; T9 (bulk/multi-select, `Command::Batch` design) is the next natural Tier-2 pick.
- **Notes for Codex:** If you add a test that reads `OriginalData/` (any `reference_*_path()`/`reference_*_bytes()`/`reference_*_document()`
  helper, in any crate), it MUST start with the skip-guard pattern above or CI will red again the moment it merges — there is no other way to
  make a fixture-dependent test degrade gracefully in Rust (no runtime-conditional `#[ignore]`). Don't rely on a regex/static scan to catch every
  corpus-dependent test if you're auditing this later — the reliable oracle is running the suite with `OriginalData/` absent and
  `KNXBENCH_PRODUCT_CORPUS` pointed at a nonexistent path, and guarding whatever fails.

- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 (t11-session-log worktree)
- **Completed:** T11 session-log Task 1, fix-round-1. Independent review
  (`.superpowers/sdd/2026-09-10-session-log/task-1-review.md`, commit
  `855e9a4`) flagged one blocking gap: `create_device_impl` called
  `stack.do_command(...)` directly instead of routing through `apply()`,
  so creating a device never produced a session-log entry — the one edit
  path in `apps/knx-server/src/domain.rs` invisible to the log. Fixed by
  extracting `apply()`'s log-push logic into a shared
  `log_command_outcome(state, cmd_desc, &result)` helper and calling it
  from `create_device_impl`'s own `do_command` call too (it still can't
  call `apply()` itself — it needs the enrichment pass to run under the
  same `project` lock, and returns a richer `CreateDeviceResponse` than
  `apply()` produces). Catalog-lookup failures that happen before a
  `Command` is even built (no product db, unknown catalog item) stay
  unlogged, matching the existing convention every other `*_impl`'s
  pre-`apply()` validation already follows (e.g.
  `set_individual_address_impl`'s address-parse `?`). Added regression
  test `creating_a_device_logs_an_info_entry_and_a_failed_creation_logs_an_error_entry`.
  Commit `1651b73` on branch `t11-session-log`. `cargo test -p knx-server`
  (all 20 tests) and `cargo clippy -p knx-server --all-targets` both clean.
  The review's two non-blocking findings (reset-timing race between
  `state.project` and `state.session_log`'s independent mutexes;
  `tree_with_state` built-then-discarded on `apply()`'s error branch) were
  left as-is — review explicitly marked both non-blocking/cosmetic.
- **Pending/Next Steps:** Task 1 fix-round-1 is done but not yet
  re-reviewed or merged to `main`. Next: get review sign-off on `1651b73`,
  merge `t11-session-log`, then proceed to Task 2 (frontend Log tab) per
  `docs/superpowers/specs/2026-09-08-session-log-design.md`.
- **Notes for Codex:** If `apply()`'s logging behavior changes again, keep
  `create_device_impl` in sync manually — it deliberately duplicates
  `apply()`'s lock-then-log shape via `log_command_outcome` rather than
  calling `apply()`, because of the enrichment-under-lock + richer-return-
  type constraints noted above. Any future `*_impl` that similarly can't
  call `apply()` should reuse `log_command_outcome` too, not hand-roll a
  third copy of the log-push block.

- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 17:57
- **Completed:** Task 2 (frontend `LogPanel.tsx` + `App.tsx` wiring,
  commit `7e43350` + fix-round-1 `f03057f`) and Task 3 (docs closure,
  commit `28645a2`) both landed and task-reviewed clean before this
  session started. This session addressed the final whole-branch review
  of T11 (opus review: 0 Critical, 6 Important, 9 Minor):
  - **#1/#8** (`domain.rs`): collapsed `log_outcome`/`log_command_outcome`/
    `log_undo_redo` into one `log_outcome(state, source, success_message,
    detail, result)`, plus a new `command_name()` helper (a `Command`'s
    short `Debug` variant name). `apply()`/`create_device_impl` now put
    the short name in `source`/`message` and the full `Debug` dump in
    `detail` exactly once, instead of the full dump duplicated into both
    `source` and `message` (multi-KB for `Command::Batch`/`CreateDevice`).
  - **#2** (`session_log.rs`): `from_import_report`'s `unknown` loop now
    carries `sample`/`source_path` into `detail`/`message` instead of
    dropping them; added a `report.opaque` -> info-level mapping (new
    `import:opaque` source), inserted after `unknown` in both the code
    and its doc comment's stated order. `report.inferred`/
    `SourceInfo::namespace_disagreement` are explicitly out of scope,
    noted as a residual in the doc comment and in `KNOWN_LIMITATIONS.md`
    #36.
  - **#11** (`domain.rs`): `open_project`'s import-summary log line now
    reports `mapped/read` per entity instead of `mapped` only — `read`
    is the actual import-loss signal when it exceeds `mapped`.
  - **#3** (`App.tsx`/`LogPanel.tsx`): the open Log tab never refreshed
    after a *failed* operation, since its fetch was keyed only on `tree`
    (which only changes on success). Added a `logVersion` counter bumped
    by a new `reportError(e)` wrapper (replacing all 9 `catch`-block
    `pushError(api.errorMessage(e))` calls in `App.tsx`), threaded into
    `LogPanel` as a second `refreshKey` prop/effect-dependency.
  - **#12** (`styles.css`): added `--knx-warning-color` and a
    `.log-entry-warning .log-entry-severity` rule — only `.log-entry-error`
    had severity color styling before, so warning log entries rendered
    in the default text color.
  - **#5** (`http_log_route.rs`): split the corpus-free `GET /api/log` ->
    `[]` check out of the corpus-gated test (which returned early before
    any assertion ran whenever `OriginalData/` was absent, so the route
    had zero CI coverage) into its own always-running
    `get_log_on_a_fresh_state_returns_an_empty_array` test.
  - **#6** (docs): corrected `GAP_ANALYSIS_ETS.md`/`IMPLEMENTATION_STATUS.md`'s
    T11 test-count claim — verified via `cargo test -p knx-server --lib
    -- --list` (20 total: 8 `domain.rs` + 9 `paths.rs` + 3
    `session_log.rs`) and `grep -c '#\[test\]'` per file; the actual T11-
    added count is 6 new unit tests (3 `session_log.rs`, 3 `domain.rs`),
    not 20 (the crate's unrelated `--lib` total). Also added the
    pre-`apply()` exclusion sentence, the `report.opaque`
    mapping/`report.inferred` residual sentences, and a sentence noting
    the `.field-error` inline-render deviation from the original design
    spec's toast description.
  - **#7** (this file): this entry.
  - **#15**: the `.field-error`-vs-toast deviation note above, in
    `IMPLEMENTATION_STATUS.md`.
  Parked as a new `KNOWN_LIMITATIONS.md` #36 entry rather than fixed:
  the Log tab requiring an open project (#4) and unbounded `SessionLog`
  growth (#16) — both plan-level/follow-up scale, not final-review-fix
  scale.
  Commits: `6435bbd` (Rust: log-helper collapse, opaque/detail mapping,
  read-count fix, `http_log_route.rs` split), `5289fc8` (frontend:
  `refreshKey`, warning CSS), and this docs commit (see `git log` for its
  SHA — committed immediately after this entry).
  Verification gate, all clean: `cargo fmt --all --check`; `cargo test
  --workspace` (full workspace green, no failures); `cargo clippy
  --workspace --all-targets -- -D warnings` (clean); `cd apps/knx-web &&
  npx tsc --noEmit` (clean); `npm test -- --run` (122 passed, was 121);
  `npm run build` (succeeded, `dist/.gitkeep` restored after).
- **Pending/Next Steps:** Awaiting scoped re-review of this fix round;
  once clean, the coordinator merges `t11-session-log` to `main`.
- **Notes for Codex:** `log_outcome`'s signature grew two params:
  `log_outcome(state, source, success_message, detail, result)` — every
  call site either passes `None` for `detail` (save/export/undo/redo,
  unchanged shape) or `Some(cmd_desc)` (`apply()`/`create_device_impl`,
  the command's full `Debug` dump). `log_command_outcome` and
  `log_undo_redo` no longer exist — do not reintroduce them; add a new
  `log_outcome` call site instead. `LogPanel` now requires a
  `refreshKey: number` prop in addition to `tree` — any new render site
  (tests included) must pass both. `KNOWN_LIMITATIONS.md` #36 documents
  two known-open gaps (Log tab needs an open project; no log entry cap)
  that are deliberately *not* fixed here — don't treat them as
  regressions if you see them again.

- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 18:20
- **Completed:** Merged **T11 (session log)** to `main` — PR
  [#5](https://github.com/KNXBench-Labs/KNXBench/pull/5), merge commit
  `6067ef8`, 11 commits, fork point `9403bd8`. The scoped re-review of the
  final-review fix round (see the entry above) came back with all ten
  targeted findings **Addressed**, no new blocking issues, verdict Ready
  to merge; the re-reviewer independently re-derived the corrected
  test-count claim (3 new `session_log.rs` + 3 new `domain.rs` = 6 new
  unit tests; the crate's 20 lib tests include 9 unrelated `paths.rs`
  ones) rather than trusting the commit's own numbers. Coordinator then
  re-ran the full gate directly on the merged HEAD before finishing:
  `cargo fmt --all --check`, `cargo test --workspace` (all green, zero
  failures), `cargo clippy --workspace --all-targets -- -D warnings`
  (clean), `npx tsc --noEmit` (clean), `npm test` (122/122),
  `npm run build` (clean, `dist/.gitkeep` restored). GitHub Actions on
  PR #5 green on both jobs ("Build, test, lint" 5m45s, "License and
  advisory gate" 56s) before merging. Worktree
  `.worktrees/t11-session-log` removed, branch deleted locally and on
  `origin`, and the plan's `.superpowers/sdd/2026-09-10-session-log/`
  workspace deleted per subagent-driven-development.
  Also merged, separately and independently: **PR
  [#6](https://github.com/KNXBench-Labs/KNXBench/pull/6)** (`0eb348f`,
  branch `docs-b4-reconcile`, now deleted), a one-line reconciliation of
  `docs/GAP_ANALYSIS_ETS.md`'s **B4** table row, which still described
  building parts as read-only with "no `Command` exists for either yet"
  while the T8 backlog entry thirty lines below in the same file had read
  **Done (2026-09-08)** ever since T8 landed. Verified before writing:
  `CreateBuildingPart`/`DeleteBuildingPart`/`RenameBuildingPart`/
  `MoveDeviceToBuildingPart` all present in
  `crates/knx-core/src/command.rs`, with the Project Explorer create row
  and Inspector rename/delete/move UI driving them. Kept out of the T11
  branch deliberately (CLAUDE.md: do not mix unrelated changes).
- **Pending/Next Steps:** T11 and B4 are both done and merged; nothing is
  outstanding on either. Next actionable backlog items, in
  `docs/GAP_ANALYSIS_ETS.md`'s own Tier-3 order: **T12** (CSV
  group-address import/export, closes **C2**), **T13** (project
  documentation export — PDF/HTML report, closes **D4**), **T14**
  (project diff/compare, closes **C1**). `KNOWN_LIMITATIONS.md` **#36**
  (Log tab unreachable without an open project; no `SessionLog` growth
  cap) is a genuine, deliberately-parked T11 follow-up if anyone wants a
  small self-contained pick instead.
- **Notes for Codex:** The main checkout had untracked, byte-identical
  copies of `docs/superpowers/plans/2026-09-10-session-log.md` and
  `docs/superpowers/specs/2026-09-08-session-log-design.md` that blocked
  the post-merge fast-forward ("untracked working tree files would be
  overwritten"). Both were confirmed identical to the now-committed
  versions (`git show origin/main:<path> | diff - <path>`) before being
  removed — nothing was lost. Worth checking for the same situation
  whenever a branch commits a spec/plan that was previously only a local
  working-tree file: the merge will refuse to fast-forward until the
  untracked copy is gone. `.worktrees/session3-ets-import` remains the
  only other worktree (stale, unrelated, untouched).

- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 21:00
- **Completed:** Task 7 (final task, docs-only) of the **T12** plan
  (`docs/superpowers/sdd/2026-09-10-csv-group-address-exchange/`, branch
  `t12-csv-group-addresses`, worktree
  `.worktrees/t12-csv-group-addresses`). Tasks 1-6 (the `knx-csv` crate,
  `Command::UpdateGroupAddress`, the writer/planner, the server routes, the
  CLI subcommands, the web buttons) were already implemented and reviewed
  before this task started; this task only reconciled documentation —
  no production code touched. Files changed: `docs/IMPORT_EXPORT.md` (new
  §11, the format table, the honesty statement that no verified ETS CSV
  sample exists anywhere in this repo or the KNX Standard v3.0.0 corpus,
  and what import never does), `docs/COMPATIBILITY.md` (a new §2 verified
  row for the corpus-gated round trip, a new §4 "not supported" row naming
  ETS CSV/`.esf` explicitly), `docs/GAP_ANALYSIS_ETS.md` (C2 rewritten
  "Closed", T12 struck through with the full technical account, matching
  how C4/T10 and D7/T11 were closed), `docs/IMPLEMENTATION_STATUS.md` (new
  T12 entry appended after T11's), `docs/KNOWN_LIMITATIONS.md` (five new
  entries, **#38-#42**: unverified ETS interoperability; never
  re-addresses/deletes/manages ranges; export-only columns never applied
  plus no `Description`/`Comment` columns; the German-locale Excel
  separator hazard; and `command_sync.rs`'s module doc overstating
  `sync_after_command` as live when it has no callers anywhere in
  `crates/`/`apps/` — pre-existing, not caused by this task, just made
  easier to trip over by this task's own no-op `UpdateGroupAddress` arm in
  that file). `docs/ROADMAP.md` was **not** touched — it names neither T12
  nor C2. Every count written down (50 `knx-csv` tests, 5 server
  integration tests, 5 CLI integration tests, 4 `knx-core`
  `UpdateGroupAddress` tests, 11+4 frontend tests) was re-derived directly
  from `cargo test -p knx-csv -- --list` and `grep`/`grep -c` against the
  actual test files at doc-writing time, not copied from any prior task
  report. Full report:
  `.superpowers/sdd/2026-09-10-csv-group-address-exchange/task-7-report.md`.
- **Pending/Next Steps:** Run the full gate (`cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test
  --workspace`, `cargo run -p xtask -- check-layering`, `cargo deny check`,
  then `apps/knx-web`'s `npx tsc --noEmit && npm test -- --run && npm run
  build`, restoring `dist/.gitkeep`), commit the docs
  (`git add docs && git add -f .ai && git commit`), then hand off for
  review — this task does not merge or push. Once reviewed and merged,
  the branch/worktree should be cleaned up the same way T11's was.
- **Notes for Codex:** No production code changed in this task —
  everything above is doc reconciliation only, so if you're picking up
  from here expect `git diff main` on this branch to be entirely under
  `docs/` and `.ai/`. The two "parked" items recorded in
  `KNOWN_LIMITATIONS.md` #42 and in the T12 `IMPLEMENTATION_STATUS.md`
  entry (session-log `csv-import:<kind>` source-string redundancy, and
  `command_sync.rs`'s stale module doc) are deliberately *not* fixed here
  — don't treat them as regressions if you see them again; they're
  recorded, not forgotten.

- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 21:20
- **Completed:** Two things, in this order.

  (1) **T12 closed and merged to `main` locally.** Both remaining reviews
  returned clean: the Task 7 documentation review (spec ✅ / quality ✅,
  0 findings — every count in the doc commit independently re-derived, the
  pre-report amend confirmed to have lost nothing via an empty
  `git diff --stat 354474d 3561220`) and the final whole-branch review over
  all 17 commits (Critical 0, Important 0, Minor 1). The final reviewer ran
  its own mutation against the seam this branch historically got wrong
  twice — removing the `+1` byte-inclusion offset in `read.rs`'s `line_of`
  failed 3 of 4 named line-number regression tests with no collateral — and
  re-ran every gate itself. The one Minor finding: `IMPORT_EXPORT.md` §11 and
  `KNOWN_LIMITATIONS.md` #38 claimed a second ETS-shaped column profile
  could be added "without a rewrite" because "the column-mapping layer
  already exists for this exact purpose", when `read.rs`'s `map_headers` is
  a single hard-coded `match`. Fixed in `b85f92c` (prose only, both
  sentences now describe the actual code). Merged as `04b7ef5`; worktree
  `.worktrees/t12-csv-group-addresses` removed, branch deleted, SDD plan
  workspace deleted from both checkouts. Merged-result verification on
  `main`: `cargo test --workspace` exit 0, `npm test -- --run` 137/137.

  (2) **Roadmap extended with a motion/animation constraint**, by explicit
  user request ("die Animationen sollen togglebar sein, wenn sie
  implementiert werden"). Branch `roadmap-motion-toggle`, single commit
  `dc45f96`, merged as `0b2f8e5`, worktree and branch cleaned up. The rule:
  every animation this application ships must be switchable off from inside
  the application, and `prefers-reduced-motion: reduce` always wins over the
  in-app choice. Investigating it surfaced a regression nobody had recorded:
  cycle 11 shipped exactly such a control (three-level
  `off`/`subtle`/`standard`, driving `--knx-transition-duration`), and cycle
  13's theme rewrite deleted `ThemePanel.tsx` — the surface it lived on —
  without replacing it. Verified: no `.ts`/`.tsx` file under
  `apps/knx-web/src` mentions motion at all; only `styles.css`'s token and
  three `@media (prefers-reduced-motion: no-preference)` blocks remain, so
  the OS preference is currently the only control and it is all-or-nothing.
  Recorded as gap **D11**, backlog **Tier 7 / T27**, `KNOWN_LIMITATIONS.md`
  **#43**, and a "Cross-cutting — Motion and animation" section in
  `ROADMAP.md`. Two stale references fixed while there: `D8` and `T25` both
  still pointed at the deleted `ThemePanel.tsx`.
- **Pending/Next Steps:** Nothing is pushed — the standing user instruction
  is to skip the GitHub workflow entirely (no push, no PR, no CI, no remote
  merge) until further notice, so `main` is ahead of `origin/main` by design.
  Next backlog items in order: **T13** (project documentation export, HTML
  first, closes **D4**) and **T14** (project diff/compare, closes **C1**).
  Still parked and unscheduled: `KNOWN_LIMITATIONS.md` #36 (Log tab
  unreachable without an open project, no `SessionLog` growth cap), #42
  (`command_sync.rs`'s stale module doc), and now #43/T27.
- **Notes for Codex:** T27's design question, if you pick it up, is whether
  the motion setting is one global duration multiplier (cycle 11's approach —
  one CSS token, trivially honoured) or a per-category switch, which is more
  useful once "animation" means a telegram travelling along a bus line rather
  than a button fading on hover. Not decided; deliberately left to a design
  spec. Also note that D11 is *not* an ETS parity gap — ETS has no comparable
  animation — it is filed in the UI section only because that is where the
  control would live.

- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 21:45
- **Completed:** Three things on `main`, all committed locally, nothing pushed.

  (1) **T13 design spec and implementation plan written** (`a986382`):
  `docs/superpowers/specs/2026-09-10-project-documentation-export-design.md`
  and `docs/superpowers/plans/2026-09-10-project-documentation-export.md`.
  The feature is a self-contained HTML project documentation export (closes
  gap **D4**), deliberately HTML-first rather than PDF: the browser's own
  print-to-PDF is the PDF path, so the crate ships `@media print` rules
  instead of a PDF writer dependency. New pure crate `knx-report`
  (`&Project` in, `String` + typed warnings out; no filesystem, no HTTP, no
  SQLite, **no clock** — the generation timestamp is passed in by the
  caller, which is what makes the output byte-deterministic in tests).
  Public surface: `render_html(&Project, &ReportOptions) -> HtmlReport`,
  with `ReportOptions { generated_at }`, `HtmlReport { html, warnings }`,
  `ReportWarning { location, detail }`. Seven tasks: crate + escaping +
  document shell + layering rules; derived model indices and orphan
  detection; the renderer; `POST /api/project/documentation-export` plus a
  session-log entry; a CLI subcommand; a web button; corpus proof and
  documentation reconciliation. Reconnaissance for it is at
  `.superpowers/sdd/t13-research/recon.md` — its load-bearing findings:
  `GroupAddressEntry` carries no DPT field, `knx-projection`'s DTOs are
  lossy for topology/buildings, zero HTML or templating crates exist
  anywhere in the workspace, and nothing in `docs/` describes ETS's own
  report contents (which is why the plan forbids every "ETS report" /
  "ETS-compatible" claim — parity is unmeasurable without a sample).

  (2) **Plan amendment settling two questions it had left open** (`a523e05`),
  both ruled before any implementer was dispatched. Task 2's test list had
  demanded a warning for a device with `address: None`, which the spec never
  listed and which `crates/knx-core/src/device.rs:25-26` documents as valid
  state — dropped, because warning on it would bury the real findings under
  noise from every half-authored project. Task 2 also left "maps to an empty
  list (or is absent — assert which)" undecided — ruled **absent from the
  map**, renderer treats a missing key as "no links", so there is one
  representation instead of two and no test that passes either way.

  (3) **Roadmap and gap analysis extended with an in-application help
  system** (`b2b74ec`), by explicit user request ("roadmap update:
  documentation (At the end of it all) hilfe System (UI, Hover over usw)").
  Recorded as gap **D12**, backlog **Tier 8 / T28**, and a new
  "Cross-cutting — In-application help and user documentation" section in
  `ROADMAP.md`, deliberately scheduled after everything else because help
  text describes a UI that is still changing (cycle 13 deleting
  `ThemePanel.tsx` out from under cycle 11's motion control is the cited
  precedent). Current state measured from the code before writing it:
  exactly **one** `title=` attribute in the entire frontend
  (`Inspector.tsx:218`), four `aria-label`s, zero `aria-describedby`, no
  tooltip component, no help panel, no `F1` handler,
  `commandRegistry.ts`'s `shortcutHint` visible only inside the Command
  Palette, and all nine `docs/` files developer-facing and unreachable from
  the running application. T28 is bound to **T25** (message-catalogue
  extraction — help strings are the largest new body of user-facing text)
  and **T27** (motion switchable off — a tooltip that fades is an
  animation), and is explicitly distinct from both T13's documentation
  export and the deferred in-app project-notes feature.
- **Pending/Next Steps:** T13 is in flight on branch
  `t13-documentation-export` (worktree `.worktrees/t13-documentation-export`,
  forked from `a986382`), executed via subagent-driven-development with its
  ledger at `.superpowers/sdd/2026-09-10-project-documentation-export/progress.md`.
  Task 1 (the `knx-report` crate, escaping helpers, document shell, and the
  two `xtask` layering rules) is committed as `bcbaa6c` and under task
  review; Tasks 2-7 remain. Separately, `KNOWN_LIMITATIONS.md` **#36** is
  being closed on branch `fix-36-log-tab` (worktree
  `.worktrees/fix-36-log-tab`): a 1000-entry `SessionLog` cap with a
  synthetic drop-notice entry, and the Log tab made reachable without an
  open project. Still nothing pushed — the standing user instruction is to
  skip the GitHub workflow entirely (no push, no PR, no CI, no remote merge)
  until further notice. After T13: **T14** (project diff/compare, closes
  **C1**). Parked and unscheduled: #42 (`command_sync.rs`'s stale module
  doc), #43/T27 (motion toggle), #D12/T28 (help system).
- **Notes for Codex:** If you pick up T13, the non-obvious constraint is
  that `cargo run -p xtask -- check-layering` walks **dev-dependency** edges
  too (`xtask/src/layering.rs:94-98` takes every edge regardless of
  `dep_kinds`). That is why the plan puts T13's corpus test in `knx-app`
  rather than in `knx-report` — the same reason
  `crates/knx-app/tests/csv_roundtrip.rs` lives where it does. A
  `dev-dependencies` entry on `knx-etsproj` inside a pure crate fails the
  layering gate exactly like a real dependency would.

- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 21:10
- **Completed:** Closed `KNOWN_LIMITATIONS.md` #36 (the T11 follow-up
  parked at the end of that feature's own entry), branch
  `fix-36-log-tab`, worktree `.worktrees/fix-36-log-tab`. Two commits.

  (1) `019ed90` — `SessionLog` growth cap. New documented
  `MAX_ENTRIES: usize = 1000` const in `apps/knx-server/src/
  session_log.rs`. Past it, `push()` drops the oldest real entry per
  call and pins a synthetic `Severity::Warning`/`source: "log"` entry at
  index 0 naming the running total dropped so far, refreshed on every
  further drop, never itself dropped or duplicated, and counted against
  the cap so `entries().len()` never exceeds 1000. `reset()` clears the
  dropped count too. Wire shape (`GET /api/log` → bare `Vec<LogEntry>`)
  is untouched, so T12's `from_csv_import_report` and the existing
  `apps/knx-server/tests/http_log_route.rs` integration tests needed no
  changes — deliberately verified by reading both before touching
  anything, per the brief's own warning. 5 new unit tests in
  `session_log.rs`'s own `#[cfg(test)]` module: under the cap, exactly
  at the cap, one past it (names 1 dropped), well past it (cap + 250,
  names 250), reset-after-a-drop.

  (2) `b6853c8` — Log tab reachability. `App.tsx`'s "Log" button lost its
  `disabled={!tree}`; the `.workspace` slot now renders on
  `tree || logOpen` rather than `tree` alone. `ProjectExplorer` stays
  gated on `tree` (it genuinely needs a project); `LogPanel`'s prop type
  is now `tree: ProjectTree | null`. With a project open the layout is
  byte-for-byte the same as before (same slot, same Inspector/Dashboard
  swap on close). New `apps/knx-web/src/App.test.tsx` — the first
  App-level test in this repository — 2 tests: reachable with no
  project open (button enabled, clicking it renders `.log-panel` with
  whatever `getSessionLog()` returned, no `.project-explorer` present),
  and unchanged behaviour with one open (both panels present, closing
  the tab returns to Dashboard). No existing test asserted the Log
  button was disabled without a project, so there was nothing to update
  there — the brief anticipated that case but it didn't occur.

  Both written test-first (failing for the right reason before
  implementation, confirmed by running them against the unmodified
  code). Gates all clean on the worktree: `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test
  --workspace`, `cargo run -p xtask -- check-layering`, `npx tsc
  --noEmit`, `npm test -- --run` (139/139), `npm run build` (with the
  `dist/.gitkeep` restore afterwards). Docs updated:
  `KNOWN_LIMITATIONS.md` #36 rewritten closed (kept its number, "—
  resolved (2026-09-10)" suffix, **Resolved.**/**Originally.**
  structure matching #17's precedent), `IMPLEMENTATION_STATUS.md` gained
  a "T11 follow-up" entry right after T11's own.

  Note for the record: `apps/knx-web` had no `node_modules` in this
  worktree (each worktree needs its own, and none had been installed
  here yet). Symlinked it to the main checkout's
  `apps/knx-web/node_modules` rather than running `npm install`, since
  the two checkouts share the same `package-lock.json` and a symlink
  costs nothing to undo. Not committed (gitignored either way) — if you
  hit `Cannot find package 'vitest'` in a fresh worktree, that symlink
  (or an actual `npm install`) is why.
- **Pending/Next Steps:** Nothing pushed (standing instruction: skip the
  GitHub workflow). Not merged to `main` yet either — that's for
  whoever reviews this branch next. Backlog otherwise unchanged from the
  entry above: **T13** (project documentation export) and **T14**
  (project diff/compare) are next in order. `KNOWN_LIMITATIONS.md` #42
  (`command_sync.rs`'s stale module doc) and #43/T27 (motion toggle)
  remain open and unscheduled.
- **Notes for Codex:** `session_log.rs`'s `dropped` counter counts real
  entries removed, not overflowing calls — see the fix-round entry below.

- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 22:05
- **Completed:** Fix round 1 on branch `fix-36-log-tab`, one commit on
  top of the three above. Review caught a real bug in `019ed90`'s
  `dropped` counter, not just a documentation nit: it counted
  *overflowing push calls*, not real entries actually removed, and those
  two numbers diverge from the very first drop onward, by exactly one,
  forever. The synthetic entry was claiming "1 log entry dropped" while
  2 were actually gone — silently misreporting the exact thing this
  mechanism exists to report accurately.

  Fixed in `apps/knx-server/src/session_log.rs`: `push()` now increments
  `dropped` by 2 on the push that first exceeds the cap (one entry
  evicted for being oldest, one more to make room for the synthetic
  entry itself) and by 1 on every overflowing push after that — matching
  how many real entries actually leave `entries()`. `synthetic_drop_notice`
  simplified to always say "entries" (`dropped` is never 1 under this
  accounting, so the singular branch was dead). Rewrote both `push`'s doc
  comment and the module doc comment to describe the corrected
  accounting; the old "incremented per call that overflows, not per
  entry removed" explanation is gone, not edited around.

  Tests: `one_past_the_cap_...` renamed to
  `one_past_the_cap_drops_two_real_entries_and_names_two_dropped` and now
  asserts the synthetic entry names 2 dropped (not 1) and that both
  `entry 0` and `entry 1` are gone. `well_past_the_cap_...` now asserts
  251 (not 250) for cap + 250 pushes. New test
  `dropped_plus_retained_always_equals_total_pushes_past_the_cap` pins
  the invariant directly — parses the number out of the synthetic
  entry's own message and asserts `dropped + retained == total_pushed`
  at two overflow sizes (1 and 250 past the cap) — the kind of test that
  would have failed against `019ed90`'s original accounting and so would
  have caught this before it shipped. `docs/KNOWN_LIMITATIONS.md` #36 and
  `docs/IMPLEMENTATION_STATUS.md`'s T11-follow-up entry updated to match
  (test count 5 → 6, "1"/"250" → "2"/"251", "per call" phrasing removed).

  Gates all clean: `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo test --workspace`, `cargo run -p
  xtask -- check-layering`. Frontend untouched this round, so `npm
  test`/`tsc`/`build` were not re-run (nothing in `apps/knx-web` changed).
- **Pending/Next Steps:** Same as above — nothing pushed, not merged.
  **T13**/**T14** next in order; `KNOWN_LIMITATIONS.md` #42/#43 remain
  open and unscheduled.
- **Notes for Codex:** If you touch `session_log.rs`'s eviction logic
  again, add a test in the same shape as
  `dropped_plus_retained_always_equals_total_pushes_past_the_cap` for
  whatever you change — it pins the actual invariant instead of a
  hard-coded number, which is what would have caught the original bug
  a round earlier.

- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 22:30
- **Completed:** **Limitation #36 closed and merged to `main` locally** as
  merge commit `1ead75b` (five commits: `019ed90` cap, `b6853c8` reachable
  Log tab, `6366002` docs, `bf60528` drop-count correction plus invariant
  test, `5fc4737` a one-line doc fix). The final whole-branch review
  (sonnet, `da79328..bf60528`) came back **Ready to merge**: 0 Critical, 0
  Important, 3 Minor. It hand-traced `push()`'s arithmetic itself at the
  cap, one past it and far past it rather than trusting the comments or the
  tests, mentally reverted the accounting to confirm all six new tests would
  fail against the old buggy version, and re-ran every gate independently.
  One of its three Minor findings was acted on before merging (`5fc4737`):
  `KNOWN_LIMITATIONS.md` #36 opened with "two independent fixes, one commit
  each" while admitting two paragraphs later that Part B had needed a second
  commit — a document disagreeing with itself, now fixed. The other two were
  left: a `tree: null` unit test local to `LogPanel.test.tsx` (the path is
  already covered through `App.test.tsx`, and the prop is used only as an
  effect-dependency identity), and `entries.remove(1)`'s O(n) shift per
  overflowing push (trivial at a 1000-entry cap; a `VecDeque` ring buffer
  would only be worth it if the cap grew a lot, and CLAUDE.md wants
  performance work driven by measurement).

  The merge conflicted in this file only — both sides appended a dated block
  to the end. Resolved by keeping both, in the order they were written.

  Merged-result gates on `main`, all green and all re-run after the merge,
  not before it: `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets -- -D warnings` (clean — note the two `large_enum_variant`
  errors that older entries in this file mention are long since fixed),
  `cargo test --workspace` (0 failed across every crate), `cargo run -p
  xtask -- check-layering`, `cargo deny check` (advisories/bans/licenses/
  sources ok), `npx tsc --noEmit`, `npm test -- --run` (139/139),
  `npm run build` (`dist/.gitkeep` restored afterwards). Worktree
  `.worktrees/fix-36-log-tab` removed and branch `fix-36-log-tab` deleted.
- **Pending/Next Steps:** **T13** (project documentation export) continues on
  branch `t13-documentation-export` — Task 1 (the crate, its escaping and its
  document shell) complete and reviewed clean at `bcbaa6c`; Task 2 (the pure
  derived model in `model.rs`) complete and reviewed clean at `d44d566`, which
  includes a fix for `build_range_forest` silently dropping group ranges whose
  `parent`/`children` ids dangle; Task 3 (the renderer, `render.rs`) in flight;
  Tasks 4-7 (CLI surface, HTTP route, web UI button, corpus test) queued with
  their briefs already extracted. Nothing on that branch is merged yet.
  After T13: **T14** (project diff/compare, closes **C1**).
  Nothing is pushed and nothing will be — the standing user instruction is
  to skip the GitHub workflow entirely until further notice, so `main` is
  ahead of `origin/main` by design. Still parked and unscheduled:
  `KNOWN_LIMITATIONS.md` #42 (`command_sync.rs`'s stale module doc),
  #43/T27 (motion toggle), D12/T28 (in-application help system).
- **Notes for Codex:** Remaining worktrees are
  `.worktrees/t13-documentation-export` (active work) and
  `.worktrees/session3-ets-import` (stale, unrelated, untouched for several
  sessions — 0 commits ahead of an old `main`). If you need a clean tree,
  the second one is almost certainly safe to remove, but it is not mine to
  delete on a guess.

- **Last Agent:** Claude
- **Timestamp:** 2026-09-10 23:10
- **Completed:** Task 7 of the T13 SDD plan (worktree
  `.worktrees/t13-documentation-export`, branch `t13-documentation-export`,
  since reviewed and merged into `main` — see Pending/Next Steps).
  Corpus-gated integration test plus the documentation close-out for
  **T13**/**D4** (HTML only).

  Added `crates/knx-app/tests/documentation_export.rs`: imports the
  maintainer's real reference project
  (`OriginalData/DemoProjects/Unser Zuhause ets4 - 2025-12-15.knxproj`,
  gitignored, local-only — skips with an `eprintln!` when absent), renders it
  with `knx_report::render_html`, and asserts every formatted group address
  and every escaped device name appears in the output, `<table>`/`</table>`
  and `<tr>`/`</tr>` counts balance, the Summary section's nine rows match
  counts independently recomputed from `Project` (not from `knx-report`'s own
  `compute_counts`), the document is self-contained (no `<script`, no
  `http://`/`https://`), and a second render with the same timestamp is
  byte-identical. Confirmed against the real corpus (not the skip path): 36
  devices, 907 communication objects, 514 group addresses, 248799 bytes of
  HTML, 1 structural warning. `knx-app/Cargo.toml` gained
  `knx-report.workspace = true` under `[dev-dependencies]`, for the same
  layering reason `csv_roundtrip.rs` needs `knx-csv` there:
  `check-layering` walks dev-dependency edges too, so `knx-report` itself
  must never depend on `knx-etsproj`, and `knx-app` is the one crate allowed
  to see both sides. Git worktrees don't carry over gitignored directories
  from the main checkout, so a local-only, uncommitted symlink
  `OriginalData -> /mnt/daten-i/Sourcecode/KNXBench/OriginalData` was created
  inside this worktree to exercise the real corpus path; it is not tracked
  and was never staged.

  Documentation: `docs/IMPORT_EXPORT.md` gained a new "§12. Project
  documentation export" section (crate/API summary, self-contained-document
  properties, section list, "own document, not an ETS report" framing,
  surfaces). `docs/GAP_ANALYSIS_ETS.md`: D4 row closed for HTML only, with
  in-app printing and PDF-without-a-browser stated as explicitly still open
  in the row's own text; T13 backlog bullet closed with full crate/route/
  CLI/test citations and the real corpus numbers.
  `docs/KNOWN_LIMITATIONS.md` gained seven new entries, §44–§50, covering: no
  ETS report parity (cross-ref §38), no native PDF, no manufacturer/product/
  program name resolution, no parameter values or module-instance arguments
  (cross-ref §3/T18), single-language rendering (cross-ref §37/T26), no
  in-app print preview, no section selection.
  `docs/IMPLEMENTATION_STATUS.md` gained a dated append-only entry
  ("T13, project documentation export, HTML only (2026-09-10)") with the same
  re-derived numbers. `docs/ROADMAP.md` was checked (`grep -n
  "T13|D4\b|documentation export|printing"`, no matches) and correctly left
  untouched for the T13/D4 work, per the task brief.

  Gates, all green, all re-run from this worktree: `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace` (718 passed, 0 failed — corpus test confirmed to
  have actually run, not skipped), `cargo run -p xtask -- check-layering`,
  `cargo deny check` (advisories/bans/licenses/sources all ok), and from
  `apps/knx-web`: `npx tsc --noEmit`, `npm test -- --run` (145 passed),
  `npm run build` (succeeded; `dist/.gitkeep`, which `npm run build` deletes,
  restored via `git checkout -- dist/.gitkeep`).

  Separately, by an explicit mid-task live user request unrelated to T13/D4,
  `docs/ROADMAP.md`'s "Cross-cutting — Motion and animation" section gained a
  short memo recording two candidate visual style directions for whenever
  T27's motion work gets designed (an Apple-like sleek/subtle/clean
  direction, and a "techy glitch / cyberpunk OS" direction that is still
  meant to read as clean and sleek rather than noisy) — not decided, not
  designed, no task opened; kept in its own commit, separate from the T13/D4
  documentation work.

  A second, similar mid-task live request followed: a memo for an
  in-application LLM chat window for natural-language project interaction
  (explicitly not just MCP-reachable from outside, but built directly in),
  with an explicit admission that *how* it would work still needs research.
  Added a new "Cross-cutting — LLM / natural-language interaction" section
  to `docs/ROADMAP.md`, tied to the pre-existing deferred-MCP note in the
  Session 7 section (same `Command`-layer prerequisite, same "premature
  before Session 7" status), framed as a research item rather than a
  design — not decided, no task opened. Also its own commit, also
  unrelated to T13/D4.
  Branch close-out (added by the orchestrator after this entry was first
  written): the Task 7 review returned Approved (spec ✅ on all six steps,
  0 Critical, 0 Important, 2 Minor), and the final whole-branch review over
  `a986382..8a66c66` returned Approved with no new findings. The two parked
  Minors plus one doc nit were fixed in `19d3b71` (three documentation-only
  corrections: a missing field doc on `ReportWarning::detail`; the
  `escape_text`/`escape_attr` doc comments no longer claim an ordering
  hazard that only chained `String::replace` has; the design spec's Devices
  row no longer lists a `size` field per communication object, since
  `knx-etsproj` always sets `size: None`). A third parked Minor — the
  ordering of `chrono.workspace` in `apps/knx-cli/Cargo.toml` — was ruled
  invalid: the manifest convention here is workspace `knx-*` crates first,
  external crates after, and `apps/knx-server/Cargo.toml` places `chrono`
  identically. 914 tests pass on the merged result.
- **Pending/Next Steps:** Merged into `main` with `git merge --no-ff`
  (no `git pull` — a plain pull on `main` starts a history-rewriting
  rebase) and pushed. `.ai/CURRENT_STATE.md` was the only merge conflict;
  it was resolved by keeping both entries, newest first. The standing "skip
  the GitHub workflow" instruction means the CI workflow only: pushes
  happen normally, they are simply never waited on or used to gate a merge.
  Next backlog items in order are **T14** (project diff/compare, closes
  **C1**), and whatever T13's
  remaining "not implemented" items are eventually promoted to (native PDF,
  in-app print preview, section selection, manufacturer/product/program name
  resolution, parameter values, multi-language rendering) if any of them get
  scheduled. Still parked and unscheduled: `KNOWN_LIMITATIONS.md` #36, #42.
  The animation-style memo added to `ROADMAP.md` is a memo only — T27 itself
  is still unscheduled and undesigned.
- **Notes for Codex:** The corpus test only exercises the real path when
  `OriginalData/` is reachable from the crate root; from a git worktree it
  is not, because git worktrees don't carry over gitignored directories.
  The T13 worktree used a local-only symlink (`ln -s
  /mnt/daten-i/Sourcecode/KNXBench/OriginalData OriginalData` from the
  worktree root); it was never staged and was deleted with the worktree.
  Recreate it in any future worktree, or the test silently takes its skip
  path. Also: `knx-report`'s device-detail section
  reads two sources — `build_device_detail` (pure, from `knx-projection`) and
  direct `project.devices.get(id)` reads for `commissioning`/`product_ref`/
  `program_ref`/`binary_data`. This is deliberate (documented in
  `IMPLEMENTATION_STATUS.md`'s new T13 entry) but is exactly the kind of
  two-source pattern a reviewer should double check stays in sync if either
  side changes shape.


---

- **Last Agent:** Claude
- **Timestamp:** 2026-09-11 03:16
- **Completed:** T14 (project diff/compare) implemented end to end and merged
  into `main` as `5f852fe` (`--no-ff`), pushed (`bb2c88b..5f852fe`). Built by
  subagent-driven development from
  `docs/superpowers/specs/2026-09-10-project-diff-design.md` and
  `docs/superpowers/plans/2026-09-10-project-diff.md` (both landed earlier in
  `155b126`/`bb2c88b`), seven tasks, each individually reviewed, plus a
  whole-branch review and one fix commit.
  - **`crates/knx-diff`** — a new pure crate. Depends on `knx-core` only: no
    filesystem, no clock, no serde, no SQLite, no `knx-store`, no
    `knx-etsproj`. `xtask check-layering` gained a rule for it and the rule
    was proven to fire (a reviewer temporarily injected
    `knx-store.workspace = true` and watched the gate fail).
    - `key.rs` — the generic matching engine. `match_entities` matches on
      `ets_id` first, then on a natural key, and returns matched pairs,
      per-side leftovers and ambiguity groups.
    - `semantic.rs` — the per-entity `*Key`/`*Fields` types and their
      extraction functions, reimplementing `knx-etsproj::compare`'s
      `semantic_text`/`semantic_dpt`/`semantic_flag` techniques directly
      against `knx-core` rather than depending on that crate.
    - `diff.rs` — `diff_projects(&Project, &Project) -> ProjectDiff`.
  - **`POST /api/project/diff`** on `knx-server` compares the *live
    in-memory project* against a `.knxdb` file — "what would Save change",
    not "compare two files". Both its failure modes map to
    `ApiError::bad_request`.
  - **`knx diff <a.knxdb> <b.knxdb>`** on `knx-cli` prints the same thing as
    text: `+` added, `-` removed, `~` changed, `?` ambiguous. Exit `0`
    whenever a comparison was produced — a diff with changes is not a failed
    diff — and `1` only when a store could not be opened or loaded.
  - **`apps/knx-web`** gained `ProjectDiffPanel.tsx`, a "Compare with…"
    button in the existing toolbar row, and grouped counts per non-empty
    entity table. No tree view and no inline before/after highlighting —
    both are recorded as out of scope.
  - **`crates/knx-app/tests/project_diff.rs`** imports the reference project
    twice, independently, and asserts the diff between the two is empty at
    every level. It ran for real on the merged result: "project_diff corpus
    test: 36 devices, 907 communication objects, 514 group addresses".
  - Docs: `GAP_ANALYSIS_ETS.md` C1 and the T14 backlog entry closed with
    evidence, ten new `KNOWN_LIMITATIONS.md` entries (§51-60),
    `ARCHITECTURE.md`'s dependency graph, a dated `IMPLEMENTATION_STATUS.md`
    entry.
  - Gates on the merged result: 782 Rust tests passed / 0 failed / 3 ignored,
    157 frontend tests, `cargo fmt`, `cargo clippy -D warnings`,
    `check-layering` and `cargo deny check` all clean, `tsc --noEmit` clean.
  Three design decisions worth remembering, all made during the cycle:
  (1) devices get dedicated `DeviceTable`/`DeviceChange` types rather than
  reusing the generic `EntityTable`, because nesting communication objects
  and parameters inside a `Fields` type would duplicate them meaninglessly on
  both sides of a change; (2) a device whose own fields are unchanged but
  whose communication objects or parameters changed still appears in
  `devices.changed`, with an empty `changed_fields` — the alternative
  silently discards change information; (3) ordering is deterministic
  throughout and no `HashMap` may influence output anywhere, a rule the Task 1
  review had to enforce once by rejecting the first implementation.
- **Pending/Next Steps:** T14 is closed. The remaining backlog, roughly in
  the order the roadmap implies: T25/T26 (i18n, gap D10), T27 (motion toggle,
  gap D11 — the two-animation-styles memo in `ROADMAP.md` belongs here),
  T28 (in-application help, gap D12), T20 (Functions, needs an ADR first),
  T21 (graphical views, gaps D1/D2), T16 (catalog browser), T18 (parameter
  editor, needs the RESEARCH R3 spike first), T15/T17 (bus-facing UI), T19
  (KNX Secure, blocked on key material and hardware), T22 (multi-user, needs
  a design decision before any task). Four items still need the maintainer's
  explicit out-of-scope acceptance before the standing goal can be called
  complete: `.vd2` support, encrypted `.knxprod` (untested for want of a
  sample), T19's deferral, and the permanent exclusion of commissioning (E1).
- **Notes for Codex:** The corpus test's skip guard is the same one
  `documentation_export.rs` uses — it skips loudly with an `eprintln!` when
  `OriginalData/DemoProjects/Unser Zuhause ets4 - 2025-12-15.knxproj` is
  absent, and a green test run therefore proves nothing on its own. Run it
  with `-- --nocapture` and look for the counts line before believing it.
  From a git worktree the file is unreachable unless you symlink
  `OriginalData` in by hand (worktrees don't carry gitignored directories);
  the T14 worktree did exactly that and the symlink was deleted with it.
  Two things in the new crate are easy to break without noticing: the
  ordering rule (any `HashMap` touching output order is a defect, not a
  style question) and the device rule above — `apps/knx-cli`'s
  `"~ device {id}: (own fields unchanged)"` branch exists solely to keep that
  information visible, and there is now a CLI test that fails if it is
  removed. `knx-diff` deliberately carries no serde: the JSON DTOs live in
  `apps/knx-server`, and the TypeScript interfaces in `apps/knx-web/src/api.ts`
  are hand-written mirrors of those DTOs, so a field renamed on the server
  will compile fine on both sides and silently render zeroes.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-09-11 04:32
- **Completed:** Closed the two residuals left behind by T24 (standalone
  `.knxprod` product-package install), merged to `main` as `5ff6935`.
  Context first: an audit of the whole T24 plan
  (`docs/superpowers/plans/2026-09-09-standalone-product-database-install.md`)
  against the running code established that all five of its tasks had in fact
  shipped — the plan carries no SDD ledger, which made it *look* unexecuted,
  but it was implemented and merged on 2026-09-10 as `af5639a` from the
  `codex/catalog-creation-diagnostics` branch. Verified by installing all six
  corpus files through the real CLI into a scratch database: the three
  scheme-11 and two scheme-20 archives install, the duplicate Weinzierl file
  is correctly reported as already known, and the `.vd2` is refused. No
  documentation over-claim was found anywhere.
  Two real gaps did survive that audit, and this cycle closed both.
  (1) Design-spec acceptance criterion 4 — "the caller receives the archive
  hash/size in the error report where available" — was unimplemented for
  `.vd2`: the filename check ran before anything was hashed and
  `PackageError::LegacyVd2` was a payload-free unit variant. It is now
  `LegacyVd2 { sha256, len }`, the `MAX_PACKAGE_SIZE` guard moved ahead of the
  suffix check so the hashing stays bounded, and the evidence reaches both the
  CLI and the HTTP 400 body. The leading sentence
  `legacy .vd2 product data is unsupported` is unchanged, byte for byte.
  (2) The real 77265-byte `Weinzierl_730_KNX_IP_Interface_ETS2-3.vd2` had never
  been exercised by a test — only a synthetic three-byte stand-in had.
  `rejects_the_real_legacy_vd2_corpus_file_with_its_hash_and_size` and
  `a_small_vd2_still_reports_hash_and_length` now cover it, matching strictly
  on the variant and comparing the digest against one the test computes
  itself.
  `.vd2` remains unsupported. Only the refusal's reporting changed.
  Gates on the merged result: 784 Rust tests, 0 failed, 3 ignored (782 before,
  +2 new); `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
  -D warnings`, `cargo run -p xtask -- check-layering` and `cargo deny check`
  all clean. The frontend was untouched by this change, so its gates were not
  re-run. The corpus test was confirmed to have really executed, with
  `-- --nocapture` printing
  `sha256=bc84765f511fbff6ec62325c43a68673fa930ee23b9e69399a1c1ee411a72a48
  len=77265`.
  Docs reconciled in the same change: `docs/IMPLEMENTATION_STATUS.md`,
  `docs/GAP_ANALYSIS_ETS.md` and `docs/KNOWN_LIMITATIONS.md` no longer say the
  hash/size criterion is unimplemented.
- **Pending/Next Steps:** The standing goal's highest-priority item — installing
  devices from freely downloadable manufacturer product databases — is now
  complete end to end for the tested scheme-11/scheme-20 corpus, with no known
  residual. The RESEARCH R3 spike (the `when/@test` grammar in the
  ApplicationProgram `Dynamic` tree) has just been run and its findings are
  being written into `docs/` as a separate change; that unblocks **T18**
  (parameter interpretation and editor), the single largest remaining gap by
  effort. The rest of the backlog is unchanged: T25/T26 (i18n, gap D10), T27
  (motion toggle, gap D11 — the two-animation-styles memo belongs there), T28
  (in-application help, gap D12), T20 (Functions, needs an ADR first), T21
  (graphical views, D1/D2), T16 (catalog browser), T15/T17 (bus-facing UI),
  T19 (KNX Secure, blocked on key material and hardware), T22 (multi-user,
  needs a design decision). Four items still need the maintainer's explicit
  out-of-scope acceptance before the standing goal can be called complete:
  `.vd2` support, encrypted `.knxprod` (untested for want of a sample), T19's
  deferral, and the permanent exclusion of commissioning (E1).
- **Notes for Codex:** The `.vd2` refusal is deliberately ordered: size limit
  first, then the filename suffix, then hashing, and only then — for anything
  that is not a `.vd2` — any ZIP handling at all. Do not reorder those. The
  consequence is intentional and worth knowing: a `.vd2` larger than
  `MAX_PACKAGE_SIZE` reports `SizeLimit`, not `LegacyVd2`. That is honest (a
  refusal to read that many bytes is not a claim about the format) and it
  keeps the hash bounded. Also note `installs_the_readable_corpus` and the new
  `.vd2` corpus test both skip loudly via `eprintln!` when `OriginalData/` is
  absent, so a green run in an environment without the local-only corpus
  proves nothing about real files — run with `-- --nocapture` and look for the
  printed hash/length line. From a git worktree `OriginalData` is unreachable
  unless you symlink it in by hand; this cycle's worktree did that and the
  symlink was deleted with it.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-09-11 06:15
- **Completed:** Ran research risk **R3** — the `when/@test` expression grammar
  inside an `ApplicationProgram`'s `Dynamic` tree — and wrote its findings into
  the documentation. Merged to `main` as `793b7f4`.
  R3 has been open since Session 0 and every parameter-related limitation in
  `docs/` named it as the cause. The spike read the KNX Standard v3.0.0
  extraction and 7 product archives (34 application programs, 4 manufacturers,
  22630 `when`, 12149 `choose`).
  The central result: **the KNX Standard does specify the `@test` value
  grammar**, as simpleType `Condition_t` in `Project Schema23 v01.00.00`
  §1.1.3.18 — three alternatives (a single number; a space-separated list of
  numbers; a comparison `op number` with `op` one of `= != > < >= <=`), with
  the controlling parameter required to be `TypeNumber` or `TypeRestriction`,
  and for `TypeRestriction` the `Value` attribute being what gets compared. I
  verified that citation myself before letting it into the repository: the
  section is absent from the Markdown extraction's body text and lives only in
  the `.json` twin at `#/tables/82`, which I dumped and read directly.
  The Standard says **nothing** about the surrounding structural grammar
  (`Dynamic`, `Channel`, `ParameterBlock`, `choose`, `When_t`,
  `ChannelIndependentBlock`). That part is corpus-observed only, and the
  documentation says so in exactly those words.
  Three findings that will shape T18's design, all now in
  `docs/RESEARCH.md` §4.3:
  (1) 604 of 12149 `choose` elements are controlled by a `TypeNone` parameter,
  which is neither of the two types the Standard permits — a real tooling idiom
  a literal implementation would have no defined behaviour for. All 604 carry
  exactly one `when default="true"`.
  (2) "No branch matches" is reachable and common, not a corner case: 5570 of
  the 8732 default-less `choose` elements have at least one legal enumeration
  value covered by no `when`. Neither the Standard nor the KNX Association's
  Manufacturer Tool cookbook says what that means.
  (3) `ChannelIndependentBlock` was discovered during the spike and appeared
  nowhere in this repository before. That a single spike turned up an unknown
  construct is the standing proof that the observed element vocabulary is a
  superset-so-far, not a closed grammar.
  Of the six legal operators the corpus uses only `>` (13 occurrences, against
  19138 single integers and 3417 `default="true"`).
  Docs reconciled in the same change: `docs/RESEARCH.md` (new §4.3, the R3 risk
  row, the open-questions list), `ARCHITECTURE.md`, `COMPATIBILITY.md`,
  `DATA_MODEL.md`, `GAP_ANALYSIS_ETS.md` (the T18 entry), `IMPLEMENTATION_STATUS.md`,
  `IMPORT_EXPORT.md`, `KNOWN_LIMITATIONS.md` (§3's *Cause* and *Lifted when*
  lines), `ROADMAP.md`, plus
  `.ai/logs/2026-09-11_claude_r3_dynamic_grammar.md`.
  **No limitation was lifted.** KNXBench still cannot interpret or edit a
  device parameter. What changed is the recorded *cause* — from "the grammar is
  unresearched" to "the grammar is documented, the evaluator is not built" —
  and the lift condition attached to it.
  Review found one Major issue, since fixed (`31c5067`): `docs/RESEARCH.md`
  §3.4's `ModuleInstance` bullet still called the tree "unresearched", which
  the same diff had made false 400 lines further down.
  Gates on the merged result: 784 Rust tests, 0 failed, 3 ignored — unchanged,
  as a documentation-only change should be.
- **Pending/Next Steps:** **T18** (parameter interpretation and editor) is no
  longer blocked on research and is the largest remaining gap by effort. What
  it needs first is a design decision, not more reading: a policy for the
  no-match case (finding 2 above), and a parser defensive enough to preserve
  unrecognized `Dynamic`-tree constructs rather than drop them (finding 3).
  The rest of the backlog is unchanged: T25/T26 (i18n, gap D10), T27 (motion
  toggle, gap D11 — the two-animation-styles memo belongs there), T28
  (in-application help, gap D12), T20 (Functions, needs an ADR first), T21
  (graphical views, D1/D2), T16 (catalog browser), T15/T17 (bus-facing UI),
  T19 (KNX Secure, blocked on key material and hardware), T22 (multi-user,
  needs a design decision). Four items still need the maintainer's explicit
  out-of-scope acceptance before the standing goal can be called complete:
  `.vd2` support, encrypted `.knxprod` (untested for want of a sample), T19's
  deferral, and the permanent exclusion of commissioning (E1).
- **Notes for Codex:** Keep the three confidence levels apart when you touch
  §4.3 — the Standard specifies the `@test` *value* grammar and nothing else;
  the corpus shows the structure, `default="true"`, the `TypeNone` idiom and
  every count; the evaluation algorithm ("first matching test wins, default
  covers the rest", "no match means nothing under this `choose` is active") is
  *inferred* from consistency and is marked as such. Do not flatten those into
  one voice. The `Condition_t` citation cannot be found by grepping the
  extraction's Markdown — it is in the `.json` twin's table 82, so grep the
  JSON, not the prose. `Visible` was searched for and never occurs in the
  corpus (0 of 34 programs); `Access="None"` versus a `Memory` child was
  inconclusive (4976 against 4878) and is listed as an open question rather
  than an answer.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-09-11 07:05
- **Completed:**
  T18 slice 1 shipped and merged: the `ApplicationProgram/Dynamic` tree is now
  parsed, stored, backfilled and evaluated. Branch `t18-dynamic-tree`, merged to
  `main` as `1e0d073` (`--no-ff`), pushed `6e0403c..1e0d073`. Six commits:

  - `05ae176` design spec + implementation plan (D1-D11, acceptance criteria 1-8)
  - `66ef369` Task 1: schema v3, `dynamic_node`, lossless storage
  - `c7d9ed5` Task 1 fix round (six review findings, two Major)
  - `44b06a1` Task 2: v2->v3 migration backfill + pure headless evaluator
  - `2368292` Task 2 fix round (two review findings, one Major)
  - `512be02` Task 3: documentation reconciliation, stale comments retired
  - `3f2fc5b` controller fix of two factual errors in the design document

  **Storage.** `dynamic_node` holds one row per element of every `Dynamic` tree
  — the `ApplicationProgram`'s own (`module_def_id = ''`) and each `ModuleDef`'s
  (`module_def_id` = its `@Id`). `kind` is the XML local name verbatim: no enum,
  no `'Unknown'` bucket. Modelled attributes go to columns, everything else to
  `extra` *and* to the existing `UnknownCollector`. `@test` is stored verbatim by
  the parser and only interpreted by the evaluator. `module_def_id` is `NOT NULL`
  with `''` as the sentinel because SQLite permits NULLs in a non-`INTEGER
  PRIMARY KEY` and treats them as distinct, which would have voided the
  uniqueness constraint for the common case.

  **Backfill.** `migrate_v2_to_v3` creates the table and then re-reads every
  stored `source_file` blob through the same parser. This is the first
  Rust-bearing migration in the crate and the reason ADR-0011's blob store
  exists: ingest is content-hash idempotent, so "populate on next install" would
  have been a permanent no-op for every already-installed file. Each blob parses
  inside its own `SAVEPOINT`, rolled back on error before the diagnostic is
  recorded, so a failing blob leaves **zero** rows rather than half a tree. One
  bad blob never aborts the migration.

  **Evaluator.** `dynamic::evaluate` is a pure function of tree plus value map,
  returning active `ParameterRef`s and `ComObjectRef`s in document order
  (deduplicated by first occurrence) plus diagnostics. All six `Condition_t`
  operators are implemented although the corpus only ever shows `>`. Value
  resolution: supplied value, then `parameter_ref.value`, then `parameter.value`,
  then `MissingValue`. No branch matching means nothing activates and it is
  always reported — `NoBranchMatched`, with `MissingValue`/`NonNumericValue` split
  out for the two causes. `TypeNone`-controlled `choose` has its own path
  (`UnexpectedTypeNoneShape` for any shape other than a sole
  `when default="true"`). Unrecognized kinds are opaque, not descended,
  `UnrecognizedNode`. `Module` is recognized but not expanded:
  `ModuleNotExpanded`.

  **Evidence.** `cargo test --workspace` 809 passed / 0 failed / 3 ignored on the
  merged result (784 before this cycle). All five gates green on `main` including
  `cargo deny check`. Corpus counts reproduced independently by three separate
  reviewers and matching `docs/RESEARCH.md` §4.3 exactly: choose/when 1646/2252,
  5/5, 509/982, 0/0; zero dangling `choose/@ParamRefId`; zero `UnparsableTest`,
  `UnresolvedParamRef` and `UnexpectedTypeNoneShape` across all four archives.

  **Documentation.** `IMPLEMENTATION_STATUS.md` (dated entry), `KNOWN_LIMITATIONS.md`,
  `COMPATIBILITY.md`, `DATA_MODEL.md`, `GAP_ANALYSIS_ETS.md`, `ROADMAP.md`,
  `ARCHITECTURE.md` and `IMPORT_EXPORT.md` all reconciled. The stale
  "grammar is unresearched" comments in `knx-core/src/parameter.rs`,
  `knx-core/src/module.rs` and `knx-productdb/src/parse/program.rs` are gone.
  `RESEARCH.md` §4.3 gained one verified corpus observation: all 62
  `SPACE_LIST_OF_INTEGERS` `@test` values, like all 13 `OP_NUMBER` values, occur
  in `prod3` (`MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod`) alone.

  **No limitation was downgraded.** "Parameter values are retained but not
  interpreted, there is no parameter editor" still stands, verified explicitly by
  the final review. An evaluator with no user interface above it is not an editor.

- **Pending/Next Steps:**
  T18 slices 2 and 3 remain: `Module` expansion (following a `Module` node into
  its `ModuleDef`'s own stored tree, which slice 1 deliberately stops at) and the
  parameter editor itself. Neither is started.

  Remaining backlog beyond T18: T25/T26 (i18n, decision D10), T27 (motion toggle,
  D11 — the user's memo asks for two motion styles, Apple-subtle and
  cyberpunk-glitch, both clean and sleek), T28 (in-app help, D12), T20 (Functions,
  needs an ADR first), T21 (graphical views, D1/D2), T16 (catalog browser),
  T15/T17 (bus-facing UI), T19 (KNX Secure — blocked on key material and
  hardware), T22 (multi-user — needs a design decision).

  Four items still need the user's explicit out-of-scope acceptance before
  `goal.md`'s completion condition can be met at all: `.vd2` support, encrypted
  `.knxprod` (untestable without a sample), T19's deferral, and the permanent
  exclusion of commissioning (E1).

- **Notes for Codex:**
  Keep the three confidence levels of `docs/RESEARCH.md` §4.3 apart. `[D]` is what
  the KNX Standard states — that is the `@test` value grammar (`Condition_t`,
  `Project Schema23 v01.00.00.md` §1.1.3.18) and nothing else on this branch.
  `[V]` is corpus observation, which covers the whole structural grammar around
  it. `[A]` is inference, which covers the no-match policy and
  `@default`-is-the-fallback. Do not promote one to another in code or docs.

  The evaluator is reachable from nowhere: `grep -rn "dynamic::" crates/ --include=*.rs`
  outside `knx-productdb` returns zero hits, and that is deliberate. Import still
  reads `GroupObjectTree` per ADR-0014 and must not start evaluating this tree.

  Ingest now makes a second `quick-xml` pass over every `ApplicationProgram`'s
  bytes. The cost is acknowledged in the code but was never measured; if product
  installation ever feels slow, that is the first place to look.

  A fresh v3 install and a v2->v3 backfill produce identical trees by
  construction — both call `parse_dynamic_trees` — but only a nine-row fixture
  test proves it, not a full-corpus comparison. Worth strengthening if the
  migration path ever gets more complicated.

  `program_should_be_skipped` in `dynamic/parse.rs` deliberately does *not* key on
  `application_program.source_sha256` matching. That looks like an oversight and
  is not: keying on it would have made the backfill a silent no-op for every
  ordinary already-ingested program.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-09-11 07:55
- **Completed:** R4 research cycle — `Module`/`ModuleDef` expansion semantics, documented before any design is written. Merged to `main` as `4f278e4` (merge `--no-ff` of branch `r4-module-semantics`, two commits `b3070db` and `ff24b79`; 6 files, 404 insertions, 19 deletions). Pushed `af2422e..4f278e4`; the branch is also pushed as a remote record. No production logic changed — documentation plus two doc comments.
  - `docs/RESEARCH.md` gains **§4.4**, structured exactly like §4.3 and using the same `[D]`/`[V]`/`[A]` confidence markers. It answers all eight questions the spike was given:
    - **Q1** A `Module` names its `ModuleDef` through `@RefId`, carrying the *full* id (e.g. `RefId="M-0083_A-0317-31-7DC6_MD-1"`), not a short id needing recovery through an owning element. No cross-application-program reference was observed.
    - **Q2** A `ModuleDef` repeats the `ApplicationProgram` shape one level down: its own `Static` (`Parameters`, `ParameterRefs`, `ComObjects`, `ComObjectRefs`), its own `Dynamic`, and an `Arguments/Argument` declaration block.
    - **Q3 (central)** `choose` inside a `ModuleDef/Dynamic` branches on a `ParameterRef`, **never** on an `Argument` — 57/57 distinct `choose/@ParamRefId` in `M-0083_A-0317-31-7DC6_MD-1` match a `ParameterRef`, 0/57 match an `Argument`, and the same split reproduces in the archive's two other application programs. Argument values instead drive `Memory/@BaseOffset` placement and text-template substitution. **Consequence: T18 slice 1's zero-dangling-`choose/@ParamRefId` corpus assertion needs no change when module expansion ships.**
    - **Q4** Repetition at the application-program level is N sibling `Module` elements; there is no repeat-count attribute on `ModuleDef` or `Module`. `ModuleInstance/@RepeatIndex` is a *project*-side `"NxM"` string (real observed values include `"6x1"`, `"37x2"`), corroborated by `Project Schema23 v01.00.00.md` §1.2.5.18.
    - **Q5** Instance ids are the `ModuleDef`-local id prefixed with the instance path (`MD-1_M-2_MI-1_O-2-1_R-2`).
    - **Q6** No nesting found in this corpus; no `SubModuleDef` element exists.
    - **Q7** Distribution per archive and project is tabulated.
    - **Q8** What slice 2 would force to change in the evaluator, split into additions versus behaviour changes.
  - Reconciled in the same change: `docs/GAP_ANALYSIS_ETS.md`, `docs/KNOWN_LIMITATIONS.md` (§3 and §12), `docs/DATA_MODEL.md`, and the doc comments in `crates/knx-core/src/module.rs` and `crates/knx-productdb/src/dynamic/evaluate.rs`.
  - The spike was reviewed, returned **Approved** with two Minor factual-count findings, and both were fixed in `ff24b79` after independently re-deriving the numbers rather than trusting either the author or the reviewer: 57 (not 58) distinct `choose/@ParamRefId`, and 2 (not 4) distinct `ModuleDef`+`Module`+`ModuleInstance` triples in the KV corroboration. Neither changed a conclusion.
  - Gates on merged `main`: `cargo fmt --all --check` clean, `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo test --workspace` **809 passed / 0 failed / 3 ignored**, `cargo run -p xtask -- check-layering` ok, `cargo deny check` → advisories ok, bans ok, licenses ok, sources ok.
- **Pending/Next Steps:** Design and implement **T18 slice 2 — `Module` expansion** in `knx-productdb`. The design is now evidence-backed and must explicitly settle two things §4.4 raises:
  1. **Per-instantiation activation qualification.** Slice 1 dedups activations in a flat `HashSet<String>` keyed on the raw `ref_id`. A `ModuleDef`'s local `ParameterRef`/`ComObjectRef` ids are declared **once** and reused verbatim by every sibling `Module`: in `prod3`, `M-0083_A-0317-31-7DC6_MD-1` declares 41 `ComObjectRef` ids and 12 sibling `Module` elements reference it. Walking the `ModuleDef` once per `Module` with slice 1's dedup would collapse twelve distinct instantiations into one. Activations from a module's subtree must be qualified by the instantiating `Module`'s id before dedup. This is a **behaviour change**, not an addition.
  2. **An explicit nesting policy.** No nesting exists in this corpus, so "recurse until it terminates" is not defensible. Either support exactly one level and reject deeper, or reject nesting outright with a diagnostic — decide it in the design, do not leave it to the implementation.
  Not in scope for slice 2 and not yet researched enough for it: structured argument *values* (there is no `argument` or `module_def` table anywhere in `migration.rs` today) and the `AllocatorRef` mechanism.
- **Notes for Codex:** §4.4's confidence markers are load-bearing — the only `[D]` claims in it come from `Project Schema23 v01.00.00.md`, and everything about the `Dynamic`/`Module` *structural* grammar is `[V]` corpus observation from four `.knxprod` archives plus the KV demo project. Do not promote a `[V]` to a `[D]` when quoting §4.4 in a design. Two traps cost real time in this cycle and are worth knowing: (a) the normative body of a Standard section frequently lives only in the `.json` twin under `#/tables/<n>`, not in the `.md` rendering, so grep both; (b) counting corpus facts by slicing raw XML text between `<ModuleDef ` markers silently matches across all three application programs in an archive — parse the element tree and scope to one `ApplicationProgram` instead. Both §4.4 count corrections came from re-deriving with a proper scoped parse. `OriginalData/` remains strictly read-only; scratch databases belong under the job tmp directory, never a shared default. Module expansion is still **not implemented**: a `Module` node evaluates to a `ModuleNotExpanded` diagnostic, the limitation in `KNOWN_LIMITATIONS.md` §3 stands undowngraded, and import continues to read `GroupObjectTree` per ADR-0014 without ever evaluating this tree.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-09-11 12:40
- **Completed:** **T18 slice 2 — `Module` expansion in the `Dynamic` evaluator.** Merged to `main` as `f683c4e` (merge `--no-ff` of `t18-module-expansion`, five commits `b9a9d30`, `d140923`, `22d1099`, `fb7a8a2`, `241d08d`; 14 files, 1713 insertions, 202 deletions). Pushed `a4c0b76..f683c4e`; the branch is also pushed as a remote record; worktree and local branch removed. Full architecture log: `.ai/logs/2026-09-11_claude_t18_slice2_module_expansion.md`.
  - The evaluator now follows a `Module` node into the referenced `ModuleDef`'s own stored tree. `evaluate`'s input grew from a single `DynamicTree` to a `ProgramTrees` (program tree plus every `ModuleDef` tree, loaded by the new `load_program_trees`); `evaluate` itself stays pure — no `Connection`, no I/O, no logging (D13).
  - Every activation and diagnostic is qualified by an `Option<ModuleScope>` (`module_node`, `module_id`, `module_def_id`), and the dedup key became `(Option<module_node>, ref_id)` (D14, D18). Two independent reasons this is mandatory: a `ModuleDef`'s ref ids are declared once and reused verbatim by every sibling `Module`, **and** `dynamic_node.node_id` restarts at zero in every stored tree, so node ids collide across trees. With slice 1's flat key, `prod3`'s three programs evaluate to 52/48/44 activations instead of 382/258/134 — measured during review.
  - Nesting is rejected by policy, not followed (D15): a `Module` inside a module's own expanded tree yields `NestedModuleNotExpanded`. `Diagnostic::ModuleNotExpanded` is gone, replaced by `ModuleDefNotFound` and `NestedModuleNotExpanded` (D17).
  - **No schema change** — the product database stays at v3. The `element_id` column already held `Module/@Id`; slice 1 simply never loaded it. The import path is untouched (ADR-0014).
  - Corpus regressions now enforce: zero `ModuleDefNotFound`, zero `NestedModuleNotExpanded` across the four installed `.knxprod` archives; `prod3`'s three module-bearing programs grow 22/18/14 → 382/258/134 activations across 12/8/4 distinct `ModuleScope`s; a module-free control program pinned at 145, unchanged. Every expected number was derived from the raw XML by a separate implementation before any assertion was written.
  - Design `docs/superpowers/specs/2026-09-11-module-expansion-design.md` (D12-D19, ten acceptance criteria) and plan `docs/superpowers/plans/2026-09-11-module-expansion.md`. Docs reconciled in the same slice: `IMPLEMENTATION_STATUS.md`, `KNOWN_LIMITATIONS.md` (§3 and §12), `GAP_ANALYSIS_ETS.md`, `DATA_MODEL.md`, `ROADMAP.md`, `COMPATIBILITY.md`, `ARCHITECTURE.md`, `RESEARCH.md` §4.4's slice-2 notes, plus the doc comment in `crates/knx-core/src/module.rs`.
  - Gates on merged `main`: `cargo fmt --all --check` clean, `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo test --workspace` **817 passed / 0 failed / 3 ignored** (up from 809), `cargo run -p xtask -- check-layering` ok, `cargo deny check` → advisories ok, bans ok, licenses ok, sources ok.
- **Pending/Next Steps:** **T18 slice 3 — the parameter editor** is now the only remaining T18 slice, and it is where the evaluator finally gets a consumer. It needs a design first: nothing outside `knx-productdb`'s own tests calls `evaluate` today, so slice 3 has to decide where evaluation lives in the layering (`knx-productdb` is below the application layer), how a `ValueMap` is assembled from a project's stored `ParameterInstance` strings, and what a `ModuleScope`-qualified activation looks like in a UI that has to show N channels. Also still open, in rough priority order: T25/T26 (i18n, D10), T27 (motion toggle, D11 — the two-animation-styles memo belongs here: an Apple-subtle style and a cyberpunk-glitch style, both clean and sleek), T28 (in-app help, D12), T20 (Functions — needs an ADR), T21 (graphical views, D1/D2), T16 (catalog browser), T15/T17 (bus-facing UI), T19 (KNX Secure — blocked on key material and hardware), T22 (multi-user — needs a design decision).
- **Notes for Codex:** Three things from this slice that would cost time to rediscover. (a) **`prod3`'s 44/28/14 structural `Module` rows are not the same number as the 12/8/4 the evaluator actually reaches** under the corpus's own default parameter values — the `Module`s naming `MD-2`/`MD-3`/`MD-4` sit on `choose` branches the defaults never select. Verified independently twice. The reachable count is the correct invariant; the plan document's own phrasing asserted the structural one and was wrong. (b) **The corpus evidence covers `prod3` only.** §4.4 Q7 lists seven module-bearing programs, but four live in the `kv25` demo `.knxproj` under `OriginalData/DemoProjects/`, and the corpus tests install `.knxprod` archives from `OriginalData/ProductDatabases/`. Any claim beyond `prod3`'s three programs needs new evidence first. (c) A grep pattern taken from prose misses the same prose once a code element is wrapped in backticks: `module expansion` does not match `` `Module` expansion ``, and three present-tense false claims survived the documentation pass because of it. Use a backtick-tolerant pattern when auditing. Two smaller notes for later, neither worth a change on its own: `crates/knx-productdb/src/dynamic/parse.rs:11` says "the evaluator this module does not yet have", which reads as a module-boundary statement but scans as a temporal one to a first-time reader; and the corpus tests' `db()` helper uses `tempfile::tempdir()`, which lands in `/tmp` unless `TMPDIR` says otherwise. The standing rules are unchanged: `OriginalData/` is strictly read-only, scratch databases belong under the job tmp directory, `KNOWN_LIMITATIONS.md` §3's parameter-editor limitation stands undowngraded, and nothing here claims ETS behavioural parity or KNX certification.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-09-11 13:13
- **Completed:** **T29 — a DPT codec in `knx-core`, wired into the CLI.** Merged to `main` as `db28104` (merge `--no-ff` of `t29-dpt-codec`, thirteen commits `49ab3d2`..`f3b83d4`; 19 files, 4323 insertions, 48 deletions). Pushed. Full architecture log: `.ai/logs/2026-09-11_claude_t29_dpt_codec.md`.
  - **New:** `crates/knx-core/src/dpt/codec.rs` (encode/decode) and `crates/knx-core/src/dpt/resolve.rs` (which DPT a group address carries). `knx-core` still does no I/O, parses no XML and knows nothing about the UI.
  - **Main types covered:** 1, 2, 3, 5, 6 (except `6.020 DPT_Status_Mode3`), 7, 8, 9, 12, 13, 14, 16, 17, 18. Everything else in the Standard is unimplemented and raises `UnsupportedDpt`. `KNOWN_LIMITATIONS.md` §61 is the full accounting.
  - **Layering change (ADR-0016):** `GroupValue` moved down from `knx-net` into `knx-core` and is re-exported from `knx-net` at its old path; no call site changed. A codec in `knx-core` cannot depend on the transport crate. The move surfaced a latent cEMI bug: a `GroupValue::Short(v)` above six bits overwrote the two APCI bits sharing its octet, silently changing the application service. Fixed to promote out-of-range `Short` to one-octet `Bytes`, with a regression test.
  - **Resolution is inference, not lookup.** `resolve_group_address_dpt` collects what the linked communication objects state; `GroupAddressDpt` is `None`, `Single` or `Conflict(Vec<DptRef>)`, and a conflict is **reported, never resolved**. `GroupAddress/@DatapointType` exists at schema ≥ 21 and is preserved on import but is not modelled — measured cost (`docs/RESEARCH.md` §6.1): 194/514 group addresses (38 %) resolve to no DPT, 110/514 (21 %) have no linked communication object at all.
  - **CLI:** `knx bus monitor --project <p>` now decodes values and `knx bus write` encodes them. Without `--project`, and for any uncovered main type, the output is byte-identical to before — verified by tracing every `--project`-absent path, and validation demonstrably precedes connection (checked by live-running the binary without `--dry-run` against `127.0.0.1:3671`).
  - **Three rulings the Standard does not settle**, each documented at the code and in §61: (a) main type 9's `M=2047,E=15` is `0x7FFF`, the reserved invalid-data code, so encode rejects it and the usable maximum is 670 433,28 — exactly the bound DPT-AS prints; AN188 §4's larger 670 760,96 ignores the collision and AN188 §5 reprints the smaller figure; (b) `8.010`'s practical maximum is therefore 327.66 %, not the printed 327.67 %; (c) scene numbers are carried at wire value — the +1 display recommendation is DPT-AS §3.19 NOTE 9 for `18.001` and §3.25 NOTE 16 for `26.001`, with **no such note for `17.001`**, so the offset belongs to the UI that displays it.
  - Design `docs/superpowers/specs/2026-09-11-dpt-codec-design.md` and plan `docs/superpowers/plans/2026-09-11-dpt-codec.md`. Docs reconciled in the same slice: `IMPLEMENTATION_STATUS.md`, `KNOWN_LIMITATIONS.md` (new §61), `GAP_ANALYSIS_ETS.md` (D5 and E4), `DATA_MODEL.md`, `ROADMAP.md`, `COMPATIBILITY.md`, `ARCHITECTURE.md`, `RESEARCH.md` §6.1, plus `docs/adr/0016-dpt-codec-in-knx-core.md`.
  - Gates on merged `main`: `cargo fmt --all --check` clean, `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo test --workspace` **920 passed / 0 failed / 3 ignored** (up from 817), `cargo run -p xtask -- check-layering` ok, `cargo deny check` → advisories ok, bans ok, licenses ok, sources ok.
- **Pending/Next Steps:** **T15 (Group Monitor GUI)** is now the natural next step — it is the consumer this codec was built for, and `GAP_ANALYSIS_ETS.md` D5 still records that the bus features have no front end at all. Also still open, in rough priority order: T18 slice 3 (the parameter editor — still needs a design; nothing outside `knx-productdb`'s own tests calls `evaluate` today), T25/T26 (i18n, D10), T27 (motion toggle, D11 — the two-animation-styles memo belongs here: an Apple-subtle style and a cyberpunk-glitch style, both clean and sleek), T28 (in-app help, D12), T21 (graphical views, D1/D2), T16 (catalog browser), T17 (bus-facing UI), T22 (multi-user — needs a design decision). Blocked, not deferred: **T20 (Functions)** — zero `<Function>` elements exist in any demo project, so there is no sample data to build against; **T19 (KNX Secure)** — needs key material and hardware; **A4** (schemas 12-22) — needs samples.
- **Notes for Codex:** Four things from this cycle worth not rediscovering. (a) **The KNX Standard v3.0.0 Markdown extraction has holes.** §3.19 NOTE 9 — the `18.001` scene-number display recommendation — is missing from the Markdown entirely but exists on page 49 of `03_07_02 Datapoint Types v02.02.01 AS.pdf`; §3.14.3 (`13.100 DPT_LongDeltaTimeSec`) has its body missing. Grepping the Markdown for either citation finds nothing; that is the extraction's gap, not an invented reference. (b) **Main type 9 is sign bit + 4-bit exponent + a 12-bit two's-complement mantissa whose MSB is the sign bit** — not sign-plus-magnitude. `FloatValue = 0.01 × M × 2^E`, `M ∈ [-2048, 2047]`, `E ∈ [0, 15]`. This was settled against the Standard and should not be relitigated. (c) **DPT 16 has exactly two subtypes** (`16.000` ASCII, `16.001` ISO 8859-1); `char_set_is_ascii` refusing anything else refuses a type the Standard does not define — it is not a third excluded subtype, and §61 does not list it as one. If the Standard ever defines `16.002`, §61 gains a line. (d) **No decoded value has been checked against real hardware** — every test here is against the Standard's own stated encodings, which is a weaker claim than interoperability. The standing rules are unchanged: `OriginalData/` is strictly read-only, scratch databases belong under the job tmp directory, and nothing here claims ETS behavioural parity or KNX certification.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-09-11 19:05
- **Completed:** **T15 — the Group Monitor GUI, closing gap D5 and the display half of E4.** Merged to `main` as `ee4b27d` (merge `--no-ff` of `t15-group-monitor`, sixteen commits `e3b7208`..`bf435e3` from `b88b286`; 28 files, 6947 insertions, 49 deletions). Full architecture log: `.ai/logs/2026-09-11_claude_t15_group_monitor.md`.
  - **The seam (ADR-0017).** `apps/knx-server` now depends on `knx-net` directly; no new crate was interposed. Two traits defined *in the consumer* — `GatewayConnector` and `BusTunnel` (`apps/knx-server/src/bus.rs:52` and `:64`) — because `knx_net::BusConnection` is deliberately not `dyn`-safe and widening it would have made the transport crate pay for an application crate's test strategy. Consequence: **every test in this branch runs with no KNX hardware attached.** `FakeConnector`/`FakeTunnel` are scripted with an exact outcome list, so an unexpected second `connect_tunnel` panics the fake. `crates/knx-net/tests/live_gateway.rs` is byte-for-byte unchanged.
  - **The session.** `AppState` gains `bus_session`: a bounded buffer (`MAX_TELEGRAMS = 5000`), a drain task, and `dropped_before` — a monotonic count of every telegram that existed and cannot be shown. **The branch's single load-bearing promise is that no telegram is lost silently.** There are exactly two loss paths, buffer eviction and `broadcast::error::RecvError::Lagged(n)`, and both increment that counter; the client renders a gap notice from it on every poll. The final review was asked to find a third path and traced the code end to end: there is none.
  - **Four endpoints** (`apps/knx-server/src/bus_routes.rs:45-48`): `POST /api/bus/monitor/start`, `POST /api/bus/monitor/stop`, `GET /api/bus/monitor/telegrams`, `POST /api/bus/write`.
  - **Front end.** `apps/knx-web/src/BusMonitorPanel.tsx` (table, client-side filter, connect/stop) and `BusComposeForm.tsx` (a sibling, not a child — it survives polls and resets only on a new row click via a `key`-remount). Click a row, edit it, send it back to the bus it came from.
  - `bus_session` is a `tokio::sync::Mutex`, not a `std` one — `std::sync::MutexGuard` is not `Send` and the guard is held across `connect_tunnel().await`. The cost is that `/telegrams` and `/stop` can block behind a connecting `/start`, bounded by the 10-second timeout in `knx_net::TunnelClient::connect` (`crates/knx-net/src/client.rs`). Recorded, not hidden.
  - **Nothing downgraded.** `KNOWN_LIMITATIONS.md` §61 (the T29 DPT codec's coverage accounting) is byte-for-byte untouched — verified three times, and independently by the final reviewer. A decode that fails is rendered with its stated reason; the codec was not widened to flatter the GUI. New **§62** records what this slice is not, in thirteen items: tunnelling only, one session at a time process-wide, a client-side filter that is nothing like ETS's, group telegrams only, and not one byte of it has ever been near a physical KNX installation.
  - `apps/knx-cli` still has the `ThreeLevel`-hardcoding bug that was fixed server-side in `b540264`. Fixing it here would have mixed unrelated changes (CLAUDE.md); it is recorded in the docs instead.
  - **Riding along, deliberately: the user's scope rulings of 2026-09-11.** `.vd2` out permanently; encrypted `.knxprod` out (untestable without a sample); T19 KNX Secure deferred until hardware exists, documented as a limitation meanwhile; T20 Functions deferred until the new KNX specification documentation is ready; and **E1 commissioning is not excluded at all** — it must work, and it waits on the KNX specification database. Six documents had been calling commissioning permanently out of scope; all six now say *blocked*, with the ruling quoted and dated, plus a new backlog task **T30** (Tier 5). E1's gap row stays open rather than being closed as a non-goal.
  - **Two reviews found two things, both the same failure mode** — a claim nobody made explicitly that a reader would take as true. (a) `GAP_ANALYSIS_ETS.md`'s E4 row said the GUI decodes "the way ETS's Group Monitor does", contradicting §61's own record that the codec deliberately differs from a published AN188 figure by one step; the phrase predated the branch, but the branch extended it to a new subject, which is authorship, not inheritance — fixed in `a4b90d0`. (b) Three sentences in `docs/RESEARCH.md` still called commissioning permanently out of scope after the other six sites were fixed — caused by my own brief naming five files while also saying "grep everywhere" — fixed in `696c6b3`. The final whole-branch review returned **APPROVED, zero blocking findings**, and deliberately searched for a third instance: none.
  - Its two non-blocking findings were closed anyway in `bf435e3`: `http_bus_monitor.rs`'s lag test now asserts `droppedBefore == 8` exactly instead of `> 0` (determinism established, not assumed, and derived in a comment in the test), and both bad-request tests in `http_bus_write.rs` now assert `handle.sent_calls().len() == 0` — "returns 400" and "returns 400 without touching the tunnel" are different claims.
  - Docs reconciled in the same slice: `IMPLEMENTATION_STATUS.md`, `KNOWN_LIMITATIONS.md` (§7, new §62), `GAP_ANALYSIS_ETS.md` (D5, E4, E1, T30), `ROADMAP.md`, `COMPATIBILITY.md`, `ARCHITECTURE.md`, `RESEARCH.md` §8.3, plus `docs/adr/0017-knx-server-depends-on-knx-net.md`.
  - Gates on merged `main`: `cargo fmt --all --check` clean, `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo test --workspace` **951 passed / 0 failed / 3 ignored** (up from 920 at `b88b286`), `cargo test -p knx-server` **147 / 0 / 0**, `cargo run -p xtask -- check-layering` ok, `cargo deny check` → advisories ok, bans ok, licenses ok, sources ok, `npm run test` **179 passed across 17 files**.
- **Pending/Next Steps:** **T27's first job is now a retrofit, not a constraint** — T15's telegram table shipped without ever being bound to a motion preference, so the motion toggle (D11) has to go back and cover it. The two-animation-styles memo belongs there: an Apple-subtle style and a cyberpunk-glitch style, both clean and sleek. Otherwise, in rough priority order: **T18 slice 3** (the parameter editor — still needs a design; nothing outside `knx-productdb`'s own tests calls `evaluate` today), **T17** (line scan, D6/E2 — the other bus-facing UI, and the natural sequel to this slice), T21 (graphical views, D1/D2), T25/T26 (i18n, D10), T28 (in-app help, D12), T16 (catalog browser), T22 (multi-user — needs a design decision), D8 (settings dialog), D9 (duplicate overlay implementations and accessibility), E5 (Docker `--network host`), E6 (custom routing multicast). Blocked with named conditions rather than deferred vaguely: **T30** (commissioning — the KNX specification database), **T19** (KNX Secure — key material and hardware), **T20** (Functions — the new KNX specification documentation), **A4** (schemas 12-22 — needs samples).
- **Notes for Codex:** Five things from this cycle worth not rediscovering. (a) **The drop counter is the contract.** Anything that touches `bus.rs`'s buffer or drain task has to keep every loss path incrementing `dropped_before`; a cap added anywhere else — the browser was the tempting place — creates a second accounting mechanism that can disagree with the server's, which is exactly why client-side row capping was rejected and documented as a limitation instead. (b) **`DptRef`'s text form is `DPST-{main}-{sub}` or `DPT-{main}`, nothing else.** `DptRef::parse` does not accept dotted `1.001`. That is why the wire carries `DPST-1-1` and why the compose form can round-trip a row's DPT straight back into `POST /api/bus/write`. (c) **`service` on `BusTelegramRow` is a plain `string` in `api.ts` on purpose**, not a closed union: the synthetic `"SessionClosed"` marker row has to fit it, and a union would go stale silently as the protocol layer grows. (d) **`#[tokio::test]` defaults to the `current_thread` flavor**, `tower::ServiceExt::oneshot` spawns no task, and the drain task is therefore spawned but never polled before all sends land — which is what makes `Lagged(SENT - CAPACITY)` a single deterministic number at the HTTP layer rather than a scheduling artefact. The derivation is in a comment in `http_bus_monitor.rs`; do not delete it, and do not assume the same determinism holds if anyone adds a middleware layer or switches the test flavor. (e) The failure mode this branch produced twice is **equivalence by implication** — nobody claimed ETS parity outright; a table cell and three stale sentences simply let a reader conclude it. When auditing docs, read for what a KNX integrator would take away, not for the literal sentence. The standing rules are unchanged: `OriginalData/` is strictly read-only, scratch databases belong under the job tmp directory, and nothing here claims ETS behavioural parity, KNX certification or hardware verification.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-09-11 20:25
- **Completed:** **R5 — the commissioning research spike, against two new KNX Standard knowledge bases.** Merged to `main` as `173bef7` (merge `--no-ff` of `r5-commissioning-research`, four commits `59a11be`, `8b1ff33`, `c7a9a3e`, `4750241` from `55b2d1a`; 5 files, 558 insertions, 14 deletions). Pushed. Full log: `.ai/logs/2026-09-11_claude_r5_commissioning_research.md`. **No production code was touched and nothing was run against hardware.**
  - **What unblocked this.** The user's ruling of 2026-09-11 made commissioning wait on "bis knx specs db fertig ist". Two queryable SQLite knowledge bases now exist at `/mnt/daten-i/Sourcecode/knx-spec-kb/knowledge_base/`: `knx_spec_kb_programming.sqlite` (2207 facts, 27 programming-relevant PDFs, **the only one with a `figures` table** — 317 vision-model figure descriptions) and `knx_spec_kb_full179_clean.sqlite` (16536 facts, 177 distinct PDFs, **text only** — no figures, captions or footnotes; two of the 179 PDFs yielded nothing). `knx_spec_kb_full179_unfiltered_reference.sqlite` is for comparison only and must never be queried or cited. They are **complementary, not ranked**.
  - **The headline correction.** `RESEARCH.md` §8.3 had asserted since Session 0 that "the `Legacy*` option matrix and partial-download rules are undocumented publicly". Half of that was wrong: partial download is specified in `03_05_03 Configuration Procedures` §3.5.3, CRC-driven via `PID_MCB`/`PID_MCB_TABLE`, and "Differential Download" is a formal term in `03_01_02 Glossary`. The `Legacy*` matrix genuinely is absent — one unrelated hit in each base — and vendor `Baggage` DLLs are at zero hits in both. Blocker 2 was corrected to exactly that width. **Blockers 1, 3 and 4 are untouched**: a specification database cannot make hardware unbrickable, cannot supply a vendor's DLL, and cannot hand over key material.
  - **New `RESEARCH.md` §8.4** answers nine questions with quoted citations: individual-address programming (`NM_IndividualAddress_Read`/`_Write`, Management Procedures §2.2/§2.3), the Load State Machine (Resources §4.23, normative, one LSM per Interface Object), complete download (Configuration Procedures §3.5.2, fully ordered), partial download, the Application Program Interface Object's PID table (Resources Tables 89/90/91), `A_Memory_Write`/`_Read` (Application Layer §3.5.4/3.5.5 — 1-63 octets, Verify-Mode-dependent confirmation, silent refusal of protected memory), unload and Master Reset (Management Procedures §3.7.1.2, with the Erase Code table 01h-08h), and KNX Secure's effect — which is **structural only**: it wraps these APDUs in S-AL frames under the Tool Key, it does not replace them, and it does not lift T19's deferral.
  - **§11 gains risk row R10.** The research risk is partly closed; the implementation risk, the verification risk, the `Legacy*` matrix and the vendor-DLL mechanism are all explicitly still open.
  - **Nothing downgraded.** `KNOWN_LIMITATIONS.md` §7's "Limitation." and "Impact." lines are textually unchanged — it still says the application does not program devices. Only an "Updated, 2026-09-11" paragraph and a reworded "Lifted when" were added. `GAP_ANALYSIS_ETS.md` **E1 stays open**; T30 (Tier 5) records what the databases did and did not resolve. `COMPATIBILITY.md` and `ARCHITECTURE.md` carried the old blocker-2 framing and were corrected too. `ROADMAP.md` needed no change and got none.
  - **The review round earned its keep.** Two reviewers verified 25 citations against the databases between them. Three defects, all one family — prose drifting inside quotation marks: a fabricated bracketed continuation in Q3's CRC quote (the Standard says "MaC shall use differential download algorithm", not the invented gloss), a silently truncated clause in Q2, and a stated citation convention stricter than the document actually followed. All three fixed; the four `[D]` quotes with no database fact row now carry a `[D, corpus]` marker naming the extracted Markdown they come from, and the convention sentence describes what the document really does.
  - Gates on merged `main`: `cargo fmt --all --check` clean, `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo test --workspace` **951 passed / 0 failed / 3 ignored** (unchanged — a documentation change moves no counts), `cargo run -p xtask -- check-layering` ok, `cargo deny check` → advisories ok, bans ok, licenses ok, sources ok, `npm run test` **179 passed across 17 files**.
- **Pending/Next Steps:** **T30 (commissioning) is now researched, not implemented, and the distinction is the whole point.** Its remaining blockers are physical, not documentary: hardware that can be bricked by a wrong memory image, the undocumented `Legacy*` matrix, vendor `Baggage` DLLs, and KNX Secure key material. §8.3's own recommendation — treat programming as a separate, later, explicitly-flagged effort — still holds, and nothing in this spike changes the order of the backlog. Unchanged and in rough priority order: **T27** (motion toggle, D11 — first job is the retrofit of T15's telegram table; the two-animation-styles memo belongs there), **T18 slice 3** (the parameter editor — still needs a design), **T17** (line scan, D6/E2), T21 (graphical views, D1/D2), T25/T26 (i18n, D10), T28 (in-app help, D12), T16 (catalog browser), T22 (multi-user — needs a design decision), D8 (settings dialog), D9 (duplicate overlays and accessibility), E5 (Docker `--network host`), E6 (custom routing multicast). Blocked with named conditions: **T19** (KNX Secure — key material and hardware), **T20** (Functions — no sample data; the knowledge bases do not help, see below), **A4** (schemas 12-22 — needs samples).
- **Notes for Codex:** Five things from this cycle worth not rediscovering. (a) **Any "the specification does not document this" claim must name which base was searched.** The breadth difference is enormous: `"functional block"` returns 206 hits in the full base against 1 in the programming base. A gap measured only in the narrow base is not a gap in the Standard, and this spike's Q9 was rewritten mid-flight for exactly that reason. (b) **The knowledge bases do not unblock T20 (Functions).** `knxproj` and `"Building Part"` return one hit each in the full base, both from an unrelated ETS project-schema PDF; the Standard's `"functional block"` is an interworking-model term, not ETS's UI "Functions". T20 still waits on sample data, not on documentation. (c) **The extraction pipeline chunks rather than indexing every sentence**, so a sentence can be present and correct in the extracted Markdown while having no `evidenceText` row in either base. That is why `[D, corpus]` exists in §8.4; it withholds exactly one claim — "this sentence is indexed as a fact row" — and nothing else. (d) **rtk compresses shell output and silently drops words.** An `evidenceText` read back through a pipe is not verbatim, and comparing piped output against a document manufactures findings that are not real. Write query results to a file and open them with the Read tool whenever exact wording matters — this was written into both review briefs after it bit twice. (e) **Documented is not verified.** Every finding in §8.4 is marked `[D]` (the Standard states it, quoted) or `[V]`/`[D, corpus]`; not one line of it has been run against a device, and §8.4 opens and closes by saying so. The standing rules are unchanged: `OriginalData/` is strictly read-only, scratch files belong under the job tmp directory, and nothing here claims ETS behavioural parity, KNX certification or hardware verification.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-09-11 21:35
- **Completed:** **T18 slice 3, Task 1 of 5 — the `knx-productdb` parameter read side.** Worktree `.worktrees/t18-parameter-editor`, branch `t18-parameter-editor`, commit `a7ab571` (not yet merged — four tasks remain, then a whole-branch review). Design `docs/superpowers/specs/2026-09-11-parameter-editor-design.md`, decision D22.
  - `crates/knx-productdb/src/query.rs` gains `ParameterView` (`id`, `display_order: i64`, `tag`, `name`, `text`, `text_layer: ValueLayer`, `kind`, `access`, `min_inclusive`, `max_inclusive`, `enum_options: Vec<(String, Option<String>)>`), `parameter_views(conn, program_id) -> Result<Vec<ParameterView>, ProductDbError>` and `parameter_ref_ids(conn, program_id) -> Result<HashSet<String>, ProductDbError>`. One bulk query joins `parameter_ref` -> `parameter` -> `parameter_type`, program-scoped, ordered by `parameter_ref.display_order`, reusing `com_object_view`'s existing `pick()` for `text`/`text_layer` unmodified. A second query fetches `enum_options` (`parameter_type_enum`), run only when `kind == "Restriction"`.
  - **Empirical finding, not assumed:** real corpus data omits `ParameterRef`'s `DisplayOrder` XML attribute entirely — all 543 rows for program `M-0083_A-0317-31-7DC6` on `prod3` (`MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod`) have a NULL `display_order` column, which panicked the first version of this code (`InvalidColumnType(1, "display_order", Null)`) against the non-optional `i64` design field. Fixed with `COALESCE(pr.display_order, 0)` in the `SELECT`; ordering itself is left on the raw, possibly-NULL column (`ORDER BY pr.display_order`) — no secondary sort key added, per the brief's explicit "note on ordering".
  - **The 208-vs-AP-level ambiguity, resolved per the coordinator's explicit ruling:** `parameter_ref` has no `module_def_id` column — keyed only `(program_id, id)` — so `parameter_views`/`parameter_ref_ids` cannot single out `MD-1`'s 208 rows (`RESEARCH.md` §4.4 Q3's figure). The new corpus test (`crates/knx-productdb/tests/parameter_views_corpus.rs`, `KNXBENCH_PRODUCT_CORPUS`-with-loud-`eprintln!`-skip idiom matching `dynamic_tree.rs`) instead derives and asserts the true AP-level count directly: `SELECT COUNT(*) FROM parameter_ref WHERE program_id = ?1` = **543**, cross-checked against both `parameter_views(..).len()` and `parameter_ref_ids(..).len()`, plus that every view's `id` is in the id set. 208 is recorded only as RESEARCH.md context in the module doc comment, never asserted.
  - New unit tests in `query.rs`'s own `mod tests` (matching its existing idiom, not a sibling file): three `parameter_ref`s of kind Number/Restriction/Text return three `ParameterView`s in `display_order` with `enum_options` non-empty only for the Restriction one; `parameter_ref_ids` returns exactly the declared id set; a NULL-text override falls back to `ValueLayer::Program` and a non-NULL one reports `ValueLayer::ProgramRef`, mirroring `com_object_view`'s own `pick()` tests rather than re-deriving the assertion shape.
  - Test-first was honored by reconstruction: stripped the implementation via `sed`, ran the real suite, captured genuine `cannot find function 'parameter_views'`/`'parameter_ref_ids'` compile errors, then restored the implementation from a scratch backup under the job tmp directory.
  - **No schema change** in either database — product DB stays v3, `knx-store`'s `parameter_instance` untouched. `evaluate`/`resolve_values` untouched. Design DTOs (`ParameterPanelDto` etc.) are explicitly out of scope for this task — they belong to `apps/knx-server` in Task 3.
  - Gates on this task's commit: `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo test --workspace` **955 passed / 0 failed / 3 ignored** (was 951/0/3 — +4: 3 new unit tests + 1 new corpus integration test); `cargo run -p xtask -- check-layering` ok; `cargo deny check` → advisories ok, bans ok, licenses ok, sources ok; `npm run test` (apps/knx-web, after `npm install`) unchanged at **179 passed across 17 files**.
  - Full task report: `/home/knxbench/.claude/jobs/8098e9e6/tmp/t18s3-task1-report.md`.
- **Pending/Next Steps:** Tasks 2-5 of the `t18-parameter-editor` plan remain: Task 2 (`knx-core`/`knx-store` — writing a value), Task 3 (`apps/knx-server` — assembly/routes/read model, consumes `ParameterView`/`parameter_views`/`parameter_ref_ids` directly), Task 4 (`apps/knx-web` — the parameter panel), Task 5 (reconcile the documentation set). This task (Task 1) is implemented and gated but **not yet reviewed** — review arrives from the coordinator per the plan's own constraint ("do not dispatch subagents"). Do not merge `t18-parameter-editor` before that review.
- **Notes for Codex:** (a) `ParameterView.display_order` is populated but must not drive any extra ordering logic beyond `parameter_ref.display_order` itself — a design constraint repeated here because it is easy to "fix" by adding a secondary sort key once you notice the NULLs, and that would be wrong. (b) If you touch `parameter_views`/`parameter_ref_ids` again, re-run `parameter_views_corpus.rs` with `KNXBENCH_PRODUCT_CORPUS` pointed at `OriginalData/ProductDatabases` (absolute path) — 543 is the ground truth for `prod3`'s `M-0083_A-0317-31-7DC6`, derived empirically, not designed. (c) Module-scoped fields stay read-only this slice (D25) — nothing in Task 1 adds a way to write one, and Task 2 is where that boundary actually gets tested. (d) The COALESCE-to-0 default for a NULL `display_order` is a real design tension worth a second look in review: it silently collapses "no declared order" into "first", which is defensible (it changes no currently-observed ordering, since no corpus row has ever exercised a non-zero-vs-NULL comparison) but is a genuine judgment call, not a spec-mandated one — flagged for the coordinator, not resolved unilaterally beyond this choice. The standing rules are unchanged: `OriginalData/` is strictly read-only, scratch files belong under the job tmp directory, and nothing here claims ETS behavioural parity, KNX certification or hardware verification.


---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-13 02:57
- **Completed:** UI redesign proposal only: `docs/design/2026-09-13-codex-ui-concept/README.md`, three generated concept PNGs (Porcelain editor, Graphite group addresses, Mint busmonitor), and exact prompts. Read current frontend implementation and architecture/limitations. No application functions changed, no tests added or run.
- **Pending/Next Steps:** User design decision; implementation is not authorized by this proposal. Claude's active implementation session on `main` continues independently; its pending work above remains unchanged.
- **Notes for Claude:** User explicitly clarified you are actively working on `main`. Codex stayed on `main`, touched no app/source files, made no commit, and appended this entry instead of replacing your handover. Images are AI-generated mockups with fictional data, not running-app screenshots. Multiwindow is proposed only; shared project/undo state and missing client synchronization in limitation §63 require a separate design. Full rationale and feature inventory in proposal.


---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-13 03:09
- **Completed:** Created root `codex-goal.md` as the user's requested instruction for a future `/goal` implementation run. Records explicit approval of all three UI concepts, feature preservation, themes/motion, mouse/keyboard, guarded diagnostic multiwindow scope, implementation stages, regression/visual gates, checkpoint/resume instructions, and preferred GPT-6 Astra model based on fetched official docs. No application changes, no commit, no Goal started.
- **Pending/Next Steps:** Execute the future Goal using `codex-goal.md`. The prior design approval pending status is superseded: user explicitly approved the concept. Claude's active main-session work continues independently.
- **Notes for Claude:** User requires main and preservation of your ongoing edits. Goal file makes that explicit and prohibits overwriting simultaneous changes. Automatic continuation after usage reset is requested but depends on runtime support; no scheduler was configured or guaranteed. Existing proposal remains a historical artifact; root Goal records the newer approval.


---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-13 06:15
- **Completed:** Started actual runtime Goal for codex-goal.md; inspected approved three visual references, current UI/API ownership, main and foreign work. Created coverage matrix and executable stages in `.ai/logs/2026-09-13_codex_ui_implementation.md`. No application changes or test claims yet.
- **Pending/Next Steps:** Appearance foundation tests and implementation, shell, T21/T16, full feature integration, conditional diagnostic window, real browser evidence and final gates.
- **Notes for Claude:** Codex owns web redesign + T21/T16 only on main. T17/backend and other backlog remain yours. Existing UI language/packs/module parameter support will be preserved. No commits or model switch performed. Goal active; resume from technical log.


---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-13 03:38
- **Completed:** Appearance foundation with 343 passing web tests and clean typecheck; initial three-pane shell and graphical T21 views with 3 focused tests. Existing App suite 8/8 after restoring Log toggle and Undo text. Real Chromium empty-shell inspection completed; not final screenshot proof.
- **Pending/Next Steps:** Device work area/keyboard/feature audit, T16 missing API mapping (question pending), companion investigation, populated browser proof and final gates. See `.ai/logs/2026-09-13_codex_ui_implementation.md` checkpoint 2.
- **Notes for Claude:** Frontend changes on main remain uncommitted. No backend changes. T16 requires existing-device product/program/hardware identity via server/generated projection; current DeviceDetail lacks it. Goal stays active. Initial timestamp 06:15 above was mistaken; subsequent timestamps use actual system clock.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-13 07:55
- **Completed:** Frontend workbench implemented on `main`: Porcelain/Graphite/System themes, accent/density persistence, three-pane resizable shell, keyboard file/tree navigation, central building/topology/group-address views, central device communication/parameter tabs, catalogue entry from topology, shared overlay file picker, and preserved existing commands/panels. Browser evidence: `/tmp/knx-ui-proof/{initial,building,topology,busmonitor}.png`; examples are simulated and clearly labeled. Docs updated in `IMPLEMENTATION_STATUS.md` and `GAP_ANALYSIS_ETS.md`. Proof: web 349 tests, typecheck, build, fmt, clippy, Rust 1180 tests, layering all green.
- **Pending/Next Steps:** Goal remains active and not complete. T16 existing-device product/hardware/program identity needs a server/generated `DeviceDetail` projection extension; current frontend cannot invent it. T21 spatial canvas/floor-plan authoring is outside current model and documented partial. Safe diagnostic second window remains open because Tauri capability is main-only and server state is shared (§63); do not create an unsynchronized editor. User/Claude must provide or authorize the projection boundary before T16 can close.
- **Notes for Claude:** All Codex changes are uncommitted and on `main`; do not overwrite. Claude's T17 backend work remains separate. Full technical details and exact gates in `.ai/logs/2026-09-13_codex_ui_implementation.md` checkpoint 3. No real bus traffic or native second-window claim was made.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-13 08:03
- **Completed:** Added keyboard-selectable telegram details, explicit bus-send/undo explanation, responsive monitor and fixed navigation scrolling. Durable browser screenshots/checks in docs/design/2026-09-13-codex-ui-proof cover 1920/1280/960 CSS px, themes/density/accent, keyboard flow, reduced motion and measured 120 ms hover. Focused tests 32 passed; TypeScript clean.
- **Pending/Next Steps:** Goal active; contextual building/device/address work and full CRUD/bulk/large-list proof remain. Previous frontend-completeness statement was too broad. T16 identity question unanswered. Diagnose safe additional-window approach against frozen session context. See checkpoint 4.
- **Notes for Claude:** No backend edits. GroupAddressContext freezes names/DPTs for decode AND writes; remount/poll cannot refresh. Main-only Tauri capability alone does not prove a browser portal impossible. Keep full scope, continue independent UI work. Shared docs have recent T17 changes; reread before patching.
- **Last Agent:** Codex
- **Timestamp:** 2026-09-16 09:22
- **Completed:** Read Claude's KNXBench project memories, the goal-run ledger, current Claude session tail, git branches/worktrees, and candidate task scopes. No runtime or product documentation changed. Confirmed Claude is rate-limited until 2026-09-19 and has three unfinished reserved branches: `d10-language-data` (dirty fix round), `t18-module-arguments` (clean commit, migration-version collision still to resolve), and `d10-server-prose` (clean commits, review pending). Identified code-disjoint Codex candidates: Task 17 real-browser new-project verification, the measured `knx-store::load_project` N+1 optimization, and Task 16 passive Group Monitor verification.
- **Pending/Next Steps:** User chooses whether Codex should take one or more proposed tasks. Keep Task 18 theme work, T28 help, product-database cleanup, and central server-domain changes untouched until Claude's three reserved branches are integrated.
- **Notes for Claude:** No Claude worktree or branch was modified. Chromium 140 and Playwright 1.63 are installed; the KNX gateway has a local route. If Codex takes Task 16, it will remain passive/read-only and will never address `1.1.220`. Candidate work must use a separate worktree and preserve Claude's branches.

---
- **Last Agent:** Codex
- **Timestamp:** 2026-09-16 09:38
- **Completed:** Traced the requested project-load progress and UI-version roadmap additions without changing application code or roadmap files. Confirmed ETS import and native `.knxdb` open currently await one HTTP response with no busy/progress state; `knx-web` already has the independent SemVer source `0.1.0-alpha.1`; browser title is static `KNXBench`; footer right currently displays `KNX-compatible`.
- **Pending/Next Steps:** Await user approval of the bounded documentation design, then add T37 (honest staged project-load progress for ETS import and native open) and T38 (web UI version in footer and browser title) to `docs/ROADMAP.md` and `goal.md`.
- **Notes for Claude:** No Claude branch/worktree was touched. Proposed T38 replaces only the footer's `KNX-compatible` label; the compatibility wording policy remains intact and the welcome-page positioning may remain. Version must come from `apps/knx-web/package.json`, not a duplicate literal.

---
- **Last Agent:** Codex
- **Timestamp:** 2026-09-16 09:41
- **Completed:** Added approved roadmap/backlog tasks T37 and T38 to `docs/ROADMAP.md` and `goal.md`. T37 specifies truthful staged progress for ETS `.knxproj` import and native `.knxdb` open, preserves the previous project until atomic success, prohibits fake elapsed-time percentages, and requires accessible/backend/frontend/real-project verification. T38 replaces the footer's `KNX-compatible` label with the independently versioned `knx-web` SemVer and puts the same manifest-derived version in the browser title without a duplicate literal. Updated three shifted `docs/ROADMAP.md:<line>` references in `goal.md`.
- **Pending/Next Steps:** T37 and T38 are planned only; neither application feature is implemented. Existing Claude-owned branches remain untouched.
- **Notes for Claude:** Verification: `git diff --check -- docs/ROADMAP.md goal.md` exited 0; exactly one T37 and one T38 heading exist in each file; ADR-0018 link target exists; corrected Roadmap lines 590, 692, and 743 resolve to the intended LLM, Session 7 status, and in-app help paragraphs. No tests/build run because only Markdown planning documents changed.

---

---
- **Last Agent:** Codex
- **Timestamp:** 2026-09-16 07:44:52 UTC
- **Completed:** Goal-Task 17 analysiert, ohne Implementierungsdateien zu ändern. Relevante Akzeptanzkriterien aus `docs/superpowers/plans/2026-09-13-goal-completion.md`, den echten Ablauf `NewProjectDialog -> api.newProject -> POST /api/project/new -> ProjectTree`, §80/§83 und die vorhandenen Vitest-Abdeckungen geprüft. Chromium 140 und Playwright 1.63 sind vorhanden; im Web-Paket ist Playwright noch keine lokale, reproduzierbare Entwicklungsabhängigkeit.
- **Pending/Next Steps:** Freigabe des kurzen bounded Designs abwarten. Danach separaten Worktree/Branch `launcher-browser-verify` anlegen, lokalen Playwright-Test samt Startkonfiguration implementieren, alle drei Gruppenadressstile und den Katalogzugang aus einer linienlosen Anlage im echten Browser prüfen, Dokumentation aktualisieren und die geforderten Web-Gates ausführen.
- **Notes for Claude:** Keine Quellcode-, Paket- oder Produktdokumentationsänderung. Bestehende Änderungen auf `main` und Claudes reservierte Worktrees bleiben unberührt. Die Freigabe ist die vom Skill `superpowers:brainstorming` verlangte Designschranke.

---
- **Last Agent:** Codex
- **Timestamp:** 2026-09-16 09:55 CEST
- **Completed:** Goal-Task 17 im isolierten Worktree `.worktrees/launcher-browser-verify` auf Branch `launcher-browser-verify` umgesetzt und als `3b497a02fa3fc6a24fac85cb7fff00e1243957dd` committed. Lokales Playwright 1.63, `npm run test:e2e`, Linux-Chromium-Konfiguration und `e2e/new-project.e2e.ts` hinzugefügt. Drei echte Browserfälle erzeugen `ThreeLevel`-, `TwoLevel`- und `Free`-Projekte über den realen Rust-Server, prüfen Dialogtext und Defaults, exakten POST-Body, linienlose Ein-Anlagen-Antwort, sichtbaren Stil und den aus „Nicht zugeordnet“ geöffneten Gerätekatalog. §80/§83, Implementierungsstatus und `goal.md` aktualisiert. Post-Commit-Nachweis: Vitest 468/468 in 42 Dateien; TypeScript/Vite-Build und Playwright 3/3; `git diff --check` sauber.
- **Pending/Next Steps:** Nur die Integrationsentscheidung bleibt: lokal nach `main` mergen, pushen/PR anlegen oder Branch/Worktree unverändert behalten. Wegen fremder uncommitted Änderungen auf `main` wurde nichts automatisch gemergt oder gepusht.
- **Notes for Claude:** Basis ist `main` `06db5ecfbe6356f54dc55e59e181a54049d25dfc`; der Task-Commit berührt nur `apps/knx-web`-Testtooling und Statusdokumente, keine Produktlogik oder Rust-Quelle. Browsergrenze endet beim geöffneten leeren Katalog; Produktpaketinstallation und Geräteerzeugung bleiben HTTP/Application-E2E in `http_catalog_to_device.rs`. Vollständiger Bericht: `.ai/logs/2026-09-16_codex_launcher_browser_e2e.md`.

---
- **Last Agent:** Codex
- **Timestamp:** 2026-09-16 09:59 CEST
- **Completed:** Auf Nutzerwunsch Goal-Task 17 mit explizitem `--no-ff`-Merge-Commit `2499127e271f877ab1d67b517db21cbb577f7f95` nach `main` integriert und nach `origin/main` gepusht. Vor dem Push auf dem gemergten Stand erneut verifiziert: Vitest 468/468 in 42 Dateien; TypeScript/Vite-Produktionsbuild und Playwright 3/3. Lokaler und Remote-`main` zeigen beide auf `2499127`. Die vorbestehenden lokalen Änderungen wurden vor dem Merge gestasht und danach konfliktfrei wiederhergestellt. Feature-Worktree entfernt und lokalen Branch `launcher-browser-verify` gelöscht.
- **Pending/Next Steps:** Goal-Task 17 ist vollständig integriert; keine Aufgabe daraus bleibt offen. Die wiederhergestellten, vorbestehenden lokalen Änderungen an `.ai/CURRENT_STATE.md`, `docs/ROADMAP.md`, `goal.md` sowie `docs/PROJECT_ANALYSIS_2026-09-15.md` bleiben bewusst uncommitted.
- **Notes for Claude:** Merge-Commit enthält nur Task-17-Commit `3b497a0` zusätzlich zur bisherigen `main`-Basis. Vollständiger Bericht mit Red/Green- und Browsernachweisen: `.ai/logs/2026-09-16_codex_launcher_browser_e2e.md`.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-16 13:39 CEST
- **Completed:** Lizenzstand geprüft, ohne Produktcode zu ändern. `Cargo.toml` und `README.md` nennen derzeit `AGPL-3.0-or-later`, es gibt aber keine `LICENSE`- oder `COPYING`-Datei; `docs/KNOWN_LIMITATIONS.md` behandelt die Angabe weiterhin als Platzhalter. Für das Ziel „gewerbliche Nutzung durch KNX-Fachleute erlauben, kommerziellen Weitervertrieb verhindern“ ist die Sustainable Use License 1.0 die passendste standardisierte Ausgangsbasis. PolyForm Noncommercial 1.0.0 wäre strenger und würde regelmäßig auch den Einsatz in bezahlten KNX-Projekten ausschließen. AGPL verhindert Verkauf oder kommerziellen Weitervertrieb nicht.
- **Pending/Next Steps:** Nutzerentscheidung zur erlaubten gewerblichen Eigennutzung und zu konkurrierendem Hosting abwarten. Anschließend Lizenztext, SPDX-Metadaten, README und bekannte Einschränkung gemeinsam aktualisieren; vor Veröffentlichung Drittcode und bereits erteilte Lizenzen abschließend prüfen.
- **Notes for Claude:** Keine Lizenzänderung beschlossen oder umgesetzt. `git shortlog` zeigt nur `github@knxbench.com` als bisherigen Commit-Autor, was eine Neulizenzierung erleichtert, aber keine vollständige Rechteprüfung ersetzt. Bereits unter AGPL veröffentlichte Versionen bleiben für vorhandene Empfänger unter AGPL nutzbar.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-16 13:51 CEST
- **Completed:** Kommerzielle Vermarktung und KNX-Mitgliedschaft anhand aktueller offizieller KNX-Unterlagen und des deutschen Interoperabilitätsrahmens eingeordnet, ohne Produkt- oder Lizenzcode zu ändern. Eine eigenständige KNX-kompatible Anwendung erfordert nach den öffentlich zugänglichen Regeln nicht automatisch eine KNX-Mitgliedschaft. Offizielle ETS-App-Entwicklung verlangt dagegen KNX-Mitgliedschaft mindestens als Interested Party; Herstellerkennung, registrierte/zertifizierte KNX-Produkte und KNX-Logo-Nutzung unterliegen Mitgliedschaft, IPR-/Markenvertrag und dem jeweiligen Registrierungs- oder Zertifizierungsverfahren. Das Markenrisiko des Produktnamens `KNXBench`, die Rechte an Produktdaten und `.knxproj`-Interoperabilität wurden als Themen für eine juristische Freigabe vor Vermarktung festgehalten.
- **Pending/Next Steps:** Nutzer muss Vertriebsmodell festlegen: proprietäre Endnutzerlizenz mit optional einsehbarem Quellcode oder source-available plus separate kommerzielle Lizenz. Vor Marktstart schriftliche Einordnung der KNX Association für das konkrete Standalone-Produkt, Markenprüfung von `KNXBench`, Drittcode-/Datenrechteprüfung, CLA für externe Beiträge und Prüfung der anwendbaren Vertriebs-, Datenschutz- und Cyber-Resilience-Pflichten einholen.
- **Notes for Claude:** Offizielle Quellen geprüft: KNX-Mitgliedschaftsseite (Gebühren und Kategorien), ETS-App-Entwickleranforderungen, Produktzertifizierungsvoraussetzungen, KNX-Spezifikationszugang sowie Logo-Regeln. Für kleine Unternehmen beträgt die veröffentlichte Interested-Party-Gebühr derzeit 1.000 EUR jährlich; diese Kategorie ermöglicht laut KNX-Roadmap keine Produktzertifizierung. Keine Aussage getroffen, dass eine eigenständige ETS-Alternative offiziell KNX-zertifizierbar sei.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-16 14:00 CEST
- **Completed:** Nutzerentscheidung präzisiert: KNXBench soll vom Nutzer nicht kommerziell verwertet werden; auch Dritte sollen es nicht verkaufen oder kommerziell weitervertreiben. Für den angenommenen Anwendungsfall, dass KNX-Fachbetriebe KNXBench intern in bezahlten Projekten einsetzen dürfen, bleibt Sustainable Use License 1.0 die passendste standardisierte Ausgangsbasis. Die frühere Annahme einer möglichen Eigenvermarktung wurde in der Tagesnotiz korrigiert.
- **Pending/Next Steps:** Vor einer tatsächlichen Lizenzumstellung noch festlegen, ob interne geschäftliche Nutzung ausdrücklich erlaubt bleiben soll. Danach kanonischen Lizenztext hinzufügen und `Cargo.toml`, `README.md` sowie `docs/KNOWN_LIMITATIONS.md` konsistent aktualisieren; Rechte- und Markenprüfung bleibt erforderlich.
- **Notes for Claude:** Noch keine Lizenzdateien oder SPDX-Metadaten geändert. AGPL bleibt bis zu einer ausdrücklichen Umsetzungsanweisung nur der dokumentierte Platzhalter.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-16 16:43 CEST
- **Completed:** Lizenzpräferenz erneut anhand der Nutzerklärung aktualisiert: Vorrang hat, dass jeder KNXBench kostenlos privat und beruflich nutzen kann; eine Einschränkung gegen Verkauf oder kommerziellen Weitervertrieb wird nicht mehr gewünscht. Dafür ist `AGPL-3.0-or-later` die empfohlene Lizenz und stimmt mit den bestehenden Angaben in `Cargo.toml` und `README.md` überein. Sustainable Use License und PolyForm Noncommercial sind damit verworfen.
- **Pending/Next Steps:** Wenn die Lizenz formal umgesetzt werden soll, kanonischen AGPL-3.0-Lizenztext als `LICENSE` hinzufügen und `docs/KNOWN_LIMITATIONS.md` aktualisieren. Vor Veröffentlichung Drittcode-, Produktdaten- und Markenrechte weiterhin prüfen.
- **Notes for Claude:** Noch keine formale Lizenzumsetzung vorgenommen. AGPL erlaubt kostenlose private und kommerzielle Nutzung, Änderungen und Weitergabe unter Copyleft-Bedingungen; sie verbietet Dritten nicht, für Distribution oder Dienstleistungen Geld zu verlangen.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-16 17:19 CEST
- **Completed:** Die vom Nutzer endgültig gewählte Projektlizenz `AGPL-3.0-or-later` vollständig umgesetzt. Den unveränderten kanonischen GNU-AGPLv3-Text als `LICENSE` hinzugefügt; README verlinkt und erklärt die Lizenz; `xtask` erbt nun wie alle übrigen Workspace-Pakete `license.workspace = true`. Bekannte Einschränkung 10 und Gap F3 als erledigt markiert, dabei den alten Anker erhalten. Roadmap, `goal.md`, Implementierungsstatus, Projektanalyse und VD4-Rechtsbewertung auf die getroffene Entscheidung aktualisiert. Tagesnotiz, `MEMORY.md` und `.ai/logs/2026-09-16_codex_agpl_license.md` aktualisiert.
- **Pending/Next Steps:** Für die Lizenzentscheidung und ihre Repository-Umsetzung bleibt nichts offen. Marken-, Produktdaten- und Drittcodeprüfungen bleiben eigenständige Veröffentlichungsthemen und werden durch die Lizenzwahl nicht ersetzt.
- **Notes for Claude:** Verifikation: `cargo deny check licenses` Exit 0 (`licenses ok`, nur bestehende Warnungen zu nicht angetroffenen Allowlist-Lizenzen); alle 16 Workspace-Pakete melden per `cargo metadata` `AGPL-3.0-or-later`; `LICENSE` entspricht mit 34.523 Bytes, 661 Zeilen und SHA-256 `0d96a4ff68ad6d4b6f1f30f713b18d5184912ba8dd389f86aa7710db079abcb0` dem offiziellen GNU-Text; 40 aktive Dokumente ohne veraltete Placeholder-/Undecided-Aussage; fünf Lizenzlinks auflösbar; `git diff --check` Exit 0. Vorbestehende uncommittete Änderungen in `.ai/CURRENT_STATE.md`, `docs/ROADMAP.md`, `goal.md`, `docs/PROJECT_ANALYSIS_2026-09-15.md` und `docs/VD4_PRODUCT_DATABASE_IMPORT.md` wurden nicht überschrieben; dort wurden nur lizenzbezogene Passagen ergänzt oder korrigiert.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-16 18:42 CEST
- **Completed:** Offene Aufgaben gegen den tatsächlichen Stand von `main` und Claudes drei reservierte Worktrees abgeglichen, ohne Produktcode zu ändern. Claude belegt weiterhin `d10-language-data` (inklusive uncommitteter Fixrunde), `d10-server-prose` und `t18-module-arguments`; deren gemeinsame Konfliktflächen sind insbesondere `knx-productdb`, `apps/knx-server/src/domain.rs`/`routes.rs`, Parameter-/Settings-UI sowie die gemeinsam geführten Statusdokumente. Als unabhängig umsetzbar bestätigt: gemessene `knx-store::load_project`-Bulk-Load-Optimierung, Docker-/Linux-Release-Artefakthärtung, bereichslose Gruppenadress-Exportintegrität und die fünf fehlenden `Space/@Type`-Varianten. Die ersten beiden sind am stärksten getrennt; die letzten beiden benötigen später eine sorgfältige Dokumentzusammenführung.
- **Pending/Next Steps:** Nutzer kann einen der unabhängigen Tasks auswählen. T37, zentrale Dirty-/Savepoint-/Session-State-Härtung, Produktdatenbank-Startupstatus, Pfad-Policy, T18-Restarbeit, D10 und T28 vorerst nicht parallel zu Claudes Branches beginnen. Überholte Einträge in `goal.md` nicht als offenen Codebedarf behandeln: E4, mehrere T18-Teile, §84 und die kleinen Review-Funde sind bereits auf `main` erledigt.
- **Notes for Claude:** Keine Claude-Branch-/Worktree-Datei angefasst und keine Tests ausgeführt, da dies eine reine Zustands- und Überschneidungsanalyse war. `main` bleibt auf `77bb473`; bestehende uncommittete Lizenz-/Dokumentationsänderungen bleiben erhalten.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-16 19:37 CEST
- **Completed:** Die gemessene `knx_store::load_project`-Optimierung im isolierten Branch `optimize-load-project` vollständig implementiert und geprüft. Drei geordnete Bulk-Scans ersetzen 65.000 Kommunikationsobjekt-Punktabfragen; öffentliche API und SQLite-Schema bleiben unverändert. Tests sichern Geräte-/Objektzuordnung, gespeicherte Positionsreihenfolge statt ID-Reihenfolge, Overrides einschließlich `UnknownOverrideAttr`, Modulzuordnung, Linkreihenfolge und das bisherige Ignorieren verwaister Zeilen. Der Finalreview fand und reproduzierte zunächst einen neuen Panic bei malformed orphan rows; `cd9455f` begrenzt alle Scans wieder auf Objekte mit vorhandenem Gerät, Scoped Re-Review danach sauber. Commits: `fd3e898`, `6ad8c70`, `cd9455f`, `45c93b6`. Fresh final gates auf `45c93b6`: Format, Workspace-Clippy, Workspace-Tests, Layering, Release-Benchmark und `git diff --check` Exit 0. Frischer Benchmark `open_ms=238.723` gegenüber dokumentiertem Vorwert `994.615`; einzelne Shared-Machine-Beobachtung, kein stabiler Multiplikator. Bericht `.ai/logs/2026-09-16_codex_load_project_optimization.md`.
- **Pending/Next Steps:** Nur Integrationsentscheidung offen: Branch lokal nach `main` mergen, pushen/PR anlegen oder Branch/Worktree behalten. Verbleibende per-device/device-binary N+1-Abfragen sind als separater, kleinerer Mess-/Optimierungskandidat dokumentiert und nicht Teil dieses Tasks.
- **Notes for Claude:** Claude-Worktrees und Claude-Dateien unverändert. Branch basiert auf `main`/`origin/main` `77bb473`; Worktree `/mnt/daten-i/Sourcecode/KNXBench/.worktrees/optimize-load-project` ist sauber. Hauptcheckout behält seine vorbestehenden uncommitteten Lizenz-/Dokumentationsänderungen.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-16 20:07 CEST
- **Completed:** `optimize-load-project` auf Nutzerwunsch mit explizitem `--no-ff`-Merge-Commit `ad26c93af0cba700e2dce46aaefd83c442baa88f` nach `main` integriert und erfolgreich zu `origin/main` gepusht. Merge-Autor `KNXBench <github@knxbench.com>`. Gemergter Stand: `cargo fmt --all --check`, Workspace-Clippy, fokussiertes `knx-store` (66 Unit + 2 Reference), kompletter Workspace-Test unter dem dokumentierten clean-checkout Corpus-Skip und Layering Exit 0. Feature-Worktree und lokaler Branch anschließend entfernt. Vorbestehende uncommittete Lizenz-/Dokumentationsänderungen im Hauptcheckout blieben erhalten.
- **Pending/Next Steps:** Für die Kommunikationsobjekt-Bulk-Load-Optimierung bleibt nichts offen. Separater lokaler Testumgebungsfund: `OriginalData/ProductDatabases` ist vorhanden, aber `Weinzierl_730_KNX_IP_Interface_ETS4_v1.knxprod` fehlt; deshalb scheitert der optionale Corpus-Test im Hauptcheckout an `No such file or directory`. Keine privaten Corpusdaten verändert. Verbleibende device/binary N+1-Abfragen bleiben eigenständiger Optimierungskandidat.
- **Notes for Claude:** `main` und `origin/main` stehen beide auf `ad26c93`. Normaler Workspace-Testfehler war vollständig auf den unvollständigen gitignorierten Corpus zurückgeführt: gezielter Test mit `KNXBENCH_PRODUCT_CORPUS=/tmp/knxbench-absent-product-corpus` übersprang wie vorgesehen und passierte; vollständiger Workspace-Test mit derselben clean-checkout-Umgebung Exit 0. Bericht `.ai/logs/2026-09-16_codex_load_project_optimization.md`.


---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-16 22:01 CEST
- **Completed:** Nutzerkorrektur umgesetzt: Der reale Weinzierl-Corpus heißt nur noch `Weinzierl_730_KNX_IP_Interface_ETS4.knxprod`, ohne `_v1`. `dynamic_tree.rs` prüft Content-Hash-Idempotenz jetzt durch zweimalige Installation derselben Bytes; `standalone_packages.rs` erwartet vier vorhandene Archive. Kompatibilitäts-, Gap-, Status-, Research-, Roadmap- und historische Spec-Angaben auf vier Archive, vier distinct packages und sechs gemessene Application-Program-Dateien korrigiert. Commit `416455b` mit Autor `github@knxbench.com` erstellt und zu `origin/main` gepusht. Drei direkt betroffene Real-Corpus-Tests, vollständiges `knx-productdb` ohne lokalen Corpus, Format, `knx-productdb`-Clippy und Diff-Check bestanden.
- **Pending/Next Steps:** Unabhängiger lokaler Corpusmangel bleibt: `Weinzierl_730_KNX_IP_Interface_ETS2-3.vd2` fehlt ebenfalls, daher scheitert nur der dafür vorgesehene Real-VD2-Corpus-Test bei aktiviertem lokalen Corpus. Keine VD2-Annahme oder private Corpusdatei verändert. Vorbestehende uncommittete Lizenz-/Dokumentationsänderungen bleiben erhalten.
- **Notes for Claude:** `main` und `origin/main` stehen beide auf `416455b`. Der Corpus-Fix-Commit enthält ausschließlich neun gezielt gestagte Test-/Dokumentdateien; vorbestehende Dirty-Hunks in `GAP_ANALYSIS_ETS.md`, `IMPLEMENTATION_STATUS.md` und `ROADMAP.md` wurden nicht mitcommittet.


---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-17 07:49 CEST
- **Completed:** Wiederhergestellte lokale Legacy-Datei `OriginalData/ProductDatabases/Weinzierl_730_KNX_IP_Interface_ETS2-3.vd2` verifiziert (77.265 Bytes, Modus 0600). Der zuvor fehlgeschlagene Real-VD2-Test `rejects_the_real_legacy_vd2_corpus_file_with_its_hash_and_size` besteht; anschließend kompletter `cargo test -p knx-productdb` mit dem realen lokalen Corpus Exit 0. Keine Produkt-, Test- oder Dokumentdatei geändert.
- **Pending/Next Steps:** Der lokale Produktdaten-Corpus hat für `knx-productdb` keinen bekannten fehlenden Fixture-Blocker mehr.
- **Notes for Claude:** Verifikation lief auf `main`/`origin/main` `416455b`; ausschließlich `.ai/CURRENT_STATE.md` als vorgeschriebener Handover ergänzt.

---
- **Last Agent:** Codex
- **Timestamp:** 2026-09-17 07:53 CEST
- **Completed:** `docs/COMPATIBILITY.md` records the 2026-09-17 revalidation of the restored 77,265-byte `Weinzierl_730_KNX_IP_Interface_ETS2-3.vd2` fixture, names `rejects_the_real_legacy_vd2_corpus_file_with_its_hash_and_size`, and records that the complete `knx-productdb` suite passed. Committed only that documentation file as `f7893ec` (`docs(compatibility): record restored VD2 verification`) with author `KNXBench <github@knxbench.com>` and pushed `main` to `origin`.
- **Pending/Next Steps:** Nothing remains for the restored VD2 fixture verification note.
- **Notes for Claude:** VD2 remains intentionally unsupported and is verified as a typed rejection; the note does not claim VD2 import support. `origin/main` resolves to `f7893ec046d2eac6da0ed5823beae402489e3cb3`. Pre-existing uncommitted licence and documentation work remains uncommitted.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-17 08:10 CEST
- **Completed:** Aktuellen `main`-/Roadmap-/Goal-/Analyse-Stand auf noch übernehmbare Aufgaben geprüft. Die `knx-store::load_project`-Bulk-Optimierung ist bereits integriert (`ad26c93`) und kein offener Kandidat mehr. Drei vorhandene Claude-Worktrees wurden als belegt erkannt: `d10-language-data` (dirty), `d10-server-prose` und `t18-module-arguments`. Als nächste Codex-Arbeiten identifiziert: sofort konfliktarm sind Linux-Paketierung, Performance-Baseline außerhalb des bereits optimierten Store-Open-Pfads sowie kleine Import-/Export-Datenverlustkorrekturen außerhalb der belegten Sprach-/Moduldateien. Die P0-Integritätsbaseline, Desktop-/Server-Pfad-Policy, Product-DB-Startzustand, atomarer `ProjectSessionState` und T37 bleiben fachlich stark, überschneiden aber `apps/knx-server/src/domain.rs`/`routes.rs` mit dem noch nicht integrierten `d10-server-prose`-Branch und sollten erst nach dessen Integration begonnen werden. Keine Produktdatei geändert.
- **Pending/Next Steps:** Nutzer wählt einen Task. Empfehlung für sofortige Arbeit: Linux-Paketierung als klar isolierter Liefer-Task. Nach Integration von `d10-server-prose`: zuerst Projektwechsel/Savepoint-Integrität, danach Trust-Boundary für Dateipfade und Server-Bindung.
- **Notes for Claude:** Nicht in `crates/knx-productdb` Sprach-/Migrationsteile, Parameter-/Settings-UI oder die T18-Modulargumente eingreifen, solange die drei bestehenden Worktrees aktiv sind. `main` und `origin/main` stehen auf `f7893ec`; bestehende uncommittete Lizenz-/Dokumentationsänderungen wurden nicht verändert, außer diesem angehängten Handover-Eintrag.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-17 16:57 CEST
- **Completed:** Linux-Paketierung nach Nutzerwahl auf ein `x86_64`-AppImage festgelegt. Isolierten Worktree `.worktrees/linux-appimage` auf Branch `linux-appimage` erstellt. Architektonische Spezifikation `docs/superpowers/specs/2026-09-17-linux-appimage-design.md` geschrieben, auf Platzhalter/Widersprüche geprüft und als `96e3f06` (`docs: design Linux AppImage packaging`) mit Autor `KNXBench <github@knxbench.com>` committed. Design umfasst Tauri-Bundlevertrag, manuelle/tag-basierte GitHub-Workflows, Versions-/Artefaktvalidierung, echten AppImage-Build und Startprobe, Dokumentation sowie explizite Grenzen.
- **Pending/Next Steps:** Nutzerreview der geschriebenen Spezifikation abwarten. Nach Freigabe gemäß Brainstorming-Workflow mit `superpowers:writing-plans` den Implementierungsplan erstellen; danach erst Bundle-, Workflow-, Test- und Dokumentdateien ändern.
- **Notes for Claude:** Branch basiert auf `f7893ec` und enthält bisher ausschließlich den Design-Commit. Hauptcheckout bleibt mit vorbestehenden uncommitteten Lizenz-/Dokumentänderungen dirty; diese wurden nicht in den Worktree kopiert. Die Spezifikation setzt den kanonischen Root-`LICENSE`-Text als Bundle-Eingang voraus, der im Hauptcheckout derzeit untracked vorliegt.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-17 17:04 CEST
- **Completed:** Nutzer hat die AppImage-Designspezifikation freigegeben. Detaillierten TDD-Implementierungsplan `docs/superpowers/plans/2026-09-17-linux-appimage.md` erstellt, gegen die Spezifikation auf Abdeckung, Platzhalter und Widersprüche selbstgeprüft und als `f8c8684` (`docs: plan Linux AppImage packaging`) committed. Plan umfasst Rust-`xtask`-Vertragsprüfung, Tauri-Bundle/AGPL-Ressource/Icons, ADR 0021, getrennte GitHub-Build-/Release-Jobs, realen Build/Extraktion/15-s-Startprobe, Statusdokumentation und Finalgates.
- **Pending/Next Steps:** Gemäß Writing-Plans-Workflow Ausführungsart wählen lassen: subagent-driven oder inline. Danach Plan vollständig umsetzen; noch keine Bundle-, Workflow- oder Produktdatei geändert.
- **Notes for Claude:** Worktree `.worktrees/linux-appimage`/Branch `linux-appimage` ist clean und enthält nur Design-Commit `96e3f06` plus Plan-Commit `f8c8684` auf Basis `f7893ec`. Die drei bestehenden Claude-Worktrees bleiben unberührt. Root-`LICENSE` liegt weiterhin nur im dirty Hauptcheckout und wird laut Plan unverändert mit geprüftem Hash in den Paketierungsbranch übernommen.
- **Last Agent:** Codex
- **Timestamp:** 2026-09-17 17:27 CEST
- **Completed:** Theme-Palette in `apps/knx-web/src/theme.ts` und `styles.css` geprüft und dem Nutzer kurz zusammengefasst. Keine Produktdateien geändert.
- **Pending/Next Steps:** Keine.
- **Notes for Claude:** System-Theme folgt dem Betriebssystem und löst zu Porcelain oder Graphite auf; Bitcoin DeFi hat feste Orange-/Gold-Akzente, Porcelain und Graphite unterstützen fünf Akzentfarben.

---

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-17 18:46 CEST
- **Completed:** Linux-AppImage-Paketierung im isolierten Branch `linux-appimage` bis `8c8cbd6` vollständig implementiert und per taskweisen sowie vollständigem Branch-Review freigegeben. Tauri baut genau ein `x86_64`-AppImage; `xtask check-appimage` prüft Version, Architektur, reguläre ausführbare Datei, Bundle-Ressourcen, semantische Workflow-Bedingungen, `NO_STRIP`, sicheren Smoke-Test und Release-Repository-Kontext. GitHub-Workflow trennt read-only Build/Artefakt von Push-Tag-Release mit `contents: write`; manuelle Läufe können nicht veröffentlichen. Reales lokales Artefakt `KNXBench_0.1.0-alpha.1_amd64.AppImage`, 105.839.096 Bytes, SHA-256 `b0ec49aebcec984ffdce21639306713862fc6c7baf15f7247f060ac314cdaef7`, extrahiert mit ausführbarem Desktop, Metadaten, Icons, 88 Frontenddateien und bytegleicher AGPL-Lizenz; zweimal 15 s sichtbar gestartet, Status 124, keine KNX-Aktivität. Fresh final verification auf `8c8cbd6`: `cargo test --workspace` Exit 0; Workspace-Clippy/Format Exit 0; 49 xtask-Tests Exit 0; 471 Frontendtests und Frontendbuild Exit 0; Layering, AppImage-Vertrag, `cargo deny`, Diff-Check Exit 0. Design, Plan, ADR 0021, README, Roadmap/Goal/Status und `.ai/logs/2026-09-17_codex_linux_appimage.md` sind committed. Keine Veröffentlichung, kein Push, Merge, Tag oder Workflow-Dispatch erfolgt.
- **Pending/Next Steps:** Integrationsentscheidung des Nutzers abwarten: lokal nach `main` mergen, Branch pushen/PR erstellen oder Branch/Worktree behalten. Bekannter vorbestehender Gate-Fehler bleibt: `check-headers` meldet 170 headerlose Dateien bei Ceiling 168; der AppImage-Branch fügt keine Headerverletzung hinzu. Der GitHub-Workflow ist konfiguriert, aber nicht ausgeführt; lokaler Arch-Build brauchte eine temporäre, nicht committed GTK-Deployment-Plugin-Anpassung, daher kein Ubuntu-CI- oder distributionsweiter Kompatibilitätsclaim.
- **Notes for Claude:** Worktree `/mnt/daten-i/Sourcecode/KNXBench/.worktrees/linux-appimage` und Branch `linux-appimage` sind clean; Basis `f7893ec`, Head `8c8cbd6`. Gesamt-Review fand und Fixwelle schloss vier Punkte: `GH_REPO`, Status-0-Smoke-Pass, AppImage-Verzeichnis statt Datei, README-`<version>`-Umleitung. SDD-Scratch-Verzeichnis wurde nach cleanem Review entfernt. Zwei Codex-Rulings: alle Zielpfadprüfungen liefen vom Repo-Root; Workflow bleibt ausdrücklich konfiguriert/unexecuted und lokale Arch-Evidenz wird nicht als CI/Portabilität ausgegeben. Vorbestehende Dirty-Hunks im Hauptcheckout wurden nicht verändert außer diesem angehängten Handover-Eintrag.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-17 19:00 CEST
- **Completed:** Nutzeroption 1 ausgeführt: `linux-appimage` lokal mit explizitem `--no-ff`-Merge-Commit `e5b78430616e36ae81739e41f75c999519611299` (`merge: add Linux AppImage packaging`) und Autor `KNXBench <github@knxbench.com>` nach `main` integriert. Vorbestehende uncommittete Hauptcheckout-Arbeit vollständig gestasht, nach dem Merge wiederhergestellt und zwei erwartete Konflikte semantisch gelöst: `.ai/CURRENT_STATE.md` behält die Hauptcheckout-Historie samt AppImage-Abschluss; `goal.md` kombiniert die erledigte AGPL-Lizenzentscheidung und die erledigte AppImage-Paketierung, sodass nur noch die große Performance-Messung als Session-7-Lieferpunkt offen ist. Root-`LICENSE` entspricht unverändert dem kanonischen Hash. Gemergten tatsächlichen Stand frisch verifiziert: `cargo test --workspace -q` Exit 0; `cargo fmt --all --check` und Workspace-Clippy Exit 0; 471 Frontendtests und Produktionsbuild Exit 0; AppImage-Vertrag gegen das zuvor gebaute reale Artefakt, Layering, `cargo deny check` und `git diff --check` Exit 0. Temporären Stash nach erfolgreicher Wiederherstellung gelöscht; AppImage-Worktree entfernt und lokalen Feature-Branch gelöscht. Kein Push, Tag, Release oder Workflow-Dispatch erfolgt.
- **Pending/Next Steps:** `main` ist 15 Commits vor `origin/main`; Push bleibt eine separate Nutzerentscheidung. Bekannter vorbestehender Gate-Fehler bleibt: `cargo run -p xtask -- check-headers` meldet 170 headerlose Dateien bei Ceiling 168; der AppImage-Change fügte keine Headerverletzung hinzu. GitHub Actions bleibt konfiguriert, aber unexecuted; lokaler Lauf beweist Arch/XWayland, nicht Ubuntu-CI oder allgemeine Distributionskompatibilität.
- **Notes for Claude:** Vorbestehende lokale Änderungen wurden bewusst uncommitted erhalten: `.ai/CURRENT_STATE.md`, `README.md`, `docs/GAP_ANALYSIS_ETS.md`, `docs/IMPLEMENTATION_STATUS.md`, `docs/KNOWN_LIMITATIONS.md`, `docs/ROADMAP.md`, `goal.md`, `xtask/Cargo.toml` sowie untracked `docs/PROJECT_ANALYSIS_2026-09-15.md` und `docs/VD4_PRODUCT_DATABASE_IMPORT.md`. `LICENSE` ist durch den Merge jetzt tracked und bytegleich zur vorher untracked Version. Sicherheitsbackups des Vor-Merge-Zustands liegen in `/tmp/knxbench-pre-appimage-merge.patch` und `/tmp/knxbench-pre-appimage-untracked.tar.gz`.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-17 19:28 CEST
- **Completed:** Reconciled the stale Session 7 status after proving the required large-project benchmark already exists on `main`. Fresh `cargo test -p knx-app --release --test perf_baseline -- --ignored --nocapture` passed with the deterministic 5,000-device / 20,000-group-address / 20,000-communication-object project: export 82.790 ms, import 185.351 ms, open 230.286 ms, projection 23.018 ms, and 41 searches in 19.510 ms. Updated `goal.md` and `docs/ROADMAP.md` to mark Session 7 complete; `git diff --check` passed. Investigated the next T30 R20 commissioning issue without bus access: the documented cause remains unproven and `KNX_GATEWAY` is unavailable, so no retry behavior was changed. Reproduced the independent `check-headers` gate failure at 170 absent headers versus ceiling 168 and traced both regressions to `apps/knx-web/e2e/new-project.e2e.ts` and `apps/knx-web/playwright.config.ts`, added after ratchet commit `93fc6c9` without ADR-0018 headers.
- **Pending/Next Steps:** With user approval of the bounded design, add one truthful ADR-0018 first-line header to each of the two Playwright files, rerun `cargo run -p xtask -- check-headers`, focused Playwright/config checks if available, and `git diff --check`. T30 R20 still needs a read-only hardware run that separates reverse-order from one-tunnel-per-address behavior before any protocol retry fix; never contact `1.1.220`.
- **Notes for Claude:** Session 7's performance deliverable was already implemented in commits `855ae39`, `5fb3e4a`, and `e321ef3`; the stale goal caused the false open item. No product code changed in this pass. The current main checkout retains all pre-existing uncommitted licence/document changes. `main` remains 15 commits ahead of `origin/main`; no push, tag, release, or hardware access occurred.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-17 19:31 CEST
- **Completed:** Restored the ADR-0018 header ratchet by adding truthful first-line TypeScript headers to `apps/knx-web/e2e/new-project.e2e.ts` and `apps/knx-web/playwright.config.ts`. Fresh `cargo run -q -p xtask -- check-headers` passed with 122 well-formed headers, 168 absent headers at ceiling 168, and 30 generated files skipped. Fresh `git diff --check` passed; the scoped diff contains only the two comment headers.
- **Pending/Next Steps:** Header-gate repair is complete. T30 R20 remains the highest-priority concrete commissioning investigation but cannot be fixed from current evidence: `KNX_GATEWAY` is not configured, and a safe read-only run must distinguish reverse-order behavior from one fresh tunnel per address before changing retry semantics. No hardware write is authorised.
- **Notes for Claude:** The two headerless files were introduced after ratchet commit `93fc6c9`; AppImage work added no header violation. No production behavior changed, no commit or push was requested or performed, and all pre-existing dirty documentation/licence changes remain present.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-17 20:19 CEST
- **Completed:** Aligned the Docker frontend build with `apps/knx-web`'s Node contract by changing `node:20-alpine` to `node:22-alpine`. The first real smoke run exposed a pre-existing stale fixture path in `apps/knx-server/scripts/smoke-test.sh`; fixed it to accept `KNXBENCH_REFERENCE_PROJECT` and default to the current `OriginalData/DemoProjects/Unser Zuhause ets4 - 2025-12-15.knxproj` location. Updated `docs/KNOWN_LIMITATIONS.md` §25 and `docs/IMPLEMENTATION_STATUS.md`, including the now-complete Session 7 status. Fresh full Docker smoke test built the image, started the server, passed `/healthz`, and imported the mounted real project with zero errors. Frontend stage reports Node `v22.23.2` with no `EBADENGINE`; shell syntax, header ratchet (122 valid / 168 absent at ceiling 168 / 30 generated), and `git diff --check` pass. Added `.ai/logs/2026-09-17_codex_node22_docker.md`.
- **Pending/Next Steps:** Docker Node alignment and smoke-test repair are complete. Highest-priority T30 R20 remains evidence-blocked until a configured gateway can run the approved read-only reverse-order and fresh-tunnel comparison. D10 and T18 areas remain reserved by three non-merged Claude worktrees.
- **Notes for Claude:** No bus access, image publication, commit, or push occurred. The Docker smoke image remains local; its temporary container and `.smoke-data` directory were removed by the script trap. All pre-existing dirty licence/document work remains preserved.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-17 20:20 CEST
- **Completed:** Re-measured the project-analysis frontend warning finding on current `main`: `npm test --prefix apps/knx-web -- --run` passed 471/471 tests across 42 files in 3.17 s with zero React act-environment warnings, zero not-wrapped-in-act warnings, and zero Vitest stderr blocks. No frontend change was needed. Investigated the next release gap: `.github/workflows/ci.yml` still does not build or smoke-test the Docker artifact, and the gitignored real project used by the local smoke test is unavailable in CI. Existing HTTP routes already support a corpus-free new/save-as/open persistence cycle.
- **Pending/Next Steps:** Await approval for the bounded Docker CI design: make `smoke-test.sh` always verify health plus new/save/reopen without external fixtures, retain real `.knxproj` import as an optional `KNXBENCH_REFERENCE_PROJECT` extension, add a Docker smoke job to CI, update README/current status, and prove both fixture-free and real-corpus modes locally.
- **Notes for Claude:** D10 and T18 worktrees remain non-merged and untouched. Proposed Docker CI work is isolated from their files. Current Docker Node 22 change and smoke-path repair remain uncommitted in the main checkout.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-17 20:54 CEST
- **Completed:** Added an independent Docker artifact smoke job to `.github/workflows/ci.yml`. Reworked `apps/knx-server/scripts/smoke-test.sh` so CI needs no private corpus: it builds and starts the shipped image, checks `/healthz`, creates a native project, saves it to the mounted volume, replaces in-memory state, reopens the saved `.knxdb`, and verifies the installation survived. `KNXBENCH_REFERENCE_PROJECT` optionally adds real ETS import and an explicit missing path fails before build. Fixture-free and real-corpus modes both passed locally; missing-fixture rejection, workflow YAML parse, shell syntax, header gate, and `git diff --check` passed. Updated README, Roadmap, Implementation Status, and dated project analysis. Added `.ai/logs/2026-09-17_codex_docker_ci.md`.
- **Pending/Next Steps:** Continue remaining independent hardening tasks. Highest P0 candidate not reserved by Claude is the schema-11 export loss for range-less group addresses. T30 R20 remains hardware-evidence blocked; D10/T18 and server session/route work remain reserved by non-merged Claude worktrees. GitHub Docker job is configured but has not run remotely.
- **Notes for Claude:** No commit, push, image publication, or KNX bus access occurred. Current checkout retains pre-existing dirty licence/docs plus the verified header, Node 22, smoke-script, and CI changes.
