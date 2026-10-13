# Unassigned work: two bounded goal sessions

Planning-only audit, 13 October 2026. **No implementation, ownership transfer,
lock acquisition, tests, commit, push, release or deployment is authorized by
this document's creation.** The owner may start either prepared goal separately.
Run Goal A before Goal B; separate subjects do not make shared API/UI/docs safe
for concurrent editing.

## Evidence and scope

Freshly fetched source: `4c0bea4723d050bef745e32ee2d52a4b978eb9d6`.
Shared root: `a1fcc8f2af37581a1dca79223c665187c15dec5e`, deliberately stale and
containing foreign edits. Audit uses the fetched source, not root source files.

Inspected all 189 catalogue summaries, mechanically parsed all 202 ledger rows,
read OPEN_WORK/ROADMAP and current architectural/host contracts, inspected
selected detailed limitation histories and relevant implementation entry points,
and reconciled recent handovers with profile session records, worktrees,
processes and the open GitHub issue list. This is **static source/planning
verification**, not a new runtime, full-product, native or hardware acceptance.

Machine-derived ledger inventory:

| Status | Rows |
| --- | ---: |
| DONE | 62 |
| ACCEPTED_BOUNDARY | 121 |
| LATER | 18 |
| BLOCKED_EXTERNAL | 1 |
| Total | 202 |

All 19 nonterminal rows have owner `later`; no old alpha/UI/commission owner
residue exists. **`later` is a routing category, not a live session assignment.**
Conversely, terminal status does not mean all remaining capability limits have
been implemented. Catalogue dossiers without rows must not receive invented
formal statuses.

At observation there were no linked worktrees or product test/build workers;
only the shared root was registered. Latest published web-lock disposition is
`released by claude-release-alpha11`. This is a snapshot, not a future lock lease.

## Do not duplicate these tracks

| Work | Disposition / ownership evidence |
| --- | --- |
| Alpha 11 publication | Delivered by `eb27dff1f64f`; immutable release, no second release task. |
| Issues #8/#9/#10, onboarding flake, KL-170 tester UI | Delivered in fresh main. Do not reimplement them. |
| Notes host, instructions host, Time/Slider host, catalogue update/restore UI, notes CO targets/browser guard/review translations | Delivered; their broader later rows do not erase those slices. |
| Parked native-evidence and UX adoption | Delivered, including residue/ADR-index reconciliation. Do not adopt the old branches again. |
| KL-170 byte-order observations and first tester report | Follow-up of `15b70297b767` and release/reporter communication; waiting for actual external evidence, not free encoder implementation. |
| Group-address issue #4 / GA7 / GA8 | Reporter-follow-up `245ced5b5271`; derived numeric tree/virtualization only on the recorded request or failing measurement. |
| ETS 5.7 issue #7 reporter project and issue #6 host confirmation | Existing reporter follow-ups. Public sample tests are not the reporter's private project/host verification. |
| SmartSensor/econ host, legal decision, data-only project transfer | Owner track `1c590e22e529`; host/legal decision explicitly parked. Do not execute/extend/publish the host or confidential research. A technically possible data-only transfer is not an unassigned task. |
| Additional large-document restructuring | Proposal belongs to `468712901140`, awaiting owner approval. No automatic journal/research/ledger restructuring in these goals. |
| Optional real third-party MCP client acceptance | Existing host/tooling handoff and reserved client choice. Not a new general MCP writer task. |
| SpectrumKNX cooperation | Existing discussion `4da65393214e`; no integration scope assumed. |

Source: newest [.ai/CURRENT_STATE.md](../../.ai/CURRENT_STATE.md),
[OPEN_WORK](../OPEN_WORK.md), [data-tooling ownership](../contracts/data-tooling-ownership.md),
[group-address goal](../goals/group-address-four-views.md).

## Goal A — data integrity, model and evidence-gated import

