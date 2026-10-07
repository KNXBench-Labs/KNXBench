# Alpha-readiness evidence and dispositions

## New Telegram-flow Alpha prerequisite — user decision 2026-10-04

The user confirmed and authorized [the session-local flow contract](TELEGRAM_FLOW_VISUALIZATION.md)
for the Alpha. UI U19–U21 and alpha AR20/AR21 are additional open package work,
not a reinterpretation of completed source IDs or a silent scope waiver.
Final readiness/artifact/review requires AR21's integrated flow acceptance;
AR19 still requires separate release consent. Existing owner exceptions,
commissioning safety gates and the frozen source-ID inventory remain intact.
A plan is not a vertical feature receipt: immediate values/7-second expiry,
configured recipients, directed traffic, freeze, retained resting lines,
theme/motion and explicit overload/loss handling must be actually verified.


## AR00 reconciliation baseline

User-started offline alpha queue, 2026-10-01. Inspected source/maintained docs
at `307a5970ad5147437dffce4a86047e85e3319186`; the dated input inventory
[OFFENE_PUNKTE](archive/OFFENE_PUNKTE.md) is unchanged. This is dispatch/provenance
reconciliation, not a fresh full implementation, security or compatibility audit.
The execution owner is [alpha-release-goal](../alpha-release-goal.md); historical
[goal](archive/goal.md) is not a second executor. UI/commissioning remain their owners.

### Evidence adopted, not relabelled

- UI source `36e6b6af`, closure `dfa0cc79`, receipt `8a51b74d` are ancestors
  of the baseline. U0–U13 and both ISSUE-12 boxes are closed. Original
  independent GPT-6.1-Sol review: **changes required**, user-approved reviewer
  substitution; subsequent fixes and separate in-session review are not a
  second independent verdict. See `.ai/logs/2026-10-01_codex_ui-u13-fixes.md`
  and [implementation status](IMPLEMENTATION_STATUS.md).
- Native WebKitGTK/real screen-reader/discovery multicast are still unverified.
  A `DONE` UI acceptance inventory row closes that dated U13/checkbox task,
  not those separate platform boundaries or optional owner follow-ups.
- Commissioning cleanup/worker/directory hardening receipts are adopted at
  their actual offline scope. Complete per-device recovery and new exact-write
  authorization remain missing; no hardware operation is authorized here.
- Startup worktree/process inventory found no other active alpha/old-goal
  Cargo or CLI executor. The existing commissioning/review checkouts are
  foreign and untouched. This observation is not a permanent concurrency lock.
- Root research/statistics/handover and unpublished telemetry/shutdown files
  are foreign. They are not copied, edited or committed. Paperclip's old
  `7b64496` observation is historical, not current dispatch authority.
- Statistics refresh is blocked on the canonical root's foreign modified
  `stats.md` and integrated-history synchronization by its owner. Safe fallback:
  retain the report untouched, do not fabricate a refresh; unblock only when
  the owner provides a current canonical checkout and safe report disposition.

### Stale assertions verified from implementation and existing tests

| Former task | Actual contract / evidence inspected |
| --- | --- |
| KL-18 | `domain::open_project` passes `None` to `replace_project_state`; replacement transaction atomically sets `store_path`. Desktop delegates to this domain. `http_project_routes::importing_replaces_a_dirty_project_with_a_clean_baseline` asserts no imported store path. |
| KL-23 | `fs_routes::stream_temp_file` uses bounded `ServeFile` frames and body-owned temporary path. Both `fs_routes` cleanup/stream tests and `http_fs_routes` unsaved-project download regression exist; SQLite serialization still creates a temporary file. |
| KL-24 | `FsPicker.tsx` has `multiple`, native file drop and sequential `uploadFiles`. `FsPicker.test.tsx` tests sequential upload, protected-mode drop, conflicts and partial failure. Opening a project remains singular. |
| ISSUE-04 | Issue-plan rows are ticked; `http_project_routes` saved-edit/save-failure/undo-baseline tests and `App.test.tsx` locale/success-only timestamp test match the actual contract. |

