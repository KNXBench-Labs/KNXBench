# 23. K13 reset recovery scope (2026-09-30)

[Research index](../commissioning.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="23-k13-reset-recovery-scope-2026-09-30"></a>
<!-- BEGIN preserved record-2-032 -->
## 23. K13 reset recovery scope (2026-09-30)

**[V]** The public `knx device reset-address` previously connected after a
valid address list and confirmation phrase without performing an automatic
pre-write storage backup. MP §2.18's first broadcast read identifies devices
in programming mode, not their internal storage. ADR-0058 now blocks the
confirmed CLI before tunnel opening; an offline test with a bound local UDP
socket witnesses zero datagrams. Plan-only and direct simulator procedures
remain available.

**[V]** RESEARCH §19.16 records one authorized `1.1.67` live reset followed
by address recovery and an unchanged application-memory dump. The operator
had separately captured a persistent backup; the CLI had not verified it or
bound it to every storage area a reset could change. The device's programming
LED stayed on despite the unevaluated restart.

**[I]** Neither that single-device observation nor the general address-reset
procedure establishes the complete manufacturer-specific affected-storage
scope for every target or every simultaneously pressed device. ADR-0057's
Architecture and Device Reader references (§22) caution against treating a
network address or an arbitrary memory range as a whole-device backup. A
safe reopening needs an identified device/mask, proven storage scope,
pre-send durable per-device backup/readback and a separate recovery plan. No
access key may be guessed; no new hardware go is implied.


<!-- END preserved record-2-032 -->