| Package | Actual remaining work | Readiness / acceptance |
| --- | --- | --- |
| A1 | Stop read-only native CLI/comparison paths migrating the original (`KL-157`) | Implementable by reusing `open_existing_read_only`; do not reinvent migration/copy logic. Preserve old/current input bytes, working history and opaque context. Ordinary writable Open remains a separately disclosed boundary, not silently changed policy. |
| A2 | Revision-bound parameter writes and installation renames (shared-project concurrency, checked-editor/installation-node limits) | Implementable. Actual request DTOs contain only etsId/raw or name; client reply suppression is not server concurrency control. Bind real incarnation/revision and exact installation/target, validate+persist+publish atomically, retain stale drafts and return a typed conflict. Define compatibility of old callers explicitly. No per-user/account system. |
| A3 | Orderly stop awareness of download/address workers (`KL-163`) | Implementable as admission shutdown plus a configurable bounded drain of existing workers, tested offline. Never invent safe cancellation of a partially written device; timeout/second signal/kill remain disclosed. Docker/operator timeout contract must match the server's wait. No running user server/container is stopped. |
| A4 | Functions backend/model (`MODEL-07`); multi-installation Site design (`MODEL-05`, `KL-127`) | Fresh current-schema work only. WIP `1154b72f` is schema-12, unreviewed and failed acceptance; read it as evidence, never merge it wholesale. Functions need normalized model/commands, retained unknowns, import mapping, native root/history migrations, diff/report/projection/API and independent fixtures. Site work first resolves installation ownership and actual sample evidence in an ADR; if insufficient, deliver a concrete blocked design rather than invent cross-installation moves. UI belongs only to B1. |
| A5 | Legacy atomic types 3/string and 5/long-enum (`KL-128`) | Existing offline conversion oracle is a useful research lead. Compare admitted source and independent conversion before adding mappings; test ranges/defaults/enums/retained unsupported data. Do not execute ETS/vendor code, a download or guessed legacy procedures. Missing oracle semantics is an item blocker, not full-goal failure. |
| A6 | Remaining module/source contracts (`PDB-04`, `KL-68`); encoding/loaded-image prerequisites (`KL-171/172`) | Bounded evidence review only where something new exists. Public Functions/schema documentation is not a Color/Picture codec or Repeat semantic contract. No repeated login/SDK requests, guessed bytes, activation rules or stored-image download reuse. Record exact absent contract and safe fallback. KL-170 remains the other owner's external follow-up. |
| A7 | Future media/differential-download routing (`KL-110`, `GAP-T30-04`) | Reconcile the already delivered prerequisite/test-plan scope. Runtime implementation is deliberately excluded. State the remaining specification/device/recovery gates; do not rerun an old research package or claim PL/differential support. |

### Concrete source references for A

- `crates/knx-store/src/migration.rs`: `open_existing_and_migrate` opens read-write;
  `open_existing_read_only` already migrates older stores in a memory copy.
- `apps/knx-cli/src/main.rs`: read-only export/diff/helper paths still use the
  former opener; `crates/knx-app/src/comparison.rs` does too.
- `apps/knx-server/src/routes.rs`: `SetParameterValueRequest`,
  `RenameInstallationBody`; `domain.rs`: authoritative parameter/rename services.
- [KL-157](../limitations/KL-157-in-place-native-project-migration.md),
  [KL-163](../limitations/KL-163-server-shutdown-and-active-writes.md),
  [parameter host](../contracts/parameter-editor-host.md),
  [installation-node](../contracts/installation-node.md).
- `apps/knx-server/src/graceful_stop.rs`: current 5-second request drain and
  monitor/scan release, explicitly no wait for device write workers.
- `crates/knx-productdb/src/legacy/mapping_program.rs`: current atomic-type switch;
  [legacy mapping research](../research/legacy-vd-mapping.md).
- [parked triage](2026-10-12-parked-work-triage.md),
  [DATA_MODEL](../DATA_MODEL.md), [IMPORT_EXPORT](../IMPORT_EXPORT.md).

## Goal B — workbench completion, UX and platform evidence

Start only after A has ended and its implemented contracts are published or its
research-only outcomes are explicitly recorded. Never manufacture a UI for a
backend package A could not admit.

