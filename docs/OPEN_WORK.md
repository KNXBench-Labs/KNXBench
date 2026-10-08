# Open work after the 0.1 alpha

**Audit:** 2026-10-08, against `origin/main` `b34afcc8`. This page is the one
place that answers "what is not done?". It replaces the three finished alpha
goals (removed 2026-10-08, see [REMOVED_DOCS](history/REMOVED_DOCS.md)). It does not change any
status by itself: per-ID status stays in the [ledger](status/LEDGER.md),
behaviour boundaries in [KNOWN_LIMITATIONS](KNOWN_LIMITATIONS.md).

Short version: **every goal file is finished.** Nothing in the ledger is
`TODO`, `IN_PROGRESS`, `BLOCKED_EXTERNAL` or `WAITING_*`. What remains is
(1) work that is running or waiting right now, (2) unpublished local work in
the root checkout, (3) deferred items with a recorded user decision, and
(4) later scope that never was an alpha task.

## How this was checked

- Goal checkboxes, counted from `origin/main`: `alpha-release-goal.md` 93
  checked / 7 open, `goal-ui.md` 37 / 0, `goal-commission.md` (no checklist,
  §4 completion condition). The 7 open boxes are all user-deferred or
  optional (section 3).
- Ledger counts (its own *Counts* line, `check-ledger` green): snapshot 180 rows
  — DONE 38, ACCEPTED_BOUNDARY 119, LATER 23; post-snapshot 11 rows — DONE 9,
  ACCEPTED_BOUNDARY 2.
- Handover `.ai/CURRENT_STATE.md` (newest entries per track), worktree list,
  and a path-by-path check of which root files exist on `origin/main`.

## 1. Running or waiting right now

| Track | State | Next step | Where |
| --- | --- | --- | --- |
| Legacy ETS3 product databases (`.vd3`–`.vd5`) | L1 inspection, L2 import and secret withholding are on `main` | **L3:** server upload, password dialog, remembered password (one 0600 file under `$XDG_CONFIG_HOME/knx/`), visual web check. Then the `.vd5` package (Siemens sample: 173 MB payload exceeds the 64 MiB bounds; measure memory first). **L4** download later. | [ADR-0094](adr/0094-legacy-exim-product-files.md), [VD4 import](VD4_PRODUCT_DATABASE_IMPORT.md), KL §128; worktree `legacy-vd-l3-20261008` |
| Devices navigation ("Geräte" view, every device mention links to the editor) | Grill-me round 1 (Q1–Q5) asked, not answered | Answers, then synthesis and an explicit go | `.ai/logs/2026-10-08_claude_devices-view-grilling.md` |
| Project-evolution story | Edition `2026-10-08.4` (cutoff 8 October) on `main`; [goal](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/PROJECT_EVOLUTION_GOAL.md) delivered; `2026-10-08.4` approved for publication on 8 October | Publication step: public page variant (no private-preview banner, no `noindex`), hosting on the owner's domain, optional WebKit check | [story/](../story/README.md), ADR-0068 |
| Commissioning refusal buckets (ADR-0086) | House: 32 untested / 2 unsupported / 1 excluded; corpus 1 verified + 89 untested of 246 programs | Remaining buckets need better evidence: module instances (placement rule, §19.11), `placed by Property` (9 MDT programs), floats (contradicting data), `Priority=High`, parameter-value, non-memory masks (`07B0`, `2705`, `0912` …) | [research/commissioning.md](research/commissioning.md) §19.11–§19.18 |

## 2. Unpublished local work in the root checkout

None since 2026-10-08. The root checkout was synchronized after the public
launch; its two finished local packages are delivered:

- **Community evidence** (File → Analyze support gaps…) is in the repository,
  and its intake lives here (ADR-0091). Retiring the interim
  `KNXBench-Contributions` repository is the owner's call.
- **Marketing website** is in the repository (`website/`, ADR-0095). Its
  remaining launch gates are deployment go and release-mode design, host
  privacy page and Pages/domain/HTTPS ([WEBSITE.md](WEBSITE.md)).

Do not work in the root checkout; publish each package from its own worktree
off `origin/main`.

## 3. Deferred by a recorded user decision

These stay open on purpose. Reopen only on a new decision.

