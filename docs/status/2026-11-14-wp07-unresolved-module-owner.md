# WP07-S1 — Unresolved module-owner projection (14 November 2026)

## Integration-candidate boundary

The bounded source fix distinguishes an imported module communication object
whose own instance authority is unresolved from a genuinely unscoped object.
After evaluation, the former projects `Undetermined` with no channel, rather
than accidentally taking the unscoped `Inactive` path. This is a read-only
server overlay, not a new module-instance mapping contract.

The corrected source candidate is independently reviewed; its focused results
below are retained evidence from the feature branch. Main-first integration is
prepared as `1c8b146fb4897734942da1b465260b41d3cc74d5` on main base
`ed6524b969fde98938313d2003e0f4806586399b`. **Fresh integrated selected package
gate and publication remain pending.** This documentation successor runs no
product tests, builds or gates and changes no runtime or test source.

## Source and evidence epochs

| Epoch | Identity and admitted result |
| --- | --- |
| Original implementation | `630e8a4f32e6d4cd11c4e7a484ca34209f084d97`, base `28db298af49e870b654db758c3d0b91baa343ee0`; two production files and one HTTP test file |
| Original regression evidence | Three HTTP failures on unchanged original production: `Inactive` instead of `Undetermined` (exit 101). The adapter unit RED used modified production and is supporting evidence only, not an unchanged-production RED |
| Initial focused GREEN | 17 unit and 7 HTTP passes; 101 neighbouring tests passed. Initial review gap: the explicit owner-id-not-held fallback lacked a discriminating HTTP regression |
| Test-only correction | `8873e0150f345592f1e296f655bf77021b9057be`; adds dangling and foreign-device owner cases in the same HTTP file, production unchanged |
| Corrected focused GREEN | 17 unit and 9 HTTP passes, exit 0; one corpus-dependent test remains ignored |
| Exact M3 discrimination | Only the missing/foreign owner-id lookup fallback is changed to `Unscoped`: 17 unit passes, HTTP 7 passes / 2 assertion failures, exit 101. Both new response assertions kill it; restored source returns 17 unit + 9 HTTP passes, exit 0, with the same ignored test |
| Corrected checks and review | Retained fmt, clippy and header checks exit 0. Independent finding-specific Standards/Spec delta review `review02`: APPROVE, F1 CLOSED; no product tests rerun by that review |
| Main-first composition | Parent verified the feature patch, including context, equals the reviewed patch after excluding hunk row offsets; no extra source delta, all non-WP07 files inherited from main. This is source-composition evidence, not fresh integrated execution |

The prior reviewer also ran unnecessarily broad **SIMULATED** suites (1002
unit and 934 integration passes / 73 ignored). This remains a historical scope
deviation, not fresh package acceptance, not additional authority and not a
network-free claim. Those suites were not repeated for the correction or this
documentation successor. The ignored corpus case supplies no corpus coverage.

## Implemented classification and controls

`domain.rs` gathers existing source references and module-instance ids, then
builds one of three overlay scopes: genuinely unscoped, resolved own module,
or unresolved module. Existing `MD-` source-id recognition is used only as a
negative classification when no owner id is present. A stated owner id absent
from this device's instance set is unresolved too, even if another device
holds an instance with the same RefId. This neither establishes positive
module identity nor defines new RefId, repeat or numbering grammar.

`com_object_activation.rs` projects unresolved module scope as `Undetermined`
and no channel when evaluation ran. Missing ProductDB/program data or a missing
Dynamic tree retains the earlier `NotEvaluated` boundary. Genuine unscoped
activation and the existing resolved-instance decision rules stay unchanged.

Maintained fictional fixtures in
`apps/knx-server/tests/it/http_device_detail.rs` exercise actual import mapping
and `GET /api/device/{id}`. The retained corrected results cover:

- Missing and explicitly empty `ModuleInstances` containers: both remain
  unresolved, not known-zero authority. The imported module object is retained.
- Static `Active` and genuine unscoped `Inactive` controls in the same evaluated
  fixture; the resolved module control is `Active` with its own channel. Thus
  unrelated uncertainty is not what makes the orphan assertion pass.
- Explicit dangling and actual foreign-device instance ids through the domain
  lookup and HTTP response. These are deliberately constructed in-memory test
  states after a resolved control, not malformed persisted-store load witnesses.
- Number zero, stored `is_active`, source references and object identities stay
  unchanged. The imported ownerless fixture survives native save/reopen with
  object order, parameter rows, group-address entities and links preserved;
  the read overlay is recomputed. Repeated GETs preserve the project snapshot
  and history lengths before and after reopen separately.
- A parameter edit needing unsupported module instance/number mapping is still
  refused as a whole, with the project and history unchanged. This does not
  authorize creating a missing module object.

Production scope is only `apps/knx-server/src/domain.rs` and
`apps/knx-server/src/com_object_activation.rs`; tests are in the HTTP file above
and the activation module. No persistence, schema, stored-owner, ID allocation,
numbering, ordering or history policy changes. No frontend change.

## Limits and remaining work

The admitted behavioural input is a source-only KNXBench contract, with
independently fictional fixtures. This record introduces no public ETS
observation provenance, ETS compatibility claim, manufacturer/raw export data,
VM observation or hardware/bus result.

The missing/foreign-id cases prove the read-path classification for their
constructed states. They do **not** prove malformed persisted-store reopen/load
reachability or save/reopen of those malformed-owner states. The separate
ownerless-import save/reopen witness must not be extended to that claim.

Whole [WP-07](../open-work/WP-07-module-instance-authority.md) remains
evidence-blocked: the full independently counted mapping witness covering
missing, duplicate, nested and repeated authority is not yet accepted.
[KL-068](../limitations/KL-068-repeated-module-activation.md), positive identity,
repeat expansion, number offsets/allocators, module creation/deletion and write
authority remain unchanged. Fresh integrated selected package acceptance and
publication are separate pending delivery steps, not supplied by focused GREEN
or patch equivalence. WP-12 consent and all foreign-owner stop lines remain
in force; this slice grants no release or deployment authority.

## Retained admitted inputs

Private handles below identify summaries, not tracked raw evidence. Root:
`<profile>/evidence/2026-11-13-wp05-14/`.

- `scoping/WP07_S1_ADMITTED_CONTRACT.md` — bounded source-only behaviour contract.
- `implementation-wp07/PARENT_AUTHOR_VERIFY.md` — historical initial epoch;
  its open F1 is superseded by the correction and closed delta review.
- `implementation-wp07/fix01/PARENT_VERIFY.json` — corrected GREEN, exact M3,
  restored GREEN and retained checks.
- `implementation-wp07/review02/RESULT.json` — independent F1 closure and limits.
- `implementation-wp07/mainfirst01/PARENT_INTEGRATION.json` — source composition,
  no build/test execution or publication.
