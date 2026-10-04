# Alpha-readiness evidence and dispositions

## AR00 reconciliation baseline

User-started offline alpha queue, 2026-10-01. Inspected source/maintained docs
at `307a5970ad5147437dffce4a86047e85e3319186`; the dated input inventory
[OFFENE_PUNKTE](../OFFENE_PUNKTE.md) is unchanged. This is dispatch/provenance
reconciliation, not a fresh full implementation, security or compatibility audit.
The execution owner is [alpha-release-goal](../alpha-release-goal.md); historical
[goal](../goal.md) is not a second executor. UI/commissioning remain their owners.

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

### Handed over

**For the UI session / Web-lock holder:** the dialog string
`debugReport.privacyTelegrams` (en/de) names addresses and names only; add
that the file also keeps every telegram value (text values included) and its
timestamp. Backend wording to mirror: `report.md`'s second paragraph
(`apps/knx-server/src/debug_report.rs`, `report_markdown`).

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

| Source ID | Priority | Primary route | Status | Evidence / remaining disposition |
| --- | --- | --- | --- | --- |
| `KL-116` | P0 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §116; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-139` | P0 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §139; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-140` | P0 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §140; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `SAFE-01` | P0 | `goal-commission.md` — owner only | WAITING_OWNER | goal-commission.md status / docs/RESEARCH.md §22–24; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-79` | P1 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §79; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `KL-61` | P1 | AR09 | TODO | docs/KNOWN_LIMITATIONS.md §61; Retained boundary; AR09 verifies subcases before changing status |
| `KL-99` | P1 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §99; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-112` | P1 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §112; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-136` | P1 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §136; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-138` | P1 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §138; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-141` | P1 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §141; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-142` | P1 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §142; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-7` | P1 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §7; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-92` | P1 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §92; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `DEBUG-01` | P1 | `goal-commission.md` — owner only | WAITING_OWNER | goal-commission.md status / ADR-0051; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `SAFE-02` | P1 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md: Commissioning readiness / ADR-0049; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `SAFE-03` | P1 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §7 / goal-commission.md §3; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `DATA-01` | P1 | AR02 | DONE | Nine checked allocators; synthetic maximum-ID/native/CSV/CLI/HTTP/mapper and rollback regressions; three behavioral mutants; final offline gate receipt .ai/logs/2026-10-01_codex_alpha-id-exhaustion.md. Parked mutation enforcement and catalog UI scope remain separate. |
| `KL-129` | P1 | AR03 | WAITING_DECISION | docs/KNOWN_LIMITATIONS.md §129; Reserved user decision; see decision contract above |
| `KL-106` | P1 | AR13 | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §106; AR13: fixture audit across every class and channel; `report.md` now names every kept class and the telegram file's values/timestamps; dialog wording handed to the Web-lock holder; no anonymity claim |
| `DOC-01` | P1 | AR00 | DONE | goal.md §3 / §12.2 / docs/LIMITATION_TRIAGE.md / apps/knx-server/src/domain.rs; AR00 source/test and provenance reconciliation above; doc/ledger gate receipt in alpha-queue log |
| `KL-1` | P1 | AR06 | BLOCKED_EXTERNAL | docs/KNOWN_LIMITATIONS.md §1; Missing independent sample/source; exact fallback/unblock contract above; no invented semantics |
| `KL-13` | P1 | AR08 | TODO | docs/KNOWN_LIMITATIONS.md §13; Retained boundary; AR08 verifies subcases before changing status |
| `PDB-09` | P1 | AR05 | DONE | Published 04900fbc; master_language_evidence/master_evidence_rederive and three actual private cases verify Languages reporting, retained bytes and report history; manufacturer-report contract scopes the remaining unknown semantics |
| `R-MODULE-01` | P1 | `goal-commission.md` — owner only | WAITING_OWNER | docs/RESEARCH.md §19.11 / goal-commission.md; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-130-GATE` | P1 | AR01 | DONE | docs/KNOWN_LIMITATIONS.md §130 (Gate); AR01 runtime-root/coverage CLI and scan regressions, old removed-tree reproduction, five behavioral mutants; verification delivery above |
| `RELEASE-01` | P1 | AR18 | WAITING_OWNER | goal.md §9–10; Named final acceptance prerequisites above; not ready on historical receipts alone |
| `RELEASE-02` | P1 | AR18 | WAITING_OWNER | goal.md §1 / §10; Named final acceptance prerequisites above; not ready on historical receipts alone |
| `KL-22` | P1 | AR13 | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §22; AR13: every declared route (97 pairs) checked unauthenticated; bind address has no override; TLS/roles/audit/CSRF remain deployer boundaries (checklist in ALPHA_READINESS) |
| `KL-63` | P1 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §63; goal.md §6: T22 explicitly parked outside v1 must-haves |
| `KL-8` | P1 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §8/26; goal.md §6: Secure deferred 2026-09-11; not Secure support |
| `UI-01` | P1 | `goal-ui.md` — owner only | DONE | goal-ui.md §0 / docs/IMPLEMENTATION_STATUS.md: U13; U13 closure dfa0cc79 / receipt 8a51b74d; source 36e6b6af; native/multicast boundaries retained |
| `UI-02` | P1 | `goal-ui.md` — owner only | DONE | goal-ui.md §0 / docs/superpowers/plans/2026-09-21-user-reported-issues.md; U13 closure dfa0cc79 / receipt 8a51b74d; source 36e6b6af; native/multicast boundaries retained |
| `KL-126` | P2 | AR14 | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §126; AR14: four untested guards pinned by `reconciliation_refuses_foreign_duplicate_ambiguous_and_stale_selections` (4/4 mutants); identity is never inferred from occupancy |
| `KL-29` | P2 | AR14 | DONE | docs/KNOWN_LIMITATIONS.md §29; AR14: CLI `bus monitor`/`route-monitor`/`bus write --project` parse and print in the project style; names of all installations shown (`knx_core::resolve_project_group_address_names`, shared with the server). Tests `cli_bus_address_style` (RED 3/5), `monitor_line_uses_the_project_style_and_all_names`, `names_shared_across_installations_are_all_shown`; 7/7 mutants. Residue: `route-send` has no `--project`, keeps three-level |
| `KL-31` | P2 | AR14 | BLOCKED_EXTERNAL | docs/KNOWN_LIMITATIONS.md §31; AR14: found and fixed cross-group delivery on Linux (`IP_MULTICAST_ALL` off; `a_custom_group_telegram_reaches_its_group_and_not_the_default_one`, RED then GREEN, mutant caught). Missing: real router traffic on a custom group — authorized live owner; fallback: default group unchanged, custom group documented unverified; unblock: an authorized custom-group run |
| `KL-62` | P2 | AR14 | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §62; AR14: item 13 fixed (CLI style), item 12 closed (`every_styles_telegram_destination_round_trips_through_write` adds ThreeLevel). Tunnelling-only/single-session/client-filter scope retained; live transmit evidence external; row cap is UI-owner work |
| `KL-72` | P2 | AR14 | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §72; AR14: Standard-fixed per-address cost, not liftable; pacing/exclusion contract pinned by existing knx-core/knx-net/CLI/HTTP scan tests (dossier in ALPHA_READINESS) |
| `KL-73` | P2 | AR14 | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §73; AR14: pinned by `a_probe_asks_only_for_the_mask_version` (mutant caught); identity needs a separate verified procedure |
| `KL-74` | P2 | AR14 | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §74; AR14: protocol limit pinned by `a_negative_l2_confirm_is_vacant_like_total_silence` and `a_negative_l2_confirm_still_waits_out_the_whole_window` |
| `KL-75` | P2 | AR14 | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §75; AR14: trade-off pinned by `a_slow_answer_is_vacant_after_a_short_window_and_occupied_within_a_long_one` (mutant caught); default stays 6000 ms |
| `KL-77` | P2 | AR14 | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §77; AR14: one-line scope pinned by the range-spanning-two-lines rejections (knx-core and CLI); no coupler traversal |
| `KL-78` | P2 | AR14 | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §78; AR14: only the own tunnel address is skipped (`self_address_is_skipped_without_sending_a_single_frame`); an immediate answer stays `Occupied`; no timing heuristic without multi-gateway evidence |
| `KL-105` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §105; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-108` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §108; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-101` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §101; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-104` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §104; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-109` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §109; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-111` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §111; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-113` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §113; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-114` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §114; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-143` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §143; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-144` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §144; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-145` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §145; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-93` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §93; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `GAP-T30-01` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/RESEARCH.md §8.7.15 / docs/superpowers/specs/2026-09-13-commissioning-download-design.md §12; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `GAP-T30-02` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/RESEARCH.md §8.7.15 / docs/superpowers/specs/2026-09-13-commissioning-download-design.md §12; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `GAP-T30-03` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/RESEARCH.md §8.6.7 / §8.7.15; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `GAP-T30-07` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/RESEARCH.md §8.7.15 / docs/superpowers/specs/2026-09-13-commissioning-download-design.md §12; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `GAP-T30-08` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/RESEARCH.md §8.7.15 / docs/superpowers/specs/2026-09-13-commissioning-download-design.md §12; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `GAP-T30-09` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/RESEARCH.md §8.7.14–15 / docs/superpowers/specs/2026-09-13-commissioning-download-design.md §12; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `R-DL-01` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/RESEARCH.md §8.6.7; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `R-DL-02` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/RESEARCH.md §8.7.15 / commissioning-download-design.md R11; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `DATA-02` | P2 | AR04 | DONE | STORAGE_COMMAND_CONTRACT.md; published 216c673e full-save fallback/native failure-history tests, integrated gates and exact remote/artifact verified; U12 editor scope is not lifted |
| `DATA-03` | P2 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md: U11 catalog batch scope; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `KL-87` | P2 | AR05 | DONE | Published 04900fbc; exact master Languages attribute reporting, namespaces and retained bytes verified by master_language_evidence; untyped Version semantics remain explicit, not a compatibility claim |
| `AUDIT-01` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md: Partial commissioning bus-activity snapshot / ADR-0055/0056; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-137` | P2 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §137; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `KL-36` | P2 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §36; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `KL-82` | P2 | `goal-ui.md` — owner only | DONE | Scoped authoritative interpretation comparison and unavailable/legacy fail-closed UI delivered at 8ceacf49; pause/cursor/race and eleven behavioral controls, all twelve integrated gates and exact remote/tree/artifact readback verified. Point-in-time/native/live/transactional limitations remain in §82 and docs/UI_ALPHA_READINESS.md; not an alpha waiver. |
| `DOC-03` | P2 | AR15 | TODO | docs/manual/known-issues.md / docs/manual/ideas-and-roadmap.md; Retained boundary; AR15 verifies subcases before changing status |
| `RELEASE-03` | P2 | AR16 | WAITING_OWNER | goal.md §5 / docs/manual/README.md / ADR-0024; Named final acceptance prerequisites above; not ready on historical receipts alone |
| `RELEASE-04` | P2 | AR19 | WAITING_DECISION | goal.md §5 / docs/ROADMAP.md Session 7; Reserved user decision; see decision contract above |
| `KL-127` | P2 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §127; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `MODEL-01` | P2 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md: U11 device editor scope / U12 structure editor scope; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `MODEL-02` | P2 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md: U11 device editor scope / U12 structure editor scope; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `MODEL-03` | P2 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md: U11 device editor scope; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `IMPORT-05` | P2 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/manual/ideas-and-roadmap.md / docs/COMPATIBILITY.md; manual ideas-and-roadmap / COMPATIBILITY: encrypted product packages explicitly excluded |
| `IMPORT-06` | P2 | AR06 | TODO | docs/manual/known-issues.md / docs/KNOWN_LIMITATIONS.md §2; Retained boundary; AR06 verifies subcases before changing status |
| `KL-11` | P2 | AR06 | TODO | docs/KNOWN_LIMITATIONS.md §11; Retained boundary; AR06 verifies subcases before changing status |
| `KL-125` | P2 | AR06 | TODO | docs/KNOWN_LIMITATIONS.md §125; Retained boundary; AR06 verifies subcases before changing status |
| `KL-128` | P2 | AR06 | TODO | docs/KNOWN_LIMITATIONS.md §128; Retained boundary; AR06 verifies subcases before changing status |
| `KL-15` | P2 | AR06 | TODO | docs/KNOWN_LIMITATIONS.md §15; Retained boundary; AR06 verifies subcases before changing status |
| `KL-2` | P2 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §2; DIN-26 decision, 2026-09-27 (all twelve accepted; ambiguous-DPT residue only for KL-12) |
| `KL-14` | P2 | AR10 | TODO | docs/KNOWN_LIMITATIONS.md §14; Retained boundary; AR10 verifies subcases before changing status |
| `KL-37` | P2 | AR10 | TODO | docs/KNOWN_LIMITATIONS.md §37; Retained boundary; AR10 verifies subcases before changing status |
| `KL-64` | P2 | AR10 | TODO | docs/KNOWN_LIMITATIONS.md §64; Retained boundary; AR10 verifies subcases before changing status |
| `KL-66` | P2 | AR10 | TODO | docs/KNOWN_LIMITATIONS.md §66; Retained boundary; AR10 verifies subcases before changing status |
| `KL-12` | P2 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §12; DIN-26 decision, 2026-09-27 (all twelve accepted; ambiguous-DPT residue only for KL-12) |
| `KL-135` | P2 | AR12 | WAITING_DECISION | docs/KNOWN_LIMITATIONS.md §135; Reserved user decision; see decision contract above |
| `KL-146` | P2 | AR07 | TODO | docs/KNOWN_LIMITATIONS.md §146; Retained boundary; AR07 verifies subcases before changing status |
| `KL-3` | P2 | AR07 | TODO | docs/KNOWN_LIMITATIONS.md §3; Retained boundary; AR07 verifies subcases before changing status |
| `KL-6` | P2 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §6; goal.md §6: vendor binaries not executed |
| `KL-68` | P2 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §68 / docs/RESEARCH.md §4.4 (RepeatIndex); goal.md §6: dated 2026-09-20 module boundary decision |
| `KL-69` | P2 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §69; goal.md §6: dated 2026-09-20 module boundary decision |
| `KL-71` | P2 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §71; goal.md §6: dated 2026-09-20 module boundary decision |
| `KL-85` | P2 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §85; DIN-26 decision, 2026-09-27 (all twelve accepted; ambiguous-DPT residue only for KL-12) |
| `KL-86` | P2 | AR05 | DONE | Published 04900fbc; scoped duplicate/DPT audit in install_reports verifies first normalized values, retained losing declarations, counts, reopen/retry; §86 missing source-winner provenance remains open, not implemented or release-waived |
| `PDB-01` | P2 | AR07 | TODO | docs/KNOWN_LIMITATIONS.md: PDB-9 parameter and Dynamic coverage boundary; Retained boundary; AR07 verifies subcases before changing status |
| `PDB-02` | P2 | AR07 | TODO | docs/KNOWN_LIMITATIONS.md: PDB-9 parameter and Dynamic coverage boundary; Retained boundary; AR07 verifies subcases before changing status |
| `PDB-05` | P2 | AR07 | TODO | docs/KNOWN_LIMITATIONS.md: PDB-9 parameter and Dynamic coverage boundary; Retained boundary; AR07 verifies subcases before changing status |
| `PDB-06` | P2 | AR05 | DONE | Published 04900fbc; dynamic_tree mixed active/skipped regression verifies one shared budget and truncation disclosure; value-dependent/unexpanded semantics stay unsupported, not exhaustively enumerated |
| `PDB-08` | P2 | AR06 | TODO | docs/KNOWN_LIMITATIONS.md: PDB-3 report history and coverage boundary; Retained boundary; AR06 verifies subcases before changing status |
| `PDB-10` | P2 | AR06 | TODO | docs/KNOWN_LIMITATIONS.md: PDB-7 catalogue metadata are source strings; Retained boundary; AR06 verifies subcases before changing status |
| `R-DYNAMIC-01` | P2 | AR07 | TODO | docs/RESEARCH.md §4.3 / Open questions; Retained boundary; AR07 verifies subcases before changing status |
| `R-MODULE-03` | P2 | AR07 | BLOCKED_EXTERNAL | docs/RESEARCH.md §4.4 / docs/KNOWN_LIMITATIONS.md: PDB-9; Missing independent sample/source; exact fallback/unblock contract above; no invented semantics |
| `R-MODULE-04` | P2 | AR07 | BLOCKED_EXTERNAL | docs/RESEARCH.md §4.4 / docs/GAP_ANALYSIS_ETS.md A3; Missing independent sample/source; exact fallback/unblock contract above; no invented semantics |
| `KL-133` | P2 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §133; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `KL-38` | P2 | AR11 | TODO | docs/KNOWN_LIMITATIONS.md §38; Retained boundary; AR11 verifies subcases before changing status |
| `KL-39` | P2 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §39; DIN-26 decision, 2026-09-27 (all twelve accepted; ambiguous-DPT residue only for KL-12) |
| `KL-40` | P2 | AR11 | TODO | docs/KNOWN_LIMITATIONS.md §40; Retained boundary; AR11 verifies subcases before changing status |
| `KL-44` | P2 | AR11 | TODO | docs/KNOWN_LIMITATIONS.md §44; Retained boundary; AR11 verifies subcases before changing status |
| `KL-47` | P2 | AR11 | TODO | docs/KNOWN_LIMITATIONS.md §47; Retained boundary; AR11 verifies subcases before changing status |
| `KL-51` | P2 | AR11 | TODO | docs/KNOWN_LIMITATIONS.md §51; Retained boundary; AR11 verifies subcases before changing status |
| `KL-60` | P2 | AR11 | TODO | docs/KNOWN_LIMITATIONS.md §60; Retained boundary; AR11 verifies subcases before changing status |
| `R-SEC-01` | P2 | Later / separate scope — not an alpha task | LATER | docs/RESEARCH.md §9 / Open questions; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `UI-03` | P2 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md: Device-checks UI boundary; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `UI-04` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md: Partial commissioning bus-activity snapshot / ADR-0055/0056; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-130-ZOOM` | P2 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §130 (Zoom); U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `KL-20` | P2 | `goal-ui.md` — owner only | DONE | UI owner keyboard/modal/help-tip implementation delivered at 2e57f8e5; twelve integrated gates/eighteen controls verified; docs/UI_ALPHA_READINESS.md retains actual native/Orca/full-theme residue, not entire-source acceptance or a release waiver |
| `KL-124` | P3 | `goal-ui.md` — owner only | DONE | UI owner implementation delivered at 6c16fe5a; docs/UI_ALPHA_READINESS.md retains exact scope and native/design residues, not a release waiver |
| `KL-76` | P3 | AR14 | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §76; AR14: both deliberate choices pinned: `an_indeterminate_probe_is_not_retried`, `a_negative_l2_confirm_still_waits_out_the_whole_window` (mutants caught) |
| `KL-102` | P3 | AR14 | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §102; AR14: branch unreachable through the public API; codec symmetry guarded by 41 per-type round-trip tests at sampled values; no fabricated trigger |
| `KL-110` | P3 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §110; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-115` | P3 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §115; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `GAP-T30-04` | P3 | `goal-commission.md` — owner only | WAITING_OWNER | docs/RESEARCH.md §8.7.15 / docs/spec-audits/2026-09-19-cp-3_5_3-partial-download.md; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `HISTORY-01` | P3 | Later / separate scope — not an alpha task | LATER | docs/GAP_ANALYSIS_ETS.md B11; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `HISTORY-02` | P3 | Later / separate scope — not an alpha task | LATER | docs/GAP_ANALYSIS_ETS.md C1 / D8; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `DOC-02` | P3 | AR00 | DONE | docs/KNOWN_LIMITATIONS.md / docs/LIMITATION_TRIAGE.md; AR00 source/test and provenance reconciliation above; doc/ledger gate receipt in alpha-queue log |
| `MODEL-04` | P3 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md: U11 catalog batch scope; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `MODEL-05` | P3 | Later / separate scope — not an alpha task | LATER | docs/RESEARCH.md §17.3; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `MODEL-06` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | goal.md §6 / ADR-0019; accepted ADR-0019 / goal.md §6: topological model, no spatial canvas |
| `MODEL-07` | P3 | Later / separate scope — not an alpha task | LATER | docs/RESEARCH.md §15 / docs/ROADMAP.md: Functions; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `IMPORT-01` | P3 | Later / separate scope — not an alpha task | LATER | docs/GAP_ANALYSIS_ETS.md C3; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `IMPORT-02` | P3 | Later / separate scope — not an alpha task | LATER | docs/RESEARCH.md: local installation data (uncommitted addition); Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `IMPORT-03` | P3 | `goal-commission.md` — owner only | WAITING_OWNER | docs/RESEARCH.md: telegram-capture audit (uncommitted addition) / goal-commission.md K19; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `IMPORT-04` | P3 | Later / separate scope — not an alpha task | LATER | docs/RESEARCH.md: local installation data (uncommitted addition); Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `KL-100` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §100; accepted ADR-0024 §2: catalogue paragraphs, not a second help store |
| `KL-48` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §48; DIN-26 decision, 2026-09-27 (all twelve accepted; ambiguous-DPT residue only for KL-12) |
| `KL-134` | P3 | Recorded boundary — AR00 provenance / AR15 claims | WAITING_DECISION | docs/KNOWN_LIMITATIONS.md §134; Technical ruling is documented; explicit user release waiver missing; see decision contract above |
| `KL-70` | P3 | Recorded boundary — AR00 provenance / AR15 claims | WAITING_DECISION | docs/KNOWN_LIMITATIONS.md §70; Technical ruling is documented; explicit user release waiver missing; see decision contract above |
| `KL-88` | P3 | Recorded boundary — AR00 provenance / AR15 claims | WAITING_DECISION | docs/KNOWN_LIMITATIONS.md §88; Technical ruling is documented; explicit user release waiver missing; see decision contract above |
| `PDB-03` | P3 | AR07 | TODO | docs/KNOWN_LIMITATIONS.md: PDB-9 parameter and Dynamic coverage boundary; Retained boundary; AR07 verifies subcases before changing status |
| `PDB-04` | P3 | Later / separate scope — not an alpha task | LATER | docs/KNOWN_LIMITATIONS.md: PDB-9 parameter and Dynamic coverage boundary; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `PDB-07` | P3 | AR05 | DONE | Published 04900fbc; scoped Dynamic/report audit and budget/migration regressions; complete acceptance and private cases pass without claiming exhaustive subordinate coverage or reconstructing install history |
| `PDB-11` | P3 | AR05 | DONE | Published 04900fbc; master_evidence_rederive, master_language_evidence and real v18 upgrade verify atomic byte-only current evidence, missing/unexamined sources and immutable measured-zero/unavailable install snapshots |
| `FUTURE-01` | P3 | Later / separate scope — not an alpha task | LATER | docs/RESEARCH.md §13 / goal.md §7; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `FUTURE-02` | P3 | Later / separate scope — not an alpha task | LATER | docs/RESEARCH.md §14 / goal.md §7; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `FUTURE-03` | P3 | Later / separate scope — not an alpha task | LATER | ADR-0031 / goal.md §7 / docs/ROADMAP.md: In-application help; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `FUTURE-04` | P3 | Later / separate scope — not an alpha task | LATER | docs/RESEARCH.md §16 / goal.md §7; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `FUTURE-05` | P3 | Recorded boundary — AR00 provenance / AR15 claims | LATER | goal.md §6 / docs/GAP_ANALYSIS_ETS.md C6; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `FUTURE-06` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | goal.md §6; goal.md §6: Linux-first platform exclusion |
| `FUTURE-07` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | goal.md §6; goal.md §6: logo remains user-owned |
| `KL-107` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §107; accepted ADR-0025 and goal.md §6: data extension, no code plug-in API |
| `KL-16` | P3 | AR15 | TODO | docs/KNOWN_LIMITATIONS.md §16; Retained boundary; AR15 verifies subcases before changing status |
| `KL-42` | P3 | AR04 | DONE | docs/KNOWN_LIMITATIONS.md §42; published 216c673e no-op/doc correction, native/mutation/full-gate evidence and exact remote/artifact readback |
| `KL-65` | P3 | AR13 | DONE | docs/KNOWN_LIMITATIONS.md §65; AR13: `KNX_REQUIRE_CLEAN_TREE=1` release builds refuse a modified tree (`crates/knx-build-stamp`, ADR-0018 amendment); development builds still name a commit, not a tree; AR17 must build with the flag |
| `KL-41` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §41; DIN-26 decision, 2026-09-27 (all twelve accepted; ambiguous-DPT residue only for KL-12) |
| `KL-45` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §45; DIN-26 decision, 2026-09-27 (all twelve accepted; ambiguous-DPT residue only for KL-12) |
| `KL-46` | P3 | AR15 | TODO | docs/KNOWN_LIMITATIONS.md §46; Retained boundary; AR15 verifies subcases before changing status |
| `KL-52` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §52; DIN-26 decision, 2026-09-27 (all twelve accepted; ambiguous-DPT residue only for KL-12) |
| `KL-53` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §53; DIN-26 decision, 2026-09-27 (all twelve accepted; ambiguous-DPT residue only for KL-12) |
| `KL-54` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §54; DIN-26 decision, 2026-09-27 (all twelve accepted; ambiguous-DPT residue only for KL-12) |
| `KL-55` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §55; DIN-26 decision, 2026-09-27 (all twelve accepted; ambiguous-DPT residue only for KL-12) |
| `KL-56` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §56; DIN-26 decision, 2026-09-27 (all twelve accepted; ambiguous-DPT residue only for KL-12) |
| `KL-9` | P3 | AR15 | TODO | docs/KNOWN_LIMITATIONS.md §9; Retained boundary; AR15 verifies subcases before changing status |
| `TOOLS-06` | P3 | AR00 | DONE | docs/paperclip-shutdown/STATUS.md / goal.md §12; AR00 source/test and provenance reconciliation above; doc/ledger gate receipt in alpha-queue log |
| `TOOLS-01` | P3 | Later / separate scope — not an alpha task | LATER | .ai/CURRENT_STATE.md (local statistics handover) / docs/AI_STATS_TELEMETRY_PLAN.md; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `TOOLS-02` | P3 | Later / separate scope — not an alpha task | LATER | docs/AI_STATS_TELEMETRY_PLAN.md; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `TOOLS-03` | P3 | Later / separate scope — not an alpha task | LATER | docs/AI_STATS_TELEMETRY_PLAN.md / .ai/CURRENT_STATE.md (local); Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `TOOLS-04` | P3 | Later / separate scope — not an alpha task | LATER | docs/AI_STATS_TELEMETRY_PLAN.md / .ai/CURRENT_STATE.md (local); Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `TOOLS-05` | P3 | Later / separate scope — not an alpha task | LATER | docs/AI_STATS_TELEMETRY_PLAN.md; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `FUTURE-08` | P3 | Later / separate scope — not an alpha task | LATER | docs/manual/ideas-and-roadmap.md; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `KL-121` | P3 | `goal-ui.md` — owner only | DONE | UI owner implementation delivered at 6c16fe5a; docs/UI_ALPHA_READINESS.md retains exact scope and native/design residues, not a release waiver |
| `KL-43` | P3 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §43; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `KL-97` | P3 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §97; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `KL-98` | P3 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §98; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `UX-01` | P3 | `goal-ui.md` — owner only | WAITING_OWNER | docs/GAP_ANALYSIS_ETS.md B10 / docs/IMPLEMENTATION_STATUS.md T11; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `UX-02` | P3 | `goal-ui.md` — owner only | DONE | UI owner implementation delivered at 6c16fe5a; docs/UI_ALPHA_READINESS.md retains exact scope and native/design residues, not a release waiver |
| `UX-03` | P3 | `goal-ui.md` — owner only | DONE | UI owner implementation delivered at 6c16fe5a; docs/UI_ALPHA_READINESS.md retains exact scope and native/design residues, not a release waiver |

**Mechanically counted execution statuses:** ACCEPTED_BOUNDARY=24, BLOCKED_EXTERNAL=3, DONE=6, LATER=20, TODO=54, WAITING_DECISION=6, WAITING_OWNER=67; total=180.

## Post-snapshot findings (outside the 180-ID ledger)

Added 2026-10-03 from the test-only product-install run over 853 public
manufacturer downloads ([corpus run](PRODUCT_DATABASE_CORPUS.md#public-crawler-corpus-run-2026-10-03),
commit `2cceea4e`). They have their own identities. The 180-ID ledger, its
mechanically counted statuses and the 110-heading recount above are a dated
snapshot and stay unchanged. AR15 recounts them.

| Source ID | Priority | Primary route | Status | Evidence / remaining disposition |
| --- | --- | --- | --- | --- |
| `KL-150` | P1 | AR06P | TODO | docs/KNOWN_LIMITATIONS.md §150; a real MDT product with nested `ModuleDef`s fails atomically on a `dynamic_node` UNIQUE constraint; synthetic RED fixture first, local package as private evidence only |
| `KL-149` | P2 | AR06P | TODO | docs/KNOWN_LIMITATIONS.md §149; CLI extension check is case-sensitive; 43/46 refused Hager files install under a lowercase name |
| `KL-151` | P2 | AR06P | TODO | docs/KNOWN_LIMITATIONS.md §151; 13 real packages exceed the ZIP size limits, including Siemens' only current download; measure before changing the bound |
| `KL-152` | P2 | AR06P | TODO | docs/KNOWN_LIMITATIONS.md §152; 2 real packages exceed the XML evidence item limit; measure before resizing |
| `KL-153` | P2 | AR06P | TODO | docs/KNOWN_LIMITATIONS.md §153; schemes 10 (146 files) and 23 (2 files) are refused; grammar evidence for scheme 23 first; unsupported schemes stay explicit refusals |

**Post-snapshot counts:** P1=1, P2=4; TODO=5; total=5.

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
