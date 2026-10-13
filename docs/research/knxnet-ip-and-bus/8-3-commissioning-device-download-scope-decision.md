# 8.3 Commissioning / device download — scope decision

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="83-commissioning--device-download--scope-decision"></a>
<!-- BEGIN preserved record-1-005 -->
### 8.3 Commissioning / device download — scope decision

Technically, the ingredients are present and machine-readable: mask-version `Procedures` and `Resources` in `knx_master.xml`, `LoadProcedures` + `AbsoluteSegment` + `AddressTable`/`AssociationTable`/`ComObjectTable` offsets in each application program, and A_Memory/A_PropertyValue management services over the bus.

It is nevertheless **not started**, and four things block it:

1. Writing wrong memory images to a real device bricks it. This needs hardware we can afford to destroy.
2. **Revised, 2026-09-11 (R5 spike, §8.4).** The generic complete/partial download load procedure *is* documented in the KNX Standard — `03_05_03 Configuration Procedures` §3.5.2/§3.5.3, CRC-driven via `PID_MCB`, and "Differential Download" is a formally defined Glossary term. What remains undocumented outside ETS/manufacturer tooling, and absent from both KNX specification databases searched, is the product-specific `Legacy*` compatibility-flag matrix that decides whether and how a given application program participates in that procedure. See §8.4 Q3/Q4 for the exact citations and the distinction between "the procedure" (documented) and "this product's flags for the procedure" (not).
3. Vendor `Baggages` DLLs participate in download for some devices.
4. KNX Secure devices require the key material handling of §9.

**Ruling, 2026-09-11.** Asked whether commissioning is permanently out of scope, the user said no: it must work too, but the work waits until the KNX specification database is finished. The four blockers above are unchanged — they are why it has not started, not a reason it never will. See [KNOWN_LIMITATIONS.md §7](../../KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked), [GAP_ANALYSIS_ETS.md](https://github.com/KNXBench-Labs/KNXBench/blob/a584007fc05a/docs/GAP_ANALYSIS_ETS.md) row **E1** (which stays open), and backlog task **T30**.

Recommendation: build toward *read/diagnose/monitor* first (Session 6), and treat programming as a separate, later, explicitly-flagged research effort. Nothing in the architecture should preclude it — hence keeping `LoadProcedures`, `Memory`, `AbsoluteSegment` and mask data in the model rather than discarding them at import.

---


<!-- END preserved record-1-005 -->
