# 8.7.12 `AN194`: what a reset does to which resource

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="8712-an194-what-a-reset-does-to-which-resource"></a>
<!-- BEGIN preserved record-1-021 -->
#### 8.7.12 `AN194`: what a reset does to which resource

`AN194 v02 Master Reset of Resources AS` tabulates, per Interface Object and PID,
the effect of each reset flavour, in columns `-` Local Reset, `02h`, `07h`, `01h`
Confirmed Restart, `none` Basic Restart, and Power Cycle. **Six columns, of which
exactly three are Erase Codes** (`02h`, `07h`, `01h`); Local Reset, Basic Restart
and Power Cycle are not Erase Codes at all. "All six columns" is therefore a
correct paraphrase of a full row and "all six Erase Codes" is not — there are
only three of those. Its vocabulary has to
be read first, because "default" means four things: *"Not influenced"* (the
client may rely on the value not changing), *"recalculate"* (*"The MaC shall read
the value before using it."*), *"KNX default"* (a value *"not specified in the
KNX Specifications"* that the client may nevertheless assume), and
*"implementation default"* (*"If the value is given in the product description
then the MaC may rely on this; otherwise, the MaC has to read the value."*).
**[D, corpus]**

The rows that matter here, from §2.3.2.2 (Device Object) and §2.3.2.3 (Address
Table Object):

- `PID_LOAD_STATE_CONTROL` and `PID_ERROR_CODE` are *"implementation default"*
  after a local reset, `02h` or `07h`, and **`"not influenced"` by Confirmed
  Restart, Basic Restart and Power Cycle**. That is an independent confirmation
  of §8.7.2's conclusion from a different document: an interrupted download is
  not repaired by power-cycling the device.
- `PID_DEVICE_CONTROL` (Verify Mode) and `PID_PROG_MODE` are *"KNX default"* in
  **all six** columns — so Verify Mode does not survive a power cycle, and
  programming mode is off after any restart, which is a second cause for
  §8.7.7's "it may be off when you get there".
- `PID_TABLE_REFERENCE` is *"recalculate"* — but **not uniformly, and the
  difference is the interesting part.** Read per Interface Object rather than as
  one row (correction, 2026-09-14; the earlier single bullet flattened five
  objects into one claim):
  - §2.3.2.3 Address Table Object and §2.3.2.10 Object Type 9 Group Object Table:
    *"recalculate"* in the `-`, `02h` and `07h` columns — and that set is one
    reset kind plus two Erase Codes, not three Erase Codes, because `-` is
    AN194's *"Local Reset to default state"* and carries no Erase Code — and
    *"not influenced"* in the `01h` (Confirmed Restart), Basic Restart and Power
    Cycle columns.
  - §2.3.2.4 Association Table Object, §2.3.2.5 Application Program Object and
    §2.3.2.6 Application Program 2 Object: *"recalculate"* in **all six**
    columns, Local Reset, Erase Codes and restarts alike.

  The design rule that survives both shapes is the stronger one: re-read
  `PID_TABLE_REFERENCE` after **any** restart, not merely after a reset, because
  the Local Reset column is *"recalculate"* in every one of the five objects. A
  cached base address is invalid in all cases; only the *reason* differs.
- `PID_RUN_STATE_CONTROL` is *"recalculate"* in §2.3.2.5 and §2.3.2.6 in **all
  six** columns — Local Reset, `02h`, `07h`, `01h` Confirmed Restart, Basic
  Restart and Power Cycle. (Statement *extended* in fix round 3: the rows read say
  all six, and the earlier wording named only Local Reset plus the Erase Codes,
  which was true but narrower than the evidence.) So the run state of an
  application program is never inferable after a reset **or after any restart**.
- `PID_MAX_APDU_LENGTH` is marked CONSTANT and *"not influenced"* everywhere —
  the one value in this area that may legitimately be cached per device.
- `PID_DOWNLOAD_COUNTER` is *"recalculate"*, so it is not a "has this device been
  touched" fingerprint across a reset.

Three limits AN194 sets on any client, all quoted: *"The interpretation of this
term is implementation specific"* (Power Cycle — so no test plan step may be
"power-cycle it"); *"The below defined Erase Codes “ex-factory” shall thus be
understood as “set back a default state” rather than “set back to the delivery
state”."*; and *"There are no requirements on whether or not an application may be
running in the MaS after a Master Reset"*. The last one is the sharpest: a device
that reports `Loaded` after a factory reset may be conformant, so device state
must always be read and never inferred from an operation performed.

**What was not read**, stated so a later reader can tell "AN194 says nothing"
apart from "nobody looked": six of AN194's twelve Interface Object sections were
transcribed (§2.3.2.2 Device, §2.3.2.3 Address Table, §2.3.2.4 Association Table,
§2.3.2.5 Application Program, §2.3.2.6 Application Program 2, plus the
§2.3.2.10 Group Object Table rows named above). The remaining six Interface
Object types in the document — §2.3.2.7 Router, §2.3.2.8 LTE Address Routing
Table, §2.3.2.9 cEMI Server, §2.3.2.11 KNXnet/IP Parameter, §2.3.2.13 Security
Interface and §2.3.2.14 RF Medium — were not transcribed, and neither was
§2.3.2.12 *"Data Security"*, which is a three-Resource table rather than an
object type. (Counts and the object list corrected in fix round 3: the prose said
five while the list named six, "fourteen" was a miscount of a thirteen-clause
range, and the earlier "among them" list invented a Polling Master and a File
Server object that AN194 does not tabulate.) They were skipped
because the download procedure this pass specifies does not
touch them. Three PIDs were checked across every object that was read and agree
everywhere: 5 `PID_LOAD_STATE_CONTROL`, 28 `PID_ERROR_CODE` and 27
`PID_MCB_TABLE`.


<!-- END preserved record-1-021 -->