These are source/test inspections plus adopted historical owner runs, **not new
product-test executions**. AR00's own doc/ledger checks are recorded in
`.ai/logs/2026-10-01_codex_alpha-queue.md`. Old headings and links are retained.

## AR01 verification delivery

The executable uses the selected runtime workspace, validates its identity,
pins metadata to that manifest and refuses missing/empty scan scopes. All
checked layering roots must be actual workspace members. The target and
coverage are emitted; see [verification targets](VERIFICATION.md).

Executed: old removed-worktree zero-file success; new deleted-target refusal;
75 unit and 12 CLI integration tests, strict xtask Clippy, fmt and all four
repository scan gates. Five guard mutations fail behaviorally and sources are
restored. Scope: 448 graph packages, 370 valid / 160 absent headers (ceiling
160), 17 generated files excluded, 323 corpus-lint Rust sources. This is not
a full product/workspace test or a real-corpus execution. In-session review
found and fixed a transitive-only policy-root gap and the missing touched-file
header; the mutation tests cover that gap after fixing the fixture exclusions.
Details: `.ai/logs/2026-10-01_codex_alpha-gate-scope.md`.

## AR02 general ID exhaustion

All nine project-local allocators now refuse exhaustion with a typed error.
The final representable ID and its entities roundtrip at native schema v9;
failed CSV/reconciliation batches apply nothing, and existing parameter edits
remain possible. Tests assert unchanged project/counters, maximum-ID
undo/redo/save/reopen, CLI exit 2 and HTTP refusal, not just a failure label.
Mapper seeded-boundary tests abort detached construction and preserve input.
See [model contract](DATA_MODEL.md) and
`.ai/logs/2026-10-01_codex_alpha-id-exhaustion.md`.

Final offline gates, 2026-10-01 UTC receipt: 141 Rust result blocks, 2,855 passed,
zero failed, 161 ignored; 77 private corpus/roundtrip tests explicitly run,
zero failure/ignored/skip markers; Web 1,312; strict Clippy, dependency policy,
typecheck, build, headers/anchors/layering/corpus policy and patch checks pass.
Three compiled behavioral mutants fail and original source is restored.
Native/ETS interoperability, actual hardware and whole-product release
acceptance are not inferred from those tests. The catalog scope and numbered
limitation counts below remain unchanged; ADR-0039 phases 3–5 still require
explicit activation (prompt unanswered, not approved).

## AR03 enforcement audit and reserved decision

[Pinned source audit](ADR0039_ENFORCEMENT_AUDIT.md) at published AR02
`e691bc1318d0785289f8132378a0f26c9a829b27`: phases 1–2 are already implemented;
the live surface has changed since the ADR's historical count. A bounded
three-package proposal is documented, not activated. `KL-129`/AR03 remain
`WAITING_DECISION`; no user answer is not approval or accepted continued
deferral. This package runs doc/ledger gates only, not new product tests.
AR02 remote/ref/artifact readback succeeded and its owned checkout, branch,
targets and scratch were removed. AR03's docs-only audit was published as
`4f47059c`; its reserved activation remains unanswered.

## AR04 storage command contract — DONE

[Explicit full-save fallback](STORAGE_COMMAND_CONTRACT.md) replaces the exported
helper's successful no-op arms without changing current production save paths.
The source audit routes all 50 command variants through one complete snapshot
write; this is not a claim of 50 individually executed variant tests.
Behavioral parameter RED/GREEN, nine focused scalar/provenance regressions,
three file-backed reopen/history/order/late-failure tests and three rejected
compiled mutants are measured. Source hashes match after restoration.
Complete coordinated gates exited 0: 142 Rust result blocks, 2,859 passed,
zero failed, 161 ignored, zero skip markers; two explicitly executed private
store roundtrips, zero ignored/failure; Web 1,312. Strict Clippy, typecheck,
build, semantic bindings, dependency and repository gates pass. All 574 guarded
source files are unchanged across gates; changed store code compilation was
verified. Published implementation `216c673e7c32a4bd82a308e06544a4fd239d7b3f`
matched the remote ref and exact source/contract artifacts; author/committer and
no-co-author policy verified. Closing doc publication/owned cleanup follows.
No parked ADR phase, U12 control, schema or commissioning gate is changed.

