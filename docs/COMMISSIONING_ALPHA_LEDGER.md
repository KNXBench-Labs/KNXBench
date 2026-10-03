# Global commissioning Alpha reconciliation

Snapshot: 2026-10-03. Owner: `goal-commission.md` (codex / Hermes).

The inventory is the **42 distinct source IDs** routed to commissioning in
[ALPHA_READINESS](ALPHA_READINESS.md), not the numbered limitation headings
alone. This ledger does not change the controller's release decision, take the
Web lock, reopen K1–K19, or authorize hardware contact. Original inventories and
historical evidence remain intact.

**Acceptance state:** the recovery package is published in `c9f77d7b` (source
`743c3d29`, integration `09cd951d`). The durable-history extension is a working
candidate with separate in-session review, 21 compiled behavioral mutants and
18/18 corrected-source gates accepted. Integrated gates and publication remain
**PENDING**. Candidate facts below are not
a claim that the extension is already on `main`.

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
receipt does not cover the upcoming merge with the published Float guard.

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
  14/14 with originals unchanged. Integrated coverage is still pending.
  Backup scope is only plan-affected memory/load states.
- **E4 — lifecycle and history candidate:** ADR-0055/0056/0062; `knx-store`
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

| Source ID | Priority | Disposition | Evidence and exact established scope | Safe fallback / remaining boundary | Exact unblock / next action |
|---|---|---|---|---|---|
| KL-116 | P0 | BLOCKED_HARDWARE | E2; MP §2.3 procedure and historical one-device evidence, not current recovery approval | Public confirmed button-driven address writes remain fail-closed | Complete action-specific original-storage backup and restore witness; identified isolated target, free destination, exactly one programming device and fresh operation-specific go |
| KL-139 | P0 | BLOCKED_HARDWARE | E1/E2; serial read succeeded historically; serial writes were ignored, including bit-2/SYSTEM variants | Serial write refuses before tunnel; Debug property action does not imply serial-write support | Independently supported target/procedure plus complete affected-storage recovery and a fresh serial-address go |
| KL-140 | P0 | BLOCKED_HARDWARE | E2; historical reset/recovery on one device; ADR-0058 now guards public reset | No automatic reset or assumed restoration; no HTTP/UI reset implementation | Verified current full affected-storage recovery, exact reset target/scope and fresh reset-specific go |
| SAFE-01 | P0 | BLOCKED_HARDWARE | E1/E2/E5; candidate suitability read does not establish exact installed application or all affected storage | No write to the proposed replacement device; property-only backup is insufficient | Establish identity/application, physically usable bench target and complete recovery before requesting exact operation-specific consent |
| KL-99 | P1 | VERIFIED_SCOPE | E5; `commissioning/mcb.rs` nibble interpretation is explicitly inferred; current comparison uses CRC/control, not access nibbles | Do not claim independently proved nibble order or use it to guess permissions | Independent bit-position reference or controlled readback before a new access-nibble consumer |
| KL-112 | P1 | VERIFIED_SCOPE | E5; required access-key assignments are explicitly unsupported, not silently omitted | Authentication with a supplied key is distinct from key replacement; no guessed/deleted/new key | Separate reviewed A_Key_Write format and deletion semantics, protected-device recovery and target-specific go before adding key writes |
| KL-136 | P1 | BLOCKED_HARDWARE | E3/E5; one 0701h image/readback and functional check were observed | Closing restart remains unconfirmed; no wider mask/product claim | Independent restart receipt if supported, plus separate identified device/program/revision evidence for any coverage expansion |
| KL-138 | P1 | BLOCKED_HARDWARE | E5; supplied/project-key authorisation and precedence are simulated and exposed; no protected-device trial | Never guess/log keys; absent/rejected access stays an explicit refusal | Suitable already-protected test device, authorized read/connection evidence; a key-setting experiment needs its own recovery/go |
| KL-141 | P1 | BLOCKED_HARDWARE | E5; typed Master Reset semantics and erasure are simulator-only | Erasing hardware scopes are refused; non-erasing tests are not factory-reset proof | Complete recovery of every affected state plus a separately authorized erasing test on disposable isolated hardware |
| KL-142 | P1 | BLOCKED_UI | E3/E5; CLI/API partial scopes and bounded three-scope one-device evidence already exist | No claim for other products/configurations; no new selector in another owner's locked Web tree | Web owner adopts plan-derived scope selector and consent/status vocabulary; new products require independent plan/restore/readback evidence |
| KL-7 | P1 | BLOCKED_HARDWARE | E3/E5; memory path and narrow hardware evidence supersede the old blanket 'blocked' title | Decline unsupported masks/images; do not infer full ETS download support from simulation | Product-specific complete plan/recovery and independent hardware evidence for each additional supported path |
| KL-92 | P1 | BLOCKED_HARDWARE | E3/E5; own simulator and one-device measurements establish only their stated scope | Label simulated, corpus-derived and live observations separately | Independent devices/traces over a declared mask/product/version matrix, each behind recovery and authorization gates |
| DEBUG-01 | P1 | VERIFIED_SCOPE | E1/E4; default-off scope, exact phrase, original PID 8/PID 14 backup before Verify Mode/property change; no-op writes nothing | Property recovery is manual/scoped; new gate has no hardware acceptance | If live acceptance is needed: separately approved target, operation-specific go and verified restoration of original properties; never broaden to serial/reset recovery |
| SAFE-02 | P1 | BLOCKED_HARDWARE | E3; plan-affected backups/load-state restore guards and offline failure refusals | Not a whole-device image or proof of live group-table restore | Explicit pre-write baseline, complete declared affected scope, isolated fault/restore test and readback under a fresh go |
| SAFE-03 | P1 | PARTIAL_BACKEND | E3/E4; returned-error disconnect and dead-worker reconciliation preserve unknown effects/reservation | Dropped futures/process/power loss do not prove cleanup; no automatic retry/restore | Durable long-session intent/terminal integration and crash fault tests remain offline work; actual device recovery needs independent hardware evidence |
| R-MODULE-01 | P1 | BLOCKED_REFERENCE | E5 §19.11; modular parameter placement has contradictory source interpretations | Keep unproved placement refused; do not invent stride/base formulas | Authorized read-only installed-image comparison with exact parameter/module identity or authoritative placement semantics |
| KL-105 | P2 | BLOCKED_HARDWARE | E5; SYSTEM control-frame bytes are encoder-tested | Successful tunnelling/download is not a TP1 priority measurement | Independent on-wire control-frame capture preserving the relevant control fields; decoder evidence alone is insufficient |
| KL-108 | P2 | VERIFIED_SCOPE | E5; MP §2.3 body/exception contradiction is recorded and the chosen exception interpretation is explicit | Public address write remains recovery-gated; no implied resolution of the Standard contradiction | Authoritative erratum or separately reviewed alternative; do not loosen occupancy/exclusion gates |
| KL-101 | P2 | VERIFIED_SCOPE | E5; one extra load-state attempt and its compound latency are documented | Do not present max_transition as a hard end-to-end deadline or remove mandatory retries for speed | If UX needs an outer budget: specify conservative unknown/partial outcome and test cancellation without claiming non-delivery |
| KL-104 | P2 | VERIFIED_SCOPE | E5; reconnect after transport release follows the documented quiet-state behavior | Reconnect cost is a stated latency boundary, not permission to reuse a dead connection | Any optimization needs measured traffic/latency and proof it preserves release/re-authorisation/quiet-state rules |
| KL-109 | P2 | VERIFIED_SCOPE | E5; same-kind duplicate parts are refused instead of guessing relative order | Keep OutOfOrder refusal for plans outside the verified single-kind ordering | Authoritative multi-instance ordering plus independent plan/readback fixtures before relaxing the guard |
| KL-111 | P2 | RECORDED_SCOPE | E5; deliberately do not implement the step that removes the individual address | Keep the device addressable; do not call the other unloads a full address unload | A separately reviewed use case, rediscovery/recovery design and fresh destructive-operation go |
| KL-113 | P2 | VERIFIED_SCOPE | E5; generic property-plan escalation uses only validated supplied parts | Never synthesize an omitted segment's payload; narrow generic semantics are not whole-device reload | Complete authenticated device inventory and all required segment data before offering a wider escalation |
| KL-114 | P2 | VERIFIED_SCOPE | E5; download-counter refusals are expressly KNXBench policy, not a universal System B mandate | Preserve conservative unavailable/changed refusal and accurate wording | Separate profile-specific policy review and fixtures before changing those refusals |
| KL-143 | P2 | BLOCKED_HARDWARE | E5; RF domain primitives/procedures are simulated; unsupported PL/IP/secure forms remain explicit | Domain-address hardware writes refused; no live RF or public RF route claim | Identified RF hardware/interface, router-mode responsibilities, complete recovery and exact go; unsupported secure/PL forms need their own scope |
| KL-144 | P2 | BLOCKED_REFERENCE | E5; RF configuration simulator requires externally supplied channel definitions | Refuse unknown codes/multiple-instance numbering; no RF configuration route/hardware write | Authoritative channel tables and numbering/link semantics, then independently identified RF hardware and operation-specific recovery/go |
| KL-145 | P2 | BLOCKED_REFERENCE | E5; linked instance overrides are image/readback-compared; unlinked communication-enable differs | Keep explicit instance flags; do not claim ETS or program-behavior parity for unlinked active objects | Independent unlinked-object configuration/readback/behavior fixture or explicit reviewed policy decision |
| KL-93 | P2 | RECORDED_SCOPE | E5; parked declarative step-list differs from executed PartKind-aware procedure | Do not advertise the dry-run list as an exact executable preview | Before exposing it: domain-level part semantics, no core-to-net dependency, and per-kind preview/execution agreement tests |
| GAP-T30-01 | P2 | BLOCKED_REFERENCE | E5 §8.6/8.7; per-Legacy flag semantics are not established | Preserve flags; refuse paths whose correctness depends on unknown effects | Authorized MT schema/manufacturer semantics or controlled differential reference evidence |
| GAP-T30-02 | P2 | BLOCKED_REFERENCE | E5 §8.7.15; unnamed LdCtrl-to-subtype mappings remain unproved | No guessed subtype or allocation fallback | Direct mapping evidence for each missing kind and an independent payload/sequence fixture |
| GAP-T30-03 | P2 | BLOCKED_REFERENCE | E5 §8.6.7; step lists exist, DLL/plugin transformations are not inferred from them | Keep vendor code inert; no execution/disassembly or claimed equivalent image generation | Manufacturer documentation or authorized isolated reference behavior, with input/output provenance and no private payload publication |
| GAP-T30-07 | P2 | BLOCKED_REFERENCE | E5; programming delay is server/octet dependent, with no proved universal formula | Preserve configured timing and report unverified device-specific adequacy | Manufacturer/device-specific timing bounds or controlled measurements; no guessed magic timeout promoted to a normative fact |
| GAP-T30-08 | P2 | BLOCKED_REFERENCE | E5/E6; profiles name memory-mapped Type 2, but this does not establish every general CP discriminator, encoding or executor mapping | No fabricated Type-2 executor or coupler admission from a reference table | Unambiguous exact-profile mapping plus independent state/encoding fixtures |
| GAP-T30-09 | P2 | BLOCKED_REFERENCE | E5/E6; direct 2705h/27B0h/2920h/2311h audit records mask restrictions and inconsistent source labels, not a universal execution-order proof | Retain only the specific proved ordering/refusal; no RF/USB/coupler or duplicate-part extrapolation | Unambiguous exact-profile dependency/placement mapping plus independent sequence/image fixtures before widening admission; the bounded offline source audit is complete |
| R-DL-01 | P2 | BLOCKED_REFERENCE | E5 §8.6.7; data step lists do not establish plugin-generated bytes | Do not execute opaque vendor baggage or guess image transformations | Manufacturer/reference transformation evidence for the exact plugin/program, then isolated regression fixtures |
| R-DL-02 | P2 | BLOCKED_REFERENCE | E5 §8.7.15/design R11; unknown payload/address/allocation semantics remain outside supported plans | Refuse unsupported controls/placement rather than silently skip or fall back | Direct per-control/profile mapping and plan-image agreement fixtures before another executor path |
| AUDIT-01 | P2 | PARTIAL_BACKEND | E4 candidate makes four one-shot kinds durable and discloses seven untracked kinds | Not complete universal history; serial/group writes and long sessions do not acquire fake receipts | Finish candidate gates/publication; then instrument declared remaining callers with conservative intent/outcomes, scoped metadata tests and the owning track's cooperation |
| UI-04 | P2 | BLOCKED_UI | E4; live snapshot/incarnation, persistence state and bounded history API are available in the candidate | No Web edit/lock takeover; volatile sessions and durable entries must not be conflated | Web owner adopts the contract, localizes closed state tokens, refreshes running rows and proves rendered unavailable/interrupted/partial cases plus native acceptance |
| KL-110 | P3 | RECORDED_SCOPE | E5/goal §3c; PL110-only Group Responser behavior is outside TP1/RF/IP scope | Do not write that property on other media; no PL commissioning claim | Separate PL goal with medium admission and the mandatory property/procedure before PL downloads exist |
| KL-115 | P3 | VERIFIED_SCOPE | E5; current master_reset.rs waits on recovery_wait (probe and answer); the limitation's dated current-caller status supersedes its historical title | Basic-Restart recovery/retry is not a generic caller contract; no extra reset/retry authorized | Keep the reconciled caller-specific status; a future generic failed-service retry needs a separate bounded Configuration Procedure contract |
| GAP-T30-04 | P3 | RECORDED_SCOPE | E5; differential chunk selection is implementation-defined, not a missing normative algorithm | Full validated plan remains safe fallback; no inferred performance/parity guarantee | Only implement a measured differential strategy with preserved-byte, recovery and readback regressions if that separate scope is chosen |
| IMPORT-03 | P3 | VERIFIED_SCOPE | E5/K19; offline private capture census is a bounded decoder comparison, not discovery or device interoperability proof | No raw telegram/serial/address data in Git; no claimed universal capture format | Broader independently sourced captures via explicit offline opt-in and aggregate-only privacy-reviewed output |

## Remaining work, not disguised as external blockers

1. Final candidate review, mutation sweep, integrated gate and publication.
2. `AUDIT-01` / `SAFE-03`: durable long-session lifecycle/intent contracts and
   explicit fault injection. Four instrumented one-shot kinds do not close them.
3. `GAP-T30-09`: bounded direct Profile investigation complete (E6); unsupported
   variants remain reference-bounded, not newly admitted or declared compatible.
4. `KL-115`: stale blanket wording is explicitly superseded by a current-caller
   status; a future generic restart-and-retry procedure is still separate scope.
5. The listed Web adoption must await its owner/lock. Hardware, manufacturer
   evidence and the controller's final review/release decisions remain separate.

## Reconciliation check

Parse the commissioning-routed rows of `docs/ALPHA_READINESS.md` and compare their
ID set with the table above. Require exactly 42 unique rows, zero duplicates,
zero omitted IDs and zero extra IDs. Counts must be derived mechanically, not
inferred from the number of numbered Known Limitations sections. The current
owner-only controller rows may adopt these scoped dispositions after delivery;
this document does not silently turn their WAITING_OWNER status into DONE.
