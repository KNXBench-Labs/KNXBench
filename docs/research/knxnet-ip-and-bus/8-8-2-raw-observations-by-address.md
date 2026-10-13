# 8.8.2 Raw observations, by address

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="882-raw-observations-by-address"></a>
<!-- BEGIN preserved record-1-027 -->
#### 8.8.2 Raw observations, by address

**`1.1.24`** — occupied, both methods agree:

| Read | Raw payload / outcome |
| --- | --- |
| `A_DeviceDescriptor_Read(0)` | `07 01h` (Mask Version `0701h`) |
| `A_PropertyValue_Read`(obj 0, PID 12, 1 elem @1) | `00 0Ch` |
| `A_PropertyValue_Read`(obj 0, PID 78, 1 elem @1) | refused, `nr_of_elem = 0` (AL §3.4.4.2's own refusal shape) |
| `A_PropertyValue_Read`(obj 0, PID 13, 1 elem @1) | refused, `nr_of_elem = 0`, same shape |
| `A_PropertyValue_Read`(obj 1, PID 5, 1 elem @1) | `01h` (`Loaded`) |
| `A_PropertyValue_Read`(obj 2, PID 5, 1 elem @1) | `01h` (`Loaded`) |
| `A_PropertyValue_Read`(obj 3, PID 5, 1 elem @1) | `01h` (`Loaded`) |

The scan probe's independent `A_DeviceDescriptor_Read(0)` on `1.1.24` agrees:
`Occupied`, mask `0701h`, 202 ms.

**`1.1.25`, `1.1.26`, `1.1.27`, `1.1.28`, `1.1.30`, `1.1.31`, `1.1.32`** —
occupied per the scan probe: `Occupied`, mask `0701h` each, round trips
86–107 ms. But every one of the seven `ManagementSession` reads listed above
for `1.1.24` returned, for every one of these seven addresses, the identical
outcome: *"no answer to A_DeviceDescriptor_Response / A_PropertyValue_Response
after 3 attempt(s) of 3s; this is a lost frame, a protected or absent target,
or a device that is not listening, and the wire does not distinguish them"* —
the session's own honestly-worded time-out, TL clause 4's `max_rep_count = 3`
exhausted every time. §8.8.3 reconciles this.

**`1.1.29`** — vacant by both methods, and this is the one address where they
agree on absence rather than disagree on presence: the scan probe got no
`L_Data.con` at all for its `T_Connect` within the full 6006 ms window (the
Standard's own presence signal, `03_05_02` §2.19); the `ManagementSession`
read got the same worded time-out as the seven false negatives above — from
that method alone, `1.1.29` is indistinguishable from them.


<!-- END preserved record-1-027 -->