## AR14 offline bus/CLI contract dossier

Claude session, 2026-10-04, on user request alongside the Codex alpha
session (AR06/AR06P untouched). Offline only: fakes, local adapters and
loopback multicast; no gateway, no bus traffic. Each claim below is either
fixed, pinned by a named test whose guard was checked with a reverting
mutant, or retained as a documented boundary.

### Defects found and fixed

| ID | Finding | Fix and evidence |
| --- | --- | --- |
| KL-29, KL-62 item 13 | CLI `bus write`/`monitor`/`route-monitor` parsed and printed group addresses three-level even for two-level/free projects (a free project's `2049` was refused; its `1/0/1` spelling was silently accepted). Names of the same raw address in several installations: last one read won (CLI **and** server Group Monitor). | `--project` now supplies the style; `route-send` (no `--project`) keeps three-level. New `knx_core::resolve_project_group_address_names` lists every distinct name (`A \| B`), used by CLI and server. `cli_bus_address_style` RED 3/5 → 5/5; server `names_shared_across_installations_are_all_shown` RED; 7/7 mutants. |
| KL-31 | On Linux a routing client bound to `0.0.0.0:3671` received every multicast group joined anywhere on the host (`IP_MULTICAST_ALL`, default on, ip(7)). A default-group client heard a custom group's telegrams, defeating the separation a custom group exists for. | `set_multicast_all_v4(false)` on Linux. `a_custom_group_telegram_reaches_its_group_and_not_the_default_one` failed before the fix with the leaked frame, passes after (three runs, no skip), mutant caught. Other platforms unverified. |
| KL-62 item 12 | The receive→write round trip was tested for free and two-level only. | `every_styles_telegram_destination_round_trips_through_write` adds three-level. |

### Contracts pinned by new tests

| ID | Claim | Test (mutant) |
| --- | --- | --- |
| KL-73 | A probe asks for DD0 only, never identity. | `a_probe_asks_only_for_the_mask_version` (descriptor type changed → caught) |
| KL-74, KL-76 | A negative L2 confirm is not a fast path to `Vacant`. | `a_negative_l2_confirm_still_waits_out_the_whole_window` (fast path → caught) |
| KL-75 | The window is the boundary for a slow-but-present device. | `a_slow_answer_is_vacant_after_a_short_window_and_occupied_within_a_long_one` (window ×10 → caught) |
| KL-76 | `Indeterminate` is not retried. | `an_indeterminate_probe_is_not_retried` (retry → caught) |
| KL-126 | Reconciliation refuses foreign session, duplicate, ambiguous and stale selections without changing project, undo history or bus. | `reconciliation_refuses_foreign_duplicate_ambiguous_and_stale_selections` (4/4 guards reverted → caught) |

### Contracts already pinned before AR14 (inspected, not rebuilt)

| ID | Existing evidence |
| --- | --- |
| KL-72 | `default_policy_matches_the_ruling` (6000/1/100), CLI `overrides_timeout_and_pause`, `zero_timeout_is_rejected_rather_than_silently_misreporting_every_device_as_vacant`; exclusion by construction: knx-core `line_scan_omits_excluded_addresses_and_contains_every_other`, `verify_catches_a_smuggled_excluded_address_without_panicking`, knx-net `a_plan_that_fails_verify_aborts_before_any_frame_is_sent`, HTTP `an_excluded_address_never_reaches_the_transport`. |
| KL-74 | `a_negative_l2_confirm_is_vacant_like_total_silence`, `a_positive_l2_confirm_with_no_application_answer_is_occupied_but_silent`, `a_disconnect_with_no_descriptor_is_busy_not_vacant`. |
| KL-77 | knx-core `a_range_spanning_two_lines_is_rejected`, CLI `range_spanning_two_lines_is_rejected`, `range_implying_a_different_line_than_line_flag_is_rejected`. |
| KL-78 | `self_address_is_skipped_without_sending_a_single_frame`; an immediate answer is `Occupied` (`occupied_address_reports_its_mask_version`), i.e. no timing heuristic filters fast responders. |
| KL-126 | `selected_scan_findings_apply_as_one_batch_and_undo_restores_content_exactly` (product-less device, one batch, exact undo, no frames), `empty_reconciliation_is_a_true_no_op_and_excluded_evidence_is_not_actionable`, `running_and_cancelled_scans_cannot_be_reconciled`, `scan_reconciliation_uses_a_matching_line_in_a_later_installation`. |
| KL-62 | Single session `409`, buffer cap and individual-frame exclusion: existing `bus.rs`/`http_bus_monitor.rs` tests and the 2026-09-16/19 passive real-gateway receipts. |

### Retained boundaries

KL-62/72/73/74/75/76/77/78/102/126 stay documented limitations: the
Standard's 6 s cost, occupancy-only evidence, the busy/vacant ambiguity,
the timeout trade-off, no retry/fast path, one line per scan, no tunnel
endpoint heuristic, an unreachable decode-error branch (codec symmetry is
guarded by 41 per-type round-trip tests at sampled values — not
exhaustively) and no identity inference. KL-31's only remainder is a real
custom-group run. Real-gateway transmit, routing, reconnect and multi-gateway
evidence are external live work; Group Monitor UI, row cap and discovery
acceptance belong to the UI owner. No scan speed promise, multi-tunnel
design or hardware support is added.

## AR08 password-import entry paths (2026-10-04)

`KL-13`, data/application half. The existing ZipCrypto reader
(`Container::open_with_password`) was reachable only from its own tests; no
second cipher was written. Entry paths now verified, each with RED-first
tests against the synthetic Info-ZIP fixture
`crates/knx-testsupport/fixtures/zipcrypto-minimal.knxproj`:

| Level | Entry path | Tests |
| --- | --- | --- |
| Library | `knx_etsproj::import_knxproj_with` / `import_knxproj_bytes_with` (`Option<&ProjectPassword>`) | `crates/knx-etsproj/tests/password_import.rs` (7) |
| Application | `knx_app::import_ets_project_with_password` | `crates/knx-app/tests/password_import.rs` (3) |
| CLI | `knx import --password-stdin`; `--password[=…]` refused without echo | `apps/knx-cli/tests/cli_password_import.rs` (5) |
| Server | `POST /api/project/import` `{ path, clientToken?, password? }`; `422` `kind: projectPasswordRequired` / `projectPasswordWrong` | `apps/knx-server/tests/http_password_import.rs` (6) |
| End to end (Web) | not done: password dialog handed to the UI owner | — |

Covered: right/wrong/missing/empty password, password on an unprotected
project, a wrong password that passes the check byte (found by the CLI
test: it used to surface as "corrupt deflate stream" and is now
`WrongPassword`), failed-import atomicity (empty store; the previously open
server project stays), native save/reload roundtrip, the lost protection
reported as an `unsupported` report entry, and redaction (password absent
from report, `Debug`, errors, CLI output, every file the import leaves,
`/api/log`, load progress, project tree). Existing container tests keep
covering corruption, size bounds, nested-path collisions and AES refusal.

Retained: real ETS4/ETS5 protected export and AES (ETS6) stay sample-gated
(COMPATIBILITY §3); `knx diff`, `POST /api/project/diff` and
`knx products ingest` take no password (KNOWN_LIMITATIONS §13).

## AR11 CSV/report decisions (2026-10-04)

Checked on `44746183`. User decisions: `KL-40` accepted as is for the Alpha
(derived CSV columns read-only, no Description/Comment, no schema change);
`KL-60` handed to the UI owner (virtualised diff tables plus search/filter
before the Alpha; backend API unchanged). No backend slice was approved, so
none was built.

| ID | Evidence |
| --- | --- |
| `KL-38` | No "ETS CSV" claim in Web catalogues, CLI or manual (repository grep); `knx-csv` 61 tests green |
| `KL-44` | Rendered report says "not an ETS report" (`knx-report` test at `render.rs:1322`); 48 tests green |
| `KL-47` | `unrenderable_parameter_kinds_and_allocator_refs_keep_raw_values_and_warn` and 2 more in `crates/knx-app/tests/documentation_composition.rs` green |
| `KL-51` | `knx-diff` module doc and the manual call it a "KNXBench project diff", never an ETS comparison |

All four stay sample- or semantics-gated as their KNOWN_LIMITATIONS
sections say.

## AR13 privacy and deployment-security dossier

Claude session, 2026-10-04, on user request alongside the Codex alpha
session. Offline only; no TLS service, role system, host or firewall change.

| Strand | Finding | Change and evidence |
| --- | --- | --- |
| Privacy (KL-106) | Redaction of the four classes held in every channel. Gap: `report.md` named only what it removes, and described `bus-telegrams.json` as carrying addresses — not its values (text included) and timestamps, which can show when the installation was in use. | `report.md` now names every kept class and the telegram file's content. `every_privacy_class_is_either_redacted_or_named_in_the_report`: one synthetic fixture per class through description, client facts and log fields (RED on the warning, GREEN after); 3/3 mutants. |
| Authentication (KL-22) | The guard test covered seven hand-picked routes; a route added outside the guard would only fail if someone also listed it. | `every_declared_route_refuses_a_caller_without_a_session_except_the_documented_four` scans all route declarations (97 method/path pairs); an added unguarded route is caught only by this test (mutant). Bind address has no override (`bind_address(auth_required)` only). |
| Provenance (KL-65) | `--version` named the last commit even for a modified tree. | `crates/knx-build-stamp` (shared by both binaries): `KNX_REQUIRE_CLEAN_TREE=1` re-runs on every build and refuses a modified/unconfirmed tree. Unit + real-git tests (10), 5 mutants; end-to-end on a real checkout: clean → stamped, edit → refused, without the always-re-run watch → falsely stamped (measured, now pinned by a test). |

### Deployment and privacy checklist (for AR15/AR17 and release notes)

1. Build release artifacts with `KNX_REQUIRE_CLEAN_TREE=1`; a development
   build's `+g<sha>` names a commit, not a tree.
2. A networked `knx-server` binds `0.0.0.0` only with `KNX_AUTH_PASSWORD_HASH`
   (preferred) or `KNX_AUTH_PASSWORD`; without either it binds `127.0.0.1`.
3. Put a TLS-terminating reverse proxy in front and set
   `KNX_AUTH_COOKIE_SECURE=1` there; plain HTTP exposes password and cookie.
4. One shared password: no accounts, roles, audit trail or CSRF tokens; not
   safe for direct Internet exposure; not multi-user isolation (§63).
5. Debug reports: IPs, home prefix and hostname are replaced in `report.md`,
   `environment.json`, `log.json`; everything else stays, as `report.md`
   says. `bus-telegrams.json` is unredacted (addresses, names, values,
   timestamps) and opt-in. Nothing is uploaded; review before sharing.
6. (AR17, 2026-10-06) A release binary built on a personal machine carries
   about 540 source paths with the builder's home directory (`~/.cargo`,
   `~/.rustup`). Build releases in CI or with `--remap-path-prefix`; see
   [ALPHA_CANDIDATE §4](ALPHA_CANDIDATE.md#4-findings).

### Handed over

**For the UI session / Web-lock holder (done 2026-10-04, goal-ui owner):**
the dialog string `debugReport.privacyTelegrams` (en/de) now mirrors
`report.md`'s second paragraph (`apps/knx-server/src/debug_report.rs`,
`report_markdown`). `bus-telegrams.json` is not redacted and keeps addresses,
group-address names, every telegram's value (text values included) and its
timestamp, which together can show when the installation was in use. A
content test in `DebugReportButton.test.tsx` pins these parts in both
languages; it failed on the previous wording.

