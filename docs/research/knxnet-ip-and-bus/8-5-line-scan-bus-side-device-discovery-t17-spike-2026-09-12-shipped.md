# 8.5 Line-scan / bus-side device discovery — T17 spike (2026-09-12), shipped (2026-09-13)

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="85-line-scan--bus-side-device-discovery--t17-spike-2026-09-12-shipped-2026-09-13"></a>
<!-- BEGIN preserved record-1-007 -->
### 8.5 Line-scan / bus-side device discovery — T17 spike (2026-09-12), shipped (2026-09-13)

**This spike documents a procedure. Nothing described here is implemented,
and the measurements in this section were taken by the controller against
their own installation, not by this repository's code.**
[GAP_ANALYSIS_ETS.md](https://github.com/KNXBench-Labs/KNXBench/blob/994ac08c0328/docs/GAP_ANALYSIS_ETS.md) row **T17** guessed that a line
scan would be built on "individual-address serial-number read services".
That guess was wrong, and this spike exists to correct it and to name the
service the Standard actually defines for this purpose. It extends §8.4's
Q1 finding rather than contradicting it: §8.4 already established that
`NM_IndividualAddress_Write` verifies a freshly-written address by opening
a connection and issuing `A_DeviceDescriptor_Read` — "the standard way to
address-and-probe a specific IA." T17's procedure is the same technique
turned around: instead of confirming an address you just wrote, you probe
an address whose occupant is unknown.

**Status (2026-09-13): implemented.** The disclaimer directly above was
accurate when it was written and is kept as written, not edited into a
retroactive lie — but it stopped being true on 2026-09-13. T17 shipped:
`ScanPlan` (`crates/knx-core/src/scan.rs`, exclusions dropped while the
candidate range is built, with an assertion); `Tpci` encode/decode and
the device-descriptor APCIs (`crates/knx-net/src/cemi.rs`); `ProbePolicy`,
`ProbeOutcome`, `probe_address`, `scan_line`
(`crates/knx-net/src/scan.rs`); and `knx bus scan`
(`apps/knx-cli/src/scan.rs`, `apps/knx-cli/src/main.rs`). See
[GAP_ANALYSIS_ETS.md](https://github.com/KNXBench-Labs/KNXBench/blob/994ac08c0328/docs/GAP_ANALYSIS_ETS.md)'s **T17** entry for what
shipped measured against what this section specified. Finding 4, appended
at the end of this subsection, adds what a live run of the shipped binary
against real hardware confirmed and narrowed; everything else below this
point is the spike's own pre-implementation procedure and reasoning,
corrected in place where it was wrong (see Finding 1) rather than
silently left to mislead a future reader.

The disclaimer's second clause — that the measurements in this section
were taken by the controller against their own installation, not by this
repository's code — is likewise only half true from here on: it still
describes every pre-implementation measurement below (the 2026-09-12
`xknx` run and Findings 1-3), but not Finding 4, whose measurements are
this repository's own shipped binary against real hardware.

**The correct procedure.** **[D]** `03_05_02 Management Procedures
v02.01.02 AS.md` §2.19 defines `NM_IndividualAddress_Check`, with a note
naming its colloquial alias directly: *"NOTE This procedure has also been
named NM_IndividualAddress_Scan."* Its purpose, quoted: *"This Network
Management Procedure shall be used by a network Management Client to check
whether a given Individual Address is occupied on the network or not."*
The sequence per candidate address `IA_test`, all **[D]** to the same
clause unless noted: (1) `T_Connect.req` (via `A_Connect`) addressed to
`IA_test`; (2) if a connection is accepted, `A_DeviceDescriptor_Read.req`
with `descriptor_type = 0` sent as `T_Data_Connected`
(`03_03_07 Application Layer v02.01.01 AS.md` §2.2, Table 1); (3) `A_Disconnect`
to close the connection, best-effort, no confirmation required
(`03_03_04 Transport Layer v01.02.03 AS.md` §3.8). Presence is decided at
step (1)/(2), not by anything the application layer says — see below.

**Why the two services the Gap Analysis and this spike both checked are the
wrong tools.** Both were worth ruling out explicitly rather than assuming;
both are wrong for this exact reason:

* `A_IndividualAddress_Read`/`_Response` — **[D]**
  `03_03_07 Application Layer v02.01.01 AS.md` §3.2.3. Sent as a system
  broadcast, addressless, and answered only by a device currently in
  *programming mode* (its programming button held/pressed). This is the
  commissioning-time "which device is in programming mode" check behind
  `NM_IndividualAddress_Read`/`_Write` (§8.4 Q1), not a way to ask "is
  address X occupied" for an arbitrary, already-commissioned address.
* `A_IndividualAddressSerialNumber_Read`/`_Response` — **[D]**
  `03_03_07 Application Layer v02.01.01 AS.md` §3.2.4. This is the
  Gap Analysis's guess, and it runs the wrong direction: it is addressed by
  a 6-octet serial number you must already know, sent as a system
  broadcast, and answered by whichever device holds that serial number
  with its current individual address and domain address. It answers "what
  address does this known device have," never "is this address occupied."
  It cannot be repurposed for a line scan without already knowing the
  serial number of every device you are trying to discover — which defeats
  the point of scanning.

**Absent vs. occupied: what the Standard resolves, and the one case it
does not.** **[D]** Presence is signalled at the Data Link Layer, not the
application layer: a device recognizing its own individual address as
destination must acknowledge at Layer 2, independent of whether any higher
layer does anything with the frame. `NM_IndividualAddress_Check` §2.19
states the mechanism outright: *"If a device that occupies IA_test is
present on the network, and does support Transport Layer connections, it
shall have no other reaction on the bus than the Layer-2 acknowledge that
initiates the above A_Connect.Lcon."* Three outcomes follow, all **[D]**:

* **No device present**: no ACK, no NAK, no BUSY at all; the sender times
  out the acknowledge window and retries per its own retry budget, then
  concludes not occupied.
* **Device present and busy with another connection**: it does not fall
  silent. Per `03_03_04 Transport Layer v01.02.03 AS.md` §3.7, *"If the
  remote Transport Layer receives … a T_CONNECT_REQ_PDU and does not allow
  for building up a new connection, the frame shall not be passed to the
  remote Transport Layer user. Instead, the remote Transport Layer shall
  send a T_DISCONNECT_REQ_PDU."* `NM_IndividualAddress_Check`'s own
  possible-reaction list treats "Disconnect received, no
  DeviceDescriptor response" as **occupied**, never absent — the
  present-but-busy safeguard the Standard requires is built into the named
  procedure, not left to the implementer to get right.
* **Device present but unable to manage even a BUSY response within its
  own timing budget**: `03_02_02 Communication Medium TP1 v01.03.03 AS.md`
  §2.4.2 requires a device to send BUSY only if it "will again be able to
  process Frames that starts 100 ms after the reception," and explicitly
  forbids sending BUSY otherwise. A device that cannot meet that 100 ms
  budget is, by the Standard's own text, indistinguishable at Layer 2 from
  an absent one. This is a documented limit of the mechanism itself, not a
  gap in this spike's reading, and no scanner built on
  `NM_IndividualAddress_Check` alone can resolve it.

One further limitation this subsection used to carry is resolved here
rather than left hanging: `03_03_04 Transport Layer v01.02.03 AS.md`
§5.5.1.5 ("Connect from the local User to a non-existing Device") and
§5.5.1.9 ("Connection timeout") are both figure-only in this corpus's
Markdown extraction — the sequence diagrams did not survive extraction
and no surrounding prose substitutes for them. But whether the Transport
Layer adds its own independent timeout logic on top of the Data Link
Layer's ACK/retry cycle does not actually hinge on those two blank
figures: clause 4, "Parameters of Transport Layer"
(`03_03_04 Transport Layer v01.02.03 AS.md:665-688`, PDF page 16 of 38)
answers it directly, **[D]**. It fixes connection timeout at 6 s,
acknowledgement timeout at 3 s and max_rep_count at 3 as Transport Layer
parameters, not Data Link Layer ones, and clause 5's "Local variables of
Transport Layer" table (`03_03_04 Transport Layer v01.02.03 AS.md:696-705`,
PDF page 17 of 38, specifically lines 702-703) names two independent
local timers, `connection_timeout_timer` and `acknowledgment_timeout_timer`,
started and stopped by separate actions in the clause's Actions table
(page 19 of 38 onward). The Transport Layer keeps its own clock; it is
not a pure pass-through of Layer 2's ACK/retry cycle. §3.7's prose had it
right; it just did not need the hedge.

**Bus-load budget.** This subsection originally carried an **[A]**
extrapolation for the absent-address case and the whole-line duration,
scaled from nine occupied-address samples — no vacant address was
available to probe at the time. The controller has since run a full line
scan, and the measurement below replaces both **[A]** figures with **[V]**
ones. The arithmetic estimate is kept alongside it, not deleted, because
it is still useful evidence of what the Standard's documented constants
alone predict — see Finding 1 for why that prediction and the measurement
disagree by two orders of magnitude, and why the disagreement is not an
error:

* **[A]**, built from **[D]** inputs (TP1 9600 bit/s per
  `03_02_02 Communication Medium TP1 v01.03.03 AS.md:151`; minimum
  `L_Data_Standard` frame 8 octets; 13-bit-time character slots; 50-bit-time
  idle; 15-bit-time-plus-30µs acknowledge timeout; All TP1 Profiles'
  optional `nak_retry = busy_retry = 3` per
  `06 Profiles v02.01.01.md:775`; all same clauses as §8.4's style of
  citation): one minimal message cycle is roughly 17.6 ms; an absent
  address costs roughly 70 ms (retries exhausted, no ACK ever); an
  occupied, responsive address costs a similar order of magnitude, roughly
  60-100 ms, dominated by the round trip for the actual
  `A_DeviceDescriptor_Response`. This models the KNX medium's own retry
  budget only — it does not, and cannot, model a KNXnet/IP client's own
  connection-timeout policy, which is what the measurement below actually
  hit.
* **[V]**, full scan of area 1 / line 1, one installation, one gateway,
  2026-09-12, via `NM_IndividualAddress_Check` (`xknx`'s
  `nm_individual_address_check`, the same §2.19 procedure cited above)
  over a KNXnet/IP tunnelling connection, sequential, 200 ms pause between
  probes. **254 addresses probed** — the whole line except one address
  excluded by construction (see the exclusion-list paragraph below) —
  with **zero probe errors**: 35 reported occupied, 219 reported vacant.
  Occupied-probe round trips ranged 13.6-6016.5 ms (median 121.1 ms);
  vacant-probe round trips ranged 6275.9-6323.8 ms (median 6279.9 ms).
  Summed probe duration: **1 385.75 s ≈ 23.1 minutes**. One installation,
  one gateway, one client implementation — this stays project-local
  **[V]**, never promoted to a Standard-normative figure, and none of
  these numbers may be written as one either.

  **Finding 1 — the absent-address cost is set by the Standard, not
  invented by a client.** The vacant-probe figures above are the tell:
  **~6.28 s** median, with under 50 ms of spread across 219 samples, is
  far too tight to be bus retry behaviour and far too slow to be a KNX
  medium timing at all. It is a Transport Layer timeout expiring on
  schedule, and the Standard names that schedule directly: **[D]**
  `03_03_04 Transport Layer v01.02.03 AS`, clause 4 "Parameters of
  Transport Layer" (`03_03_04 Transport Layer v01.02.03 AS.md:665-688`,
  PDF page 16 of 38) fixes connection timeout at 6 s, acknowledgement
  timeout at 3 s and max_rep_count at 3; clause 5's Local Variables table
  (`:696-705`, PDF page 17 of 38, specifically lines 702-703) names the
  two local timers — `connection_timeout_timer` and
  `acknowledgment_timeout_timer` — that implement them. The measured
  ~6.28 s tracks the 6 s connection timeout plus transport/IP overhead
  almost exactly, not the ~70 ms the **[A]** bus arithmetic above
  predicted, because the two figures measure different layers: the
  arithmetic modelled TP1's own retry budget; the measurement is the
  Transport Layer's connection-teardown clock running out.

  An earlier draft of this finding attributed the ~6 s figure to a
  client library's own policy constant — `xknx`'s
  `MANAGAMENT_CONNECTION_TIMEOUT`
  (`xknx/management/management.py:35-36`, the copy installed in this
  project's `.venv`) — as if KNXBench were free to pick something
  different for no particular reason. That was wrong and is corrected
  here: `xknx` did not invent 6 s, it implemented the Standard's own
  clause-4 parameter. Any conforming implementation of
  `NM_IndividualAddress_Check`, KNXBench's own included, pays ~6 s
  wherever it waits out a conforming Transport Layer connection timeout.

  The consequence is the design conclusion this whole subsection was
  building toward, corrected: **the scan's duration is dominated by the
  Standard's own Transport Layer connection timeout**, not a policy
  constant invented independently of it. That figure is documented, not
  chosen freely — clause 4 fixes it at 6 s for every conforming
  Transport Layer implementation. The TP1 arithmetic above was not wrong
  about the bus; it modelled a different layer, the medium's own retry
  budget, which is why it landed roughly two orders of magnitude below
  the measured figure. Implementing T17 still meant picking a
  `response_timeout` value for `ProbePolicy`
  (`crates/knx-net/src/scan.rs`), and that choice still governs whether a
  full-line scan finishes in minutes or drags on needlessly — too short,
  and a slow-but-present device is reported vacant (see the fast-preset
  paragraph below); too long, and a mostly-vacant line takes longer than
  it needs to clear. But the choice was made against a documented
  anchor, not blind: `ProbePolicy::default()` sets `response_timeout` to
  6000 ms, matching clause 4's connection timeout exactly, because there
  is no principled reason to wait past the point the Standard itself
  says the far end has given up. A shorter timeout remains a supported,
  explicit option (`bus scan --timeout-ms`) for lines known to run
  entirely fast, present devices — a bet on the installation, not a
  correction to the Standard's own number.

  **Why the fast preset is not the default.** The **[V]** measurement
  above is the reason a short `--timeout-ms` is not shipped as the
  default: occupied-address round trips in the very same run ranged
  **13.6-6016.5 ms**. A 1000 ms timeout — plausible-looking, since most
  occupied addresses answered in tens of milliseconds — would have
  reported the slowest observed present device, at 6016.5 ms, as vacant.
  Repeating a too-short question does not make it a better question:
  shortening the timeout does not distinguish a slow device from an
  absent one, it only moves the point at which the scan starts lying
  about which is which. `bus scan`'s default stays anchored to the
  Standard's own connection timeout for this reason; a faster preset is
  a deliberate, explicit trade a specific installation's operator can
  choose to make (`--timeout-ms`), not KNXBench's default guess.

  The earlier **[A]** 26-57 second full-line extrapolation this
  subsection carried is corrected here rather than silently replaced: it
  scaled from nine addresses that were *all occupied*, reasoning that a
  line looks like those nine. A real line is mostly *vacant*, and vacant
  is the expensive case here, not the cheap one — the earlier
  extrapolation scaled the wrong sample, in the direction that made the
  answer look better than it is. The measured figure, **23.1 minutes**,
  is dominated almost entirely by 219 vacant-address timeouts at ~6.28 s
  each.

  An unthrottled full-line scan is therefore not a multi-second burst as
  first estimated — it is a multi-minute, sustained run of
  connection-oriented traffic on a live line, for as long as the chosen
  timeout policy makes it. It still competes directly with whatever else
  needs that line's bandwidth for that whole window, including devices
  where a delayed response matters, which is exactly why pacing and an
  exclusion list are not optional polish. Any real implementation needs
  an explicit pacing/timeout policy, and an installation-specific
  exclusion list honoured **by construction** — addresses to skip
  enumerated out of the scan range itself, never filtered out afterwards
  — belonging in the domain layer, not the UI, consistent with this
  repository's standing rule against UI workarounds for domain-layer
  problems. This scan exercised exactly that rule for real: one address
  on this installation must never be read (a hazard on any read while a
  condition holds) and was dropped while the address range was built,
  with an assertion that refuses to start if the address survives into
  the list, rather than filtered out of the results afterward. That is
  the shape T17's own exclusion list needs, not a UI checkbox someone can
  forget to tick.

**Finding 2 — a scan sees its own tunnel, and the gateway's.** **[V]**
Three of the 35 addresses this scan reported occupied were not devices on
the twisted pair at all — they were KNXnet/IP tunnelling endpoints
answering from the gateway itself, and their timing gives them away. Two
answered in **13.6 ms** and **14.0 ms**, roughly an order of magnitude
faster than the 100-150 ms a real device on the bus typically needs,
because the answer never left the IP side of the gateway. The third took
**6016.5 ms** and was still reported occupied — the present-but-busy
outcome documented above (`A_Disconnect` received, no
`A_DeviceDescriptor_Response`), not silence.

One of those three was the scanner's own tunnelling connection: asking
the gateway what individual address it had assigned that connection
returned the same address the scan had just reported occupied. A line
scan detects itself, unless it is told not to.

Both facts are implementation requirements for T17, not just
observations. A scan must know its own tunnelling connection's assigned
individual address and exclude or clearly mark it rather than reporting
it as a device — this needs no heuristic, since the gateway hands that
address over during connection setup. A scan should also not present
*other* tunnelling endpoints as bus devices. The sub-20 ms response time
is a usable heuristic for spotting those, but it stays **[A]**: three
samples on one gateway with one client implementation. A genuinely fast
device, or a slower IP path on a different gateway, could break it —
nothing in the Standard promises this gap.


**What a scan does not learn.** **[D]** `A_DeviceDescriptor_Read` with
`descriptor_type = 0` returns DD0, the Mask Version — *"Identification of
an implementation, for operation like download, memory_write … In
particular, the Mask Version is read through a dedicated Application Layer
service by the S-Mode Management Client (ETS) to conclude on the
Configuration Profile of the device and on possible further discovery and
configuration steps."* (`03_01_02 Glossary v01.05.03 AS.md:236`). This
identifies the implementation family/coupler-medium class a device
belongs to, not which product it is, its manufacturer, its application
program, or its serial number — **[D]**
`06_02_01 Coupler Model 2.0 v01.01.01 AS.md` §1.5.2 makes the same point
from the coupler side: many different coupler products deliberately share
one Mask Version. Product/manufacturer identity needs a separate,
additional connection-oriented read after the scan step — e.g.
`A_PropertyValue_Read` on the Device Object (`object_index = 0`),
`PID_SERIAL_NUMBER` (PID 11) — **[D]**
`03_05_03 Configuration Procedures v02.01.01 AS.md:4797` and
`03_06_03 EMI_IMI v01.04.02 AS.md:5074`. This is not part of
`NM_IndividualAddress_Check` itself, and whether a given mask version even
supports Property services (versus only Memory-based access, as some
older masks do) is unverified by this spike — flagged, not resolved. In
short: a scan as specified by §2.19 alone can mark an address
occupied/absent and, where the device answers, its Mask Version. It cannot
by itself populate a topology view with product identity; T16's
device-catalog work would need the extra `A_PropertyValue_Read` step per
occupied address, as a real, visible scope distinction.

**Finding 3 — the bus and the project disagree, and that is the whole
point of E2.** **[V]** On this installation, the reference project's
device list and the bus scan's results did not match: addresses answered
that the project's own records did not account for, and an address the
project lists did not answer at all. Only the counts are recorded here,
deliberately: this repository is public-facing, and no addresses, device
names, or manufacturer inventory belonging to this installation are
written into it.

This mismatch is not a scan defect; it is the reason **E2**
([GAP_ANALYSIS_ETS.md](https://github.com/KNXBench-Labs/KNXBench/blob/994ac08c0328/docs/GAP_ANALYSIS_ETS.md)) is a gap worth closing at
all. An ETS project file is a *plan*; the bus is the *installation*. They
drift apart in ordinary use — devices get added by hand, replaced,
re-addressed, or removed without the project file being updated to
match. A scan that only ever confirmed what the project already claims
would be visible but pointless busywork; the value is entirely in where
the two disagree. The two directions of disagreement mean different
things, and belong in a report as two distinct findings rather than one
"mismatch" bucket: an address the scan finds occupied but the project
does not know about is an **undocumented device**; an address the
project lists that the scan finds vacant is either a **removed device, a
failed one, or one whose individual address changed** — three
possibilities a bus scan alone cannot distinguish between, and which
would need a second signal (serial number, product identity read after
the scan — see above) to tell apart.

**Finding 4 — the shipped `bus scan` against real hardware, 2026-09-13.**
**[V]** The controller ran the finished binary against the same
installation's line, distinct from and later than the 2026-09-12
measurement Finding 1 corrects above (that earlier run predates this
implementation and was produced with `xknx`, not KNXBench). No address is
named here for the same reason as Finding 3: this repository is
public-facing.

A dry run (`bus scan --dry-run`) against the full line produced 254
scannable candidates, and 255 when one existing exclusion was lifted —
consistent with `ScanPlan`'s exclusion-by-construction guarantee
(`crates/knx-core/src/scan.rs`) dropping exactly one address, not zero
and not more than one. The candidate list's first entry was device 1, not
device 0 — consistent with device 0 never being a valid candidate (the
line coupler's own address, `crates/knx-core/src/scan.rs`). A dry run
against an unroutable gateway address returned in 1 ms with exit code 0:
a dry run never opens a connection, so an unreachable gateway cannot fail
it.

Two live runs followed, both with `ProbePolicy::default()`
(`response_timeout` 6000 ms, `vacant_confirmations` 1,
`inter_probe_pause` 100 ms):

* **Run 1**, nine consecutive occupied addresses: every probe returned
  `Occupied`, round trips 84-202 ms, elapsed 1810 ms for the nine, and
  every one of the nine reported the same Mask Version, `0x0701`. That
  last fact is recorded and not generalized: nine addresses on one
  installation sharing a Mask Version says nothing about any other
  installation's device mix, and is not evidence that `0x0701` is common
  or typical.
* **Run 2**, five consecutive vacant addresses: every probe returned
  `Vacant`, each one costing 6006 ms, for an elapsed total of 30431 ms
  against a predicted 30430 ms (five × 6006 ms) — the arithmetic and the
  measurement agree to within 1 ms, which is the expected shape for a
  policy-timeout-dominated cost, not bus jitter. Zero probes in this run
  returned `OccupiedBusy` or `OccupiedSilent`. That absence is a fact
  about these five addresses on this one gateway and this one sample,
  nothing more — it does not show that `OccupiedSilent` cannot occur, or
  that it is rare; the evidence-honesty rule against promoting one
  outcome into another binds this null result exactly as it binds every
  other measurement in this section.

Both runs are consistent with the 2026-09-12 measurement's per-address
costs (occupied fast, vacant ~6 s) and with Finding 1's corrected
explanation of where the ~6 s comes from. Neither run is a substitute for
a full-line measurement repeated on this implementation; none has been
done, and this section does not claim one.

**What exists and what shipped, 2026-09-13.** The KNXnet/IP tunnelling
transport, cEMI `L_Data` encode/decode, point-to-point addressing, and a
raw-APCI escape hatch already existed in `crates/knx-net` before T17 and
are directly reused: `TunnelClient` (`crates/knx-net/src/client.rs`) for
the transport; `decode_l_data`/`encode_l_data`
(`crates/knx-net/src/cemi.rs`) for the frame codec; `IndividualAddress`
(`crates/knx-core/src/address.rs:9-50`) and `Destination::Individual`
(`crates/knx-net/src/cemi.rs`) for point-to-point addressing; and
`ApplicationService::Other` (`crates/knx-net/src/cemi.rs`) for the
raw-APCI escape hatch. `client.rs` and `cemi.rs` are cited by name, not
by line: T17 added code to both files, so any single line range picked
now would drift again as soon as either file changes further. What did
not exist as of
this section's original writing, and now does: `Tpci` encode/decode
(`Connect`, `Disconnect`, numbered `Data_Connected`, `Ack`, `Nak`) and the
device-descriptor Application Layer services
(`crates/knx-net/src/cemi.rs`); a `ProbePolicy`/`ProbeOutcome` probe of
one address implementing `NM_IndividualAddress_Check`'s
Connect → `A_DeviceDescriptor_Read` → Disconnect sequence over exactly
that Tpci support, plus `scan_line` for a range
(`crates/knx-net/src/scan.rs`); `ScanPlan`'s exclusion-by-construction
range builder (`crates/knx-core/src/scan.rs`); and the `knx bus scan` CLI
surface, dry-run and live (`apps/knx-cli/src/scan.rs`,
`apps/knx-cli/src/main.rs`). What this closes: the KNX **bus-level**
Transport Layer connection lifecycle needed to run one
`NM_IndividualAddress_Check` probe now exists and is exercised by tests
and by the live runs above. What it does not close, and was never in
scope for T17: a general-purpose, long-lived Transport Layer connection
manager for other connection-oriented services (memory writes, program
downloads); those would reuse the same `Tpci` variants but need their
own sequencing and lifetime, not this scan's single-probe-then-disconnect
shape. Also unbuilt, and explicitly out of scope: scanning across
couplers or more than one line per invocation, and any concurrency
between probes — see `KNOWN_LIMITATIONS.md` for the durable record of
what a scan still cannot do. Connectionless, unnumbered `T_Data_Group`
traffic — the ordinary group-communication path this crate already
supported before T17 — is untouched by any of this and keeps working the
same way it always did; what changed is additive, a second, connected
mode used only by the scan.

---


<!-- END preserved record-1-007 -->
