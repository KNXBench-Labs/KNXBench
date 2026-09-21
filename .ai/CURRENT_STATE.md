- **Last Agent:** Codex
- **Timestamp:** 2026-09-21 19:23 CEST
- **Completed:** T07 implemented and independently reviewed on branch `t07-dpt-explicit`, base `240792b`, head `11c8131`. `knx_core::encode` now requires `DptInputFormat`; legacy inference is isolated as `encode_inferred_format`. CLI/HTTP accept explicit format, and web exposes a visible selector with honest `Auto` compatibility mode. Public `encoding_rulings` covers every §61 project judgment for main types 1–30. Review caught and fixed fixed-width bitsets changing `00000010` from `0x02` to `0x0A`, plus valid hexadecimal `0B` rejection. No intended wire mapping changed. Full Rust/web gates pass: core 479, CLI 10, HTTP fake-tunnel 13, web 812/58, TypeScript, fmt, workspace Clippy/tests, layering, headers 181/162, anchors 377/174, deny. No KNX traffic or hardware access. Detail log: `.ai/logs/2026-09-21_codex__t07_dpt_input_formats.md`.
- **Pending/Next Steps:** Merge T07 into `main`, verify the merge commit itself, push, then continue `goal.md` at T08. User explicitly requested committing and pushing the pre-existing dirty main changes too; keep them in separate focused commits after inspection.
- **Notes Claude:** Primary evidence for KNX specification questions is now the source PDFs under `/mnt/daten-i/Sourcecode/knx-spec-kb/sources/The KNX Standard v3.0.0`; SQLite/search indexes are locators only. The first reviewer hit a usage limit after one valid finding; the completed independent review found one Important and three Minor items, all fixed. Source disk filled during a full build, so only the feature worktree's recoverable `target/` cache was deleted and final gates ran clean under `/tmp/knxbench-t07-target`.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-20 16:27 CEST
- **Completed:** Read-only status audit for the user's question about remaining KNXBench tasks. Confirmed `main` and `origin/main` are synchronized at `df287d0`; there are no additional worktrees and the checkout was clean before this required handover update. Reconciled the current roadmap, completion goal, implementation status, gap analysis and known limitations. The principal open delivery items are T30 phase-3 real-hardware write verification (requires separate explicit operation-specific authorization and a safe target), T37 truthful project open/import progress, the remaining user-manual half of T28/D12, and explicitly scoped compatibility/capability residue such as the eighteen 200-series DPT main types and module-instance limitations. Deferred items such as KNX Secure, multi-user editing and post-v1 floor-plan work remain deliberately unscheduled; numbered known limitations are not automatically backlog commitments.
- **Pending/Next Steps:** Choose the next authorized slice. T37 is the strongest self-contained implementation candidate without hardware or external samples. Do not perform a real KNX write without explicit operation-specific user approval; never contact the excluded alarm-panel address recorded in the private operational context.
- **Notes for Claude:** No product code or documentation changed and no tests ran because this was a read-only status query. Only this mandatory handover entry modifies the checkout.

---

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

---

Ältere Handover-Einträge (2026-09-08 bis 2026-09-18 04:57, 89 Stück) wurden am
2026-09-20 entfernt, weil die zugehörige Arbeit abgeschlossen und in `docs/`
dokumentiert ist. Vollständiger Stand liegt im Statusarchiv unter
`Backup/status_2026-09-20_16-37-52.zip`.