**For the alpha-release session (AR17):** build the candidate with
`KNX_REQUIRE_CLEAN_TREE=1`.

## Stable limitation identity

AR05 implementation/audit is published as
`04900fbc35b2daec5e766a32f99c263a04700e0e` at base `f4b845a3`;
exact remote ref and all 25 owned artifacts match. Final process proc_f9f87cee4329
exited 0 with all 20 expected gates, workspace 2,884 / ProductDB 585 / Web 1,312
passed and no failed test. Ordinary ignored counts are 163/24, not private
acceptance: matrix, real v18 upgrade and census separately execute one case each
with no ignored/failure/skip markers. Complete aggregate shapes and 595 frozen
sources match. Named regression/mutation evidence and in-session review are in
[MANUFACTURER_REPORT_CONTRACT](MANUFACTURER_REPORT_CONTRACT.md) and
`.ai/logs/2026-10-02_codex_alpha-manufacturer-report.md`.

AR05's duplicate/provenance scope is an audit with verified reporting boundaries,
not implementation of missing DPT source-winner provenance. KL-86 remains a
numbered residual; unsupported Dynamic semantics and historical unavailable
reports remain disclosed. This is neither user boundary acceptance nor full
manufacturer compatibility. Stable identities and limitation totals below are
unchanged. The six AR05 inventory dispositions below mean its named checklist
scope was delivered at that revision, not every source limitation was removed.
Closing documentation receipt `65b91777` is published and read back; both owned
checkouts/branch and 266 task-owned scratch entries removed, originals and
foreign root preserved. AR06 discovery proceeds on a fresh checkout.

