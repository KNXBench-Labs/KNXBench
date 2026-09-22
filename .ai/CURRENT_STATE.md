- **Last Agent:** Codex
- **Timestamp:** 2026-09-22 (Europe/Berlin)
- **Completed:** T12 task 5 fix round 1 closes the torn current-project snapshot finding. All three whole-project replacement paths now publish `project`, a reset `command_stack`, and `import_counts` while holding those locks together in the established project → stack → counts order, so `current_project_tree` can observe only the complete old or complete replacement state. Regression evidence lives in `domain::tests::current_project_tree_waits_for_an_entire_replacement`; the Task 5 report contains the full RED/GREEN record.
- **Verification:** RED: the new regression failed 0/1 with `replacement project became visible before stack/count publication`. GREEN: focused regression 1/1; all `knx-server` tests 397/397 across 35 result blocks; full Rust workspace 1,953 passed / 0 failed / 5 ignored across 92 result blocks. Rustfmt, focused server Clippy with warnings denied, and `git diff --check` passed. No KNX/LAN/multicast/gateway/hardware traffic.
- **Pending/Next Steps:** Controller review/integration only; the three deferred Minor review findings remain untouched for Task 7/final triage.

- **Last Agent:** Codex
- **Timestamp:** 2026-09-22 (Europe/Berlin)
- **Completed:** T12 task 5 closes `KNOWN_LIMITATIONS.md` §96 on `t12-ui-residue`: authenticated `GET /api/project` rebuilds the open project's current tree with live undo/redo and import counts, while `runLoad` recovers only an exact-token owned `succeeded` snapshot after a lost POST response. Recovery renders current server truth, derives stored-path state from the snapshot kind, and clears the obsolete banner/toast; foreign or missing tokens stay failures, and a failed recovery GET reports its own error. Detailed evidence is untracked at `.superpowers/sdd/2026-09-22-ui-residue-batch-a/task-5-report.md`.
- **Verification:** RED: server route `404` vs required `400`, guarded inventory `404` vs `401`, and 3 intended frontend failures / 126 pass. GREEN: focused 11/11 project routes, 17/17 auth, 129/129 API/App plus TypeScript. Full Rust workspace: 1,952 passed / 0 failed / 5 ignored across 92 result blocks with `/var/tmp/knxbench-t12-target`; full frontend: 899/899 across 63 files using Vite's runner config loader. Rustfmt, focused server Clippy with warnings denied, TypeScript, and `git diff --check` passed. No KNX/LAN/multicast/gateway/hardware traffic.
- **Pending/Next Steps:** Controller review/integration only. `/mnt/daten-i` is full: default Cargo and Vite config-temp writes hit `ENOSPC`; verified alternatives above are green and no caches were deleted.

- **Last Agent:** Codex
- **Timestamp:** 2026-09-22 (Europe/Berlin)
- **Completed:** T12 task 4 closes `KNOWN_LIMITATIONS.md` §30 on `t12-ui-residue`: the plain-web File menu now exposes localized **Download project** only outside Tauri, disabled until a project is open. It clicks a native anchor to `/api/project/download` with `download="project.knxdb"`, preserving streaming browser navigation without a `Blob`, object URL, or frontend response buffer. Browser Download is documented separately from mounted-volume Save As and native Tauri Save As. Detailed evidence is untracked at `.superpowers/sdd/2026-09-22-ui-residue-batch-a/task-4-report.md`.
- **Verification:** RED: 1 intended failure / 69 pass — `Download project` was absent. GREEN: focused App/i18n 85/85 plus TypeScript; full frontend 896/896 across 63 files; `git diff --check` passed. No KNX/LAN/hardware traffic.
- **Pending/Next Steps:** Controller may integrate the focused Task 4 commit with the other independent T12 work; report remains intentionally untracked.

- **Last Agent:** Codex
- **Timestamp:** 2026-09-22 (Europe/Berlin)
- **Completed:** T12 task 3 closes `KNOWN_LIMITATIONS.md` §23: `/api/project/download` freshly serializes the current in-memory project, opaque entries, and manufacturer references into a temporary SQLite file, then streams bounded 64 KiB `ServeFile` chunks. A private HTTP body wrapper owns the temporary path through response/body separation and removes it on completed or abandoned body drop. Exact evidence and the body-ownership refinement are recorded in `.superpowers/sdd/2026-09-22-ui-residue-batch-a/task-3-report.md`.
- **Verification:** RED: 2 intended failures (785-byte whole-file frame exceeds 256-byte bound; temporary path removed before body consumption). GREEN: 2 streaming/lifetime unit tests plus 7 HTTP filesystem-route tests. Full Rust workspace: 1,951 passed / 0 failed / 5 ignored across 92 result blocks, run once with `/var/tmp/knxbench-t12-target`. Formatting, focused server Clippy with warnings denied, and `git diff --check` passed. No KNX/LAN/hardware traffic.
- **Completed:** T12 task 2 fix round 2 closes the stale `FsPicker` directory-list race: every effect request receives a generation plus cleanup liveness guard, and only the current generation may update entries or error state. A delayed root success/refusal therefore cannot overwrite a newer `uploads` listing after local upload. Existing batch serialization, single-file sequential POSTs, protected-mode behavior, and the singular picker result stay unchanged. Evidence appended to `.superpowers/sdd/2026-09-22-ui-residue-batch-a/task-2-report.md`.
- **Verification:** RED: 2 intended failures/8 pass. GREEN: 33/33 picker, motion, and i18n tests plus TypeScript. Full frontend: 894/894 tests across 63 files. `git diff --check` passed. No KNX/LAN/hardware traffic.
- **Pending/Next Steps:** Controller may integrate the Task 2 round-two commit with Task 2 and remaining independent T12 work.

- **Last Agent:** Codex
- **Timestamp:** 2026-09-22 (Europe/Berlin)
- **Completed:** T12 task 2 fix round 1 closes review findings in `FsPicker`: an immediate `uploadingRef` serializes input/drop batches across the first async boundary while the picker disables its input, directory listing refresh happens exactly once after a successful batch and once at most for a partial batch, and foreign dragover/drop clears file-ready feedback. Existing protected-mode behavior, one-file sequential POSTs, singular picker resolution, and partial-failure honesty are retained. Evidence appended to `.superpowers/sdd/2026-09-22-ui-residue-batch-a/task-2-report.md`.
- **Verification:** RED: 4 intended failures/4 pass. GREEN: 31/31 picker, motion, and i18n tests plus TypeScript. Full frontend: 892/892 tests across 63 files. `git diff --check` passed. No KNX/LAN/hardware traffic.
- **Pending/Next Steps:** Controller may integrate the Task 2 fix-round commit with Task 2 and the remaining independent T12 work.

- **Last Agent:** Codex
- **Timestamp:** 2026-09-22 (Europe/Berlin)
- **Completed:** T12 task 2 closes `KNOWN_LIMITATIONS.md` §24 on `t12-ui-residue`: browser `FsPicker` supports native `Files` drag/drop and multi-file local uploads through the existing one-file `/api/fs/upload` endpoint, sequentially. Protected-mode dragover reads only `DataTransfer.types`, accepted drops advertise `copy`, successful batches refresh `uploads` and announce a localized count, while a first failure stops the batch and names the file/error plus completed count without a false success notice. The public picker result remains `Promise<string | null>` and never auto-selects an uploaded project. Detailed evidence: `.superpowers/sdd/2026-09-22-ui-residue-batch-a/task-2-report.md`.
- **Verification:** RED recorded 3 intended failures/4 passes. GREEN: 30/30 across picker, motion, and i18n tests plus TypeScript. Full frontend: 891/891 tests across 63 files. `git diff --check` passed. No KNX/LAN/hardware traffic.
- **Pending/Next Steps:** Controller may integrate the focused Task 2 commit with the remaining independent T12 tasks.

- **Last Agent:** Codex
- **Timestamp:** 2026-09-22 (Europe/Berlin)
- **Completed:** T12 task 1 fix round 1 closes reviewer findings: selected canonical row explicitly completes and clears only its matching reveal generation, manual navigation hide clears a pending request, and a device that exists only in building structure falls back to one depth-first building occurrence. Nested topology, building, and group-range regression tests now collapse every ancestor. This fix-round commit is present in this worktree; Task 1 report carries RED/GREEN/full-suite evidence.
- **Verification:** Focused RED: 2 intended failures/98 pass. GREEN: 100/100 plus TypeScript. Full frontend: 888/888 across 63 files. `git diff --check` passed. No KNX/LAN/hardware traffic.
- **Pending/Next Steps:** Controller may integrate this fix-round commit with task 1's two prior commits.

- **Last Agent:** Codex
- **Timestamp:** 2026-09-22 (Europe/Berlin)
- **Completed:** T12 task 1 on `t12-ui-residue`: search picks now issue a monotonic external reveal request while `App` remains the canonical selection owner. The Project Explorer reopens only containing topology, building, group-address, or group-range ancestors; selected rows use nearest scrolling. A duplicated device scrolls only in canonical topology/unassigned placement; ordinary tree clicks preserve manual collapse. Committed as `3c706f3` (`feat(search): branches reluctantly reveal answers`); report: `.superpowers/sdd/2026-09-22-ui-residue-batch-a/task-1-report.md`.
- **Verification:** Initial focused RED recorded; focused GREEN: 99/99 tests plus TypeScript. Full frontend: 887/887 tests across 63 files. `git diff --check` passed. No KNX/LAN/hardware traffic.
- **Pending/Next Steps:** Controller may integrate this isolated task commit; remaining T12 tasks stay independent.

