# Spec audit — cEMI Frame Type for frames longer than 15 octets

Audit of 2026-10-25 against the KNX Standard v3.0.0 PDFs
`03_06_03 EMI_IMI v01.04.02 AS` and `03_03_02 Data Link Layer General v01.03.02 AS`.
Licensed text is paraphrased; page numbers are the printed footers.

## GAP-CEMI-FT.1 — every encoded `L_Data` claimed a standard frame — *fixed*

**Clauses.**

- DLL §2.2.5, p. 11: a standard frame codes the APDU length in 4 bits, an
  extended frame in 8 bits (`FFh` reserved as escape code). A frame with
  `L > 15` therefore cannot be a standard frame.
- EMI_IMI §4.1.5.3.3 (`L_Data.req`), p. 79: Ctrl1 bit 7 (FT) selects the frame
  type used on the medium, `0` extended, `1` standard.
- EMI_IMI §4.1.5.3.4 (`L_Data.con`), p. 80, and §4.1.5.3.5 (`L_Data.ind`),
  p. 81: the cEMI server sets FT to the frame type actually sent or received.
- EMI_IMI §4.1.5.3.2, p. 77: EFF `0000b` is the code for standard frames and
  long frames (APDU above 15 octets); it needs no change.

**Defect.** `encode_l_data` in `crates/knx-net/src/cemi.rs` set
`CTRL1_STANDARD_FRAME` unconditionally. Once `PID_MAX_APDU_LENGTH` allows more
than 12 octets per `A_Memory_Write` (`write_limit` in
`crates/knx-core/src/commissioning/memory.rs`), a write produces `L > 15`, and
the client sent it as a standard frame. The simulator's tunnelling front-end
re-encodes confirmations and indications through the same function, so its
`L_Data.con`/`.ind` for a long frame named the wrong frame type as well.

**Fix.** `finish_l_data` clears FT when `L` exceeds `STANDARD_FRAME_MAX_LENGTH`
(15). Shorter frames keep FT = 1. Tests:
`a_frame_longer_than_fifteen_octets_is_an_extended_frame` (boundary 15/16, the
52-octet write, `.con` with error bit, `.ind`),
`an_extended_request_round_trips_unchanged`, and the front-end test
`an_extended_request_is_confirmed_as_an_extended_frame`. All three fail with
the clearing line removed.

**Not changed, still open.**

- The decoder does not keep the received FT. An extended frame with `L <= 15`
  (allowed by EMI_IMI §4.1.5.2.3, p. 75) re-encodes as a standard frame. That
  clause allows a cEMI server to correct the frame type, so this is
  conforming, but `LDataFrame` does not report the original flag.
- The encoder accepts `L = 255` although DLL §2.2.5 reserves `FFh`. No
  supported service reaches that length (`A_Memory_Write` stops at 63 octets).
- RF frames are unchanged: on RF, FT is "don't care" (EMI_IMI p. 79).
- Not tested against an interface or device. A real interface's handling of
  extended frames from KNXBench is unverified.
