# 8.7.8 Smaller findings

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="878-smaller-findings"></a>
<!-- BEGIN preserved record-1-017 -->
#### 8.7.8 Smaller findings

- **LSM Realisation Type 2 is not specified at all.** **[D]** `03_05_01`
  §4.23.3: *"This Realisation Type is not specified in this version of this
  document."* Not a gap in our search — a gap in the Standard.
- **`PID_ERROR_CODE` must be read before unloading.** **[D]** `03_05_01`
  §4.2.28: *"When the load state changes from 'Error' to a different state then
  the Error Code shall be set to '0' (no error)."* Unloading to recover destroys
  the only evidence of what failed.
- **Allocation outside `Loading` is silently ignored.** **[D]** `03_05_03`
  §3.5.1.2 Table 4: *"An allocation (if necessary) shall occur if a segment is
  in the state Loading. In all other states the memory allocation shall be
  ignored."* Combined with `PID_REFERENCE` reading zero on failed allocation,
  zero has two distinct meanings.
- **The 64 kB / 1 MB service split is decided on `base + length`.** `03_05_03`
  §3.5.1.4 Table 5 gives the limits (`A_Memory_Write` ≤ 64 kB,
  `A_UserMemory_Write` ≤ 1 MB, `PID_TABLE_REFERENCE` 20 bits) and §3.5.2's step
  list gives the rule: *"if BaseAddress plus allocated memory is lower than
  FFFFh"* → `MemoryWrite`, else `UserMemoryWrite`. **[D]** Switching on the base
  address alone writes a segment's tail through the wrong service.
- **A failed partial allocation escalates to a larger download.** `03_05_03`
  §3.5.3: when the base address reads zero the procedure does not abort, it
  *"⇒ Continue at Nr. 07"* — unload all following segments and reload them,
  because *"Due to the ascending order of the memory segments it can be
  necessary to rearrange all or a subset of the segments when modifying one
  segment."* **[D]** The *"differential download algorithm"* that a matching CRC
  enables is **named and not specified**. Undocumented.
- **Unload ends with an address erase.** `03_05_03` §3.5.4 step 07: *"Set
  SerialNumber_IndividualAddress_Write(FFFFh) via broadcast"*. **[D]** Makes a
  device unaddressable by individual address. Documented here so it is
  recognised, explicitly not implemented.
- **Master Reset Erase Codes are fully enumerated** in `03_05_02` Table 4 —
  `01h` Confirmed Restart through `08h` Erase persistently stored application
  data, with `00h` reserved and `09h`–`FFh` rejected with *"Unsupported Erase
  Code"*. **[D]** `01h` is a confirmed no-op restart and is the restart a client
  should prefer over the unconfirmed `restart_type = 0`. `02h` Factory Reset's
  effects are *"implementation dependent"*, which means this application can
  never tell a user what a factory reset will do to their device.
- **A restart may or may not drop the connection.** **[D]** `03_03_07` §3.4.2.2:
  *"The Management Server (device) may or may not break down the Transport Layer
  connection."* Client-side session state after a restart is unknown by
  specification, not by accident.
- **Cross-reference drift.** `03_05_01` Table 93 points at *"clause 3.27
  'DM_LoadStateMachineWrite' in [09]"*, but in the current `03_05_02` that
  procedure is **§3.31** and §3.27 is `DM_InterfaceObjectRead`. Follow the name.


<!-- END preserved record-1-017 -->
