# Non-terminal ledger rows — reconciliation and evidence re-check, 17 October 2026

Claude session in worktree `KNXBench-ledger-residue-20261017`, base `01c9e47b`
(claim `0133044b`). Scope: every non-terminal row of the [ledger](LEDGER.md) on
`origin/main` — 1 `IN_PROGRESS`, 2 `BLOCKED_EXTERNAL`, 16 `LATER`, confirmed by a
script that parses the ledger's ID rows. `TOOLS-01`…`05` (external ProjectStats
repository) and `FUTURE-08` (optional extra copy) are out of scope and unchanged.
No bus or device access and no contact with manufacturers or the KNX Association.
A delivered subset never made a `LATER` row `DONE`; status changes need named
tests and a revision covering the row's whole contract.

## Outcome

| Row | Old → new | Evidence (tests + revision, or source) | Remaining residue | Lift condition |
| --- | --- | --- | --- | --- |
| `MODEL-07` | IN_PROGRESS → **DONE** | `236c64ae`: `knx-diff` `tests/functions.rs` (10), `http_project_diff` `a_changed_function_is_one_row_of_the_functions_table`, `cli_project_diff` `diff_prints_a_changed_function_as_its_own_line`, `knx-mcp` `tests/tools.rs` (3 Function tests) + stdio/writer tool lists, `knx-desktop` `functions_surface_tests`, web `projectDiffView.test.ts`/`ProjectDiffPanel.test.tsx`; 8/8 mutants killed | Recorded boundaries, not row work: schemas 20/21 unmapped, `Implements`/`DefaulGroupRange` verbatim, no Functions overview outside Properties, no MCP/CLI edit | — (closed). Boundaries lift by a supported-schema export using them (ADR-0124) |
| `FUTURE-05` | LATER → LATER | Phase 0: `.knxprod` update preview/install/backup/restore shipped in CLI (ADR-0117) **and** web (ADR-0118, Goal B5) | Only an online catalogue source | A public, licence-clear catalogue feed or download interface usable without vendor approval, plus its own ADR (provenance, signature/integrity, offline fallback) |
| `FUTURE-03` | LATER → LATER | Phase 0: core, native store, diff, report opt-in, service, HTTP, Notes view, help paragraph (`help.topic.workbench.p4`), desktop reload guard all shipped | ADR-0031's explicit delete disposition is reachable only in the core (`Batch` retarget/delete + entity delete in one undo step, `knx-core/tests/notes.rs`); web/CLI deletes of a note target just refuse with a generic error. Selective import refuses source notes | A user-facing disposition flow (move to project / retarget / delete, one undo step) in the web delete path, and a selective-import note mapping |
| `FUTURE-01` | LATER → LATER | Phase 0: read-only MCP (ADR-0090, `d9056440`), opt-in literal rename writer (ADR-0116), `list_functions` (`236c64ae`) | Any other agent mutation (create/delete/re-address, links, devices, parameters, imports), in-app chat | A new named mutation with its own revision/permission/validation/audit contract, as ADR-0116 did for renames |
| `FUTURE-02` | LATER → LATER | Phase 0: AP5 literal prefix/suffix recipe in CLI (`02d80584`), web **Rename…** (Goal B7) and MCP (ADR-0116) | Templates beyond the one literal recipe, recorder, scheduler, selection expansion | A second named operation with deterministic preview and atomic undo-capable apply, or an ADR for a template layer |
| `PDB-04` | LATER → LATER | Phase 0: readonly source/HTTP + Time/Slider controls (11 Oct), Checked editor in Parameters (12 Oct), revision-bound writes from the web (Goal B1a `d705a2b8`, 13 Oct). Phase 2: no new primary source (below) | `TypeColor`/`TypePicture` value codecs; module-scoped parameter mapping | A documented codec per lexical form (or an owner-approved controlled comparison), and a module mapping contract (see `KL-68` for `RepeatIndex`) |
| `KL-170` | LATER → LATER | Phase 2: no new primary source (below) | Little-endian placement | Unchanged: probe output from ETS-programmed devices per manufacturer ([dossier](../limitations/KL-170-parameter-byte-order-download-boundary.md)) |
| `KL-171` | LATER → LATER | Phase 2: no new primary source | UTF-8 length/truncation rule | Unchanged: a documented source for octet-vs-character size and cut rule |
| `KL-172` | LATER → LATER | Phase 2: no new primary source | `LoadedImage`/`CheckSums` payload format | Unchanged: a public format description or an owner-approved controlled read-back comparison |
| `GAP-T30-04` | LATER → LATER | Phase 2: no new primary source | Differential (changed-parts) download | Unchanged: [readiness contract](../contracts/incremental-download-readiness.md) prerequisites and D3–D6 hardware approvals |
| `KL-110` | LATER → LATER | Phase 2: no new primary source | PL110 support and `PID_GROUP_RESPONSER_TABLE` | Unchanged: PL110 scoped as its own task ([readiness contract](../contracts/powerline-readiness.md)) |
| `MODEL-05` | BLOCKED_EXTERNAL → BLOCKED_EXTERNAL | Re-check: the only new private `.knxproj` files (13 Oct) have one installation each | Multi-installation Site | Unchanged (ADR-0125): a multi-installation ETS export or an explicit native-site requirement |
| `IMPORT-02` | BLOCKED_EXTERNAL → BLOCKED_EXTERNAL | Re-check: no authorized manifest; the 13 Oct pair is not admissible (below) | Historical restore-point regressions | Unchanged: authorized ETS restore-point exports plus independently reviewed baselines ([harness](../AUTHORIZED_RESTORE_EXPORTS.md)) |

