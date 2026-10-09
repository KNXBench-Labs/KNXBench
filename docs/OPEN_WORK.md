# Open work after the 0.1 alpha

**Audit:** 2026-10-08, against `origin/main` `608a204b`. This page is the one
place that answers "what is not done?". It replaces the three finished alpha
goals (removed 2026-10-08, see [REMOVED_DOCS](history/REMOVED_DOCS.md)). It does not change any
status by itself: per-ID status stays in the [ledger](status/LEDGER.md),
behaviour boundaries in [KNOWN_LIMITATIONS](KNOWN_LIMITATIONS.md).

**Freshness note:** [all twelve original ideas and the roadmap](status/2026-10-08-ideas-roadmap-audit.md)
were checked against actual source/callers. Read-only MCP, Flow, L3 legacy web
upload, commissioning history UI, website/story publication and the original
humour templates are delivered; their broader boundaries are not new tasks.
The [known-issues/status follow-up](status/2026-10-08-known-issues-status-audit.md)
also reconciles current source claims, historical limitations and evidence type;
it changes documentation, not owner rows or device behavior.

Short version: **every goal file is finished.** Nothing in the ledger is
`TODO`, `IN_PROGRESS`, `BLOCKED_EXTERNAL` or `WAITING_*`. What remains is
(1) work that is running or waiting right now, (2) parked unpublished packages
and publication follow-up, (3) deferred items with a recorded user decision, and
(4) later scope that never was an alpha task.

## How this was checked

- Historical goal checkboxes, counted before removal: `alpha-release-goal.md` 93
  checked / 7 open, `goal-ui.md` 37 / 0, `goal-commission.md` (no checklist,
  §4 completion condition). The 7 open boxes are all user-deferred or
  optional (section 3).
- Ledger counts (its own *Counts* line, `check-ledger` green, after the
  2026-10-09 owner cleanup): snapshot 180 rows — DONE 39, ACCEPTED_BOUNDARY 119,
  LATER 22; post-snapshot 11 rows — DONE 9, ACCEPTED_BOUNDARY 2.
- Handover `.ai/CURRENT_STATE.md` (newest entries per track), worktree list,
  and a path-by-path check of which root files exist on `origin/main`.
- Current ledger mechanically recounted: 191 distinct IDs, 48 delivered,
  121 accepted boundaries and 22 later-scope rows; no active/waiting rows.
  Every later-scope row is owned by `later`; no finished session owns open work.
  These are unequal source IDs, not a product-completion percentage.

## 1. Running or waiting right now

| Track | State | Next step | Where |
| --- | --- | --- | --- |
| Legacy ETS3 product databases (`.vd3`–`.vd5`) | L1 inspection, L2 import/secret withholding and L3 web upload/password handling are on `main` | Untyped `TypeNone` spacer presentation still needs a separate fix (KL §128). The large `.vd5` package (Siemens sample: 173 MB payload) exceeds the 64 MiB bounds; measure before widening them. **L4** device download remains later scope. | [ADR-0094](adr/0094-legacy-exim-product-files.md), [VD4 import](VD4_PRODUCT_DATABASE_IMPORT.md), KL §128 |
| Website / published Evolution Story | Edition `2026-10-08.4` is approved and deployed with the site; first publication is complete | HTTP→HTTPS redirect verified (301, 2026-10-09); IPv6 verification, Pages action-runtime maintenance and owner's privacy review; optional WebKit evidence remains separate | [story/](../story/README.md), [WEBSITE](WEBSITE.md), ADR-0068/0095 |

**Commissioning refusal buckets are an evidence backlog, not a running track.**
The recorded house/corpus snapshot and module-placement, `placed by Property`,
float, priority/value and non-memory-mask gaps remain in
[commissioning research §19.11–19.18](research/commissioning.md). The user's
hardware/vendor/ETS deferral in section 3 still applies; no new write or
experiment is authorized by this documentation audit.

## 2. Parked packages and publication follow-up

The root's two formerly parked packages are delivered. The separate
**documentation refresh and ideas/roadmap audit** remain local in
`KNXBench.worktrees/docs-refresh-20261008`, now based on `608a204b`: revised
README/manual, real media, documentation checker and this reconciliation.
Their review/commit/publication remains a delivery step, not a missing app
feature. No commit or push is performed by this audit.

- **Community evidence** (File → Analyze support gaps…) is in the repository,
  and its intake lives here (ADR-0091). The interim
  `KNXBench-Contributions` repository was deleted on 2026-10-08.
- **Marketing website** is in the repository (`website/`, ADR-0095) and
  deployed to knxbench.com by `.github/workflows/pages.yml`
  ([WEBSITE.md](WEBSITE.md)).

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

From the ledger's later-scope rows, historical `goal-commission.md` §3c and
the [original-ideas audit](status/2026-10-08-ideas-roadmap-audit.md). Root
`ideas.md` is ignored and is not present in a fresh clone. Each new feature
needs its own scoped goal or ADR when requested; old future-row labels do
not erase subsequently delivered functionality.

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
- **Comparison/report extensions:** applying/merging diffs, three-way
  comparison and native PDF output remain absent; comparison and browser
  print-to-PDF already exist. These are recorded boundaries, not active work.
- **Features:** project notes (`FUTURE-03`, ADR-0031 accepted, not built),
  repetitive-task automation (`FUTURE-02`), a mutation-capable LLM/MCP surface
  (`FUTURE-01`; the read-only MCP server shipped, ADR-0090), dashboard
  drill-down, mobile app, native ports and manufacturer online-catalog
  updates (`FUTURE-05`). More humour (`FUTURE-08`) means optional expansion:
  the original thirty-entry lists already exist. Per-category motion controls
  remain later scope; global motion controls already work.
- **KNX Secure research:** where usable Data Secure runtime keys come from,
  project file or keyring (`R-SEC-01`, RESEARCH §9).
- **Tooling/statistics** (`TOOLS-01` to `TOOLS-05`): where the AI statistics
  generator lives, and the measurement gaps it discloses (network traffic, end
  times, unattributed sessions, CSS/shell metrics).
- **Platform validation:** native WebKitGTK, Orca/screen readers and complete
  accessibility, Firefox for the app; these are disclosed in every UI receipt.

### Ledger owner cleanup (2026-10-09)

On the user's go the ledger rows that lagged the code were reconciled:
`FUTURE-04` now cites the delivered telegram flow (`FLOW-01`, ADR-0077/0085),
`FUTURE-01` names the delivered read-only MCP slice (ADR-0090), `FUTURE-08`
states that only optional copy remains, and the four later-scope rows still
owned by finished sessions (`FUTURE-05`, `KL-110`, `GAP-T30-04`, `KL-43`) moved
to owner `later`. Routes naming removed goal files are kept as history.

## 5. Repository chores

- **CI** is disabled (`gh workflow disable CI`, 2026-10-07) until the user
  re-enables it.
- **Repository visibility:** `KNXBench-Labs/KNXBench` became public on
  8 October 2026; the alpha.5 AppImage and checksum file are public too.
  Community-evidence/forms consolidation shipped separately; the interim
  intake repository is already retired. Private mailbox verification and the
  owner's retention decision for the private pre-launch rollback repository
  remain separate; this audit performs neither action.
- **Stale worktree:** `$TMPDIR/devnav-ro` (read-only interview scratch) can go
  once the devices-navigation interview ends.
