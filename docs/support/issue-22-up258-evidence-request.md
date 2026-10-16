# Issue #22 — prepared reporter message (not posted)

Prepared 16 October 2026. **Draft only:** no GitHub comment has been sent and no
hardware operation is requested to start. The associated
[decision](../research/2026-10-16-up258-production-decision.md) defines the exact
remaining blockers. Values below are a proposed comparison, not measurements.

---

Hi @thengsty — your identifiers and PR #24 let us inspect the exact public
Siemens package without needing your project. Thank you! We have narrowed the
next missing evidence; this is **not a programming unlock**.

**Could you provide the actual firmware string of your UP 258D12**, if it is
already known or its label is safely accessible without opening, disconnecting
or changing the installation? Please omit serial numbers and photos of the
whole label. The application version `4` is not the firmware version.

We also need one small, deliberately discriminating ETS comparison. **Please
do not program, read or capture a device for this request yet.** First let us
know whether the following can be produced **offline**, or whether you already
have suitable results. If neither is possible, a reply saying so is sufficient.

### 1. ETS-side preparation — offline only

Use local copies, the same application `M-0001_A-9205-04-9270`, version 4, and
the pinned public package. Keep all other settings and group links identical.
In **General**, choose **Sensitivity of PIR sensor = automatic**
(`P-900_R-900`, raw value `0`) in both copies. Then change only:

| Copy | Fallback time (sensitive → low sensitive) | Source reference | Raw enum value |
| --- | --- | --- | --- |
| A | 5 min | `M-0001_A-9205-04-9270_P-903_R-903` | 300 |
| B | 10 min | same reference | 600 |

This is one direct 16-bit field, not a pair of overlapping Union alternatives.
The alternate reference `P-903_R-904` must not also be active. Do not edit the
hidden `P-901` field. If the controls do not match your ETS/product version,
stop and tell us; do not substitute a different parameter.

**If your existing ETS tooling exposes the generated image offline**, we need
only the two octets at `AS-3C00 + 248` (`0x3CF8..0x3CF9`) and the one-octet
control at `+250` (`0x3CFA`) for A and B, plus the exact ETS version/build and
whether the results are generated-image bytes or actual readback. We are not
claiming ETS has a particular export button; do not install a new tool or run a
bus operation to find one.

For orientation only, 300/600 would be `01 2C` / `02 58` if high-octet-first,
or `2C 01` / `58 02` if low-octet-first. Please report observed bytes unchanged,
including a mismatch, rather than selecting the expected answer.

### 2. Acquisition — a separate, later decision

If offline images are unavailable, we must agree on a bounded collection plan
first. Each ETS programming operation, each device readback, any monitoring
operation and any restore would need its **own explicit approval** for the
specific device and scope. Nothing here approves any of them, authorises a
KNXBench write, or treats your ETS project as a complete device backup.
Use only a suitable non-critical test target with an established recovery route;
no tests on occupied/safety-relevant functions and no intentional fault injection.

If you **already have** a suitable locally held ETS full-download trace, please
say so; do not upload it. After a separate local privacy review, the useful
additional projection would be only the ordered object 1/2/3 PID 5 control
payloads (ten octets each), with their corresponding status results, and any
already-observed object 1/2/3 PID 7 response index/count/length/bytes. That can
distinguish the four source `SegFlags=0` allocations and lengths from actual
wire controls, and two-byte resource metadata from actual property responses.
Include only the Verify Mode bit's transitions if already observed. Do not add
reads, change a flag to 128, trigger a restart or guess an access key to fill
missing entries. Mark unobserved data as unavailable.

### 3. Result and publication — separate from acquisition

A minimal manually reviewed reply can be plain text:

```text
firmware: <actual string or unknown>
ets_build: <exact version/build or unavailable>
application: M-0001_A-9205-04-9270
package_sha256: 520d3fcee5b63d079ac7d0bd5e7ce9467ff84cb5cde06ac341b77aba47e3ee0f
method: <offline-generated-image / existing-ETS-write-trace / device-readback>
A: P900=0, P903=300, at_3CF8=<two observed octets or unavailable>, at_3CFA=<one observed octet or unavailable>
B: P900=0, P903=600, at_3CF8=<two observed octets or unavailable>, at_3CFA=<one observed octet or unavailable>
other_settings_and_links_unchanged: <yes / no / unknown>
optional_control_projection: <unavailable, or reviewed ordered records only>
```

For optional records, retain only operation order, service, object index,
property ID, element index/count, payload length/allowlisted payload and result.
Keep originals private. **Do not send a full project, raw capture, general dump,
keys, passwords, serials, individual/group/gateway addresses or recovery backup.**
Review and deliberately approve publication of the small result separately;
consent to an acquisition is not consent to upload it.

This comparison can settle the observed 16-bit encoding and selected load-control
facts, **not** complete allocator recovery or all parameter/Union semantics.
We will still have to finish and test the complete image, all 21 source positions
and identity-bound recovery before proposing any KNXBench device download.
Issue #22 stays open; the detector does not get volunteered as a crash-test dummy. 🙂