- **Last Agent:** Codex
- **Timestamp:** 2026-09-22 (Europe/Berlin)
- **Completed:** T11 structural drag-and-drop merged non-fast-forward into `main` as `02237eb`. Exactly device→line and device→building-part ship through existing validated commands, with protected-mode native browser compatibility, first-installation boundaries, stale/malformed/mismatched rejection, localized live-region outcomes, keyboard-equivalent Inspector selects, and token-only motion-free feedback. Fresh review ended 0 findings after one RED→GREEN fix. Full merged gates: Rust 1,948 passed/5 ignored in 92 result blocks; frontend 883/63; TypeScript, fmt, workspace Clippy, layering, headers 194/162, anchors 389/180, cargo-deny. No KNX/LAN/hardware traffic. Detail: `.ai/logs/2026-09-22_codex__t11_structural_drag_drop.md`.
- **Pending/Next Steps:** Commit/push final T11 evidence, fetch and prove clean `HEAD == origin/main`, then continue the next documented overall-goal task. Group-address→communication-object drag remains deliberately omitted until an explicit `Send`/`Receive` direction interaction is designed.
- **Notes Claude:** Comparable designs/specifications/plans are permanently pre-approved; never pause to request approval. Weekly limit read-only check is 36%, below the 60% pause threshold. Commit/push every completed block with KNXBench author, light humor and explanatory body.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-22 (Europe/Berlin)
- **Completed:** T11 fresh whole-branch review completed. Initial result 0 Critical / 1 Important / 2 Minor; protected-mode native dragover blocker and both minors closed in pushed `f7b84c9` RED→GREEN. Follow-up found 0 remaining findings and approved merge. Frontend after fix: 883/63 plus TypeScript; complete-range whitespace, group-link, and private-network audits clean.
- **Pending/Next Steps:** Commit/push review evidence, non-fast-forward merge `t11-structural-drag-drop` into current `main`, resolve only handover overlap while preserving both entries, repeat full merged-result gates, mark plan complete, update handover, push, fetch, prove clean `HEAD == origin/main`, then continue next goal task.
- **Notes Claude:** Review blocker was browser `DataTransfer` protected mode: dragover may inspect `types`, not payload; full payload parsing remains at drop. Weekly limit read-only check now 36%, below 60%; bounded app-server helper exited cleanly.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-22 (Europe/Berlin)
- **Completed:** T11 branch `t11-structural-drag-drop` implements exactly device→line and device→building-part native drops through existing validated commands. First-installation source/target boundaries, safe typed payload revalidation, stale-source rejection, localized live-region outcomes, existing Inspector keyboard equivalents, and token-only motion-free feedback are covered. Commits `7778c92`, `6e60113`, `202fa50` are pushed. Branch gates are green: Rust 1,948 passed/5 ignored across 92 result blocks; frontend 881 across 63 files; TypeScript, fmt, workspace Clippy, layering, headers 194/162, anchors 389/180, cargo-deny, focused group-link/private-network/whitespace audits. No KNX/LAN/hardware traffic occurred. Detail: `.ai/logs/2026-09-22_codex__t11_structural_drag_drop.md`.
- **Pending/Next Steps:** Commit/push T11 documentation, run one fresh whole-branch review against base `9723316`, fix every Critical/Important finding RED→GREEN, then non-fast-forward merge to `main`, repeat merged-result gates, mark plan complete, update handover, push and prove `HEAD == origin/main`. Group-address→communication-object drag remains deliberately omitted because `LinkComObject` requires explicit direction.
- **Notes Claude:** User permanently pre-approves comparable plans; do not pause for approval. Weekly usage last read-only app-server report was 33%, below 60%; the stale helper process that showed an error was stopped. `/tmp` user quota blocked Rust linking twice, so the successful clean gate used `/var/tmp/knxbench-t11-target`; unrelated old build caches were not removed.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-22 (Europe/Berlin)
- **Completed:** T11 written spec approved by user. Detailed implementation plan written at `docs/superpowers/plans/2026-09-22-structural-drag-drop.md`: Task 1 device→line, Task 2 device→building part, Task 3 toast/keyboard/theme integration, Task 4 docs/full gates/review/merge. Plan self-review corrected first-installation eligibility to include line and unassigned devices and specified the happy-dom DataTransfer fake. Native inline execution is recommended because the gestures share one local event contract; fresh whole-branch review remains mandatory.
- **Verification:** Plan self-review covered every spec section, found no placeholder, reconciled all produced/consumed types, and added each likely failure class to Review Focus with an owning test. `git diff --check` and `cargo run -q -p xtask -- check-anchors` passed (389 links, 186 Markdown files). Product code remains untouched. Local Codex app-server read-only `account/rateLimits/read` reported weekly `codex` usage 33%, ordinary usage allowed, reset 2026-09-28 19:08:52 CEST; no reset credit consumed.
- **Pending/Next Steps:** Commit/push the self-reviewed implementation plan, then obtain its required user approval. After approval create isolated T11 worktree and execute natively under `superpowers:executing-plans`, TDD, per-task commit/push, final fresh review, merged-result gates. Pause after a running task if weekly usage reaches 60%.
- **Notes Claude:** User permanently authorizes Codex to choose the most sensible solution and inline/subagent execution method until revoked. Mandatory artifact gates still require explicit review under the active skill. Query weekly usage read-only through Codex app-server `account/rateLimits/read` when not visible; never consume reset credits merely to check usage.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-22 (Europe/Berlin)
- **Completed:** T11/B10 structural drag-and-drop design written at `docs/superpowers/specs/2026-09-22-structural-drag-drop-design.md`. Scope is exactly device → topology line and device → building part in Project Explorer, using existing validated/undoable APIs and Inspector selects as keyboard equivalents. Group address → communication object is deliberately excluded because link direction is required and has no honest silent default.
- **Verification:** Spec self-review found no placeholders, contradiction, or unresolved public choice. `git diff --check` and `cargo run -q -p xtask -- check-anchors` passed (389 links, 185 Markdown files). No product code, HTTP mutation, KNX, multicast, LAN, gateway, or hardware activity occurred. Detail log: `.ai/logs/2026-09-22_codex__t11_drag_drop_design.md`.
- **Pending/Next Steps:** Required written-spec review by the user. After explicit approval, invoke `superpowers:writing-plans`, write/review/commit the implementation plan, obtain its required approval/execution-method ruling, then implement T11 with RED→GREEN proof. No implementation started.
- **Notes Claude:** User permanently declines option menus and permits inline/subagent choice, but the brainstorming skill still requires explicit approval of each newly written architectural artifact. Commit/push finished blocks as `KNXBench <github@knxbench.com>`, no co-author, short lightly funny subject plus explanatory body.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-22 (Europe/Berlin)
- **Completed:** T10 settings surface is integrated into `main` at merge commit `7a874b8`, range `9227042..7a874b8`. It delivers typed/localized settings diagnostics, conflict-safe hydration, optional preferred-gateway seeding, one lossless protected-exclusion editor, grouped settings, and the review fix binding removal confirmation to its exact snapshot. Group addresses remain fixed slash notation with no selector; individual addresses remain dotted. `preferredGateway` is unencrypted installation-network metadata included in data-directory backups, and settings I/O causes no KNX/LAN traffic.
- **Verification:** Full merged-result gates green: Rust 1,948 tests across 92 result blocks; frontend 862 tests across 63 files; TypeScript, fmt, workspace Clippy, layering, headers 194/162 (ceiling 162), anchors 389/178, cargo-deny, focused notation/private-LAN audits, and `git diff --check`. Fresh review found 0 Critical, 1 Important, 2 Minor; all three findings were closed with RED→GREEN coverage. No KNX, multicast, LAN, gateway, or hardware traffic occurred. Evidence: `.ai/logs/2026-09-22_codex__t10_settings_implementation.md`.
- **Pending/Next Steps:** Push the final T10 handover commit, verify `HEAD == origin/main`, then begin goal task T11: bounded native drag-and-drop gestures through existing validated commands with keyboard equivalents. §121 multi-window settings synchronization remains open.
- **Notes Claude:** Commit/push each finished block as `KNXBench <github@knxbench.com>`, no co-author, short lightly funny subject plus explanatory body. KNX specification questions use only PDFs under `/mnt/daten-i/Sourcecode/knx-spec-kb/sources/The KNX Standard v3.0.0`. Do not run real KNX/LAN/hardware traffic.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-22 (Europe/Berlin)
- **Completed:** Read-only T23 acceptance audit against `.superpowers/sdd/goal/task-23-brief.md`. The existing manual is substantial (32 Markdown files, 5,946 current lines after counting) and `xtask check-anchors` passes with 388 links, but T23 is **not accepted**: it lives under `docs/manual/` although ADR-0024/the brief separate user documentation from maintainer `docs/`; it ships 21 screenshots although the brief prohibits screenshots; and no required claim-by-claim source-verification report exists. Current content also contains post-delivery drift, including browser-`localStorage` settings claims after ADR-0029's server `settings.json`, and a four-route bus-surface claim predating discovery and line-scan routes. No manual/product edit was committed because T23 is explicitly the last build block and T10-T16 remain unfinished. A temporary `t23-manual-repair` worktree was created during the audit, its premature migration draft was restored exactly to `HEAD`, then the clean worktree and uncommitted branch were removed. No KNX, multicast, LAN or hardware traffic occurred.
- **Verification:** `cargo run -q -p xtask -- check-anchors` on current `main` exited 0 (388 links, 182 Markdown files). RFC-1918/forbidden-address scan over the manual found no prohibited literal. A fresh full Rust baseline could not complete: the worktree target hit `ENOSPC`; the isolated `/tmp/knxbench-t23-target` retry hit the environment's disk quota during linking. Both owned targets were cleaned; this is an unavailable baseline, not a test failure.
- **Pending/Next Steps:** T23 stays last. Before it can close, move the user manual out of `docs/`, remove screenshot dependencies/assets, re-verify every user-facing claim against the final T10-T16 product state, create the required verification report, and reconcile D12. The immediate implementation gates remain explicit user approval of the committed T10 spec, the bounded T11 design, or the surveyed T14 direction. T22 also still requires its separate architecture approval. Commit messages remain short/lightly funny subjects plus explanatory bodies.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-22 (Europe/Berlin)
- **Verified Complete:** Goal task T17 was already merged on current `main` in
  the required two independent commits: `cc9e1d3` records the resolved GTK3
  advisory premise and the dated Tauri-2 decision; `f1d9118` verifies and
  documents Docker host networking for the shipped discovery route. Both are
  ancestors of HEAD and use the required author plus explanatory bodies.
