# Restart snapshot

Updated: 2026-10-27 06:55 CET. Operational snapshot, not an append-only journal.
## WP-11 native acceptance / owner decisions (DELIVERED, observed 2026-10-27 06:34 CET)
- PUBLISHED main-first merge `cc9d8fab2`, feature `f4f112fe5`; local/tracking/live equal.
- Claude review + correction re-review APPROVE; branch/integrated13 gates green/frozen.
- Integrated Rust xtask103/0 ignored; tool461 methods/6skips; corpus no cases, no web/browser/bindings.
- All 11 owner decisions reconciled; no product runtime changes or ledger status closure.
- Actual Alpha17 native evidence:130 logged operations, not tests; two ordinary closes exit0.
- Loopback-only sandbox, real home/projects masked; no bus or real-project edit, no native runner.
- WP-11 OPEN: positive source-bound editors, capture export, pointer/hover/full matrix;
  Orca/Firefox absent, vendor/GPU evidence unverified. No inferred compatibility or hardware Go.
- Evidence `<profile>/evidence/wp11-native-20261027/`; result-doc gate/publication and owned
  evidence archival/cleanup remain. Root stays stale/dirty; preserve all foreign tracks.
- No release/deployment/key/vendor/foreign cleanup authority.
## GATE-01 SIGTERM fixture flake (IN PROGRESS, observed 2026-10-27 06:55 CET)
- Owner: this WebUI; branch `fix/gate01-sigterm-fixture-20261027`, own worktree; test-only change.
- Two pidfd-identity fixtures in `tools/tests/test_browser_tmp_launch.py` now event-bound.
- Old fixtures fail both under an injected 5 s stall, new pass; 1/12 load repro on old.
- Next: short independent review, package gate, conditional push. No own gate running yet.
## GATE-01 production case parallelism (DELIVERED, observed 2026-10-27 06:35 CET)
- PUBLISHED main-first merge `3af5ea9e4` on `277d8eca3` (feature `51ee72c1b`, fixes `644fb69f1`,
  `c7c8877a7`); local/tracking/live readbacks equal.
- Review BLOCK R1 fixed; re-review + confirmation APPROVE. Gate 11/11 green, Vitest4553,
  browser925+production557; production stages 681.7 s -> 455.7 s (ADR-0155).
- Evidence `<profile>/evidence/gate01-fullypar-20261027/`; own trees retired after result docs.
- Next: GATE-01 remainder (cross-suite parallelism, default-lane case parallelism) unreviewed.
## GATE-01 production browser lanes (DELIVERED, observed 2026-10-27 00:45 CET)
- PUBLISHED merge `745932994` (ADR-0154, 850.3 -> 681.7 s); superseded by the entry above.
## Alpha 17 release (PUBLISHED, observed 2026-10-26 22:53 CET)
- Release freeze: released by claude-release-alpha17
- Tag `v0.1.0-alpha.17` (object `0fd5fabda`) → commit `a9ea32d72`, cutoff `20ab2f7ea`; GitHub pre-release.
- Verified: full gate + extras green (pinned matrix first attempt RED harness, rerun green); dry runs
  and publish runs green; assets/SHA256SUMS/digests, Docker index+revision, isolated smokes PASS.
  [Record](../docs/evidence/release-alpha17-2026-10-26.json); Pages readback of the site successor
  open. Evidence `<profile>/release-evidence/alpha17-20261026/`.
- Other sessions resume from their own last entry on fresh origin/main; anything after cutoff
  20ab2f7ea ships in Alpha 18.
## S2-01 safe diagnostic projection (DELIVERED; result docs published `20ab2f7ea`; shipped in Alpha 17, observed 21:24 CET)
- PUBLISHED main-first merge `536e492bc` (production `5c469a5ec`, test correction `eeba38bf`);
  local/tracking/live readback equal. Whole-feature and correction reviews APPROVE.
- Branch15/integrated15 fully green: Rust2731/129ignored, corpus91/91,
  Vitest4553/306files, browser914 (847functional/67exclusive); frozen/no touched or unchecked data.
- Real simulator→private journal→reopen→HTTP/report→production EN/DE browser4 proof retained.
- Gate1 refusal preserved, test registration fixed, no gate weakened; typed counts not hw authority.
- Alpha 17 prerequisite delivered: S2-01
- Evidence `<profile>/evidence/system2-safe-failure-20261026/`; no bus/hardware/release/deployment.
## Issue #30 / S2-01 pointer width (DELIVERED, observed 2026-10-26 19:03 CET)
- PUBLISHED main-first merge `54b052885`, feature `6b8b27936`; local/tracking/live equal.
- Independent whole-feature review and correction re-review APPROVE; branch15/integrated15 green.
- Integrated Rust5063/169ignored, corpus97/97, Vitest4540/browser728; public-package witness4 green.
- Raw Two/Four capture/reopen/recovery and exact-profile rev2 policy delivered; Legacy retained.
- Object1 two-octet field observation only;2/3 source-declared hypotheses, no hardware support claim.
- Net typed non-raw refusal seam available; safe failure/report projection remains separate.
- Shared width/recovery/journal ownership RELEASED to safe-projection track after this docs publication.
- Own trees kept for docs/evidence retirement only. Evidence `<profile>/evidence/issue30-width-20261026/`.
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