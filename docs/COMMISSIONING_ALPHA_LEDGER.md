# Global commissioning Alpha reconciliation

Snapshot: 2026-10-03. Owner: `goal-commission.md` (codex / Hermes).

The inventory is the **42 distinct source IDs** routed to commissioning in
the [source-ID ledger](status/LEDGER.md) (formerly in
[ALPHA_READINESS](ALPHA_READINESS.md)), not the numbered limitation headings
alone. This ledger does not change the controller's release decision, take the
Web lock, reopen K1–K19, or authorize hardware contact. Original inventories and
historical evidence remain intact.

**Acceptance state:** the recovery package is published in `c9f77d7b` (source
`743c3d29`, integration `09cd951d`). The durable-history extension has separate
in-session review, 21 compiled behavioral mutants and **18/18 actual integrated
gates accepted** on `66dca2793fcaf50c2149d73c90364a4ae727cd06` at
2026-10-03 06:59 CEST. Source and gated acceptance documentation are **published**
at `cde52ebc1e9f9125f5b94096a6b6c001a3269bd4`: local, fetched and live `main`
refs matched; all 23 contributed artifacts and 617 frozen inputs matched remotely.
Four fresh-target documentation audits and whitespace passed before publication.
Receipt-only bookkeeping does not change that tested source or widen its scope.

Actual integrated evidence: ordinary workspace **2978/0/165** over 149 result
blocks; Web **1665** tests in **93** files; explicitly selected offline Download
**14/0/0** and Dynamic **6/0/0**, zero unknown skips, 108 scoped originals
unchanged. All **617** frozen inputs equal committed/current blobs and **17**
shadow bindings match. Formatting, strict Clippy, build, dependency policy and
four nonempty intended-root audits pass. No Chromium/native, hardware, vendor,
ETS or release acceptance is inferred from this 18-step gate.

Checkpoint: the earlier whole-candidate run completed all 18 steps, workspace
2948/0/165, Web 1559, selected private download 14/0/0 and Dynamic 6/0/0.
A subsequent in-session review reproduced pre-admission SQLite recovery writes;
that receipt is historical, not final acceptance of the corrected source.
Hot-journal and WAL refusal regressions are RED/GREEN. The corrected-source
21-mutant sweep is green with exact restoration; integration/publication still
remain pending. Reconciliation mechanically matches all 42 IDs, zero duplicates,
zero omissions and zero extras.

Corrected candidate acceptance: workspace 2950 passed / 0 failed / 165 ignored;
Web 1559 tests in 89 files; explicitly selected private download 14/0/0 and
Dynamic 6/0/0, zero unknown skips and all 108 original inputs unchanged. All
608 frozen code/config inputs and 17 shadow bindings match. Ignored ordinary
tests are not counted as passed; private results are separate scopes. This
receipt does not cover the later integration with the published Float guard,
U16 settings contract, evaluation work admission and module-scope provenance.

Integration resume, 2026-10-03: published parent `14e2eb9a` is being reconciled
in the owned commissioning worktree. Both complete owner handovers and status
sections are retained. The unpublished history ADR is renumbered to **0064**,
leaving the published work-admission ADR-0062 and scope ADR-0063 unchanged.
This resume produced the actual accepted `66dca279` evidence and published
`cde52ebc` checkpoint above. Older candidate receipts remain historical evidence.

## Meaning of dispositions

- **VERIFIED_SCOPE:** implementation/guard and existing offline evidence exist;
  only the precisely named boundary is established, not full original scope.
- **BLOCKED_HARDWARE:** requires new target-specific consent, suitable hardware,
  recovery or independent wire evidence. Keep the described safe fallback.
- **BLOCKED_REFERENCE:** no trustworthy payload/placement/profile evidence yet;
  retain/refuse rather than invent it.
- **PARTIAL_BACKEND:** foundation exists but named offline work remains open.
- **BLOCKED_UI:** backend contract exists; adoption belongs to the Web lock owner.
- **RECORDED_SCOPE:** an already documented non-goal/parked decision, not a new
  waiver granted by this audit.
- **OPEN_SPEC_AUDIT:** an offline normative investigation remains actionable.