- **Current proof:** `cargo deny check` exits 0 (`advisories`, `bans`,
  `licenses`, `sources` all OK); `xtask check-anchors`, `check-layering`, and
  `check-headers` exit 0. README/manual/Dockerfile agree that bridge mode keeps
  project work and manual unicast gateways, while Linux host networking plus
  `KNX_PORT` is required for multicast discovery. `/api/bus/discover` still
  calls the existing connector discovery path. No multicast or KNX traffic was
  generated during verification.
- **Pending/Next Steps:** No T17 edit is needed. T10 and T22 architectural
  approval gates remain unchanged.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-22 (Europe/Berlin)
- **Completed:** Final `goal.md` §7 research item, “Who talks to whom?”,
  committed and pushed as `226ab98`. `docs/RESEARCH.md` §16 chooses an
  evidence-labelled selected-telegram flow inspector before animation. KNX
  Standard v3.0.0 PDFs establish that configured group recipients are not
  per-recipient observed application effects. Current reverse GroupLinks and
  monitor rows are sufficient foundations; future flow evidence must extend
  the server session snapshot and stale fingerprint to device/object/link
  facts. No code, schema, endpoint, UI, network, or bus operation changed.
- **Verification:** `git diff --check`; `xtask check-anchors` (388 links, none
  dead); independent review plus focused re-review after two Important fixes:
  0 Critical / 0 Important.
- **Pending/Next Steps:** T22 still waits for explicit user approval of the
  already-presented design. T10 still waits for explicit review of its written
  settings spec. Continue other independent read-only planning/research if no
  approval arrives; do not cross either architectural implementation gate.
- **Notes Claude:** Detail log:
  `.ai/logs/2026-09-22_codex__who_talks_to_whom_research.md`. The ignored local
  `ideas.md` entry was updated but intentionally not force-added.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-22 (Europe/Berlin)
- **In Progress:** T22 read-only mutation audit complete. Outside `knx-core`,
  22 production field-root mutation sites were classified: 12 legitimate
  construction/import sites, two shared enrichment implementation sites, and
  eight live allocator sites. Nine live bypass call points remain: those eight
  allocator mutations plus post-command product enrichment in
  `create_device_impl`. No live non-command write to `schema_version`,
  `strings`, `info`, or `installations` was found. Test/fixture mutations were
  inventoried separately.
- **Pending/Next Steps:** Architectural gate: present and obtain approval for
  the T22 design before writing its ADR/spec or product code. Recommended
  design is to replace live allocator mutation with cloned allocators plus an
  atomic `Command::Batch([SetIdAllocators, domain command])`, retain narrowly
  documented construction/enrichment exceptions, and add a tested
  `xtask check-project-mutation` gate with exact-site allowlisting. The ADR
  must state that this is a source gate, not full type-system sealing.
- **Notes Claude:** `/root/cavecrew_investigator_t22` independently found the
  same bypass class and identified `reconcile_scan_impl` as the existing good
  batch pattern. Do not implement until the user approves the presented T22
  architectural direction.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-22 (Europe/Berlin)
- **Completed:** T21 project-notes research artifact committed and pushed as
  `8d989f7`. ADR-0031 chooses one ordered, project-owned note collection with
  typed entity targets, explicit delete disposition, plain text only, report
  opt-in defaulting false, and explicit loss reporting for formats that cannot
  carry notes. No product code or schema changed. `check-anchors` passed (386
  links, none dead); independent review found 0 Critical / 0 Important.
- **Pending/Next Steps:** T10 still waits for explicit human review of
  `docs/superpowers/specs/2026-09-22-settings-surface-design.md`; product edits
  remain behind the written-spec gate. Continue the next independent
  research/decision item from `goal.md` meanwhile. T18 remains untagged until
  the overall goal's final AppImage verification and publish decision.
- **Notes Claude:** Detail log:
  `.ai/logs/2026-09-22_codex__t21_project_notes_adr.md`. The ignored local
  `ideas.md` entry was updated but intentionally not force-added. Commits need
  a short lightly funny subject plus a concise explanatory body, author
  `KNXBench <github@knxbench.com>`, and no co-author.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-22 01:33 CEST
