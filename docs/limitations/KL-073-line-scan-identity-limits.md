# Line-scan identity limits

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-073)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 73. A line scan cannot learn product identity, manufacturer, or serial number

**Limitation.** `knx bus scan` reports an address occupied or vacant and,
when occupied, the responding device's Mask Version — nothing more. It
cannot say which product is installed, who made it, or its serial
number.

**Cause.** `NM_IndividualAddress_Check`'s only application-layer step is
`A_DeviceDescriptor_Read` with `descriptor_type = 0`, which returns DD0,
the Mask Version — **[D]** *"Identification of an implementation, for
operation like download, memory_write … In particular, the Mask Version
is read through a dedicated Application Layer service by the S-Mode
Management Client (ETS) to conclude on the Configuration Profile of the
device and on possible further discovery and configuration steps"*
(`03_01_02 Glossary v01.05.03 AS.md:236`). A Mask Version identifies an
implementation family/coupler-medium class, not a product: **[D]**
`06_02_01 Coupler Model 2.0 v01.01.01 AS` §1.5.2 notes many different
coupler products deliberately share one Mask Version. Product identity,
manufacturer, and serial number need a separate, additional
connection-oriented read after the scan step — e.g. `A_PropertyValue_Read`
on the Device Object (`object_index = 0`), `PID_SERIAL_NUMBER` (PID 11) —
**[D]** `03_05_03 Configuration Procedures v02.01.01 AS.md:4797` and
`03_06_03 EMI_IMI v01.04.02 AS.md:5074`. That step is not part of
`NM_IndividualAddress_Check` and was explicitly out of scope for T17; it
is T16's territory (device-catalog/product identity work).

**Impact.** A scan's occupied/vacant list, and its Mask Version per
occupied address, cannot by itself populate a topology view with product
identity or resolve which manufacturer's device answered. A user
reconciling a scan against a project still needs a second signal, or a
manual lookup, to identify an undocumented device.

**Lifted when.** T16 or a successor adds a `A_PropertyValue_Read` follow-up
step per occupied address; whether every Mask Version a scan might
encounter even supports Property services, versus only Memory-based
access as some older masks do, is unverified and would need checking
before that step could be relied on unconditionally.

**AR14 (2026-10-04).** Pinned by `a_probe_asks_only_for_the_mask_version`:
every frame a probe sends is `T_Connect`/`T_Disconnect` or
`A_DeviceDescriptor_Read` type 0.
<!-- END retained catalogue record -->