## Phase 0 — stale Evidence reconciled

- **FUTURE-05.** The row's Evidence still said "Web UI … later scope". The web
  catalogue update shipped on 12 October (ADR-0118: preview/apply/backups/restore
  routes; Goal B5 routes every `.knxprod` through preview). Only the row's
  original subject is left: an *online* source. KNXBench downloads nothing today
  ([contract](../contracts/catalogue-update.md)); the separate `knxprod-crawler`
  corpus run is research input, not a product feature.
- **FUTURE-03.** "Isolated UI/help/shell open" is stale. Shipped: core
  (`knx-core/src/note.rs`), native store, diff, report opt-in, application
  service, authenticated HTTP, Notes workbench view (12 Oct), help paragraph,
  draft guards including the desktop reload (14 Oct). The ADR-0031 consequence
  that is *not* user-reachable: deleting a note's target. Every delete command
  refuses an implicitly orphaned note atomically
  (`every_existing_delete_command_refuses_an_implicitly_orphaned_note_atomically`)
  and the core supports disposition plus delete in one undo step
  (`explicit_note_disposition_and_target_delete_share_one_undo_step`), but no web
  or CLI path offers it; the user sees the generic `note validation refused`
  text. Selective import refuses source notes (`selective_import/merge.rs`).
- **FUTURE-01 / FUTURE-02.** Both rows' first named slices shipped with their
  own contracts (ADR-0090, ADR-0116; AP5/B7). Residue is open-ended and stays
  `LATER`: other agent mutations; further automation operations.
- **PDB-04.** Delivered slices listed in the table. The limitations summary
  "the web host still sends unbound ones" was stale since Goal B1a
  (`d705a2b8`, 13 Oct; `ParameterPanel.binding.test.tsx`,
  `e2e/parameter-editors.e2e.ts`) and is corrected in the
  [dossier](../limitations/checked-parameter-editors.md).
- **Manual.** [Ideas and roadmap](../manual/ideas-and-roadmap.md) still called
  notes and Functions unimplemented and MCP/automation edit-free; corrected with
  dated notes.

## Phase 2 — evidence re-check (no new primary source)

**Method.** Last check: `fb66e292` (10 Oct) and the
[A6/A7 review](2026-10-13-goal-a6-a7-evidence-review.md) (13 Oct). Checked on
17 October 2026:

- **Local KNX specification collection** (`knx-spec-kb/sources`, 189 files):
  newest file 7 October; nothing newer than 10 October.
- **Private corpus** (`OriginalData/`, 515 files, names/aggregates only): 32
  files newer than 13 October — the two ETS6 plugin-pair `.knxproj` exports and
  their provenance (13 Oct), the Manufacturer Tool slide deck already assessed
  in [parameter-byte-order](../research/parameter-byte-order.md) (16 Oct), the
  Weinzierl kTux files and the System-2 public packages of the issue-22 track.
  Both `.knxproj` files: schema 23, one installation, zero `Function`,
  `ParameterByteOrder`, `TextParameterEncoding`, `LoadedImage` or `CheckSums`.
- **KNX Standard v3.0.5** (KNX news, 15 September 2026): newer than the local
  v3.0.0 collection, but available to KNX manufacturers via MyKNX only. Its
  announced topics (HSV colour control, Cloud Interface, firmware update, KNX IoT
  tests) are datapoint/system features, not product-XML parameter codecs. Not a
  public source and not obtainable without membership contact; not used.
