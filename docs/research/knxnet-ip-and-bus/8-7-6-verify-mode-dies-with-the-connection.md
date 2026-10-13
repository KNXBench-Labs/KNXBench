# 8.7.6 Verify Mode dies with the connection

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="876-verify-mode-dies-with-the-connection"></a>
<!-- BEGIN preserved record-1-015 -->
#### 8.7.6 Verify Mode dies with the connection

`03_05_01` §4.2.14 Table 11: `PID_DEVICE_CONTROL` (PID 14, `PDT_BITSET8`, DPT
21.002) bit 2 is Verify Mode On; §4.2.14.7.3/§4.2.14.7.4: the default is 0, the
client must set it actively, and it is **automatically disabled when the
Transport Layer connection closes**. **[D]**

That collides with §4.23.2.4.1's requirement to re-establish a broken TL
connection periodically during a long load transition: a download that
reconnects mid-flight silently loses verification unless bit 2 is re-asserted
per connection. This is the sort of defect that produces no symptom until it
produces a corrupted device.


<!-- END preserved record-1-015 -->