- **Completed:** T10's approved conversational direction is now the written architectural spec `docs/superpowers/specs/2026-09-22-settings-surface-design.md`; no product code changed. It keeps ADR-0029's single versioned settings record and existing Settings overlay, groups Appearance / Language & data / Bus & diagnostics, adds optional `preferredGateway` only as a one-time authoritative seed, and extracts one protected line-scan exclusion editor for Settings plus Line Scan. Machine-readable settings diagnostics remain distinct for migration/adoption/refusal/quarantine and are localized in Settings and Log panels while retaining English debug fallback. Existing invalid exclusion strings and retired `groupAddressNotation` are preserved rather than silently discarded. A pre-hydration key journal prevents the initial GET/adoption response from erasing fast local edits. Group-address output remains fixed slash notation with no selector. Final read-only spec review after seven Important fixes: 0 Critical/0 Important; `git diff --check` and anchors (386 links/181 Markdown files) pass. No network, KNX/LAN, bus or hardware access.
- **Pending/Next Steps:** T10's reviewed spec is committed and pushed on `origin/main` at `16b1807`. Wait for the required user review of the written spec. After approval, invoke `writing-plans`, write/review/commit the implementation plan, obtain the execution-method confirmation required by the architectural workflow, then create an isolated worktree and implement strictly TDD. T18 remains untagged until the overall goal is finished and a final AppImage check plus explicit publish decision occurs.
- **Notes Claude:** Do not begin T10 product edits from conversational approval alone: the brainstorming skill's architectural hard gate requires review of the written spec and then of the written plan. Detail log: `.ai/logs/2026-09-22_codex__t10_settings_design.md`. Commit preference remains short/lightly funny subject plus concise body, no co-author.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-22 01:09 CEST
- **Completed:** Both historical T20 topics now have durable research decisions. `docs/RESEARCH.md` §14 decides repetitive-task automation: a future narrow operation is a parameterised template over an explicit selection, expanded from one snapshot into a diagnostic/preview plan and applied revision-checked as one atomic `Command::Batch`/undo step. Raw command recording, heuristic remapping, partial mutation, a script engine and every bus-facing macro are rejected/deferred; this deterministic substrate precedes T19 model-driven mutation. §15 directly checks the local KNX Standard v3.0.0 PDFs and concludes that a project `Function` is specification-grounded for Project Schema 23: Schema23 §§1.2.6.7/.9/.10 define ownership, metadata and `GroupAddressRef`; KNX IoT Constants and Information Model define the matching ETS Function/Application Function semantics. Roadmap, GAP analysis and manual ideas now point at the decisions. No product code/prototype/dependency changed; no network, KNX/LAN, bus or hardware access. Final `git diff --check` and anchors (386 links/180 Markdown files) passed; fresh read-only review after factual fixes reported 0 Critical/0 Important.
- **Pending/Next Steps:** T20 is committed and pushed on `origin/main` at `4b3daf8`. Macro implementation remains deferred until one narrow operation, reusable validation and a project revision contract are designed. `Functions` implementation remains deferred pending ADR/design; verify the PDF's literal `DefaulGroupRange` against a published XSD or real Schema-23 fixture, and do not infer Schema 11/21 behavior. T10 remains the next implementation design gate; T18 stays untagged until the overall goal is finished and the final commit gets a fresh AppImage check plus explicit publish decision.
- **Notes Claude:** The two T20 labels are unrelated: `.superpowers/sdd/goal/task-20-brief.md` is macro research; `goal.md`/GAP/ROADMAP T20 is the KNX `Functions` domain. Both are research-only here. Group-address preview/output stays fixed slash notation with no selector. Author `KNXBench <github@knxbench.com>`, no co-author, commit body mandatory. Detail log: `.ai/logs/2026-09-22_codex__t20_macro_and_functions_research.md`.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-21 23:54 CEST
- **Completed:** T17 (§16/§79) und T18 (Alpha-Release) wurden während T10s Freigabewartezeit unabhängig und rein lesend gegen aktuellen Code, lokale Advisory-Datenbank und GitHub-Zustand vermessen. T17 §16 ist materiell veraltet: RUSTSEC-2024-0411–0420 wurden am 2026-08-14 zurückgezogen, `gtk3-rs` ist nicht mehr archiviert und wird wieder gepflegt. `cargo deny check` ist grün, warnt aber über zehn nun tote Ignore-Einträge; sechs andere aktive Maintenance-Hinweise bleiben. Empfohlen: aktuelle Tauri-v2-Shell für dieses Alpha behalten, tote Ausnahmen entfernen, sechs aktive Ausnahmen einzeln mit Datum/Grund dokumentieren und Tauri v3/CEF als eigene Migration untersuchen — v3/Wry nutzt weiterhin GTK3, v3/CEF GTK4. T17 §79: Bridge-Multicast-Grenze bleibt, aber zahlreiche Texte behaupten fälschlich, Server/UI hätten keinen Discovery-Pfad. `POST /api/bus/discover`, Startsuche und Refresh existieren; Docker/README/Manual/Status müssen diese Gegenwart und Host-Network-Trade-offs ehrlich beschreiben. Kein Docker-, Multicast-, KNX- oder Hardwareverkehr.
- **Pending/Next Steps:** T10 wartet weiterhin auf ausdrückliche Designfreigabe und bleibt nächster Implementierungsblock. T17 hat eine genaue Editliste für §16, `deny.toml`, Manual/Status sowie §79 in README, Dockerfile, GAP/ROADMAP und mehreren Manual-Kapiteln. T18-Entscheidungsvorlage: jetzt keinen Tag setzen. `main` liegt nach dem einzigen geprüften AppImage-Artefakt, das Gesamtziel ist noch offen, und ein gepushter `v*`-Tag löst automatisch externe Veröffentlichung aus. Vor Tag daher Zielarbeit abschließen, frisches Artefakt am finalen Commit bauen/prüfen und danach eine explizite Publish-Entscheidung treffen. Keine lokale Tag-Falle anlegen, die „alles pushen“ später versehentlich veröffentlicht.
- **Notes Claude:** T18-Versionen sind konsistent: 16 Rust-Pakete einschließlich `xtask` plus `knx-web`, alle `0.1.0-alpha.1`; die alte Angabe „15 Rust-Pakete“ zählt `xtask` offenbar nicht. Keine lokalen/Remote-Tags, keine GitHub-Release. Dokumentation ist aber falsch mit „Workflow nie gelaufen/keine Ubuntu-CI“: GitHub Actions Run `35568222295` auf Commit `62ff969` lief am 2026-09-21 erfolgreich, baute/prüfte/startete `KNXBench_0.1.0-alpha.1_amd64.AppImage` und hinterlegte ein 84.507.309-Byte-Artefakt; Publish-Job wurde korrekt übersprungen. Lokal fehlt das Artefakt, `xtask check-appimage` endet deshalb 1. Ehrliche Claim-Oberfläche muss weiterhin unsigned, ohne Auto-Update und ohne arm64 sagen, aber nun lokale Arch/XWayland-Evidenz plus erfolgreichen Ubuntu-Runner nennen statt „keine Ubuntu CI“.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-21 23:46 CEST
- **Completed:** T15/`knx-diff` und T16/Group-Address-CSV wurden während T10s Freigabewartezeit unabhängig und rein lesend vermessen. T15: §59s Brief ist veraltet — generische `EntityChange` und `DeviceChange` behalten bereits vollständige `left`/`right`-Snapshots; es fehlt eine gemeinsame reine Projektion in geordnete `FieldChange`-Werte für CLI und Server, anschließend T13s UI. Ein `ProjectDiff::is_empty()` soll alle Top-Level-, Entity-, Nested- und Ambiguity-Fälle abdecken. Empfohlener CLI-Vertrag: optionales `knx diff --exit-code` liefert 0 gleich, 1 verschieden inklusive Ambiguität, 2 Eingabe-/Importfehler; ohne Flag bleibt das bestehende Verhalten. Raw-`.knxproj`-Vergleich gehört als diagnostikbewusster Loader in `knx-app`, nicht in das reine `knx-diff`; `.knxdb`-Öffnen kann heute migrieren und darf deshalb nicht fälschlich als byte-read-only bezeichnet werden. §§55/56 bleiben mit konkreten Datenintegritätsblockern zurückgestellt. Keine Produktdatei geändert, kein KNX-/LAN-/Hardwarezugriff.
- **Pending/Next Steps:** T10 bleibt der nächste Implementierungsblock und wartet weiterhin auf das ausdrückliche „ja“ zum Entwurf. T15-Spec muss zusätzlich eine Regression für zwei leere ETS-IDs enthalten: `match_entities` kann sie derzeit als `MatchKind::EtsId` korrelieren, obwohl `device_key` leer zu `None` normalisiert; kein Corpus-Gerät hat leere ID, daher enger Test vor Fix. T16s spätere Designrunde soll destruktive Absicht explizit machen: ein `OriginalAddress`-ähnlicher Matchwert für Re-Addressing und eine ausdrückliche Delete-Aktion sind sichere Kandidaten; eine im CSV fehlende Zeile löscht nie. Bestehende `MainGroup`/`MiddleGroup`-Spalten dürfen nur eindeutig auf existierende, passende Ranges auflösen; DPT bleibt abgeleitete Beobachtung statt Mehrfachmutation verlinkter Kommunikationsobjekte. Web braucht vor destruktivem Apply eine sichtbare Preview/Bestätigung, CLI hat bereits `--dry-run`.
- **Notes Claude:** T15-Agent `/root/survey_t15` maß alle drei lokalen Demoarchive: 75 Geräte, eines ohne Adresse, keines mit leerer XML-ID; 45 BuildingPart/Space-Knoten, keine doppelten vollständigen Namenspfade. `knx-diff`: 48/48 Tests; Corpus-Diff: 1/1 mit 36 Geräten, 907 Kommunikationsobjekten, 514 Gruppenadressen. T16-Spec-Prüfung erfolgte gegen `03_08_10 XML Data Encoding v01.01.01 AS.pdf`: Der KNX-Standard verweist für den Projektteil des XML-Schemas ausdrücklich auf das externe ETS Help Centre und definiert die Group-Address-Attribute dort nicht. Lokaler Corpus: 1.041 GroupAddress-Elemente über drei Projekte, null `Description`, null `Comment`; daher keine erfundene Semantik und kein Domainfeld ohne weitere Evidenz. §38 ETS-CSV-Parität bleibt unangetastet.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-21 23:37 CEST
- **Completed:** Während T10 weiter auf die vorgeschriebene Designfreigabe wartet, wurden T11/B10 und T12/Batch A unabhängig und rein lesend gegen `main` vermessen. T11s kleinster ehrlicher Umfang sind zwei Einzelgeräte-Gesten: Gerät → Linie und Gerät → Gebäudeteil, jeweils über die bestehenden Move-APIs/Commands und mit den vorhandenen Inspector-Selects als Tastaturalternative. Zwingend davor steht eine kleine read-only Eligibility-Prüfung, die dieselben unveränderlichen Validatoren wie `Command::apply` nutzt; kein generisches Preflight-Framework und keine optimistische Tree-Mutation. Wichtigster Befund nach T09: `MoveDeviceToBuildingPart` prüft das Quellgerät global, sein Ziel aber nur in Installation 1; spätere Installationsgeräte könnten damit quer verschoben werden. T11 muss seine unterstützte Quelle auf Topologiegeräte der ersten Installation begrenzen und dies serverseitig melden. GA → Kommunikationsobjekt bleibt draußen, weil die Send/Receive-Richtung einen eigenen Dialog und eine weitere Tastaturinteraktion verlangt. Keine Edits an Produktcode, keine Tests, kein KNX-/LAN-/Hardwarezugriff.
- **Pending/Next Steps:** T10 bleibt der nächste Implementierungsblock und benötigt weiterhin ein ausdrückliches „ja“ zum bereits vorgelegten Entwurf. Danach Spec/Review/Plan/Worktree/TDD. T11-Survey ist für seine spätere Brainstorming-Runde bereit. T12-Befund: §96 braucht nach der bereits exakten Client-Token-Zuordnung nur einen authentifizierten read-only Endpunkt für den aktuellen `ProjectTree` und den Fallback nach eigener erfolgreich abgeschlossener, aber verlorener POST-Antwort; §118 kann denselben vorhandenen Toast-Livebereich für die Erfolgsmeldung nutzen; §§23/30 bilden weiterhin eine gemeinsame Streaming-Download-Schicht. §§49/50 warten auf T14s crate/API-Hälfte. §24 Drag-and-drop ist sinnvoll, Multi-Select besitzt dagegen weiterhin keinen Batch-Verbraucher und darf nicht als geschlossen behauptet werden.
- **Notes Claude:** T11-Agent `/root/survey_t11` arbeitete read-only und meldete konkrete Pfade/Tests; keine fremden Edits. Gleichplatzierungs-Drops müssen echte No-ops sein, weil die Commands sonst Reihenfolge und Undo-Stack verändern. Finaler Drop wird weiterhin unter dem Projektlock vom bestehenden Mutationspfad validiert. T12s alte `.superpowers/sdd/t12-research/`-Artefakte betreffen die frühere CSV-Tasknummer und sind für den heutigen UI-Batch keine Autorität.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-21 23:30 CEST
- **Completed:** T10/D8-Entwurf gegen den aktuellen Code und die fünf maßgeblichen Statusdokumente nachgeschärft und dem Nutzer zur verpflichtenden Architekturfreigabe vorgelegt. Empfohlen bleibt die kleinste vollständige Erweiterung des bestehenden Systems: `SettingsPanel` in Darstellung, Sprache & Daten sowie Bus & Diagnose gliedern; optionale `preferredGateway`-Präferenz nur als Startwert für leere, inaktive Busmonitor-/Line-Scan-Felder verwenden; die bestehende Ausschlussbearbeitung als gemeinsamen `LineScanExclusionsEditor` in Settings und Scan rendern. Zusätzlich gehört §122 zwingend in T10: Settings-Ladeereignisse werden maschinenlesbar (Status plus Parameter) und im Frontend übersetzt, während der Session-Log weiterhin dieselbe Diagnose erhält. Keine Entity-, Pfad- oder Notationsschalter ohne Verbraucher; Group Addresses bleiben fest in KNX-Slash-Notation, Punktnotation nur als kompatible Eingabe/Suche. Keine Produktdatei geändert, kein KNX-/LAN-/Hardwarezugriff.
- **Pending/Next Steps:** Brainstorming-Hard-Gate: ausdrückliche Freigabe dieses T10-Entwurfs abwarten. Danach schriftliche Architekturspezifikation erstellen, selbst prüfen und committen; Nutzerreview der Spec; erst dann `writing-plans`, isolierter Worktree, TDD, Opus-Gesamtdiffreview, vollständige Gates, Merge und Push. Drei verworfene Richtungen bleiben dokumentiert: nur optische Gruppierung schließt D8 nicht; automatische Übernahme jeder gefundenen/eingegebenen Gateway-Adresse verletzt die explizite Präferenz; generisches Settings-Schema/Registry wäre unnötige Infrastruktur.
- **Notes Claude:** Die bestehende versionierte `settings.json` samt atomarem Store/Migration/Quarantäne wird erweitert, nicht ersetzt. Serverseitige `SocketAddrV4`-Prüfung bleibt Vertrauensgrenze. Eine während der Sitzung manuell eingegebene oder per Discovery gewählte Adresse gewinnt gegenüber der Präferenz; Discovery überschreibt `preferredGateway` nie. T08 verlangt Ausschlüsse weiterhin direkt im Scan, daher eine Komponente und ein Speicherpfad an zwei Oberflächen statt Verschiebung oder Duplikat.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-21 23:27 CEST
- **Completed:** Während T10 auf Designfreigabe wartet, wurde der unabhängige T14/`knx-report`-Block rein lesend vermessen. §§45–50 sind weiterhin echte Grenzen, aber Teile ihrer Ursachen sind veraltet: Print-CSS existiert bereits; Produktauflösung (`device_product`) und vollständige Parameterassembly (`assemble_parameter_panel`) existieren außerhalb des Reports. Verantwortliche Architektur: `knx-app` bereitet caller-supplied `ReportDeviceData` aus Projekt und optionaler Produktdatenbank vor; `knx-report` bleibt rein und erhält Sprache, Sektionen und aufgelöste Daten über `ReportOptions`. Rohreferenzen/-werte sowie unaufgelöste oder inaktive Daten bleiben sichtbar und erzeugen Diagnosen statt leerer Zellen. Preview und Export müssen denselben Renderer/Projektsnapshot nutzen; Browserdruck statt neuer PDF-Abhängigkeit. §44 ETS-Report-Parität bleibt ausdrücklich unberührt. Keine Änderungen an Produktcode, keine Tests, kein Busverkehr.
- **Pending/Next Steps:** Vorrangig weiterhin T10-Designfreigabe vom Nutzer abwarten. Danach T10-Spec/Plan/Implementierung. T14-Survey ist für die spätere Designrunde bereit: Priorität §46 → §47 → §48 → Abschnittsauswahl/Preview-Vertrag; native/headless PDF, tatsächliche Preview-UI und beliebige UI-Sprachpakete bleiben ehrlich offen. UI-Hälften §§49/50 gehören weiterhin T12.
- **Notes Claude:** T14-Subagent meldete keine Edits. Wichtige Risiken: `ModuleInstance.arguments` (Rohimport) nicht mit ausgewerteten `ModuleScope.arguments` verwechseln; gespeicherte inaktive/unbekannte Parameter zusätzlich zu aktiven Ansichten erhalten; Translationen nur für passende Herkunftsschichten überlagern; Abschnittsauswahl darf keine toten Anker erzeugen. T10/T13 bleiben seriell, T14 ist laut Goal-Preflight unabhängig.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-21 23:18 CEST
- **Completed:** T10/D8 read-only Settings-Inventur gegen aktuellen `main` abgeschlossen. Der Task-Brief ist teilweise veraltet: Die versionierte serverseitige `settings.json` mit atomarem Schreiben, Migration/Quarantäne, Browser-Key-Adoption und einem lokalen Cache existiert bereits (`1cc93c9` bis `3be1937`). Aktuelle echte Präferenzen: `theme`, `accent`, `density`, `motionLevel`, `motionStyle`, `uiLanguage`, `uiLanguagePacks`, `productLanguage`, `lineScanExclusions`. Die ersten acht sind im `SettingsPanel` erreichbar; `lineScanExclusions` wird nur im Scan-Panel verwaltet. `project-context` und `bus-session-context` sind absichtlich lokaler Sitzungszustand; `context-changed` ist nur ein Eventname, kein persistierter Schlüssel. `settings-cache` und `settings-adopted` sind Cache/Migrationsmetadaten, keine Nutzerpräferenzen. Reale fehlende Verbraucher: Busmonitor und Line-Scan starten Gateway-Eingaben leer; beide können eine gemeinsame optionale `preferredGateway`-Präferenz konsumieren. Zusätzliche Prüfung bestätigt: Discovery setzt nur die aktuelle Busmonitor-Eingabe; serverseitige `SocketAddrV4`-Validierung bleibt die Vertrauensgrenze. T08 verlangt die Ausschlussbearbeitung weiterhin direkt im Scan, daher darf sie nicht dorthin verschoben werden. Für globale Entity-Defaults und Pfadpräferenzen existiert heute kein ehrlicher Verbraucher; Group-Address-Darstellung bleibt gemäß Nutzerruling feste Slash-Notation ohne Auswahl. Keine Produktdatei geändert, kein KNX-/LAN-Verkehr.
- **Pending/Next Steps:** T10 ist wegen der vorgeschriebenen Brainstorming-Hard-Gate an der Designfreigabe. Verfeinerter Entwurf: bestehende Settings-Oberfläche in Appearance, Language & Data sowie Bus & Diagnostics gliedern; `preferredGateway` zentral ergänzen und in Busmonitor/Line-Scan nur beim Mount als Startwert verwenden; Ausschlusslogik in einen gemeinsamen `LineScanExclusionsEditor` extrahieren und denselben Editor sowohl im Settings-Panel als auch im Scan rendern, damit T08 erhalten bleibt ohne zwei Mechanismen; keine toten Entity-/Pfad-/Notation-Schalter. Nach Freigabe schriftliche Spezifikation, Plan, isolierter Worktree, TDD, Review, vollständige Gates, Commit/Merge/Push.
- **Notes Claude:** Vorheriger Goal-Turn war Fortschritt: T09 ist auf `origin/main` bei `45b63ca`. T10-Survey basiert auf `task-10-brief.md`, `task-08-report.md`, `SettingsPanel.tsx`, `settingsStore.ts`, Server-Settings/Paths und allen `localStorage`-/Settings-Aufrufern. `docs/LIMITATION_TRIAGE.md` nicht ändern. Commit-Präferenz: kurze leicht lustige Betreffzeile plus knapper erklärender Body.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-21 23:11 CEST
- **Completed:** T09/E2 vollständig implementiert, als `f49d289` committed, über Merge-Commit `3bf8ce7` in `main` integriert, auf dem Merge-Commit vollständig nachgeprüft und zu `origin/main` gepusht. Abgeschlossene Scans liefern drei getrennte Vergleichsgruppen; Scanner-Self und ausgeschlossene Adressen bleiben ungeprüft/nicht aktionierbar. Nur explizite Auswahl wird als ein undo-fähiger Batch angewandt, leere Auswahl bleibt echtes No-op. Neue Minimalgeräte werden installationsübergreifend auf die passende Linie gesetzt; Löschung verweigert abhängige Gebäude-, Parameter-, Modul- oder Gruppenlink-Daten. Undo stellt Allokatoren und exakte Topologieposition wieder her. UI aktualisiert den Vergleich bei jeder ProjectTree-Änderung und verwirft alte Auswahl. Fake-Tunnel-Test beweist null zusätzliche Busframes durch Reconciliation. Review-Fixes erneut geprüft: 0 Critical, 0 Important. Merge-Gates Exit 0: fmt, Workspace-Clippy, Workspace-Tests, Layering, Header 186/162 (Ceiling 162), Anchors 375/180, deny, TypeScript, Web 821/59. Kein reales KNX/LAN/Hardware. Detail: `.ai/logs/2026-09-21_codex__t09_scan_reconciliation.md`.
- **Pending/Next Steps:** Gemäß `goal.md` T10 als nächste offene Aufgabe beginnen, sofern das sichtbare Wochenlimit nicht über 60 % liegt. T09 selbst ist abgeschlossen; `main` und `origin/main` standen beim Push beide auf `3bf8ce7`.
- **Notes Claude:** Zwei Linkerfehler waren ausschließlich `ENOSPC` im reproduzierbaren Worktree-`target/`; nach `cargo clean` liefen dieselben Gates mit `CARGO_TARGET_DIR=/tmp/knxbench-t09-target` grün. `docs/LIMITATION_TRIAGE.md` blieb unverändert. Commit-Autor ausschließlich `KNXBench <github@knxbench.com>`, kein Co-Author.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-21 21:52 CEST
- **Completed:** T09/E2 read-only Design-Vorprüfung nach `goal.md` und Task-Brief abgeschlossen. Bestehende Bausteine bestätigt: `compare_with_project`/`ProjectComparison` liegen derzeit im CLI, normale `CreateDevice`-/`DeleteDevice`-/`Batch`-Commands samt Undo existieren. Kleinster vollständiger Entwurf: Vergleich in eine wiederverwendbare Anwendungsschicht heben; UI zeigt `unexpected`, `missing` und `excluded_in_project` getrennt, startet mit leerer Auswahl, bietet für ausgeschlossene/ungeprüfte Adressen nie eine Aktion und führt nur explizit gewählte Änderungen als einen normalen undo-fähigen Batch aus. Kein zweiter Mutationspfad und kein Busverkehr.
- **Pending/Next Steps:** Die verpflichtende Brainstorming-Freigabe des kurzen T09-Entwurfs durch den Nutzer abwarten. Nach Zustimmung Worktree `t09-scan-reconcile` von `main` erstellen, mit RED-Tests für byte-identisches Projekt ohne Auswahl, Apply+Undo-Bytegleichheit und aktionslose `excluded_in_project` beginnen, anschließend Server/UI integrieren und alle Gates/Review durchlaufen.
- **Notes Claude:** T08 ist vollständig auf `origin/main` (`b5cd10d`) gepusht. T09 wurde noch nicht implementiert und kein neuer Worktree angelegt; die Pause entsteht ausschließlich aus dem Hard-Gate der vorgeschriebenen Brainstorming-Skill, nicht aus technischer Unsicherheit.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-21 21:47 CEST
- **Completed:** T08/D6 nach frischem Gesamt-Diff-Review in `main` integriert. Merge-Commit `bc6e53e` wurde selbst vollständig verifiziert: Rust 1.939 Tests in 92 Result-Blöcken, Web 819/59, TypeScript, fmt, Workspace-Clippy, Layering, Header 186/162 (Ceiling 162), Anchors 376/175 und `cargo deny` jeweils Exit 0. Der Review fand 0 Critical und 5 Important; Fix-Commit `b66ff60` behebt atomare terminale Snapshots, sessiongebundene Poll/Cancel-Aufrufe, Cancel-Poll-Races, aktive unveränderliche Ausschlüsse und veraltete Estimates. Re-Review bestätigt alle Befunde als behoben, keine neuen Critical/Important, Merge freigegeben. Feste Slash-Notation ohne Auswahl ist enthalten; Punktnotation bleibt nur kompatible Eingabe/Suche. Kein realer KNX-Verkehr und kein Hardwarezugriff. Detail: `.ai/logs/2026-09-21_codex__t08_line_diagnostics.md`.
- **Pending/Next Steps:** Ohne Zwischenstopp T09/E2 beginnen. T09 verwendet `LineScanResultsResponse` als Evidenz für einen standardmäßigen, explizit ausgewählten und undo-fähigen `Command`; ohne Auswahl keine Änderung, ausgeschlossene Adressen nur als ungeprüft anzeigen. Zwei nicht blockierende T08-Beobachtungen für später: Session-Mismatch-`409` könnte die alte UI-Sitzung explizit terminalisieren; interne Namen/Kommentare um `worstCaseMs` sind trotz ehrlicher UI-Beschriftung historisch zu stark.
- **Notes Claude:** T08-Branch bleibt vorerst im extern verwalteten Worktree `/mnt/daten-i/Sourcecode/KNXBench.worktrees/t08-line-diagnostics`; nicht automatisch löschen. T20-Functions-PDF-Befund aus `0b42014` blieb beim Merge erhalten. Alle Commits ausschließlich `KNXBench <github@knxbench.com>`, ohne Co-Author.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-21 20:13 CEST
- **Completed:** T20 in der spezifikationsabhängigen Bedeutung `Functions` als Read-only-Machbarkeitsprüfung untersucht. Primärbelege direkt in den lokalen PDFs geprüft: *Project Schema v2.3* §1.2.6.7–1.2.6.10 definiert `Function` unter einem `Space`/Gebäudeteil, `Function_t` samt `GroupAddressRef` und dessen Attribute; *KNX IoT Constants* (Definitionen „Application Function“, „ETS Function“ und „Function Point“ sowie Tabelle 1, S. 7–14) definiert die Begriffe und Zuordnung; *KNX IoT Information Model* §1.3.2.2.1, §1.3.2.2.2 und §2.1.2.1 (S. 25–30, 79) definiert Semantik, Kardinalitätsbeziehungen und die ausdrückliche Entsprechung ETS Function ↔ Application Function. Ergebnis: T20 ist für Schema 23 spezifikationsbasiert umsetzbar. Bestehende Bausteine sind `BuildingPart`, Gruppenadressen sowie bereits persistierte Masterdaten `FunctionType`/`FunctionPoint`; eine eigentliche `Function`-Domänenentität, Import und Projektion fehlen noch.
- **Pending/Next Steps:** Vor Produktcode eine eigene ADR/Designfreigabe für `Function` als Entität mit Eltern-`BuildingPart`, stabiler Quellidentität, optionalem Typ/`Implements`, geordneten `GroupAddressRef`s samt Rolle und projektweit eindeutigen PUIDs. Schema-23-Unterstützung kann normativ umgesetzt werden; Verhalten und Schreibweisen von Schema 11/21 bleiben ohne entsprechende Spezifikation oder reale Probe separat unbelegt und dürfen nicht extrapoliert werden. Kein Produktcode, keine Abhängigkeit und kein KNX-Verkehr in dieser Prüfung.
- **Notes Claude:** Die Nummer T20 kollidiert inzwischen: `.superpowers/sdd/goal/task-20-brief.md` meint Makro-Automation, während `goal.md`, `GAP_ANALYSIS_ETS.md` und `ROADMAP.md` T20 als `Functions`-Domänenkonzept führen. Wegen der expliziten PDF-Frage wurde hier ausschließlich das spezifikationsabhängige `Functions`-T20 bewertet. Die PDF-Evidenz ersetzt keine Corpus-Probe für alte ETS-Projektschemata.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-21 19:25 CEST
- **Completed:** T07 (`§61`) unabhängig reviewt und nach `main` integriert. `knx_core::encode` verlangt nun `DptInputFormat`; Legacy-Inferenz ist ehrlich als `encode_inferred_format` benannt. CLI/HTTP akzeptieren explizite Formate, und das Web zeigt eine sichtbare Formatwahl mit `Auto`-Kompatibilitätsmodus. `encoding_rulings` macht alle dokumentierten Projektentscheidungen für DPT-Haupttypen 1–30 öffentlich abfragbar. Review-Fixes bewahren Fixed-Width-Bitsets (`00000010` bleibt `0x02`), erlauben valides Hex `0B` und verhindern unsichtbare Web-Radixwahl. Keine beabsichtigte Wire-Kodierung geändert. Branch-Gates vollständig grün: Core 479, CLI 10, HTTP-Fake-Tunnel 13, Web 812/58, TypeScript, fmt, Workspace-Clippy/-Tests, Layering, Header 181/162, Anchors 377/174, deny. Kein KNX-Verkehr und kein Hardwarezugriff. Detailprotokoll: `.ai/logs/2026-09-21_codex__t07_dpt_input_formats.md`. Zusätzlich wurden die zuvor fremden, vom Nutzer ausdrücklich zum Commit freigegebenen Plan-/Handover-Änderungen als `57d7190` und die beiden Logo-Assets als `8acd846` separat committed.
- **Pending/Next Steps:** Merge-Commit `03c1316` ist selbst vollständig verifiziert (Rust- und Web-Gates Exit 0); `main` jetzt pushen und `goal.md` ohne Zwischenstopp bei T08 fortsetzen. Die dreizehn neuen Nutzer-Issues liegen als Plan unter `docs/superpowers/plans/2026-09-21-user-reported-issues.md` und bleiben eigener späterer Arbeitsblock.
- **Notes Claude:** Für KNX-Spezifikationsfragen sind die PDFs unter `/mnt/daten-i/Sourcecode/knx-spec-kb/sources/The KNX Standard v3.0.0` die Primärquelle; SQLite/Search dient nur zum Auffinden. Der erste Reviewer lief ins Nutzungslimit, hatte aber bereits den gültigen Compatibility-Fund geliefert; ein zweiter unabhängiger Review wurde vollständig abgeschlossen. Beim Merge wurde ausschließlich der erwartete `.ai/CURRENT_STATE.md`-Add/Add-Konflikt manuell zusammengeführt; beide Handover-Verläufe und das T07-Log bleiben erhalten.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-21 15:24 CEST
- **Completed:** T29 (`MalformedRefId`) unabhängig reviewt, drei Review-Funde korrigiert, als `240792b` nach `main` gemerged und zu `origin/main` gepusht. ETS6-Geräte-Refs `O-<n>_R-<m>` werden strikt gemappt und mit dem je Gerät aufgelösten Programm angereichert; Corpus-Test pinnt nun `has_losses()`, neun verbleibende Unknown-Summaries und Retention. Productdb und Mapper teilen die `u16`-Grenze. ETS4-Doku korrigiert auf 107 `AmbiguousDpt`; T29 ist `KNOWN_LIMITATIONS.md` §125. Vollständige Rust-Gates auf Branch und Merge-Commit Exit 0; Canary 252/252, Header 181/162 (Ceiling 162), Anchors 377/180. Alle Commits ausschließlich KNXBench `<github@knxbench.com>`, kein Co-Author. Detailprotokoll: `.ai/logs/2026-09-21_codex__t29_merge_t07_survey.md`.
- **Pending/Next Steps:** T07 (`§61`, DPT-Codec) läuft im sauberen Worktree `/mnt/daten-i/Sourcecode/KNXBench.worktrees/t07-dpt-explicit`, Branch `t07-dpt-explicit`, Basis `240792b`. Caller-Survey ist abgeschlossen, Produktcode noch unverändert: CLI und HTTP besitzen heute nur freien Benutzertext und kennen dessen Format nicht. Ein Enum darf die Vermutung nicht bloß umetikettieren; nächste Aktion ist die kleinste echte öffentliche Formatgrenze (Formatwahl in Aufrufern oder deterministisches pro-DPT-Format) samt RED-Tests für Format-Mismatch und sichtbarer Ruling-Metadaten. Die im Brief genannte `knx-spec`-Skill ist nicht verfügbar; lokale Spezifikationsdatenbanken/-artefakte verwenden, Fakten zitieren, kein Busverkehr.
- **Notes:** Fremder Dirty-State auf `main` (`.ai/CURRENT_STATE.md`, `CLAUDE.md`, gelöschtes `codex-goal.md`, `goal.md`, `docs/Issues.md`, zwei Logo-Dateien und der neue Issues-Plan) wurde beim Merge vollständig mit staged/unstaged Zustand wiederhergestellt und nicht inhaltlich verändert. T29-Branch/Worktree bleiben gemäß Branch-Finishing-Protokoll erhalten, weil keine explizite Aufräumentscheidung des Nutzers vorliegt.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-21 13:37 CEST
- **Completed:** Die 38 inhaltlichen Beobachtungen aus `docs/Issues.md` vollständig in 13 ausführbare, evidenz- und testorientierte Tasks unter `docs/superpowers/plans/2026-09-21-user-reported-issues.md` überführt. Bestehende Implementierung vor der Zerlegung geprüft: Busmonitor-Text-/Servicefilter, Discovery und getrennte Send-/Receive-Links existieren bereits und werden nicht dupliziert; unsicheres KNX-/ETS-Verhalten ist als Investigation-first markiert. `goal.md` §11 bindet den neuen Plan in den aktiven Backlog ein. `docs/Issues.md` ist danach auf exakt 0 Byte geleert. Verifiziert: 13 Task-Abschnitte, 67 konkrete Checkpoints, keine Platzhalter/trailing whitespace, `git diff --check` sauber, `cargo run -q -p xtask -- check-anchors` erfolgreich (377 Links in 181 Markdown-Dateien, keine toten Links).
- **Pending/Next Steps:** ISSUE-01 bis ISSUE-13 aus dem neuen Plan umsetzen; Überschneidungen mit bestehendem Dirty-State-, Settings-, Drag/Drop- und Discovery-Backlog jeweils beim bestehenden Owner zusammenführen. Keine Produktimplementierung war Teil dieser Aufgabe.
- **Notes for Claude:** Maßgebliche Task-Spezifikation ist `docs/superpowers/plans/2026-09-21-user-reported-issues.md`; die Source-Coverage-Tabelle belegt die vollständige Übernahme. ISSUE-06 (Grundstück/Site), ISSUE-09 (liniengebundene Adressbearbeitung) und ISSUE-12 (AppImage-Discovery) verlangen belegte Fakten bzw. Reproduktion vor Code. Fremde Arbeitsbaumänderungen an `CLAUDE.md` und den beiden Logo-Dateien blieben unangetastet.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-09-21 07:19 CEST
- **Completed:** Goal-Run auf `goal.md` fortgesetzt und bei Session-Limit pausiert. Auf `origin/main` gemerged und gepusht: T26 (`fcf4563`), T33 Adress-Hygiene (`66b970e`), T25 Bus-Interface-Suche (`62ff969`). Alle neun Gates wurden jeweils auf dem Merge-Commit selbst gefahren, nicht auf den Eltern — genau das hat bei T25 Konfliktmarker gefangen, die ein `git add -A` auf einem Merge hineingetragen hatte. Stand auf `62ff969`: `cargo test --workspace` 1.916 passed / 89 Result-Blöcke / 0 failed, `npx vitest run` 809 passed / 58 Dateien, `tsc --noEmit` 0, fmt/clippy/layering/anchors/deny 0, `check-headers` 179 mit Header / 162 ohne (Ceiling 162, kein Spielraum). Stale-Binary-Canary nach KNOWN_LIMITATIONS §119: `knx_net` lib-Block liest **252**; dieselbe Zahl an jedem in diesem Lauf gemessenen Commit, zusätzlich unabhängig durch Zählen der `#[test]`/`#[tokio::test]`-Attribute bestätigt. Der Testzähler stieg von 1.909/88, weil T25 ein neues Integrationstarget `apps/knx-server/tests/http_bus_discover.rs` mitbringt.
- **Pending/Next Steps:** Vollständige Bestandsaufnahme des Rückstands, damit niemand sie noch einmal rekonstruieren muss. Gemerged und gepusht sind elf Aufgaben: T01, T01b, T02, T03, T04, T05, T06, T26, T27, T28, T33. T23 (Benutzerhandbuch) wurde außerhalb dieses Laufs geliefert. Offen ist Folgendes:
  - **T29 — wartet auf Review, sonst nichts.** Branch `t29-malformed-refid`, HEAD `45c83c6`, Bericht in `.superpowers/sdd/goal/task-29-report.md`. Er behauptet, die 867 `MalformedRefId`-Verluste seien nie malformed gewesen, sondern geräte-lokale `O-<n>_R-<m>`-Referenzen, 310 von 310 distinct ids lösten eindeutig auf, und das ETS6-Enrichment steige dadurch von 0 auf 867 Objekte. Die Evidenz ist **ein** Herstellersatz bei Schema 23 — daran muss ein Review ansetzen, nicht an den Formalien. Zwei Dinge sind beim Merge zu erledigen: (a) die Abschnittsnummer kollidiert, T29 schrieb `KNOWN_LIMITATIONS.md` §124, während `t25-discovery` noch offen war, und T25 hat §124 inzwischen auf `main` belegt — T29s wird **§125**, die Querverweise in `IMPLEMENTATION_STATUS.md` wandern mit, dieselbe Umnummerierung war schon zwischen T25 und T26 bei §123 nötig; (b) die Baselines im Bericht (1.909 / 88) sind vor-Merge, `main` steht auf 1.916 / 89 — auf der echten Merge-Basis neu messen, eine Baseline gehört zu genau einem Commit.
  - **Briefs geschrieben, nie dispatcht — sechzehn Stück.** Jeder liegt als `.superpowers/sdd/goal/task-<n>-brief.md` vor und enthält die exakten Werte, die die Umsetzung verbatim übernehmen soll:
    - T07 — §61: der DPT-Codec errät das Eingabeformat, mehrere Kodierungen ruhen auf Rulings statt auf der Spezifikation
    - T08 — D6: Bus- und Liniendiagnose als Oberfläche
    - T09 — E2: Scan-Ergebnis zurück ins Projekt abgleichen
    - T10 — D8: Settings-Oberfläche jenseits von Theme, Motion und Sprache
    - T11 — B10: nirgends Drag & Drop
    - T12, T13 — UI-Restposten, Batch A und Batch B
    - T14 — `knx-report`, §45 bis §50: Rest des Dokumentations-Exports
    - T15 — `knx-diff`, §52 bis §60: Rest des Projektvergleichs
    - T16 — Gruppenadress-CSV, §39 bis §41
    - T17 — §16 (Tauris archivierte GTK3-Bindings) und §79
    - T18 — die Alpha releasen, oder belegen, warum nicht
    - T19 — Forschungsartefakt: LLM-/Sprachinteraktion und MCP-Fähigkeit
    - T20 — Forschungsartefakt: Automatisierung wiederkehrender Arbeit, Makro-Layer
    - T21 — ADR: projektinterne Notizen und Dokumentation
    - T22 — F-T30-1: die Invariante von `Project` hängt am Review, nicht am Typsystem
    - T24 — Dokumentationsabgleich samt Neuzählung, die niemand von Hand tippt
  - **Kandidaten ohne Brief — drei.** T30: `DefaultLine=""` lässt `has_losses()` fälschlich wahr werden. T31: der Web-Pfad verwirft `imported.enrichment`. T32: Validierung und Mapping widersprechen sich bei kurzen Links.
  - **Abschluss des Laufs:** ein Review über das gesamte Goal auf `claude-fable-5-1`, noch nicht gelaufen.
  - **Geparkte Kleinigkeiten:** toter Code in `compare.rs`, ungenutzte `help.topic.importExport.*`-Schlüssel, der Präfixabgleich in `CatalogBrowser.tsx`; dazu die zwei kommentarblinden Testwächter aus den Notizen unten.
