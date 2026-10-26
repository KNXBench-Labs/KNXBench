# Restart snapshot

Updated: 2026-10-26 20:03 CET. Operational snapshot, not an append-only journal.
## Alpha 17 release plan (PLANNED by claude-release-alpha17, observed 2026-10-26 20:03 CET)
- Owner go: once the S2-01 safe diagnostic projection is delivered, cut, gate and publish Alpha 17.
- Prerequisite: S2-01 safe projection only. Its session finishes as usual; delivery is read
  from its usual delivered heading, or it adds the line → `Alpha 17 prerequisite delivered: S2-01`.
- No freeze yet. Other packages continue; anything merged after the cutoff ships in Alpha 18.
- At cutoff a separate `Release freeze: taken by claude-release-alpha17` entry appears here;
  then hold pushes to `main` until `Release freeze: released by claude-release-alpha17`.
- Not in scope: S2-02 closure (needs reporter re-test on Alpha 17), S2-03 (evidence), PRG-01
  (programming track), GATE-01 (gate tooling). No hardware/bus/deployment work.
## S2-01 safe diagnostic projection (REVIEWED; gate2 running, observed 20:33 CET)
- User go through main publication remains; candidate `5c469a5ec` plus test-only `eeba38bf` local.
- Own `<sources>/knxbench-system2-safe-failure-20261026`, branch
  `feat/system2-safe-failure-projection-20261026`; integration tree suffix `-integrate-20261026`.
- Whole-feature/correction reviews APPROVE; original semantic RED retained; no production change in fix.
- Gate1 red:847 browser pass/4 opt-in skips refused by validator; later Rust/corpus not run.
- Test-registration fix independently APPROVE; final production default9+capture4 pass, no skips.
- Branch15 gate2 `proc_8dbdb3708730` PID3048799; source frozen; main-first integrated gate next.
- Main snapshot/release-plan preserved; conditional push only after both gates/readback.
- Result docs/retirement pending; inherited English retained-status paragraph disclosed, new entries EN/DE.
- Evidence `<profile>/evidence/system2-safe-failure-20261026/`; no bus/hardware/release/deployment.
## Issue #30 / S2-01 pointer width (DELIVERED, observed 2026-10-26 19:03 CET)
- PUBLISHED main-first merge `54b052885`, feature `6b8b27936`; local/tracking/live equal.
- Independent whole-feature review and correction re-review APPROVE; branch15/integrated15 green.
- Integrated Rust5063/169ignored, corpus97/97, Vitest4540/browser728; public-package witness4 green.
- Initial push alone failed auth; unchanged accepted source published with one-shot gh helper.
- Raw Two/Four capture/reopen/recovery and exact-profile rev2 policy delivered; Legacy retained.
- Object1 two-octet field observation only;2/3 source-declared hypotheses, no hardware support claim.
- Net typed non-raw refusal seam available; safe failure/report projection remains separate.
- Shared width/recovery/journal ownership RELEASED to safe-projection track after this docs publication.
- Own trees retained only for docs/verified evidence retirement; no further runtime edits or own gate.
- Evidence `<profile>/evidence/issue30-width-20261026/`; no bus/release/deployment/vendor work.
## PRG-01/S2-03 CLI durable-tunnel slice (runtime DELIVERED; result docs pending)
- PUBLISHED main-first merge `a8004cb2e` on `8459b5fe8`, feature `46c38df26`; readbacks equal.
- Independent review APPROVE; merge13 Rust521/21ignored, corpus17/17; tools440/6skipped.
- No web/bindings selected. Native ACK characterization and lexical call-site proof are distinct.
- Own cli-durable-tunnel/integrate trees remain for result-doc gate/push and verified cleanup.
- Evidence `<profile>/evidence/cli-durable-tunnel-20261026/`; no hardware or policy change.
## GATE-01 spawn/signal slice (runtime DELIVERED, result documentation pending)
- PUBLISHED main-first merge `ed156ef9b`; local/tracking/live readbacks equal.
- Review/re-review APPROVE. Merge11/web65 green: Vitest4539, browser1478 incl all16 production suites;
  no Rust/corpus/bindings selected, frozen inputs and no user-data changes.
- Own browser-tmp-spawn/integrate worktrees remain for docs gate/publication then verified cleanup.
- Evidence `<profile>/evidence/gate01-spawn-signals-20261026/`; no runtime runner.
- Old parked branch retained; no proven field-timeout cause, lane-policy change or hardware claim.
## DIAG-02 CLI follow-up (DELIVERED runtime; result documentation pending)
- PUBLISHED main-first merge `23e1ff3ec` on `62089b033`, fixes `e5cfa1e7b`; readbacks equal.
- Review and focused re-review APPROVE. Merge14 green: Rust2702/127ignored, corpus91/91;
  tools431 registered/6skipped. Browser/bindings not selected; web bundle prerequisite built.
- Own `<sources>/knxbench-diag03-integrate-20261026` stays for result-doc gate/publication,
  then own trees/cache cleanup. Evidence `<profile>/evidence/diag03-cli-20261026/`.
