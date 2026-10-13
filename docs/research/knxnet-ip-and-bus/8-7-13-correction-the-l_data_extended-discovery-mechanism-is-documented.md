# 8.7.13 Correction: the `L_Data_Extended` discovery mechanism **is** documented

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="8713-correction-the-l_data_extended-discovery-mechanism-is-documented"></a>
<!-- BEGIN preserved record-1-022 -->
#### 8.7.13 Correction: the `L_Data_Extended` discovery mechanism **is** documented

§8.7.5 first recorded it as undocumented. It is `03_05_01` §4.3.7
`PID_MAX_APDU_LENGTH` (PID 56, `PDT_UNSIGNED_INT`), whose §4.3.7.2.1 states:
*"A Management Client supporting the L_Data_Extended-frame has to check this
value before starting download. If the PID_MAX_APDU_LENGTH is not present in the
Device Object, then the Management Client shall manage the device with
L_Data_Standard-frames with an APDU-length of maximal 15 octets."*, with the
value *"in the range between 15 and 254"* and 255 reserved as an ESCape Code.
**[D]**

That also explains where the 12 comes from: 15-octet APDU minus the 3 octets of
`A_Memory_Write` overhead (`03_03_07` §3.5.4 ignores a request with
`number > Maximum APDU Length – 3`) is exactly 12. The constant is arithmetic,
not folklore.

Two adjacent traps from the same clause: the property must be read from the
**Device Object**, because *"If PID_MAX_APDU_LENGTH is solely in the Router
Object …, then the device shall only support L_Data_Extended-frames for Routing
and only L_Data_Standard-frames for Management."*; and `PID_EXT_FRAMEFORMAT` is
**not** the mechanism — `03_05_01` §4.16.7.2.5: *"This Property is reserved for
LTE-Mode devices."*


<!-- END preserved record-1-022 -->