There are **110 numbered headings**, **109 distinct numbers**, two meanings of
130, and no 94. Seven are resolved/clarification-only: 18, 23, 24, 42 (AR04),
90, 95, 130-GATE (AR01). Therefore **103 numbered residual boundaries** remain:
K1=5, K2=30, K3=54, K4=13 (102 triaged); 105 is wire-evidence-only and unclassified.
No heading or legacy fragment is destructively renumbered.
**Update 2026-10-06 (AR15 recount):** 119 numbered heading lines, 118
distinct numbers; eleven are signposts (the seven above plus 149, 150, 152,
156), so **108 numbered residual boundaries** remain: K1=5, K2=30, K3=58,
K4=14 (107 triaged), 105 still unclassified. Counts and the script are in
[LIMITATION_TRIAGE](LIMITATION_TRIAGE.md) and
`.ai/logs/2026-10-06_claude_ar15-slice2.md`; the release view is
[ALPHA_SCOPE_MATRIX](ALPHA_SCOPE_MATRIX.md).
`KL-8` also routes 26 (Secure); `KL-130-GATE` and `KL-130-ZOOM` identify the
separate headings exactly. All other numbered IDs use `KL-<number>`.

Unnumbered aliases below identify source sections, not new inventory tasks.
Multiple input IDs can describe separate aspects of the same section.

