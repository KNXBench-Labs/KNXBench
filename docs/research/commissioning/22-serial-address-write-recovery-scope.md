# 22. Serial-address write recovery scope (2026-09-30)

[Research index](../commissioning.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="22-serial-address-write-recovery-scope-2026-09-30"></a>
<!-- BEGIN preserved record-2-031 -->
## 22. Serial-address write recovery scope (2026-09-30)

**[V]** `serial_number_write` (MP §2.5) can observe the former address by
serial number and the proposed address's occupancy before broadcasting; the
current CLI/HTTP paths did not persist that state before send. A later
readback/HTTP response is volatile and cannot recover an interrupted write.
The new shared application precondition refuses confirmed CLI/HTTP writes
before opening a tunnel (ADR-0057); read-only lookup and the protocol
simulator are unaffected.

**[D]** The KNX Association's Architecture v3.0 §2.6 says network identifiers
and formats do not necessarily describe device-internal storage
([specification PDF](https://pahl.de/download/dissertation/ds2os.lab/files/03_01_01_architecture_v3.0.pdf),
p. 9). The Association's [Device Reader documentation](https://support.knx.org/hc/en-us/articles/115001822070-Device-Reader)
requires the memory range and address space and potentially a supplied
access key for reading a device's memory. **[I]** Neither source identifies the complete
manufacturer-specific storage side effects of an MP §2.5 address write; a
saved old address or an arbitrarily chosen memory region is therefore not
proof of a full affected-storage backup. No access key may be guessed.

**[A]** Do not mistake this fail-closed availability change for a verified
recovery implementation or an authorization for another live test. Research
per-device/mask scope and prove durable pre-send persistence/readback plus a
recovery path before considering reopening, then add phase-aware activity
without claiming a Telegram's send is a verified device effect.


<!-- END preserved record-2-031 -->