| Package | Actual remaining work | Readiness / acceptance |
| --- | --- | --- |
| B1 | Workbench consumers of A's Functions/Site outcomes and revision guards | Bind real normalized identities, reuse Explorer/Inspector/selection, preserve source provenance, support undo/redo/reopen. No duplicate planner or own revision counter. Only implement admitted Site operations. |
| B2 | Native desktop draft/reload guard and targeted platform validation | Browser beforeunload is already delivered. Verify desktop webview navigation/reload separately and add a native guard where feasible. Use isolated fictional projects; exercise real WebKitGTK, Orca and Firefox when available. Report missing environment or inaccessible native actions honestly. No whole-WCAG/native certification from Chromium. |
| B3 | Read-only stored BusAccess/IP inspection | Reuse retained source/payload and application projection boundaries. Display exact provenance and unknowns; bound data, disclose sensitive endpoints, default to redacted fields/explicit reveal. Never connect to a retained endpoint or auto-select it as a gateway. Refuse ambiguous source-device/line identity. |
| B4 | Native-authored device instruction workflow (FUTURE-03 residue) | Reuse existing normalized instruction overlays and preview/confirm/undo/native history. Distinguish no imported original from empty imported original; reset cannot invent one. No broader RTF/resource execution or ETS export. |
| B5 | Supplied-file catalogue workflow follow-ups (`FUTURE-05`) | Reviewed backup deletion with fresh allowlisted identity and confirmation, and routing/deprecating the web direct-install escape in favour of existing preview/apply. Keep restore and legacy routes working; no arbitrary path deletion, online source, application rebinding or deferred version selector/pinning. |
| B6 | Achievement concurrency (`KL-164`) | Add bounded idempotent event/delta handling so concurrent increments survive and retries do not double count. Test two clients, duplicate/reordered requests, persistence failure and reset/restart boundaries. No user accounts and no reconstruction of missing historical increments. Shared installation record remains. |
| B7 | Bounded naming recipe UI (`FUTURE-02`) | Reuse `knx-edit`/existing literal prefix-suffix recipe, explicit stable-ID selection, revision-bound preview, confirmation and one undo step. No macro VM/recorder/scheduler, generated selection, agent writer or bus rights. |
| B8 | Current prose/report/doc reconciliation (`KL-37/48/66`, `TOOLS-01–05`, optional `FUTURE-08`) | Complete EN/DE report-owned/newly touched prose while retaining raw technical evidence. Fix source-contradicting catalogue/OPEN_WORK/ROADMAP summaries. Existing collector is in separate ProjectStats: document its actual provenance/measurement gaps, not a new KNXBench collector or changes in another workspace. Humour mechanism is complete; extra copy is optional and must never obscure refusals. No giant-doc relocation under the other owner's pending proposal. |

### Concrete source references for B

- [native evidence](../limitations/native-import-evidence-and-recorded-device-state.md):
  retained BusAccess/IP data is not displayed; import reports and recorded states
  are already delivered.
- [instruction host](../contracts/instruction-editor-host.md): native-authored
  devices lack imported-source authority in the current workflow.
- [catalogue contract](../contracts/catalogue-update.md): preview/apply/restore and
  backup listing already exist, no deletion endpoint or device version rebinding.
- `apps/knx-server/src/achievements.rs`, `achievement_routes.rs`: current counter
  merge is max; [KL-164](../limitations/KL-164-shared-achievement-counters.md).
- `apps/knx-web/src/draftUnloadGuard.ts`, [note host](../contracts/project-note-editor-host.md):
  browser guard exists; desktop reload acceptance remains separate.
- [naming contract](../contracts/offline-name-recipe.md): existing service/CLI,
  not a missing automation engine to build from scratch.
- `crates/knx-report/src/render.rs`: EN/DE chrome exists, but selected header/field
  labels remain report-owned English strings.

## All nonterminal ledger rows: disposition, not a duplicate status ledger

| Source IDs | Assignment / what remains |
| --- | --- |
| `MODEL-07`, `MODEL-05` | A4 model/contracts/import; B1 only the admitted workbench consumers. |
| `PDB-04` | A6 exact missing contract/mapping evidence. Existing Time/Slider hosts are done; Color/Picture values remain blocked, not a free coding task. |
| `FUTURE-02` | B7 bounded UI reuse; a general automation engine remains outside these goals. |
| `FUTURE-03` | B2/B4 native guard/instruction residue; notes service/host is done. Do not close the broad row without checking its own original contract. |
| `FUTURE-05` | B5 supplied-file workflow residue. Online catalog and version switching are not implemented by it. |
| `TOOLS-01`, `TOOLS-02`, `TOOLS-03`, `TOOLS-04`, `TOOLS-05` | B8 reconciliation/provenance. Runtime collector changes need a separate ProjectStats task, reliable input and the correct workspace. Missing measurements remain N/A. |
| `FUTURE-08` | B8 optional copy only; no missing humour system. |
| `KL-110`, `GAP-T30-04` | A7 prerequisite routing only, runtime/hardware remains excluded. |
| `KL-171`, `KL-172` | A6 gated research only when new evidence exists. |
| `KL-170` | Existing owner/reporter follow-up, excluded from new encoder work. |
| `IMPORT-02` | Externally blocked: explicitly authorized same-project revision exports and independently reviewed baselines. Existing harness is done. No goal can fabricate that input. |
| `FUTURE-01` | Existing opt-in literal MCP writer is delivered. Third-party client acceptance is reserved; every further mutation/agent/chat workflow requires separate scoped design, not these goals. |

