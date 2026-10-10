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

Short version: **the original alpha goal files are finished at their recorded
scope, not every later feature.** The current ledger includes the owner-approved
source import expansion and its missing authorized-export evidence. Dated audit
counts below remain historical. What remains is
(1) work that is running or waiting right now, (2) parked unpublished packages
and publication follow-up, (3) deferred items with a recorded user decision, and
(4) later scope that never was an alpha task.

## Public-source research follow-up (10 October 2026)

Package 9's bounded research pass for KL-170, KL-171 and KL-172 is complete in
[the source review](research/schema23-offline-boundaries.md). Implementation
rows stay LATER: per-type endian placement, KNX UTF-8 capacity/termination and
the loaded-image binary format still lack an admitted contract. Existing
refusals/source retention remain unchanged; no hardware follow-up authorized.

## How this was checked

- Historical goal checkboxes, counted before removal: `alpha-release-goal.md` 93
  checked / 7 open, `goal-ui.md` 37 / 0, `goal-commission.md` (no checklist,
  §4 completion condition). The 7 open boxes are all user-deferred or
  optional (section 3).
- Ledger counts (its own *Counts* line, `check-ledger` green, after the
  2026-10-09 owner cleanup): snapshot 180 rows — DONE 41, ACCEPTED_BOUNDARY 119,
  LATER 20; post-snapshot 11 rows — DONE 9, ACCEPTED_BOUNDARY 2.
- Handover `.ai/CURRENT_STATE.md` (newest entries per track), worktree list,
  and a path-by-path check of which root files exist on `origin/main`.
- Current ledger mechanically recounted after import expansion: 191 distinct IDs,
  52 delivered, 121 accepted boundaries, 17 later-scope rows and 1 externally
  blocked row (`IMPORT-02`); no active/running source work. The earlier census
  above remains historical. Every later-scope row is owned by `later`; the
  external restore-export prerequisite belongs to the maintainer/ETS owner.
  These are unequal source IDs, not a product-completion percentage.

## Import expansion source integration (10 October 2026)

Selected-device/line merge into the open project, the authorized revision-export
harness and read-only cvexc declaration analysis have completed local acceptance.
The owner subsequently authorized commit/merge/push; [integration acceptance](status/2026-10-10-import-expansion-integration.md)
is separate from the [historical local record](status/2026-10-10-import-expansion-verification.md).
Remaining: real authorized ETS revision exports with independently reviewed
baselines, future-model closure before model changes, and primary cvexc semantics
before enabling any runtime rule. Synthetic browser/CLI checks are not real
ETS/hardware/native accessibility evidence. No release/deployment or bus action.

## Import-integrity merged-source acceptance (10 October 2026)

[Integration proof](status/2026-10-10-import-integrity-integration.md) covers
actual original-input production/native checks and independent source/persistence
comparison. The current ledger has 197 source IDs: 54 delivered, 120 accepted
boundaries, 22 later-scope rows and one external prerequisite; these are not a
product-completion percentage. The dated earlier inventories remain historical.
No release or hardware action follows from successful source integration.

## Private schema-23 sample review (10 October 2026)

A read-only review of a privately supplied ETS6 project lifted KNOWN_LIMITATIONS
§125 and recorded six new limitations (§170–§175) plus evidence updates for §1,
§6, §8, §11, §52, §68, §106, §134, §146 and PDB-9. Open follow-ups, none
authorized yet: little-endian and UTF-8 image encoding (needs a documented
source), repeated-module semantics with a `ValueMap` instance dimension (ADR
first), applying inactive-object overrides after reactivation (preservation
and reporting repaired by the import-integrity package),
content-based classification of project user files/add-in data and the
stored-image reference (research only). The redaction audit of retained network
endpoints is done (§106, 2026-10-10: MAC addresses redacted, every other output
withholds the data or names it). The secure-capable-but-not-activated case is
pinned by a synthetic witness (§8): treated exactly like a plain program, no
Secure semantics added. The import-integrity package now has separate
[merged-source acceptance](status/2026-10-10-import-integrity-integration.md):
archive identity, object trees, unassigned placement, metadata provenance and
source/diagnostic preservation are repaired. Retained extensions and offline
execution limits remain research/follow-up work, not wider runtime support.

