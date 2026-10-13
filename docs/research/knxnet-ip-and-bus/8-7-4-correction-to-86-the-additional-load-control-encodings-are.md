# 8.7.4 Correction to §8.6: the Additional Load Control encodings are documented

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="874-correction-to-86-the-additional-load-control-encodings-are-documented"></a>
<!-- BEGIN preserved record-1-013 -->
#### 8.7.4 Correction to §8.6: the Additional Load Control encodings are documented

§8.6.4 concluded that *"the argument encodings are the part inference cannot
supply"*. That is too strong. `03_05_02 Management Procedures` §3.31.3
specifies `DM_LoadStateMachineWrite_RCo_IO` as **exactly 10 octets** written to
`PID_LOAD_STATE_CONTROL` (property_id 5, start_index `01h`, nr_of_elem `01h`) —
`01h`/`02h`/`04h` + 9 × `00h` for Start Loading / Load Completed / Unload, all
zeros for No Operation, and `03h` + subtype + 8 octets for Additional Load
Controls — with field tables for subtypes `00h` AllocAbsDataSeg, `01h`
AllocAbsStackSeg, `02h` AllocAbsTaskSeg, `03h` TaskPtr, `04h` TaskCtrl1, `05h`
TaskCtrl2, `0Ah` Relative Allocation and `0Bh` Data Relative Allocation. **[D]**

What remains undocumented is narrower than §8.6 stated, and worse in one
respect. The **`LdCtrl*` element names** still return zero hits across the
extracted Standard corpus, so the mapping from the 13 unmatched product-data
kinds to these subtypes is the gap — not the payload layouts. And the hazard is
sharper than an error message: **[D]** §3.31.3 on subtype `0Ah`, *"If the
requested number of octets is not supported by the Management Server (device)
then the Load State Machine of the loadable part shall change to error."* Per
§8.7.2, `Error` is escapable only by `Unload`, and **[D]** Table 93 says unload
makes the data *"undefined"*. A guessed subtype destroys a configuration
silently rather than failing loudly.


<!-- END preserved record-1-013 -->
