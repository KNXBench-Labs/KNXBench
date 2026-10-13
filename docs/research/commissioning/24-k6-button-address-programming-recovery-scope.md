# 24. K6 button-address programming recovery scope (2026-10-01)

[Research index](../commissioning.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="24-k6-button-address-programming-recovery-scope-2026-10-01"></a>
<!-- BEGIN preserved record-2-033 -->
## 24. K6 button-address programming recovery scope (2026-10-01)

**[V]** The public CLI and HTTP button-based address programming paths
previously passed a valid new-address phrase and could open a tunnel. Neither
created a durable pre-send backup/readback for the actual device selected by
pressing its programming button. The generic download backup is plan-scoped
and cannot automatically prove the storage effects of MP §2.3. ADR-0059
now rejects confirmed public calls before tunnel or lock acquisition; an
offline CLI UDP listener and HTTP simulator witness no datagram/connector call.
Plan/phrase reads and direct simulated protocol sessions are unaffected.

**[V]** RESEARCH §19 and KNOWN_LIMITATIONS §116 describe a user-authorized
`1.1.67 → 1.1.68 → 1.1.67` run with readback and unchanged application
dump. That verifies the observed run, not an automatic full-storage backup
for arbitrary future button-selected devices, nor a successful restart
acknowledgment.

**[I]** MP §2.3's one-device button count establishes a protocol recipient,
not the recipient's product identity, affected-storage scope or durable
recovery record. A safe reopening requires those facts and a persisted,
read-back, device-bound pre-send backup plus an abort/restore plan. ADR-0057's
Architecture/Device Reader sources (§22) warn that network identifiers and
arbitrary read ranges are insufficient as full manufacturer-specific memory
evidence. No access key may be guessed; no new hardware permission is implied.

**[V] Public-entry audit, 2026-10-01 (offline).** The CLI and server's
confirmed K6 and serial-address routes, plus the K13 CLI, invoke their
respective pre-tunnel recovery gates. CLI device download and restore both
call `device_download::execute`, whose `run_memory_download_with_backup`
reads the plan's overwritten regions and touched load states before the
first mutation; `write_backup` fsyncs the file and directory and compares a
readback before allowing that write. This is *plan-scoped*, not a complete
device image, and does not establish a manufacturer-independent rollback.
The Debug-gated service-control CLI/HTTP action saves and rereads the exact
property element before writing bit 2; that receipt does not cover any
other storage. K14's erasing operation remains hardware-blocked. Group-value
sends (`knx bus write`, `POST /api/bus/write`) belong to `goal.md` (§5),
not a commissioning write gate; their receiver effects remain unverified.
No write route was reopened or hardware contacted in this audit.

**[V] Web projection boundary.** The still-locked Program address panel
fetches a valid phrase, opens its consent dialog, then on a confirmed start
displays the server's `412` message as an alert. It does not claim a
successful write, but still offers a now-unavailable action and has no
focused UI regression for the durable-recovery refusal. The Web lock owner
must decide the affordance and add its test; the server gate is the safety
boundary in the meantime. No Web source was changed here.

**[V] Official diagnostic-source cross-check, 2026-10-01 (offline).** KNX's
[Individual address](https://support.knx.org/hc/en-us/articles/360018775719-Individual-address)
documentation describes programming-mode response counts and a line scan of
addresses and mask versions; neither is a complete product/application or
storage-scope proof. [Device Info](https://support.knx.org/hc/en-us/articles/360018777979-Device-Info)
can report manufacturer, order number, serial, firmware and application, but
its available fields depend on mask and some group-communication information
cannot be read from some devices. The Association's
[Device Reader](https://support.knx.org/hc/en-us/articles/115001822070-Device-Reader)
documents a caller-selected memory range and address space. Its property
export leaves array properties larger than 64 bytes blank if they were not
loaded on demand. These are diagnostic facilities, not a documented complete
pre-write image of the button-selected device. The cited pages do not specify
which persistent areas MP §2.3 changes for our MDT 0701h target.

**[V] MDT manual supplied by the operator, 2026-10-01.** The public
[Taster 55 / Plus 55 / Plus TS 55 technical manual, version 1.3](https://www.mdt.de/fileadmin/user_upload/BE-TA55xx-02_MDT_TM_V13_DE.pdf)
(07/2025) covers `BE-TA55Px.x2`, explicitly including `BE-TA55P2.02` and
`BE-TA55P2.G2`. That is the *right product family*, unlike the AMI actuator
manual supplied earlier. It does not establish that the live `1.1.67` is
this `.x2` generation: the operator previously named `BE-TA55P2.G1`, the
local product-file comparison was only consistent with `.01`, and
`PID_ORDER_INFO` was not read (RESEARCH §8.8.5). The manual covers user
functions, commissioning and ETS parameters; no complete affected-storage
inventory or recovery image for MP §2.3 is established from it. The user
explicitly said non-public manufacturer documents are **not a blocker to
continuing the project**. This is not an authorization for a live write or
a substitute for durable, read-back pre-write recovery evidence.

**[V] Read-only order-identity feasibility, 2026-10-01 (offline).** KNX
Association's [Device Info description](https://support.knx.org/hc/en-us/articles/360018777979-Device-Info)
qualifies its order number: only when the manufacturer supplies it in a
readable/decodable form; otherwise the result is hexadecimal. Weinzierl's
[BAOS user guide, §5.2.5](https://weinzierl.de/images/download/documents/baos/weinzierl_knx_baos_users_guide.pdf)
lists `PID_ORDER_INFO` as property 15 of its own Device Object with a
generic-ten type; that is an example of property access, **not** proof that
MDT `1.1.67` exposes the same bytes, how to decode them, or that `.G1` and
`.G2` can be distinguished by them. The existing KNXBench
`ManagementSession::read_property` would surface a refused property as an
error; there is currently no public read-only `PID_ORDER_INFO` CLI route.
Even a successful raw read could narrow identity only after an independently
verified MDT mapping; the project product reference describes intent, not
the hardware. No property read, gateway contact, key use or live operation
occurred in this source/code check. A future explicitly scoped read-only
preflight must use the named device and gateway, the one-tunnel exclusion,
an operator/project-supplied key if actually needed (never guessed), and
report raw bytes or refusal without presenting an inferred model as fact.
This does not supply affected-storage coverage or lift the K6 write gate.

**[V] Alternate K6 candidate presence, 2026-10-01 16:55 CEST (read-only
hardware).** After the operator reported `1.1.67` unavailable and requested
another pushbutton, the project-labelled `1.1.32` was selected as a *candidate*
within the pre-approved `1.1.24`–`1.1.32` read range. A production CLI
`knx bus scan` dry run named exactly one candidate (`1.1.32`). Immediately
before the live command, no *local* KNX process or UDP socket for the gateway
was observed. One tunnel to the previously confirmed gateway probed only
`1.1.32`; the reviewed CLI path sends `T_Connect`, DD0 read and
`T_Disconnect` (no independent wire capture). The CLI reported
`occupied (mask 0x0701)` in 199 ms, summary `1 probed / 1 occupied`, exit 0.
No access key, property read, programming button, write or other address was
used. This is **current address occupancy and mask only**: it does not prove
the ETS project label `BE-TA55P8.01`, manufacturer, order number, serial,
physical accessibility or a durable recovery image. The older twofold
`.G2` manual cannot be transferred to this eightfold candidate. A write to
`1.1.32` needs its own device-specific recovery proof and fresh go; the prior
`1.1.67` authorization does not follow the candidate.

**[V] Separate bounded identity probe and offline cross-check, 2026-10-01.**
With exactly `KNX_IDENTIFY_ADDRESS=1.1.32` and the IP-only discovered tunnelling
gateway, the explicitly ignored `knx-net` `live_identify` test used
`ManagementSession::read_only` and `AuthorisationPlan::Skip`; it passed **1/1**
in 0.61s, disconnected cleanly, and again read mask `0701h`. Device Object
`PID_MANUFACTURER_ID` and `PID_HARDWARE_TYPE` each returned octets;
`PID_PROGRAM_VERSION` answered **no elements** (a refusal, not an empty
version). No key, write, other address or programming-mode broadcast was
involved. The two-octet manufacturer value agrees with MDT's `M-0083`
master-data entry documented in §8.8.5. A bounded in-memory inspection of
the local `MDT_KP_BE_01_Push_Button_V15a.knxprod` found that the six-octet
hardware type differs from the historical twofold target but occurs as a
**prefix** in two of eight Device Object (`ObjIdx=0`, `PropId=78`)
`LdCtrlCompareProp` records: `A-0020-15-7F81` and `A-0024-15-6E79`. Each
record has **ten** octets; a six-octet prefix match is neither a successful
full comparison nor proof of the installed product/order/application or
affected storage. The project names the latter program, which is evidence of
intent, not the installed image. Raw endpoint and property bytes were not
copied into Git. The operator replied “go” to the candidate-suitability
question; an exact write plan, complete durable recovery and operation-specific
write confirmation remain absent. See the scoped `.ai` read-only log.

**[I] K6 recovery input still missing.** Before implementing a backup-based
reopening, obtain the actual target's product/application identification and
authoritative affected-storage mapping, including non-memory state and
readability/access restrictions. Prove exact per-device coverage and persist
and read back all pre-write bytes/properties before any send; a Device Reader
file with blank/unread fields or a generic memory range cannot qualify. The
older *source-only* check did not contact a gateway; the later read-only probes
above did, but grant no write authorization. No access key may be guessed.

---

<!-- END preserved record-2-033 -->
