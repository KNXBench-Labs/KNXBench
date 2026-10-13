# Memory-control access-nibble inference

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-099)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 99. `MemoryControlBlock`'s access-nibble order is inferred, not spec-stated

**Limitation.** RES §4.2.27, Table 12, p. 39, lays out `PID_MCB_TABLE`'s
eight-octet element as Segment Size 1 (4 octets), CRC Control Byte (1
octet), Read Access 1 and Write Access 1 sharing one octet as two 4-bit
nibbles, then CRC (2 octets). The table names which nibble is Read Access 1
and which is Write Access 1 by column position only; it does not state
which nibble is high and which is low, the way some other split-byte
KNX fields (e.g. `PDT_UNSIGNED_CHAR` sub-fields elsewhere in RES) do. This
project's `crates/knx-core/src/commissioning/mcb.rs` reads Read Access 1 as
the high nibble and Write Access 1 as the low nibble, matching the table's
left-to-right column order — a reasonable convention, but an inferred one,
not a cited one. Neither `PID_MCB_TABLE` nor a load-verify path in Task C2's
scope (CP §3.5.3) reads or acts on these two fields; only the CRC (octets
6-7) and the CRC Control Byte's bit 0 (RES §4.2.27.1.1, Table 13, p. 39) are
compared, so this order has no effect on any current comparison outcome.

**Cause.** The printed table's ruling groups Read Access 1 and Write Access
1 visually into one narrow column, and the corresponding text run gives no
bit-numbered breakdown the way Table 13 does for the CRC Control Byte's
individual bits. Nothing else in RES §4.2.27 cross-references this nibble
order either.

**Impact.** None measured. `MemoryControlBlock::read_access` and
`::write_access` are populated and round-trip correctly under this
project's own convention, but a device or a second implementation that
reads the nibbles the other way round would silently disagree with this
project's labelling of which access level applies to which side — while
still agreeing on every octet's raw bit pattern, since nothing is
reordered, only relabelled.

**Lifted when.** A KNX-certified reference implementation, a conformance
tool, or explicit bit-numbered spec text for Table 12 (matching Table 13's
style) settles the nibble order, or a real device's read/write-access
behaviour is observed to disagree with this project's current labelling.

**Accepted boundary, 2026-09-28 (goal-commission K8).** User decision
2026-09-28, asked for each K8 entry to fix it or accept it: *"das was am
sinnvollsten ist"* (whatever makes the most sense), i.e. the recommendation.
The nibble order stays inferred and labelled as such; the "Lifted when"
condition above still applies, and a device that disagrees reopens it.
<!-- END retained catalogue record -->