| Item | Decision | Ledger |
| --- | --- | --- |
| ADR-0039 phases 3–5 (seal the id allocator, source-mutation gate) | Deferred past the alpha (2026-10-04); phases 1–2 and AR02 are the boundary | `KL-129` ACCEPTED_BOUNDARY |
| Package/application version pinning and a selector | Continued deferral (2026-10-04); first-installed winner with disclosed candidates | `KL-135` ACCEPTED_BOUNDARY |
| Live tunnel from a Docker bridge container (Route Back) | Optional, only with a user go; offline HPAI tests are delivered | `KL-155` DONE (residue in §155) |
| New real-hardware, power-loss, vendor and ETS validation | Removed from commissioning scope (2026-10-04) | many `commission` rows ACCEPTED_BOUNDARY |
| Durable pre-write recovery for K6 button / serial / K13 address writes | Confirmed public address writes fail closed before any tunnel until device-specific recovery exists; every new write needs its own go | ADR-0057–0059, `goal-commission.md` §3 (removed, see REMOVED_DOCS) |
| Reset UI, Web partial-scope selector | Reset UI: accepted refused boundary with a user notice; selector handed to UI and delivered (`KL-142` DONE) | `UI-04` ACCEPTED_BOUNDARY |

## 4. Later scope (never alpha tasks)

From the ledger's 23 `LATER` rows, `goal-commission.md` §3c and the root
`ideas.md`. Each needs its own goal or ADR when the user asks for it.

- **Commissioning, new media and procedures:** Powerline (PL110/PL132), KNX IP
  device configuration (masks `5705h`/`57B0h`), coupler filter tables
  (`0912h`/`091Ah`), `NM_Router_Scan`/`NM_SubnetworkDevices_Scan`, KNX Data
  Secure (`DM_SecureSync_*`), Easy Modes, USB interface configuration
  (`KL-110`, `GAP-T30-04`). RF stays simulator-only.
- **Model/import:** KNX `Functions` (`MODEL-07`, needs an ADR and samples);
  several installations under one Site (`MODEL-05`, needs an ADR); selective
  import of single lines/devices (`IMPORT-01`); ETS restore-point revisions as
  extra private regressions (`IMPORT-02`, needs authorized ETS export);
  semantic evaluation of the `knx_cvexc` XML files (`IMPORT-04`); dedicated
  time/colour/picture/slider parameter editors from Type attributes and UIHints
  (`PDB-04`); independent `.knxproj` samples for untested schemas
  (12/13/14/20/22) and an AES-protected ETS6 sample.
- **History:** persistent undo history (`HISTORY-01`); versioned backups /
  project time travel beyond autosave (`HISTORY-02`).
- **Features:** project notes (`FUTURE-03`, ADR-0031 accepted, not built),
  repetitive-task automation (`FUTURE-02`), a mutation-capable LLM/MCP surface
  (`FUTURE-01`; the read-only MCP server shipped, ADR-0090), dashboard
  drill-down, mobile app, other operating systems, a bigger catalogue of
  humorous messages (`FUTURE-08`).
- **KNX Secure research:** where usable Data Secure runtime keys come from,
  project file or keyring (`R-SEC-01`, RESEARCH §9).
- **Tooling/statistics** (`TOOLS-01` to `TOOLS-05`): where the AI statistics
  generator lives, and the measurement gaps it discloses (network traffic, end
  times, unattributed sessions, CSS/shell metrics).
- **Platform validation:** native WebKitGTK, Orca/screen readers and complete
  accessibility, Firefox for the app; these are disclosed in every UI receipt.

### Ledger rows whose text lags the code

Reported, not changed here (the owner edits its rows):

- `FUTURE-04` (animated "who talks to whom" view) is `LATER`, but the
  session-local telegram flow shipped as `FLOW-01` (DONE) and its follow-up
  (ADR-0085). The row could become DONE/ACCEPTED_BOUNDARY with that evidence.
- `FUTURE-01` cites only research; the read-only MCP adapter (ADR-0090) is a
  delivered slice of it.

## 5. Repository chores

- **CI** is disabled (`gh workflow disable CI`, 2026-10-07) until the user
  re-enables it.
- **Repository visibility:** `KNXBench-Labs/KNXBench` is private; going public
  needs the launch consolidation of community forms/guides (ADR-0091 in the
  root, section 2).
- **Stale worktree:** `$TMPDIR/devnav-ro` (read-only interview scratch) can go
  once the devices-navigation interview ends.