## 1. Running or waiting right now

| Track | State | Next step | Where |
| --- | --- | --- | --- |
| Legacy ETS3 product databases (`.vd3`–`.vd5`) | L1 inspection, L2 import/secret withholding, L3 web upload/password handling, the VD5 installer layout with measured bounds and L4 download plans from `s19_block` are on `main` | A first live legacy download (needs the maintainer's device go; none has run). Map atomic types 3 (`string`) and 5 (`long enum`) once ETS's conversion is compared (the `.vd5` has 1,515 such parameters; candidate oracle `SIEMENS_KNX_PDB_Nov_2016_ETS4.knxprod`). Merged legacy procedures (`07B0h`) stay refused. | [ADR-0094](adr/0094-legacy-exim-product-files.md), [VD4 import](VD4_PRODUCT_DATABASE_IMPORT.md), KL §128 |
| Website / published Evolution Story | Edition `2026-10-08.4` is approved and deployed with the site; first publication is complete | HTTP→HTTPS (301) and remote IPv6 HTTPS (four probes, 200/TLS valid) verified on 2026-10-09; Pages Node-24 actions and demo links deployed/verified; owner's privacy review and optional WebKit evidence remain separate | [story/](../story/README.md), [WEBSITE](WEBSITE.md), ADR-0068/0095 |

**Commissioning refusal buckets are an evidence backlog, not a running track.**
The recorded house/corpus snapshot and module-placement, `placed by Property`,
float, priority/value and non-memory-mask gaps remain in
[commissioning research §19.11–19.18](research/commissioning.md). The user's
hardware/vendor/ETS deferral in section 3 still applies; no new write or
experiment is authorized by this documentation audit.

## 2. Parked packages and publication follow-up

**Offline AP1 diagnostics (2026-10-09):** local implementation and fresh-main
integration acceptance are complete; normal main publication is verified.
[Delivery/readback](status/2026-10-09-offline-ap1-verification.md). Not a release.
[Contract](OFFLINE_PROCEDURE_RESOLUTION.md), ADR-0098: 69 partial source-bound
reconstructions / 6 source-limit unavailable from 75 selected real candidates;
no expanded real sequence, complete plan or new live support. Normal main publication is owner-authorized and tracked in the delivery receipt;
later semantic materialization is not implicitly authorized. Existing commissioning ledger dispositions are unchanged.

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

Native restart-safe undo and project versions have been delivered in the
user-authorized history package (code `62464763`, [contract](PROJECT_HISTORY.md),
[verification](status/2026-10-09-project-history-verification.md)). They are no
longer future work. Same-file versions still need independent backups; native
accessibility, physical power-loss and hardware/ETS recovery remain separate.

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
  several installations under one Site (`MODEL-05`, needs an ADR); ETS
  restore-point revisions as extra private regressions (`IMPORT-02`, requires
  authorized ETS exports and reviewed baselines). Selective lines/devices and
  bounded `knx_cvexc` content analysis have verified local implementation and
  separately authorized [source integration](status/2026-10-10-import-expansion-integration.md);
  neither replaces the missing authorized revision exports. Dedicated
  time/colour/picture/slider parameter editors from Type attributes and UIHints
  (`PDB-04`); independent `.knxproj` samples for untested schemas
  (12/13/14/20/22) and an AES-protected ETS6 sample.
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

- **Docker Hub release image** (ADR-0097, KL §168): live since
  `v0.1.0-alpha.6`; Alpha.7 version/latest and both architecture manifests
  externally verified on 2026-10-10, including anonymous HTTPS/save/reopen
  smoke. [Publication evidence](evidence/release-alpha7-2026-10-10.json).
  Open by choice: image signing, Docker Hub description sync, scheduled
  base-image rebuilds, and the `latest` rule at the first stable release.
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
