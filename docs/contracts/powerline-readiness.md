# Powerline readiness and future test plan — KL-110

Prepared 2026-10-11 against source `aeb5a386`. **Preparation only; no hardware
or protocol execution authorized or performed.** The [ledger](../status/LEDGER.md)
keeps KL-110 `LATER / RECORDED_SCOPE`. This is independent of
[changed-parts download preparation](incremental-download-readiness.md).

## Established boundary and evidence

- [KNOWN_LIMITATIONS §110](../KNOWN_LIMITATIONS.md#110-pid_group_responser_table-stays-unimplemented-on-every-medium)
  records the PL110-only group-responser requirement. Its cited RES §4.16.8.2.5
  and CP §3.5.3 footnotes distinguish PL110 from other media; the property must
  not be added to TP1/RF/IP as a generic completeness fix.
- The current core [partial-variant model](../../crates/knx-core/src/commissioning/partial_download_variant.rs)
  and [download sequencer](../../crates/knx-net/src/commissioning/download.rs)
  are existing source, not evidence that a Powerline path works. An imported
  medium reference, a mask identifier or a successful IP tunnel is not a
  verified PL110 transport or supported device.
- The [historical spec audit](../spec-audits/2026-09-19-cp-3_5_3-partial-download.md)
  includes both early findings and later corrections; use the maintained
  limitation and re-read original clauses before future implementation.
  The [research index](../RESEARCH.md) defines documented versus verified facts.

No PL110 bench, interface capability, manufacturer recovery procedure or
full pre-write image has been admitted by this package. PL132, other
Powerline variants and mixed-media compatibility are not inferred from PL110.

## Prerequisites before a separately proposed implementation

1. Name the exact Powerline variant, manufacturer/product, mask, application
   version and authorized test device. Supply identity and transport evidence,
   not a device-name heuristic. Define explicit refusal for every other medium.
2. Identify an interface and gateway path actually capable of that variant.
   Document the bridge's medium addressing, supported operations and limitations
   from primary protocol/manufacturer sources. Do not assume generic KNXnet/IP
   tunnelling alone supplies the missing media-specific behavior.
3. Have a qualified operator establish an isolated, electrically safe bench,
   appropriate coupling/isolation and protection, and absence of unintended
   installation targets. This document is not a mains wiring procedure.
4. Inventory every persistent effect of connection setup, authorization and the
   proposed write, including group-responser state. Define a complete,
   identity/version-bound backup and manufacturer-supported restoration path;
   a partial diagnostic dump or application payload is not a device image.
5. Separate authority into **offline implementation**, **specific read-only
   bench identification**, and **each exact mutation/recovery exercise**.
   Earlier TP/RF/IP approval and the general Session2 goal confer none of these.

## Future phases and exit criteria

| Phase | Proposed activity after its own authorization | Required exit evidence |
| --- | --- | --- |
| P0 — no hardware | Re-read original standard/manufacturer sources and define the media adapter contract, exact unsupported boundaries and recovery coverage. | Clause/source registry, reviewed target/transport/recovery matrix; all unresolved effects fail closed. |
| P1 — offline implementation | Pure codec/service/simulator tests for the admitted variant; no sockets or devices. Include wrong-medium, wrong-mask, missing identity, unsupported properties, stale plan, cancellation and interrupted-operation cases. | Actual tests and named negative controls; TP1/RF/IP never gain a group-responser write; missing recovery proof refuses before connection activity. |
| P2 — separately approved read-only bench | Identify exactly one nominated device through the verified interface; read only the specifically approved state. Inventory any setup-side effects before calling an operation read-only. | Identity/medium/version agree with the plan; bounded captures and persisted/read-back recovery evidence; no secrets in public logs. |
| P3 — separately approved mutation | Execute one reviewed operation on the isolated device after immediate target and complete recovery checks. | Requested state read back from the device, relevant service/function verified, baseline compared; transport acknowledgement alone is insufficient. |
| P4 — separately approved recovery | Exercise only a reviewed restoration/failure case whose recovery was proven before P3. | Exact covered state restored and function verified; unresolved side effects reported, never silently waived. |

Stop on ambiguous targets, variant/identity mismatch, unavailable normative
semantics, incomplete/unreadable backup, unverifiable receipt persistence,
unknown setup effects, timeouts, unexpected state changes or operator cancellation.
Do not automatically reconnect, retry a write, broaden the medium scope, unload,
reset or invoke full download as an unconfirmed recovery action.

## Handover and acceptance

The deliverable here is this preparation plan, not a Powerline implementation.
Future evidence should bind source SHA, exact device/interface identity,
application/mask versions, operation scope, approved bench and authorization,
pre-send recovery receipt, traces, readbacks and outcome. Public reports omit
real addresses/identities and secrets where required; originals stay private.
Only suitable real hardware and separately authorized observations can support
hardware compatibility claims. Existing full-download behavior remains unchanged.