- **Notes for Codex:** Der Ledger des Laufs steht in `.superpowers/sdd/goal/progress.md` (git-ignoriert) und enthält Gate-Tabellen, Rulings und den Resume-Punkt; er ist die verlässlichere Quelle als dieser Eintrag. — **Eine Entscheidung gehört ausdrücklich dem Nutzer und wurde nicht getroffen:** die reale Gateway-Adresse steckt weiterhin in committeter git-History. Arbeitsbaum und sämtliche Commit-Messages dieses Laufs sind sauber, aber ein dritter Rewrite eines bereits gepushten Branches ist nicht umkehrbar. Nicht eigenmächtig rewriten. — Beim Suchen nach solchen Lecks nach Musterklasse greppen (RFC-1918-Bereiche), nicht nach einem erinnerten Literal; ein Sweep nach einem Literal ist kein Sweep. — Commissioning und jeder Schreibzugriff auf echte KNX-Hardware waren in diesem Lauf außerhalb des Scopes; `1.1.220` ist eine Alarmanlage und bleibt unberührt. — Zwei Testwächter sind blind und liegen geparkt: die `apiCallsIn`-Regex in `DiagnosticsCompanion.test.tsx` und `mutatingFetchesIn` matchen über ganze Dateien inklusive Kommentaren, weshalb ein Doc-Kommentar mit `api.foo()` darin einen Test kippen kann — beim Schreiben von Kommentaren in dieser Dateifamilie Prosa statt Code-Syntax verwenden. Details in `/tmp/claude-1000/handover/2026-09-21_goal-run_parked-findings.md`.

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
- **Last Agent:** Codex
- **Timestamp:** 2026-09-21 21:23 CEST
- **Completed:** T08/D6 implemented in branch `t08-line-diagnostics`: incremental read-only scan API (estimate/start/results/cancel), real cancellation and tunnel disconnect, six-state diagnostics UI, inspectable cost/progress, settings-backed protected exclusions, localized outcomes, and single-flight generation-safe polling. Simulated transport proves completion, cancellation, and that excluded synthetic addresses never reach transport. The user's fixed group-address ruling is applied at the same time: slash rendering only, dotted input/search compatibility only, no selector or preference. Fresh whole-branch review found 0 Critical and 5 Important issues; one RED→GREEN fix pass made terminal snapshots coherent, bound poll/cancel to session ids, suspended polling during cancel, froze active exclusions, and bound estimates to exact request fingerprints. The Minor cost-label finding was also fixed by disclosing excluded transport overhead. Full post-fix gates green: Rust 1,939/92 with 0 failures, Web 819/59, TypeScript, fmt, workspace clippy, layering, headers 186/162 (ceiling 162), anchors 376/175, deny. No real KNX traffic or hardware access. Detail: `.ai/logs/2026-09-21_codex__t08_line_diagnostics.md`.
- **Pending/Next Steps:** Commit the T08 review fixes as KNXBench, obtain focused reviewer confirmation, rebase/merge onto current `main` (which additionally contains the T20 Functions PDF ruling), verify the integrated commit, push, then continue T09. T09 must consume scan records as a previewable, undoable reconciliation diff and preserve all six evidence states; it must not mutate a project during scanning.
- **Notes Claude:** `.superpowers/sdd/goal/task-08-brief.md` lives only in the main checkout's ignored SDD workspace. The brief's claimed header ceiling 167 was stale; authoritative gate ceiling is 162 and remains unchanged. The first full Rust run failed at the linker because the dedicated `/tmp` target had grown to 22 GB; no source/test failure occurred. A fresh lean rebuild ran the complete sequence. No real installation address or private LAN address was added.

