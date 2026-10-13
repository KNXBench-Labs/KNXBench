# Unlinked active communication objects

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-145)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

<a id="145-instance-level-flag-overrides-are-not-written-into-the-group-object-table"></a>
## §145 Instance flags are written; unlinked active objects can still differ

Instance-level flag overrides (`Layer::Instance`/`Layer::UserEdit`) are
written into active group-object table entries; malformed or empty flags
are refused, not replaced with product defaults. The corpus regression
checks 12 overridden, linked objects against read-only device octets
(RESEARCH §19.13). No KNXBench download to those house devices has run, so
this is image/read-back comparison rather than hardware download evidence.

**Remaining difference.** ETS clears the communication-enable bit for an
active object without a group-address link; KNXBench leaves it set. With no
association that object cannot send or receive either way. Neither program
behaviour nor download parity for unlinked objects is asserted.
<!-- END retained catalogue record -->