## Limitations already lifted or easily misread

- **KL-29:** final 4 October history and current CLI source already use project
  address style and shared distinct-name resolution. Generated summary still
  describes three-level/flattened behavior. This is a B8 summary repair, not new
  monitor implementation.
- **KL-13:** final dossier update records real public protected ETS4/ETS5.7
  exports decrypting on 12 October. Summary still says only synthetic evidence.
  Fix that narrow claim; **AES ETS6 remains refused**.
- **KL-43:** per-category motion controls are delivered; later-scope prose in
  OPEN_WORK/ROADMAP/root-local ideas still describes them as absent.
- **FUTURE-03/FUTURE-05:** old cumulative evidence cells contain superseded
  UI-absent statements, while dated host updates and source show actual delivery.
  Preserve history but make the current residue explicit; do not close broad
  rows merely by deleting their history.
- Read-only MCP uses the existing copy-migration primitive, so “every reader
  migrates in place” needs caller-specific wording. CLI/comparison still require A1.
- Documentation audits may update generated summaries outside retained-history
  markers. Preserve every retained dossier record and alias; regenerate with
  `python3 tools/limitations_catalogue.py --write`. The ledger alone owns status.

## Remain outside both implementation goals

No evidence supplied by this audit lifts these boundaries:

- Proprietary Color/Picture codec contract, repeated-module runtime semantics,
  UTF-8 image layout/termination, LoadedImage/CheckSums download semantics.
- AES ETS6 import without a real authorized protected sample; untested project
  schemas and schema-20 SubModule grammar without adequate independent fixtures.
- KNX Secure, PL/RF/IP-device configuration, filter tables, new live downloads,
  power-loss/recovery, K6/serial/address writes, vendor hosts/plugins.
- Explicitly deferred allocator phases 3–5 and product version pinning/selector.
- General diff apply/three-way merge, native PDF, independent textual project
  format, executable extension API, full macro/agent UI, mobile or other native OS
  ports: technically conceivable **new product scope**, not missing pieces to
  silently append to these bounded sessions.
- CI re-enable, image signing/base-image rebuilds, release/tag/assets/image push,
  running-container replacement, mailbox/privacy/legal/retention decisions.

An accepted boundary is not “impossible forever”; it is simply not permission
for this audit or these focused implementation goals to expand that contract.

## Public-source follow-up

Repository research remains the evidence of record for the missing format
contracts. A help-center page extraction attempt failed because the configured
extractor backend is search-only; a direct HTTP retrieval of the candidate page
returned 403. **No fresh public document body, PDF clause, diagram or value
contract is admitted by this audit.** A URL, search lead or prior citation is
not proof of retrieval, and this access failure does not establish global absence.

The final citation review removed unsupported draft wording about inspected
Schema20/Parameters/Modules material and release-note contents. Those claims
must not be reused as evidence for these goals. The implementation/task routing
is supported by the actual repository and source inspection, not that wording.

See the earlier exact-definition/mapping investigation in
[product-database research](../research/product-database.md). No fresh SDK/type
body, login bypass, vendor execution or codec implementation is claimed.
Current Color/Picture, module, UTF-8 and loaded-image prerequisites remain as
recorded there and in the source-bound contracts; only genuinely new authorized
material can lift them.

## Goal start/delivery rules

Each actual goal invocation must re-fetch and recheck newest ownership/queue and
source. Use its own worktree, never reset or synchronize the shared dirty root.
Take/release the existing web lock per UI package; preserve foreign branches,
WIP, stash, private research and journals. Goal B starts after Goal A ends.

Use small reviewed packages, test-first where behavior changes, relevant
migration/native/roundtrip/concurrency/HTTP/client tests, real isolated browser
acceptance, complete frozen-source gates and the existing shared gate leases.
Run corpus-backed cases against the actual root-only corpus through supported
paths/overrides; a worktree skip is not evidence. Keep failed attempts separate.

Update affected docs/contracts/ADR/status/limitations and handover. Commit as
`KNXBench <github@knxbench.com>` without coauthor. When authorized by the actual
goal prompt, normally integrate/push each fully green package and verify exact
remote/source readback, then archive evidence and clean only owned scratch.
No subagents or quota queries. Respect explicit pauses. Continue independent
packages despite a named item blocker. End each goal with delivered versus
blocked versus excluded outcomes, not a whole-compatibility claim.