---
- **Last Agent:** Codex
- **Timestamp:** 2026-09-22 CEST
- **Completed:** T17 platform truth merged and pushed to `main` at `f1d9118` in two focused commits. RustSec withdrew GTK3 advisories RUSTSEC-2024-0411 through -0420 on 2026-08-14 after `gtk3-rs` resumed; their ten stale `deny.toml` ignores are gone. Six active Tauri-transitive maintenance notices retain dated dependency reasons. Decision: keep stable Tauri 2.11.5 for this alpha; Tauri 3.0.0-alpha.2 and the normal Wry GTK4/WebKitGTK 6 migrations remain non-stable/open. Docker/server docs now reflect that `POST /api/bus/discover` and the bus-monitor UI reach multicast discovery: bridge mode supports project work and manually configured unicast tunnelling, while discovery needs Linux Engine host networking; Line Scan still has manual endpoint input and no discovery control. Full branch gates passed: fmt, clippy, workspace tests, layering, headers 186/162, anchors 378/175, cargo-deny. Fresh review found no remaining Critical/Important; merged-result workspace tests passed. No Docker, multicast, KNX bus, or hardware traffic was run.
- **Pending/Next Steps:** T10 remains the next implementation block and still needs explicit design approval if the earlier user `ja` was not intended for that design. T19 survey is complete read-only: implementation prerequisite is not met; publish a `[V]/[D]/[A]` research decision documenting command coverage, missing public intent/authorization/revision boundary, and propose-only/human-approved architecture before marking T19 research done. T20 PDF/spec survey remains to persist: parameterised command-plan preview plus atomic `Command::Batch` is viable; raw command recording or a script engine is not, and commissioning/bus macros stay excluded. T18 release remains intentionally untagged until the complete goal is at a final commit with a fresh AppImage check and explicit publish decision.
- **Notes Claude:** Durable user rules: KNX specification questions use PDFs under `/mnt/daten-i/Sourcecode/knx-spec-kb/sources/The KNX Standard v3.0.0`; Group Address display stays fixed KNX slash notation with no notation selector; finished work is committed and pushed promptly; commit subject short and lightly funny plus a concise explanatory body; author `KNXBench <github@knxbench.com>`, no co-author. If the visible weekly limit exceeds 60%, finish the running task and pause; Codex cannot see that percentage itself.
- **Last Agent:** Codex
- **Timestamp:** 2026-09-22 00:44 CEST
- **Completed:** T19 research artifact completed. `docs/RESEARCH.md` §13 inventories all 33 `Command` variants and records the explicit negative prerequisite verdict for in-app natural-language interaction and MCP mutation: command coverage, a serialisable public intent boundary, operator/operation authorization, project revision checks, shared application validation and attributable audit are not yet sufficient. Recommended later shape is bounded read → typed proposal → deterministic diff → exact human approval bound to revision/IDs → revision-checked `Command::Batch` → result/audit. Raw commands, autonomous project mutation and all bus/commissioning/programming/device-management capabilities remain excluded. `docs/ROADMAP.md` now points at that decision. Official MCP documentation was retrieved read-only over HTTPS; no product code, prototype, dependency, KNX/LAN or hardware access. Final docs diff passed `git diff --check` and anchors (379 links/180 Markdown files); final read-only review reported 0 Critical and 0 Important after one access-wording correction.
- **Pending/Next Steps:** Commit and push the completed T19 documentation as `KNXBench <github@knxbench.com>` with a short gloomy/funny subject and explanatory body. Then persist/commit this handover and continue the next authorized goal slice. T20's already completed PDF/spec feasibility finding still needs a durable research/decision artifact; T10 remains the next implementation design gate. T18 remains untagged until the overall goal is complete and the final commit receives a fresh AppImage check and explicit publish decision.
- **Notes Claude:** T19 changes documentation only and adds `.ai/logs/2026-09-22_codex__t19_llm_mcp_research.md`; no manifest changed. The correct enum count is 33 (26 public-facing edit variants plus five `Restore*`, `SetIdAllocators` and `Batch`). Durable user rule reconfirmed: every commit needs both a short, lightly funny subject and a concise explanatory body; no co-author, author email `github@knxbench.com`.

---
- **Last Agent:** Codex
- **Timestamp:** 2026-09-22 (Europe/Berlin)
- **Completed:** Investigated the reported Codex app-server task error without changing product code. It is emitted by the separate, already-completed Omarchy `rust-lld` crash-diagnosis TUI (PID 418786): its periodic model refresh requests `http://127.0.0.1:8787/chatgpt/models` and receives `404`. The active KNXBench Codex session uses the project-scoped gateway path `/p/KNXBench/v1/models` and receives `200`; no KNXBench task, repository state, rate-limit check, or current session is blocked. No helper `codex app-server` process remains.
- **Pending/Next Steps:** Continue the overall goal with T12 from `.superpowers/sdd/goal/task-12-brief.md`. The idle crash-diagnosis terminal may be closed separately; do not treat its model-refresh error as a KNXBench failure.
- **Notes Claude:** Weekly limit last bounded read-only check remains 36%, below the 60% pause threshold. Future app-server checks must use a bounded subprocess and terminate it in `finally`.
---