No disposition means ETS parity, KNX certification, a release waiver, or proven
crash/power-loss recovery. Reference entries in Known Limitations remain public
records of the residual scope.

## Evidence catalogue

- **E1 — pre-write property recovery:** ADR-0051; `knx-net`
  `commissioning/service_control.rs`, `knx-app` `service_control_backup.rs`,
  CLI `device_service_control.rs`, server `service_control_routes.rs` and
  `http_service_control.rs`. Format 2 retains original PID 8 and PID 14.
  Nine compiled behavioral mutants and the published integrated gate cover
  ordering, backup refusal, strict octet width and unchanged unrelated bits.
- **E2 — public safety admission:** ADR-0057/0058/0059 and the CLI/server address,
  serial and reset entry points. Confirmed address/reset operations still
  refuse before tunnelling; simulated procedures are not new write permission.
- **E3 — download/recovery:** `knx-net` `commissioning/memory_download.rs` and
  `commissioning/download.rs`; server `device_download.rs`,
  `device_download_routes.rs` and `http_device_download.rs`; ADR-0049.
  The previous published offline private target was 13/13, all 108 originals
  unchanged, in-process `SimTunnel` only. The extension adds a fourteenth,
  explicit unavailable-history admission test; corrected candidate coverage is
  14/14 with originals unchanged; actual integrated coverage is also 14/14.
  Backup scope is only plan-affected memory/load states.
- **E4 — accepted backend lifecycle/history source:** ADR-0055/0056/0064; `knx-store`
  `activity_history.rs`; server `one_shot_activity.rs`, `bus_activity_routes.rs`,
  `http_activity_history.rs` and `http_service_control.rs`. Focused tests prove
  reopen, ring eviction, prior-incarnation unknown, bounded pages, foreign/future
  refusal, malformed metadata, sticky failures and no property write when
  intent persistence fails. This is metadata, never a recovery image.
- **E5 — documented protocol/scope evidence:** relevant numbered entries in
  [KNOWN_LIMITATIONS](KNOWN_LIMITATIONS.md), [RESEARCH](RESEARCH.md) §8.6–8.7,
  §19 and §22–24; the dated [partial-download audit](spec-audits/2026-09-19-cp-3_5_3-partial-download.md) and other audits linked
  from those documents. `[D]` is a directly documented fact, `[V]` a measured
  observation, `[A]` an application policy/inference. Preserve those distinctions.
- **E6 — bounded direct Profile audit:**
  [profile-order boundaries](spec-audits/2026-10-03-commissioning-profile-order-boundaries.md).
  Four primary PDFs and their chapter bodies/printed pages were inspected;
  contradictions and exact mask restrictions are retained, not guessed away.

## Complete per-ID reconciliation

Priority, status and owner disposition of these rows are in the
[source-ID ledger](status/LEDGER.md) (*Owner disposition* column), since
2026-10-04 (AR14D D2). This table keeps the evidence.