- No bus/hardware. System-2/never-connected/legacy cause gaps remain; root not synchronized.
## Next open tasks (26 October 2026, this WebUI session)
- WP-21/23/22 published, own trees/claims retired after evidence verification.
- WP-24 PUBLISHED `8dd6cf72d`, APPROVE, gate11/web34; Vitest4475/browser908 +catalogue22.
  Next: result-doc gate/publish, verified private evidence retirement, own tree cleanup.
- WP-21–24 runtime scopes delivered; remaining queue retains evidence/owner boundaries.
- Root preserved; no bus/deployment/release. Evidence `<profile>/evidence/next-open-tasks-20261026/`.
## DIAG-02 server/web diagnostic (DELIVERED)
- Published `4209645b4`, result docs `62089b033`; original planning STOP superseded by owner's Go.
- Status/log/history/report keep cause; initial server/web and later CLI acceptance are in docs/status/.
- System-2 remains outside this diagnostic contract; no bus/deployment authority.
## WP-16 AES metadata consistency (26 October 2026, this WebUI, DELIVERED)
- PUBLISHED `161e3b14b`; review APPROVE, gate 14/14 (Rust 5,033, corpus 155/155); readback equal.
- Evidence `<profile>/evidence/wp16-aes-20261026/`. Own trees retired after result docs.
## DIAG-01 web profile (26 October 2026, this WebUI, DELIVERED)
- PUBLISHED `a4bb0aa4e`; review + re-review APPROVE, gate 15/15 (919 browser); readback equal.
- Evidence `<profile>/evidence/diag01-web-20261026/`. Own tree retired after result docs.
## Other owners and preservation
- Root intentionally old/dirty: preserve private/untracked files and other edits.
  Alpha16 belongs to another track. No release/deployment/hardware authority.
- Issue30 UI delivered `946871e8b`, main `ec766a9b2`, APPROVE, gate11 incl897 browser.
  System2 prestate package DELIVERED `a9abdba33`, main `1cc975ff0`; claim released (15:10 CET).
  Owner rule: pre-write hard = hardware type/mask/manufacturer; serial, prior program version,
  load state, PID 7 are observations; memory/Loaded/written version verification stays hard.
## Active programming improvements
- Bounded memory-budget source PUBLISHED `1e6229cea3`; local/tracking/live equal.
  Feature `22d14c863`, original6 semantic RED, independent reviews accepted.
- Integrated16 green: Rust6818/229ignored, corpus155/155 in9targets, bindings same;
  tools425executed/6skipped; web/browser not selected, bundle separately built.
- First integrated Rust101 stays failed; exact original log/binary retained.
  Separate reviewed test-only workspace fixture fix `172bf53b`; no production change.
- MergeId PUBLISHED `be2687796`/`5bb8d076`; B2timeout remains failed/unrepinned.
- All24 non-Secure dispositions reconciled;23 retain exact remaining evidence blockers.
  PRG-01 stays OPEN; no broad capability, family, device baseline or recovery closure.
- Own `<sources>/knxbench-regular-memory-budget-20261025`, branch
  `integrate/regular-memory-budget-20261026`; next result-doc gate/publication,
  verified private evidence archive and own clean-worktree/build-cache retirement.
- cEMI/client/gateway/System2 foreign-owned. No bus/Secure/deployment/vendor work.
  Path/parameter/customer-write/exclusion/runtime/exemplar/effect/recovery boundaries retained.
## S2-01 diagnostic subdelivery
- PUBLISHED `5f1ad454e`; local/tracking/live refs matched after push; claim released.
- Independent Opus APPROVE, integrated gate14 green: Rust3279/131ignored, corpus91/91.
- Raw prefix private journal only; wrong-width refusal retained. No hardware/recovery claim.
- Width model blocked on real bytes; structured projection open; S2-02 planned P1.
- [Acceptance](../docs/status/2026-10-25-system2-pid7-diagnostic.md) owns diagnostic scope.
## Issue30 S2-02 follow-up (observed 2026-10-25 23:40 CET, DELIVERED)
- Feature `aa2f39f1b`, main `a15b8f0eb`; review APPROVE, package gate 15/15 green.
- Issue #30 updated (comment 6404087010); open, waiting for reporter's refusal code.
- Old opt-in recovery timeout also fails on base `866cb5ca6`; cause unresolved, untracked here.
- Next: on reporter reply, trace the reported refusal kind. No live bus access.
## Simulator gateway / System2 CLI differential (26 October 2026, delivered)
- PUBLISHED CLI merge `44efafa9c`, feature `ff2e45148`; corrected review APPROVE.
- Branch/merge gate15 accepted; Rust2641/126ignored, corpus91/91. Final delta11/Web34,
  browser908 accepted; Rust/Cargo/tools equal earlier gate (results carried with proof).
- APDU55/15 isolated CLI runs finished; journal verified, simulator teardown verified.
- Private evidence `<profile>/evidence/simulator-differential-20261026/`; no hardware claim.
- Next: result-doc publication and own-tree retirement; root remains unsynchronized.
## Retained boundaries
- Family/format prerequisites retained; System2 diagnostics CLI-only, SIGKILL cleanup limited.
- No vendor contact/Ollama/downloads/excluded models; excluded device off-limits.
- AGENTS.md applies; docs own status, vault decisions, sessions conversation history.