| Stable section ID | Exact KNOWN_LIMITATIONS heading | Input IDs / owner |
| --- | --- | --- |
| UB-BACKUP-DIRECTORIES | Backup directory synchronization is not a disk-loss or confinement proof | SAFE-03; commissioning |
| UB-BUS-ACTIVITY | Partial commissioning bus-activity snapshot (ADR-0055) | AUDIT-01, UI-04; commissioning |
| UB-STRUCTURE-EDITOR | U12 structure editor scope (ISSUE-05) | DATA-02, MODEL-01, MODEL-02, KL-127; mixed layers retain primary routes |
| UB-DEVICE-EDITOR | U11 device editor scope (ISSUE-09) | MODEL-01, MODEL-02, MODEL-03; UI |
| UB-CATALOG-BATCH | U11 catalog batch scope (ISSUE-07) | DATA-01, DATA-03, MODEL-04; mixed layers retain primary routes |
| UB-READINESS-BACKUP | Commissioning readiness and pre-write backup boundary (ADR-0049) | SAFE-02, UI-03; commissioning / UI acceptance |
| UB-PDB-DYNAMIC | PDB-9 parameter and Dynamic coverage boundary | PDB-01/02/03/04/05/06/07, R-MODULE-03; data/owner routes below |
| UB-PDB-REPORT | PDB-3 report history and coverage boundary | PDB-07/08/09/11; alpha |
| UB-PDB-METADATA | PDB-7 catalogue metadata are source strings, not capabilities | PDB-10; alpha |