| Source ID | Evidence and exact established scope | Safe fallback / remaining boundary | Exact unblock / next action |
|---|---|---|---|
| KL-116 | E2; MP §2.3 procedure and historical one-device evidence, not current recovery approval | Public confirmed button-driven address writes remain fail-closed | Complete action-specific original-storage backup and restore witness; identified isolated target, free destination, exactly one programming device and fresh operation-specific go |
| KL-139 | E1/E2; serial read succeeded historically; serial writes were ignored, including bit-2/SYSTEM variants | Serial write refuses before tunnel; Debug property action does not imply serial-write support | Independently supported target/procedure plus complete affected-storage recovery and a fresh serial-address go |
| KL-140 | E2; historical reset/recovery on one device; ADR-0058 now guards public reset | No automatic reset or assumed restoration; no HTTP/UI reset implementation | User decision 2026-10-05: no HTTP/UI reset; the missing reset UI is an accepted, safely refused unsupported boundary with a user notice. A future reset needs verified full affected-storage recovery, exact reset target/scope and a fresh reset-specific go |
| SAFE-01 | E1/E2/E5; candidate suitability read does not establish exact installed application or all affected storage | No write to the proposed replacement device; property-only backup is insufficient | Establish identity/application, physically usable bench target and complete recovery before requesting exact operation-specific consent |
| KL-99 | E5; `commissioning/mcb.rs` nibble interpretation is explicitly inferred; current comparison uses CRC/control, not access nibbles | Do not claim independently proved nibble order or use it to guess permissions | Independent bit-position reference or controlled readback before a new access-nibble consumer |
| KL-112 | E5; required access-key assignments are explicitly unsupported, not silently omitted | Authentication with a supplied key is distinct from key replacement; no guessed/deleted/new key | Separate reviewed A_Key_Write format and deletion semantics, protected-device recovery and target-specific go before adding key writes |
| KL-136 | E3/E5; one 0701h image/readback and functional check were observed | Closing restart remains unconfirmed; no wider mask/product claim | Independent restart receipt if supported, plus separate identified device/program/revision evidence for any coverage expansion |
| KL-138 | E5; supplied/project-key authorisation and precedence are simulated and exposed; no protected-device trial | Never guess/log keys; absent/rejected access stays an explicit refusal | Suitable already-protected test device, authorized read/connection evidence; a key-setting experiment needs its own recovery/go |
| KL-141 | E5; typed Master Reset semantics and erasure are simulator-only | Erasing hardware scopes are refused; non-erasing tests are not factory-reset proof | Complete recovery of every affected state plus a separately authorized erasing test on disposable isolated hardware |
| KL-142 | E3/E5; CLI/API partial scopes and bounded three-scope one-device evidence already exist | No claim for other products/configurations; no new selector in another owner's locked Web tree | User decision 2026-10-05: Web selector handed to the UI owner ([handoff](#handoff-to-the-ui-owner-2026-10-05)); new products still need independent plan/restore/readback evidence |
| KL-7 | E3/E5; memory path and narrow hardware evidence supersede the old blanket 'blocked' title | Decline unsupported masks/images; do not infer full ETS download support from simulation | Product-specific complete plan/recovery and independent hardware evidence for each additional supported path |
| KL-92 | E3/E5; own simulator and one-device measurements establish only their stated scope | Label simulated, corpus-derived and live observations separately | Independent devices/traces over a declared mask/product/version matrix, each behind recovery and authorization gates |
| DEBUG-01 | E1/E4; default-off scope, exact phrase, original PID 8/PID 14 backup before Verify Mode/property change; no-op writes nothing | Property recovery is manual/scoped; new gate has no hardware acceptance | If live acceptance is needed: separately approved target, operation-specific go and verified restoration of original properties; never broaden to serial/reset recovery |
| SAFE-02 | E3; plan-affected backups/load-state restore guards and offline failure refusals | Not a whole-device image or proof of live group-table restore | Explicit pre-write baseline, complete declared affected scope, isolated fault/restore test and readback under a fresh go |
| SAFE-03 | E3/E4; returned-error disconnect and dead-worker reconciliation preserve unknown effects/reservation | Dropped futures/process/power loss do not prove cleanup; no automatic retry/restore | Durable long-session intent/terminal integration and crash fault tests remain offline work; actual device recovery needs independent hardware evidence |
| R-MODULE-01 | E5 §19.11; modular parameter placement has contradictory source interpretations | Keep unproved placement refused; do not invent stride/base formulas | Authorized read-only installed-image comparison with exact parameter/module identity or authoritative placement semantics |
| KL-105 | E5; SYSTEM control-frame bytes are encoder-tested | Successful tunnelling/download is not a TP1 priority measurement | Independent on-wire control-frame capture preserving the relevant control fields; decoder evidence alone is insufficient |
| KL-108 | E5; MP §2.3 body/exception contradiction is recorded and the chosen exception interpretation is explicit | Public address write remains recovery-gated; no implied resolution of the Standard contradiction | Authoritative erratum or separately reviewed alternative; do not loosen occupancy/exclusion gates |
| KL-101 | E5; one extra load-state attempt and its compound latency are documented | Do not present max_transition as a hard end-to-end deadline or remove mandatory retries for speed | If UX needs an outer budget: specify conservative unknown/partial outcome and test cancellation without claiming non-delivery |
| KL-104 | E5; reconnect after transport release follows the documented quiet-state behavior | Reconnect cost is a stated latency boundary, not permission to reuse a dead connection | Any optimization needs measured traffic/latency and proof it preserves release/re-authorisation/quiet-state rules |
| KL-109 | E5; same-kind duplicate parts are refused instead of guessing relative order | Keep OutOfOrder refusal for plans outside the verified single-kind ordering | Authoritative multi-instance ordering plus independent plan/readback fixtures before relaxing the guard |
| KL-111 | E5; deliberately do not implement the step that removes the individual address | Keep the device addressable; do not call the other unloads a full address unload | A separately reviewed use case, rediscovery/recovery design and fresh destructive-operation go |
| KL-113 | E5; generic property-plan escalation uses only validated supplied parts | Never synthesize an omitted segment's payload; narrow generic semantics are not whole-device reload | Complete authenticated device inventory and all required segment data before offering a wider escalation |
| KL-114 | E5; download-counter refusals are expressly KNXBench policy, not a universal System B mandate | Preserve conservative unavailable/changed refusal and accurate wording | Separate profile-specific policy review and fixtures before changing those refusals |
| KL-143 | E5; RF domain primitives/procedures are simulated; unsupported PL/IP/secure forms remain explicit | Domain-address hardware writes refused; no live RF or public RF route claim | Identified RF hardware/interface, router-mode responsibilities, complete recovery and exact go; unsupported secure/PL forms need their own scope |
| KL-144 | E5; RF configuration simulator requires externally supplied channel definitions | Refuse unknown codes/multiple-instance numbering; no RF configuration route/hardware write | Authoritative channel tables and numbering/link semantics, then independently identified RF hardware and operation-specific recovery/go |
| KL-145 | E5; linked instance overrides are image/readback-compared; unlinked communication-enable differs | Keep explicit instance flags; do not claim ETS or program-behavior parity for unlinked active objects | Independent unlinked-object configuration/readback/behavior fixture or explicit reviewed policy decision |
| KL-93 | E5; parked declarative step-list differs from executed PartKind-aware procedure | Do not advertise the dry-run list as an exact executable preview | Before exposing it: domain-level part semantics, no core-to-net dependency, and per-kind preview/execution agreement tests |
| GAP-T30-01 | E5 §8.6/8.7; per-Legacy flag semantics are not established | Preserve flags; refuse paths whose correctness depends on unknown effects | Authorized MT schema/manufacturer semantics or controlled differential reference evidence |
| GAP-T30-02 | E5 §8.7.15; unnamed LdCtrl-to-subtype mappings remain unproved | No guessed subtype or allocation fallback | Direct mapping evidence for each missing kind and an independent payload/sequence fixture |
| GAP-T30-03 | E5 §8.6.7; step lists exist, DLL/plugin transformations are not inferred from them | Keep vendor code inert; no execution/disassembly or claimed equivalent image generation | Manufacturer documentation or authorized isolated reference behavior, with input/output provenance and no private payload publication |
| GAP-T30-07 | E5; programming delay is server/octet dependent, with no proved universal formula | Preserve configured timing and report unverified device-specific adequacy | Manufacturer/device-specific timing bounds or controlled measurements; no guessed magic timeout promoted to a normative fact |
| GAP-T30-08 | E5/E6; profiles name memory-mapped Type 2, but this does not establish every general CP discriminator, encoding or executor mapping | No fabricated Type-2 executor or coupler admission from a reference table | Unambiguous exact-profile mapping plus independent state/encoding fixtures |
| GAP-T30-09 | E5/E6; direct 2705h/27B0h/2920h/2311h audit records mask restrictions and inconsistent source labels, not a universal execution-order proof | Retain only the specific proved ordering/refusal; no RF/USB/coupler or duplicate-part extrapolation | Unambiguous exact-profile dependency/placement mapping plus independent sequence/image fixtures before widening admission; the bounded offline source audit is complete |
| R-DL-01 | E5 §8.6.7; data step lists do not establish plugin-generated bytes | Do not execute opaque vendor baggage or guess image transformations | Manufacturer/reference transformation evidence for the exact plugin/program, then isolated regression fixtures |
| R-DL-02 | E5 §8.7.15/design R11; unknown payload/address/allocation semantics remain outside supported plans | Refuse unsupported controls/placement rather than silently skip or fall back | Direct per-control/profile mapping and plan-image agreement fixtures before another executor path |
| AUDIT-01 | E4 published accepted source makes four one-shot kinds durable and discloses seven untracked kinds | Not complete universal history; serial/group writes and long sessions do not acquire fake receipts | Instrument declared remaining callers with conservative intent/outcomes, version-evolution policy, scoped metadata tests and the owning track's cooperation |
| UI-04 | E4; live snapshot/incarnation, persistence state and bounded history API are available in published accepted backend source | No Web edit/lock takeover; volatile sessions and durable entries must not be conflated | Web owner adopts the contract, localizes closed state tokens, refreshes running rows and proves rendered unavailable/interrupted/partial cases plus native acceptance |
| KL-110 | E5/goal §3c; PL110-only Group Responser behavior is outside TP1/RF/IP scope | Do not write that property on other media; no PL commissioning claim | Separate PL goal with medium admission and the mandatory property/procedure before PL downloads exist |
| KL-115 | E5; current master_reset.rs waits on recovery_wait (probe and answer); the limitation's dated current-caller status supersedes its historical title | Basic-Restart recovery/retry is not a generic caller contract; no extra reset/retry authorized | Keep the reconciled caller-specific status; a future generic failed-service retry needs a separate bounded Configuration Procedure contract |
| GAP-T30-04 | E5; differential chunk selection is implementation-defined, not a missing normative algorithm | Full validated plan remains safe fallback; no inferred performance/parity guarantee | Only implement a measured differential strategy with preserved-byte, recovery and readback regressions if that separate scope is chosen |
| IMPORT-03 | E5/K19; offline private capture census is a bounded decoder comparison, not discovery or device interoperability proof | No raw telegram/serial/address data in Git; no claimed universal capture format | Broader independently sourced captures via explicit offline opt-in and aggregate-only privacy-reviewed output |