- **support.knx.org articles** "Differential Download" (360000160060) and
  "Download & Flags": HTTP 403 to automated retrieval; the help-centre API
  answers 401; no Wayback snapshot exists. Unavailable, not assessed.
- **Independent implementations** (secondary, never defining):
  - `XKNX/xknx-editor` (GPL-2.0) head `4239f002` (17 Oct): the encoder's
    byte-order, text and colour paths are unchanged since the cited `e3a2216e`
    (only `DownloadInvisibleParameters` and property keying changed). Its new
    *preflight* reads device bytes and lists changed ranges before a **full**
    download — a read-only comparison like KNXBench's compare plan, not a
    changed-parts write.
  - `metaneutrons/knx-rs` (GPL-3.0, created April 2026, default-branch head
    27 Sep): `knx-rs-prod` hashes `TypeColor` as one `Space` attribute and
    writes `TextParameterEncoding` through verbatim; no codec, byte-order or
    truncation semantics.
  - OpenKNX `OFM-DFA` issue 42 (July 2025, previously uncited): OpenKNX
    developers observed that, without an explicit `TextParameterEncoding`, ETS
    writes strings in the project's encoding, and fixed their producer to
    declare one. A secondary observation about the *default* encoding; it says
    nothing about UTF-8 field sizing or truncation.
  - `jgrahamc/koolenex` (last push 13 Oct, a memory fix) and
    `OpenKNX/OpenKNXproducer` (last push 27 Aug): nothing new for these rows.

**Per-row result — no new contract, no implementation.**

| Row | Exact missing evidence | How the row would be lifted |
| --- | --- | --- |
| `KL-170` | A defining rule for multi-octet placement under `ParameterByteOrder=LittleEndian`; corpus base-image patterns are not proof | Read-only probe output (ADR-0119) from at least one ETS-programmed device per affected manufacturer, then a reviewed encoder change limited to the observed whole-octet numbers with exact boundary tests |
| `KL-171` | Whether a UTF-8 `TypeText` `SizeInBit` counts octets or characters, and how a sequence that does not fit is cut | A documented source for both rules; then image tests with 1–4-octet characters at, at+1 and over the boundary, and a corpus check of the 21 `utf-8` programs |
| `KL-172` | The binary format inside inflated `LoadedImage`/`CheckSums` | A public format description, or an owner-approved controlled read-back on the owner's own device (aggregate agreement only enters the repository) |
| `PDB-04` | A documented `TypeColor` codec (lexical forms differ, `@Space` uninterpreted) and any `TypePicture` value contract; module-scoped mapping beyond the delivered subset | A documented codec per form with round-trip and refusal tests, or an owner-approved controlled comparison; a module mapping contract with real repeated-module evidence (`KL-68`) |
| `GAP-T30-04` | Device scope, immutable image receipt, exact baseline, writable-region diff, recovery receipt and provenance ([readiness](../contracts/incremental-download-readiness.md#required-design-evidence-before-implementation)); the KNX help article on differential download is unreachable | The six design prerequisites met for one nominated device/topology and the applicable CP/RES/profile clauses, then D3–D6 under separate hardware approvals |
| `KL-110` | Any PL110 scope: no device, medium adapter or test bench | PL110 scoped as its own task (P0–P1 offline first, P2–P4 each with its own approval) per the [readiness contract](../contracts/powerline-readiness.md) |

## Externally blocked rows — lift conditions re-checked

- **MODEL-05** ([ADR-0125](../adr/0125-multi-installation-site-is-blocked.md)).
  No multi-installation ETS export arrived: both new private `.knxproj` files
  have one installation. No native-site requirement was recorded by the owner.
  Unchanged; no workaround.
- **IMPORT-02** ([harness](../AUTHORIZED_RESTORE_EXPORTS.md)). No authorized
  manifest exists. The 13 October pair is two byte-distinct states of one project,
  but its own provenance states it was produced by an offline host around the
  ETS object model (`ToolVersion="0.0.0.0"`, no licence certificate, store
  HMAC/installer binding not executed), is "KNXBench import test input only",
  and comes from no restore point. It belongs to the parked plugin-host track
  and has no independently reviewed baseline, so it does not meet the condition.
  Unchanged; no workaround.

Sources retrieved for this review are kept in private scratch only; no licensed
or GPL text was copied into the repository.