The `Closed entries and historical links` section is not a tenth unnumbered
limitation. UI-03's device-checks boundary is also retained in the readiness
section, not silently invented as a numbered issue.

## Decision and external-gate contracts

**User decisions 2026-10-04:** `KL-129`/AR03 deferred past the Alpha
(phases 1–2 plus AR02 are the boundary); `KL-135`/AR12 deferred
(first-installed winner, candidates disclosed); `KL-70`, `KL-88`, `KL-134`
accepted as Alpha boundaries; `KL-40` accepted, `KL-60` handed to the UI
owner (AR11 section). Only `RELEASE-04` (AR19) still waits for a decision.

| Gate / rows | Missing input and responsible owner | Safe fallback | Exact unblock condition |
| --- | --- | --- | --- |
| KL-70, KL-88, KL-134 | Maintainer-selected behavior/safety rules exist, but an explicit user **release-scope waiver** was not found in the inspected decision sources | Keep active-parameter refusal, current master-name policy and inert baggage; no new feature or execution | User accepts these precise residual boundaries for alpha, or requests scoped work; do not label technical rulings user consent |
| KL-129 / AR03 | User activation of parked ADR-0039 phases 3–5 | Keep phases 1–2 guards; AR02 handles exhaustion independently | Recorded activation scope or explicit continued-deferral decision |
| KL-40 / KL-60 / AR11 | User CSV/paging scope choice | Existing validated read-only columns and disclosed paging | User-approved schema/backend scope or explicit boundary acceptance; UI changes require owner receipt |
| KL-135 / AR12 | User version/pinning policy | Existing first-winner policy with candidates disclosed | Approved versioned selection policy or explicit deferral; frontend acceptance remains owner-dependent |
| KL-1 | Independent missing-schema/modular project samples; operator/sample owner | Preserve bytes/report unknowns; no broader compatibility claim | Authorized independent fixtures and executed corresponding import/native tests |
| R-MODULE-03/04 | Verified AllocatorRef/nested-module source/sample; research owner | Preserve opaque/raw values and bounded/read-only handling | Primary semantics plus suitable regression fixture, not a guessed formula |
| KL-13 subcases | Genuine protected ETS export and password; operator/sample owner | Current ZipCrypto library tests; AES refusal | Safe entry-path plumbing tests for AR08; real ETS sample for interoperability; real AES sample plus separate implementation evidence for AES |
| UI rows | Owner receipt at exact stated scope; GUI/accessibility evidence where disclosed | Adopt completed U13, retain platform/optional residues and existing consumers | Relevant owner's new evidence or explicit alpha exception; no duplicated U packages |
| Commissioning rows | Device-specific complete recovery, exact identity, hardware/spec evidence and separate go; commissioning owner | Fail-closed actions and disclosed simulator/one-device scope | Owner's scoped receipt and explicit release disposition; this session never opens a real socket |
| RELEASE-03 | User manual location/screenshots/acceptance | Existing maintainer manual, ADR-0024 no bundled docs | Recorded user policy and dated finished-product manual checklist |
| RELEASE-01/02 | Finished candidate/artifact, integrated gates, chosen independent whole-product reviewer | Not ready; owner UI review is not the whole-product review | AR18 prerequisites and actual independent verdict with blocking findings resolved |
| RELEASE-04 | Exact release/tag/publication approval; user | No tag/upload | Explicit AR19 approval of the actual reviewed candidate |
| LATER rows | Separate scoped decision/ADR; future/tooling owner | No alpha implementation prerequisite | New explicit scope; telemetry/private/history artifacts remain unversioned unless separately authorized |