## Remaining work, not disguised as external blockers

1. Actual integrated source and gated acceptance are published/read back at
   `cde52ebc`; closing receipt/cleanup bookkeeping follows separately. Earlier
   failed mutations/verifiers stay recorded, not relabelled green.
2. `AUDIT-01` / `SAFE-03`: every commissioning CLI caller (download, restore,
   service-control write, compare, serial read) and server caller (download
   session, compare, serial lookup, service-control read/write) now records
   durable history; published with integrated gates at `ae567d00`. Confirmed
   address programming/serial write/reset refuse before any tunnel and so
   acquire no receipts. Process/power-loss recovery stays unproven (user notice).
3. `GAP-T30-09`: bounded direct Profile investigation complete (E6); unsupported
   variants remain reference-bounded, not newly admitted or declared compatible.
4. `KL-115`: stale blanket wording is explicitly superseded by a current-caller
   status; a future generic restart-and-retry procedure is still separate scope.
5. The listed Web adoption is handed to the UI owner (user decision
   2026-10-05, below). Hardware, manufacturer evidence and the controller's
   final review/release decisions remain separate.

## Handoff to the UI owner (2026-10-05)

User decision 2026-10-05: the remaining Web halves go to the `goal-ui.md`
owner, who holds the Web lock; commissioning takes no lock and edits no Web
source. The rows stay open until the UI half is delivered. Field names are
copied from `apps/knx-server/src/device_download_routes.rs`.

