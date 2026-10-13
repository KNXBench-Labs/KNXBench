# 8.7.11 `DPT_ErrorClass_System` 20.011, enumerated from its own PDF

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="8711-dpt_errorclass_system-20011-enumerated-from-its-own-pdf"></a>
<!-- BEGIN preserved record-1-020 -->
#### 8.7.11 `DPT_ErrorClass_System` 20.011, enumerated from its own PDF

`03_07_02 Datapoint Types v02.02.01 AS`, DPT ID **20.011**,
`field1 = ErrorClass_System`, `Range: [0 to 18]`, encoding `N8`: 0 no fault,
1 general device fault, 2 communication fault, 3 configuration fault,
4 hardware fault, 5 software fault, 6 insufficient non volatile memory,
7 insufficient volatile memory, 8 memory allocation command with size 0
received, 9 CRC-error, 10 watchdog reset detected, 11 invalid opcode detected,
12 general protection fault, 13 maximal table length exceeded, 14 undefined load
command received, 15 Group Address Table is not sorted, 16 invalid connection
number (TSAP), 17 invalid Group Object number (ASAP), 18 Group Object Type
exceeds (PID_MAX_APDU_LENGTH – 2), and *"19 to 255 : reserved, shall not be
used"*. **[D]** Verbatim in the design spec §5.6, which is the copy the DPT
main-type-20 codec should use.

Two things fall out of it. **Values 6, 8, 9, 13 and 14 are download
diagnostics** — they say the *client's* request was wrong, which is what resolves
§9.2's "state is `Error` and I do not know why". And **value 15 is a validity
rule for the download itself**: `03_05_01` §4.16.3 requires *"The Group Addresses
shall be sorted in ascending order with increasing memory locations."*, so a
client that writes an unsorted Group Address Table manufactures a runtime error
report. For contrast, the neighbouring `20.012 DPT_ErrorClass_HVAC` has
`Range: [0 to 4]`; the reserved boundary differs per sub-type and cannot be
shared across main type 20.


<!-- END preserved record-1-020 -->