`ACCEPTED_BOUNDARY` below requires the named dated decision or accepted ADR;
it is not inferred from "no task exists". FUTURE-05 is merely unscheduled and
is `LATER`, not an accepted release exception. The three disputed behavior rows
remain `WAITING_DECISION`. No wait blocks unrelated ready offline packages.

## Complete per-ID execution ledger

### UI-owner follow-up evidence (2026-10-02)

The separately authorized UI session audited all 24 UI-routed rows and gated
four concrete implementation gaps: UX-02, UX-03, KL-121 and KL-124. Its complete
per-ID matrix and exact native/offline qualifications are in
[UI_ALPHA_READINESS](UI_ALPHA_READINESS.md). Implementation publication
`6c16fe5aaf764d78f62382f867f59b6ae77f8dce` and exact remote/tree equality
are verified. Four owner rows are DONE at that implementation scope only.
Existing U13
closures, all inventory IDs/priorities/routes and other sessions' dispositions
remain unchanged. Native/Orca/multicast evidence, KL-82 and domain/API-dependent
work remain explicit; no alpha-release exception is inferred.

The subsequent KL-82 owner candidate adds actual server session/project
interpretation comparison and fail-closed unavailable/legacy handling, with
pause/cursor/race regressions. Focused suites, eleven behavioral negative
controls and mocked monitor Chromium tests pass. All twelve candidate gates
pass (Web 1,343 / Chromium 35 / Rust 2,890, zero failed, 163 ignored; 570-source
freeze). All twelve steps repeated on integrated source `8ceacf49`, now
published with exact remote/tree/artifact readback; see the same owner matrix.
The scoped KL-82 implementation row is DONE, not a source-wide/native/alpha
waiver. Earlier source
counts are not evidence for this new candidate, and no native/live-bus or
transactional write guarantee is inferred.

Each row retains its one primary route and original priority. The evidence
column points to the **current retained source**, not a claim that its entire
implementation was freshly tested. Open alpha code contracts are verified in
their named AR package. Owner rows have the shared fallback/unblock contract
above and are not new tasks assigned to an already closed owner queue.

The 180-row table and its status count moved to the
[source-ID ledger](status/LEDGER.md#snapshot-ids) on 2026-10-04 (AR14D D2,
[ADR-0076](adr/0076-one-ledger-is-the-status-of-record.md)); its evidence
column is the ledger's *Evidence* column.

## Post-snapshot findings (outside the 180-ID ledger)

Added 2026-10-03 from the test-only product-install run over 853 public
manufacturer downloads ([corpus run](PRODUCT_DATABASE_CORPUS.md#public-crawler-corpus-run-2026-10-03),
commit `2cceea4e`). They have their own identities. The 180-ID ledger, its
mechanically counted statuses and the 110-heading recount above are a dated
snapshot and stay unchanged. AR15 recounts them.

Their rows and counts are in the [ledger](status/LEDGER.md#post-snapshot-ids).

**Unblock input for AR07 (status unchanged here):** the same corpus contains
sample evidence that the ledger above reports as missing. Its status change is
up to the AR07 executor:

- `R-MODULE-03` (AllocatorRef sample missing): 10 crawled packages use
  `NumericArg … AllocatorRefId=` with 1,070 uses in total (ABB 4 packages,
  MDT 5, Siemens' HVAC bundle 1). Three further MDT packages declare an
  `Allocator` without referencing it.
- `R-MODULE-04` (no real nested-module example): exactly one of 852 crawled
  ZIP packages nests `ModuleDef`s (MDT `RF-TAL55Bx0x-01S`, 11 nested, depth
  2). This is the package that `KL-150` cannot install yet.

These files are public downloads that may not be redistributed. They can serve
as local, private evidence. A committed regression fixture still has to be
synthetic or authorized.