| Row | Task for the UI owner | Published backend contract | Acceptance |
|---|---|---|---|
| KL-142 | Offer complete / parameters / group addresses / both before asking for a plan; show what the partial plan omits | `POST /api/device-download/plan` body `{ address, partial?: { parameters: bool, groupAddresses: bool } }` (absent = complete). Response adds `partial: bool` and `notWritten: [address, octets][]`; `POST /api/device-download/start` re-derives the same partial plan from `planId` and refuses a different one | Rendered scope choice, `partial`/`notWritten` shown before confirmation, an unsupported-scope refusal rendered, intercepted request bodies asserted; no new write permission or phrase change |
| KL-140 | None: no reset UI (accepted boundary). Optional: link the user notice from the commissioning view | — (no HTTP reset route exists; CLI reset fails closed before a tunnel, ADR-0058) | — |
| UI-04 | Unchanged from the row above: adopt the activity history contract | `GET /api/bus/activity`, `GET /api/bus/history` | As in the UI-04 row |

**UI owner delivery, 2026-10-06.** `KL-142`: delivered in `2f2a6892` (ledger
`DONE`). `UI-04`: the Web half is delivered — *Live activity* tab for
`GET /api/bus/activity` and self-refreshing running rows in *Activity history*
([KNOWN_LIMITATIONS](KNOWN_LIMITATIONS.md#partial-commissioning-bus-activity-snapshot-adr-0055),
"Web adoption, 2026-10-06"). Closing `UI-04` stays with its owner; native
acceptance is an accepted boundary by the user decision of 2026-10-04.

## Owner reconciliation (2026-10-05)

The owner set the status of all 42 rows in the source-ID ledger. The
user scope decision of 2026-10-04 removed new hardware, power-loss, vendor
and ETS validation from this goal, so the 36 rows that waited only for that
evidence or a recorded non-goal became `ACCEPTED_BOUNDARY` (11 hardware, 11
reference, 10 verified-scope, 2 recorded-scope) or `LATER` (`KL-110`
Powerline, `GAP-T30-04` differential download). `SAFE-03` and `DEBUG-01` are
`ACCEPTED_BOUNDARY` (offline contracts closed; live and power-loss recovery out
of scope), `AUDIT-01` is `DONE` at commissioning scope, `KL-140` was accepted
on 2026-10-05. Open: `KL-142` and `UI-04`, both Web halves with the UI owner
(handoff above). Every row keeps its safe fallback from the table above: an
accepted boundary is a refusal or a bounded claim, not new support.

## Owner status history

Moved verbatim from `goal-commission.md` on 2026-10-06 (AR14D D5, agreed by
the commissioning owner); only relative links changed. Every ID named here has
its row in the [source-ID ledger](status/LEDGER.md), which is current.

### Where things stood (goal-commission.md, 2026-10-01)

K1–K19 have historical implementation evidence at their documented scope.
CLI/Web download and button-driven address programming ran on MDT `1.1.67`
with device-specific approval; complete and all three partial download
scopes were read back. A
pre-write region backup and restore ran on that device for complete and
parameters-only download. K13 address reset was performed and recovered on
that device; K14 destructive Master Reset remains hardware-refused. K12 serial
address write was ignored by this device even after a system-priority fix;
read-only identification succeeded. RF K16/K17 is simulator-only and has no RF
hardware or product-facing route. K19 decoded 71 private cEMI frames offline;
no raw frames belong in Git. See [RESEARCH](RESEARCH.md),
[limitations](KNOWN_LIMITATIONS.md) and
[implementation status](IMPLEMENTATION_STATUS.md) for the per-operation
evidence and refusal boundaries.

**Current safety boundary:** the historical address runs do not provide
durable complete recovery for a later device. Confirmed public button
programming (ADR-0059), serial address writes (ADR-0057) and K13 reset
(ADR-0058) now refuse *before tunnel opening* until their respective
device-specific pre-write backup/readback/abort contracts are verified.
Read-only plans, identification and simulator work remain available. The
Debug bit-2 route (ADR-0051) instead has an offline-tested, property-only
backup gate and a default-off explicit UI action, not a full device restore
or new live evidence. K7 has a simulator interrupted-run/retry regression;
this is not a live interruption. No K14 erase or RF hardware test is claimed.
The user approved experimental K6 investigation but no new write. The
previous target `1.1.67` is reportedly off the bus; `1.1.32` is a read-only
identification **candidate**, not a verified model or approved write target.
Confirm its physical role, identity and per-device recovery before asking
for a new operation-specific go. Never transfer the old go or infer complete
storage from a diagnostic dump (RESEARCH §24). The current handover in
`.ai/CURRENT_STATE.md` wins if evidence advances.

## Reconciliation check

Parse the rows with owner `commission` in `docs/status/LEDGER.md` (until
2026-10-04: the commissioning-routed rows of `docs/ALPHA_READINESS.md`) and compare their
ID set with the table above. Require exactly 42 unique rows, zero duplicates,
zero omitted IDs and zero extra IDs. Counts must be derived mechanically, not
inferred from the number of numbered Known Limitations sections. The current
owner-only controller rows may adopt these scoped dispositions after delivery;
this document does not silently turn their WAITING_OWNER status into DONE.
