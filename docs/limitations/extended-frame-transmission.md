# Extended-frame transmission

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-extended-frame-transmission)

Since 2026-10-25 the cEMI encoder clears the Frame Type flag for every
`L_Data` frame whose length octet exceeds 15, as the Data Link Layer and
EMI/IMI clauses require ([spec audit](../spec-audits/2026-10-25-cemi-frame-type.md)).
Before that, a memory write longer than 12 octets, which a device with
`PID_MAX_APDU_LENGTH` above 15 allows, went out marked as a standard frame.

Limits:

- Verified only by unit tests and the simulator's KNXnet/IP front-end on
  loopback. No real interface or device has received an extended frame from
  KNXBench.
- `decode_l_data` does not keep the received FT, so a short extended frame
  re-encodes as a standard frame. EMI/IMI §4.1.5.2.3 allows that correction.
- The reserved length `FFh` is not rejected by the encoder; no supported
  service reaches it.
