# 8.7.10 The authorisation model, in full

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="8710-the-authorisation-model-in-full"></a>
<!-- BEGIN preserved record-1-019 -->
#### 8.7.10 The authorisation model, in full

`03_03_07` §3.5.7 `A_Authorize_Request-service` specifies the whole model and it
is small enough to state completely. **[D]**

- A key is *"four octets long and of data type unsigned32"*. Access levels are
  *"(unsigned8) between 0 (maximum level, i.e., maximum access rights) and 3
  (minimum level …) or 0 … and 15 (minimum level …)"* — **lower is more
  powerful**, and both ranges exist.
- Lifetime: *"A current access level shall be valid until the connection is
  released or a new key is indicated with the A_Authorize_Request service."* So
  authorisation is **per connection**, not per operation, and every reconnect
  must re-authorise — the same structural trap as Verify Mode in §8.7.6, and for
  the same reason.
- Not authorising is **not** an error: *"if the communication partner does not
  authorize itself, the Remote Management shall select the maximum access level
  protected with FFFFFFFFh as the current access level."*
- Authorising with a wrong key is **worse than not authorising**: *"if the
  communication partner authorizes itself with an invalid key, the Remote
  Management shall select the minimal access level (this is level 3 or level
  15)"*. There is no negative response. A guessed key therefore silently reduces
  the access the client would have had by doing nothing — which is why the design
  spec forbids sending any key the user did not supply.

`03_05_02` §3.5 `DM_Authorize (flags, keys)` wraps it, requires `DM_Connect`
first, runs *"only when it is required by the Management Server"*, and says the
support question is answerable up front: *"Whether or not a Management Server
supports authorisation can directly be retrieved from the Device Descriptor Type
0 (mask version)."* Its `_RCo` sequence guards the exchange with
*"if authorization is required (key != FFFF FFFFH)"*, making `FFFFFFFFh` the
client-side "no key configured" sentinel.

§3.5.2 `DM_Authorize2_RCo` handles the asymmetry above: it *"does not presume
that the device has been locked with the key that is provided to the procedure.
Therefore, it authorizes subsequently with the key FFFFFFFFh and with the key
client_key and continues with the key that gives the maximal access rights."* Its
error handling is absolute — *"Failure of any of the contained Application Layer
Services shall lead to failure of the entire Configuration Procedure."* — which is
the safe direction: step 03 failing means the destructive step 05 never runs.

**Scoping, corrected 2026-09-14.** This paragraph used to call §3.5.2 *"the
procedure a real client should run"*. It is not unconditionally that. The clause
carries a *Use • Profiles* row naming **System 2** and **BIM M112**, while the
download this project specifies is `03_05_03` §3.5.2 **System B**; §3.5.1
`DMP_Authorize_RCo` carries no profile restriction. Claiming §3.5.2 as the
default would be asserting cross-profile applicability that no clause states, so
the design spec §10.4 rules the other way: **MP §3.5.1 is the default for System B
and §3.5.2's two-key comparison is a defensive extension, off by default**, with a
regression test asserting that the default issues exactly one
`A_Authorize_Request` per connection and never the `FFFFFFFFh` probe.

Key writing is `03_05_02` §3.6 `DM_SetKey (flags, keys, level)` over `03_03_07`
§3.5.8 `A_Key_Write`: *"Every device shall be able to handle exactly one key per
access level. The number of access levels supported by a device is Profile
dependent."*, `FFFFFFFFh` invalidates a level's key, and you cannot grant what
you do not hold — *"The current access level shall be less or equal to the access
level indicated … otherwise the remote application process shall return FFh"*.
Not implemented in this project, and the design spec says why: a key write can
lock the client out of its own reach.

How insufficient access **presents on the wire** is the part that matters for
error reporting, and it splits by service family: classic memory and property
services fail silently (`03_03_07` §3.5.4: *"the service indication shall be
ignored"*), while the extended services carry `FCh E_ACCESS_DENIED` in their
return-code tables (`03_03_07` §3.4.5.5, §3.4.8.3, §3.4.9.1, §3.4.9.2). **[D]**
So an unauthorised classic write is indistinguishable from a lost frame, and the
level has to be established before writing rather than diagnosed after.


<!-- END preserved record-1-019 -->
