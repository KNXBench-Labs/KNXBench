# 8.7.3 Correction to §8.4: `PID_OBJECT_INDEX` is not the LSM selector

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="873-correction-to-84-pid_object_index-is-not-the-lsm-selector"></a>
<!-- BEGIN preserved record-1-012 -->
#### 8.7.3 Correction to §8.4: `PID_OBJECT_INDEX` is not the LSM selector

§8.4 states each LSM is *"addressed independently by `object_index`
(`PID_OBJECT_INDEX`, PID = 29)"*, and its PID table row 29 reads *"Addresses
which Interface Object/LSM a given access targets"*. That conflates two
different things.

`03_05_01 Resources` §4.2.29: `PID_OBJECT_INDEX` is **read-only** and *"shall
contain the local Object Index of the Interface Object in which it is located"*
— a property a device reports about itself. **[D]** The thing that selects
which Interface Object a property access targets is the **`object_index` field
of `A_PropertyValue_Read`/`_Write`** (`03_03_07` §3.4.4), not PID 29. The
earlier rows are left in place above rather than rewritten, with this
correction as the authority; the design spec §3.2 states it for implementers.


<!-- END preserved record-1-012 -->
