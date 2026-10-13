# 8.7.2 The Load State Machine transition table exists and is complete

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="872-the-load-state-machine-transition-table-exists-and-is-complete"></a>
<!-- BEGIN preserved record-1-011 -->
#### 8.7.2 The Load State Machine transition table exists and is complete

§8.4 Q2 had the states and the events but not the transitions. `03_05_01
Resources` **Table 94 – Load State Machine transition table** (PDF page 296)
gives all 6 states × 5 load events plus a `Device Restart` row, with the legend
*"I: intermediate state / M: mandatory / O: optional / R: recommended"*. It is
reproduced in full in the design spec §5.4. **[D]**

Four facts fall out of it that the implementation is built on:

- **`Error` is a trap.** Every load event except `Unload (04h)` leaves the state
  at `Error`. A retry loop that re-sends `Start Loading` after an error never
  terminates.
- **`Start Loading` from `Loaded` goes straight to `Loading`**, and `03_05_01`
  Table 92 says *"Only in this state [Loaded] the associated data shall be
  considered as valid; in all other states the data shall be considered as
  invalid"*. So beginning a download invalidates a working configuration before
  any payload is sent. There is no stage-then-commit semantics anywhere in the
  procedures read.
- **`Device Restart` while `Loading` is `R: Loading` / `O: Error`**, and
  `Device Restart` from `Loaded` is *"Loaded (Error in case of error detection at
  start-up)"*. Combined with §4.23.1's *"The state of the Load State Machine
  shall be stored in non-volatile memory"*, this answers the interrupted-download
  question outright: an interrupted download leaves a persistently invalid part,
  and power-cycling does not fix it.
- **A device may legitimately not answer at all** in state `LoadCompleting` —
  Table 94's footnote: *"a device may be offline during state LoadCompleting and
  therefore not react to load events from a MaC"*. Treating that silence as
  failure and retrying drives a correct device into `Error` (`R: Error` from
  `LoadCompleting`).


<!-- END preserved record-1-011 -->
