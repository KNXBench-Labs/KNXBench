# T17 — the line scan, implemented

Closes **T17** in [GAP_ANALYSIS_ETS.md](../../GAP_ANALYSIS_ETS.md)'s Tier 4
and, with it, **E2**. The research is done and is not reopened here:
[RESEARCH.md §8.5](../../research/knxnet-ip-and-bus.md#85-line-scan--bus-side-device-discovery--t17-spike-2026-09-12-shipped-2026-09-13)
names the procedure (`NM_IndividualAddress_Check`, `03_05_02 Management
Procedures v02.01.02 AS.md` §2.19), rules out the two services that look
plausible and are not, and carries a full-line measurement. What is left is
implementation.

## Context

A KNXnet/IP gateway tells you what gateways exist. It does not tell you what
devices sit on the line behind it. `knx-net` today speaks group
communication — `TunnelClient::send` builds an `L_Data.req` whose
application service is one of the three `A_GroupValue_*` — and every other
APCI decodes into `ApplicationService::Other { apci, data }`, preserved but
uninterpreted. The Transport Layer is not modelled at all: the TPCI octet's
upper bits are folded into `Other`'s `apci` field on the way in and are only
ever written as `0` on the way out.

`NM_IndividualAddress_Check` is connection-oriented. It needs `T_Connect`,
`T_Data_Connected` with a sequence number, `T_ACK`, and `T_Disconnect` —
none of which exist here yet. That is the bulk of this plan; the scan itself
is a loop around them.

**Presence is decided at Layer 2, not by the application.** A device that
owns the probed address acknowledges the `T_Connect` frame whether or not it
answers anything afterwards. Three outcomes, all **[D]** to §2.19 and quoted
in RESEARCH.md §8.5: an accepted connection answering
`A_DeviceDescriptor_Read` is **occupied**; a `T_Disconnect` with no
descriptor response is **occupied and busy**, never absent; silence is
**vacant**. A fourth case is documented and unresolvable — a device that
cannot meet TP1's 100 ms BUSY budget (`03_02_02 Communication Medium TP1
v01.03.03 AS.md` §2.4.2) is indistinguishable from an absent one, by the
Standard's own text.

## Bus safety — binding, and not negotiable

The user's instruction is carried **verbatim** into every dispatch that
touches the bus. It is not reproduced here, and the redaction is deliberate:
the instruction names a device type at a specific address and encloses the
list of addresses the user confirmed are safe to read — which is an
occupied-address list by another name, and Global Constraint 5 keeps those
out of committed text. The controller holds the verbatim wording and passes
it on; this file carries only what a reader of the plan needs.

What it amounts to, after the user widened it twice:

> **1.1.220 must never be read or written.** It is a safety-critical device
> that reacts to being addressed at all. Every other address on the line may
> be probed.

1.1.220 is named because the user put it in the instruction themselves and
because the exclusion is useless if the code cannot say what it excludes.
It is the only address this repository names.

**No subagent on this plan runs a live scan.** Live-bus validation is the
controller's alone. Every task here is implemented and tested against
in-process fakes; the hardware appears exactly once, in the controller's own
validation step between Task 4 and Task 5.

## Global Constraints

1. **Exclusions hold by construction.** An excluded address is never
   generated into the candidate list, never probed and then filtered, never
   removed from results afterwards. `ScanPlan` asserts its own invariant
   before a scan may start and returns an error — not a panic in library
   code — if an excluded address survived into the range. Anything that
   builds a probe list from raw integers instead of from a `ScanPlan` is a
   defect, not a shortcut.
2. **No behaviour change to group communication.** `decode_l_data` and
   `encode_l_data` keep round-tripping every existing fixture byte for byte.
   The three `A_GroupValue_*` services are unnumbered data (TPCI `0x00`) and
   must still encode to exactly the octets they encode to today. Every
   existing `knx-net` test passes untouched; if one needs editing, stop and
   report, because that means the wire format moved.
3. **Nothing is silently discarded.** A TPCI or APCI this code does not
   interpret keeps landing in a variant that preserves the raw octets and
   the data, the way `ApplicationService::Other` already does.
4. **Evidence markers.** `[D]` documented in the KNX Standard with exact
   file and section, `[V]` locally verified, `[A]` assumption. No promotion
   between them. **Corrected 2026-09-13 by the PDF survey.** This constraint
   used to read "the timeout policy in Task 3 is `[A]` … the Standard does not
   supply a client connection timeout". The Standard does supply one:
   `03_03_04 Transport Layer v01.02.03 AS`, clause 4 "Parameters of Transport
   Layer", page 16 of 38, `connection timeout: time interval of 6 s`. So
   Task 3's `response_timeout` default of 6000 ms is `[D]`; its
   `inter_probe_pause` of 100 ms, its `vacant_confirmations`, and the fast
   preset stay `[A]` resting on `[V]` measurements, and none of those three may
   be written as `[D]`.
5. **The repository is public.** No device names, no manufacturer inventory,
   no occupied-address lists, nothing identifying the maintainer's
   installation may reach committed text. Counts and distributions are
   fine; addresses are not. The one address named anywhere in this
   repository is 1.1.220, and only as the exclusion example, because the
   user put it in the instruction themselves.
6. Every gate green before a task is called done: `cargo fmt --all
   --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo test --workspace --no-fail-fast` (sum all 77 `test result:`
   lines — never read a tail), `cargo run -p xtask -- check-layering`,
   `cargo run -p xtask -- check-headers`, `cargo deny check`, and from
   `apps/knx-web`: `npm test -- --run` and `./node_modules/.bin/tsc
   --noEmit`. Baseline on `main` (873c37e): **1092 passed / 0 failed / 3
   ignored** across 77 lines; npm **340 passed across 31 files**; headers
   **74 well-formed / 169 without a header at ceiling 169 / 22 generated
   skipped**.

## Task 1 — `ScanPlan`: the address range, with the exclusions built in

`crates/knx-core/src/scan.rs`, new module, plus its `lib.rs` line. No I/O,
no dependencies beyond what `knx-core` already has — this is pure domain and
must be testable without a bus.

```rust
pub struct ScanPlan { /* private */ }

pub struct ScanPlanBuilder { /* private */ }

pub enum ScanPlanError {
    /// An excluded address reached the candidate list. This is the
    /// assertion Global Constraint 1 demands, as a value.
    ExcludedAddressInRange(IndividualAddress),
    EmptyRange,
    /// `first > last`, or the two are not on the same line.
    RangeNotOnOneLine,
}
```

`ScanPlan::line(area, line)` covers device addresses 1..=255 on that line —
**not** 0, which is the line coupler's own address
(`03_05_01 Resources v02.01.05 AS.md`, cited exactly by the implementer
after checking; if the citation does not hold up, say so and make it `[A]`
with the reasoning instead of inventing a section number). `ScanPlan::range`
takes an explicit first/last on one line. Both take an exclusion set.

The exclusion set is applied **while the candidate vector is built**, not
after. `ScanPlan::addresses()` returns the candidates; `ScanPlan::verify()`
walks them once and returns `Err(ExcludedAddressInRange(_))` if any excluded
address is present. Task 3's scan calls `verify()` before its first probe
and refuses to start on `Err`.

Tests: a line plan omits every excluded address and contains every other;
excluding the whole range yields `EmptyRange`; `verify()` on a
hand-constructed plan with a smuggled-in address returns the error rather
than panicking; device 0 is never a candidate; a range spanning two lines is
rejected. One test must name 1.1.220 specifically and prove it cannot be
probed when excluded — that is the user's rule, and it deserves a test that
fails loudly if someone ever reorders the filter.

## Task 2 — the Transport Layer on the wire

`crates/knx-net/src/cemi.rs` only (plus its tests).

**(a) TPCI.** `LDataFrame` gains a `transport: Tpci` field:

```rust
pub enum Tpci {
    /// TPCI 0b00xxxxxx — what every group service uses today.
    UnnumberedData,
    /// TPCI 0b01ssssxx, `seq` 0-15.
    NumberedData { seq: u8 },
    /// TPCI 0b10000000.
    Connect,
    /// TPCI 0b10000001.
    Disconnect,
    /// TPCI 0b11ssss10.
    Ack { seq: u8 },
    /// TPCI 0b11ssss11.
    Nak { seq: u8 },
}
```

Bit layout **[D]**, and the citation is already made: the source is
`03_03_04 Transport Layer v01.02.03 AS`, clause 2 "TPDU", **Figure 3 —
Transport Control Field** on page 6 of 38. The Markdown extraction of that
document destroyed the figure (it is a bit table, and only its caption
survives near line 190), so the citation is to the original PDF at
`/mnt/daten-i/Sourcecode/knx-spec-kb/sources/The KNX Standard v3.0.0/03 Volume 3 System Specifications/03_03_04 Transport Layer v01.02.03 AS.pdf`.
The controller read that page on 2026-09-12; the figure gives, in octet 6,
bit 7 as the Data/Control flag and bit 6 as Numbered:

| PDU | Octet 6 | Value |
| --- | --- | --- |
| `T_Data_Broadcast` / `T_Data_Group` / `T_Data_Individual` | `0 0 0 0 0 0 0 0` | `0x00` |
| `T_Data_Tag_Group` | `0 0 0 0 0 1 0 0` | `0x04` |
| `T_Data_Connected` | `0 1 SeqNo SeqNo SeqNo SeqNo 0 0` | `0x40 \| seq << 2` |
| `T_Connect` | `1 0 0 0 0 0 0 0` | `0x80` |
| `T_Disconnect` | `1 0 0 0 0 0 0 1` | `0x81` |
| `T_ACK` | `1 1 SeqNo SeqNo SeqNo SeqNo 1 0` | `0xC2 \| seq << 2` |
| `T_NAK` | `1 1 SeqNo SeqNo SeqNo SeqNo 1 1` | `0xC3 \| seq << 2` |

NOTE 1 under the figure reserves the encoding `BFh`. The sequence number is
four bits, so `seq` is 0-15, and the doc comments above should read
`0b01ssssxx` etc. with that in mind. The implementer uses these values as
given and does not re-derive them; if its own reading of the PDF disagrees,
it reports the disagreement rather than silently picking one.

`Connect`, `Disconnect`, `Ack` and `Nak` carry no application layer at all:
their frames are one octet of TPCI and nothing after it. `ApplicationService`
therefore gains a `None`-shaped variant — name it `NoApplicationPdu` — for
frames whose payload ends at the TPCI octet. Decoding a control PDU must not
demand the second octet the group path requires, which is a real change to
`decode_l_data`'s length arithmetic and the most likely place for a
regression: Global Constraint 2 is what protects you there.

**(b) Device descriptor.** Two new `ApplicationService` variants:

```rust
DeviceDescriptorRead { descriptor_type: u8 },
DeviceDescriptorResponse { descriptor_type: u8, data: Vec<u8> },
```

APCI `0x300 | descriptor_type` and `0x340 | descriptor_type` respectively —
i.e. `0b1100000000` and `0b1101000000`, with the descriptor type in the low
six bits. Corrected on 2026-09-12 by the controller: the plan first wrote the
response as `0b1100010000` and cited §3.1.6, and both were wrong.
`03_03_07 Application Layer v02.01.01 AS.md:324` Table 1 lists
`0 0 0 0 0 0 0 0 A_DeviceDescriptor_Read-PDU` and
`0 1 0 0 0 0 0 0 A_DeviceDescriptor_Response-PDU` under the same `1 1`
prefix; the service clause is **§3.4.2.1** (line 2597). `[D]`. These are 10-bit APCIs, unlike the 4-bit
group ones — the existing `short_apci` path handles the group case only and
must keep doing exactly that.

Tests: every `Tpci` variant round-trips encode→decode→encode byte-identical;
a `T_Connect` frame decodes with `NoApplicationPdu` and no length error; an
`A_DeviceDescriptor_Read` on a numbered connection round-trips with its
sequence number intact; a hand-built fixture of a real
`A_DeviceDescriptor_Response` decodes to the right descriptor type and data;
an unknown 10-bit APCI still lands in `Other` with its octets intact.

## Task 3 — one probe, and the timeout policy that decides what a scan costs

`crates/knx-net/src/scan.rs`, new module, plus `lib.rs`. Depends on Task 1's
`ScanPlan` and Task 2's `Tpci`.

```rust
pub struct ProbePolicy {
    pub response_timeout: Duration,
    pub vacant_confirmations: u8,
    pub inter_probe_pause: Duration,
}

pub enum ProbeOutcome {
    Occupied { mask_version: Option<u16> },
    OccupiedBusy,
    Vacant,
    /// The scanner's own tunnelling connection. Never probed.
    SelfAddress,
}
```

**The policy ruling, made here so the implementer does not have to guess.**
**Rewritten 2026-09-12 after a PDF survey overturned its premise.** The first
version of this ruling rested on RESEARCH.md §8.5 Finding 1, which attributed
the 6 s cost of "nobody answered" to `xknx`'s own
`MANAGAMENT_CONNECTION_TIMEOUT = 6` — "a client policy constant, not a bus
fact". That is wrong, and the correction is `[D]`:
`03_03_04 Transport Layer v01.02.03 AS`, clause **4 "Parameters of Transport
Layer"**, page **16 of 38** (text survives extraction at
`03_03_04 Transport Layer v01.02.03 AS.md:665-690`) defines

> connection timeout: time interval of 6 s; timeout to breakdown a connection
> acknowledgement timeout: time interval of 3 s; timer to start a repetition
> if no acknowledgement was received
> max_rep_count: 3; maximum of T_Connect.req repetitions

and the event/action legend a few pages on (`:760-779`) names
`connection_timeout_timer` and `acknowledgment_timeout_timer` as two separate
local timers, not a pass-through of the Data Link Layer's ACK cycle. `xknx`
did not invent 6 s; it implemented the Standard.

**Ruling:** `response_timeout` defaults to **6000 ms** — the Standard's own
`connection_timeout`, `[D]` — with `vacant_confirmations: 1` and
`inter_probe_pause` **100 ms** `[A]`. A full line costs about **23 minutes**,
which is what the controller measured, and the measurement is now explained
rather than merely observed.

**Why the fast default was dropped.** The same `[V]` measurement that
motivated 1000 ms also refutes it: occupied round trips ranged
**13.6-6016.5 ms**. The median was 121 ms, but the maximum was a device that
answered only after just over six seconds. A 1000 ms timeout reports that
device **vacant**, and two confirmations at 1000 ms do not rescue a device
that is consistently slow — they just ask the same too-short question twice.
That is a present device silently dropped from the result, which
Global Constraint 3 forbids and which CLAUDE.md's priority order
(Correctness → Data Integrity → … → Performance) settles the same way.

**The fast path stays available, and stays honest.**
`--timeout-ms 1000 --confirmations 2` costs about 2.1 s per vacant address and
finishes a line in roughly **9 minutes**. The CLI documents it with the
measurement above, in as many words: on the installation this was measured on,
at least one present device answered after 6016.5 ms and would be reported
vacant by that setting. A scan run below the Standard's `connection_timeout`
reports "no answer within N ms", which is not the same claim as "absent", and
the report must not blur the two.

**Cost if wrong:** the default scan takes 23 minutes instead of 9. Slow, and
visible in the first second of use; the alternative failure is a missing
device nobody notices. `response_timeout`'s default is `[D]`; the 100 ms
pause and the fast preset are `[A]` resting on `[V]` measurements, and all
three must be written as such.

**The self-address rule.** `TunnelClient::assigned_address()` already
returns the address the gateway assigned this connection during setup — no
heuristic needed, per RESEARCH.md §8.5 Finding 2. The scan reports it as
`SelfAddress` and never probes it. Do **not** implement the sub-20 ms
"that's another tunnelling endpoint" heuristic: it is `[A]` on three samples
from one gateway, and a wrong guess here silently deletes a real device from
the results. Note it in the docs as a known limitation instead.

`probe_address(tunnel, addr, policy)` runs §2.19: `T_Connect` →
`A_DeviceDescriptor_Read(0)` as `T_Data_Connected` → read the answer →
`T_Disconnect`, best-effort, no confirmation expected
(`03_03_04 Transport Layer v01.02.03 AS.md` §3.8). A received `T_Disconnect`
with no descriptor response is `OccupiedBusy`, **not** `Vacant` — §2.19's own
possible-reaction list says so, and getting this backwards is the single
most damaging bug this task can ship, because it reports live devices as
absent.

`scan_line(tunnel, plan, policy, progress)` calls `plan.verify()` first and
returns its error unrun, iterates sequentially (the tunnelling protocol
allows one outstanding request per direction — `Tunnelling v01.07.01 AS`
§2.6, already cited in `client.rs`), and reports progress through a callback
so a caller can show something during nine minutes of scanning.

Tests use an in-process fake, not hardware: a fake tunnel that answers
scripted outcomes per address. Cover — occupied with a descriptor; busy
(disconnect, no response); vacant (silence, and assert the confirmation pass
really probed twice); self-address skipped without a single frame sent; a
plan whose `verify()` fails aborts before any frame is sent. That last one is
the bus-safety test and it is not optional.

## Task 4 — `knx bus scan`

`apps/knx-cli/src/main.rs`, and `USAGE`.

```
knx bus scan --gateway <host:port> --line <area.line>
             [--range <first>-<last>] [--exclude <addr>[,<addr>...]]
             [--timeout-ms <n>] [--pause-ms <n>] [--project <path.knxdb>]
```

Output: one line per probed address as it completes (address, outcome, round
trip in ms, mask version where known), then a summary — counts of occupied,
busy, vacant, excluded, plus the elapsed total and the policy actually used,
because a scan result without its timeout policy is not interpretable.

`--project` turns on the comparison **E2** is actually about: addresses the
bus answered that the project does not list, and addresses the project lists
that did not answer. Counts and a table, printed, not written back into the
project — this task discovers, it does not reconcile.

`--exclude` parses into Task 1's exclusion set and nowhere else. Refuse to
start on a malformed address rather than silently dropping it from the
exclusions: a typo in an exclusion list is exactly the failure the user's
1.1.220 rule exists to prevent, and "I ignored the argument I could not
parse" is the wrong answer to it.

Tests: argument parsing, including a malformed `--exclude` and a `--range`
spanning two lines; the summary formatter against a fixed result set; the
project-comparison logic against a fixture project and a fixed scan result.
No test in this task opens a socket.

## The controller's live validation — between Task 4 and Task 5

Not a subagent task. The controller runs `knx bus scan` against
**192.0.2.1**, on a **small approved subrange only** — nine consecutive
addresses from the user's own list, which stays out of this file — with
**1.1.220 excluded** and the exclusion verified in the emitted
plan before the first frame leaves the machine. The result — counts,
timings, and whether the policy ruling holds up against a real gateway —
goes into Task 5's docs as `[V]`, with no addresses named.

## Task 5 — the documents

`docs/` only.

`GAP_ANALYSIS_ETS.md`'s **T17** entry moves to `Done`, records what shipped
against what §8.5 specified, and states the timeout policy with its `[A]`
marker and the live-validation result as `[V]`. **E2**'s row changes from a
gap to a partially-closed one: the scan reports the disagreement, nothing
reconciles it yet.

`KNOWN_LIMITATIONS.md` gains a new section for what a scan cannot learn,
drawn from §8.5 and not re-derived: no product identity, no manufacturer, no
serial number without a separate `A_PropertyValue_Read` per occupied
address; the TP1 100 ms BUSY budget case that is indistinguishable from
absence; other tunnelling endpoints reported as devices because the
detection heuristic was deliberately not implemented; one line at a time, no
cross-line scanning through couplers.

**Added 2026-09-12 after the PDF gap survey:** `RESEARCH.md` §8.5 also has a
correction to make, and it is not cosmetic. Finding 1 currently frames the 6 s
cost of an unanswered probe as "a client policy constant KNXBench will choose
for itself", attributing it to `xknx`'s `MANAGAMENT_CONNECTION_TIMEOUT = 6`.
That is wrong: `[D]` `03_03_04 Transport Layer v01.02.03 AS`, clause 4
"Parameters of Transport Layer", page 16 of 38 — text preserved in the
extraction at `03_03_04 Transport Layer v01.02.03 AS.md:665-690` — defines
connection timeout **6 s**, acknowledgement timeout **3 s** and
max_rep_count **3** as Transport Layer parameters, and the event/action legend
at `:760-779` names `connection_timeout_timer` and
`acknowledgment_timeout_timer` as two separate local timers rather than a
pass-through of the Data Link Layer's ACK cycle. `xknx` implemented the
Standard. The same section's "whether the Transport Layer adds its own
timeout logic … is not fully pinned down from this corpus" sentence is
answered by the same clause and must go. Cite the clause and page.

While there, record why the fast preset is not the default: the `[V]`
measurement's occupied round trips ran 13.6-**6016.5** ms, so a 1000 ms probe
reports the slowest present device vacant, and repeating a too-short question
does not make it a better question.

`IMPLEMENTATION_STATUS.md` gets its dated entry. `RESEARCH.md` §8.5 gains
one paragraph — the spike said "nothing described here is implemented",
which stops being true.

## Out of scope

* Reconciling the bus against the project (writing discovered devices into
  the project file). That is its own task with its own destructive-edit
  questions.
* Product/manufacturer identity per `A_PropertyValue_Read`. §8.5 names it as
  a separate step and T16's work, and it stays there.
* Scanning across couplers, or scanning more than one line per invocation.
* The sub-20 ms tunnelling-endpoint heuristic — deliberately not
  implemented, see Task 3.
* Any concurrency: probes are sequential, one outstanding request per
  tunnelling connection.
