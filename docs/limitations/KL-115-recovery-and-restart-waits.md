# Recovery and restart waits

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-115)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 115. `MasterResetResponse::recovery_wait` and `SessionTiming::restart_basic_t1` compute durations nobody waits on yet

**Current status, 2026-10-03.** The blanket title and original account below
are historical, not a description of current callers. The typed
`master_reset()` procedure in `commissioning/master_reset.rs` waits on
`probe.recovery_wait(&timing)` and the erasing answer's recovery wait, then
performs a bounded mask-read retry at the expected address. The
individual-address procedure also uses `restart_basic_t1` for its documented
reconnect wait. These are caller-specific implementations; they do not add a
generic failed-service restart/retry contract or authorize erasing hardware.
The public reset recovery gate remains in force. Retain this heading/anchor
for historical inbound links; the original "no reader" sentences are
superseded by this status.

**Limitation.** `[C15]` `ManagementSession::restart_master_reset`
(`crates/knx-net/src/commissioning.rs`) sends the confirmed Master Reset
and decodes the device's `A_Restart_Response` into a `MasterResetResponse`,
whose `recovery_wait` method correctly computes MP §3.7.1.2.2, p. 81's
floor — the greater of the reported Process Time and this session's own
`SessionTiming::restart_responsive_again` — because *"the process time is
thus a minimal time for the MaC to wait, not a maximal time."* Its sibling
field `SessionTiming::restart_basic_t1` (MP §3.7.1.1.2, p. 79, Figure 19)
is the same shape of value for the *other* restart type: the earliest
point at which a caller may expect a Basic Restart to have already
succeeded. Both are documented on the methods that produce or return them
and neither is read by any method's own logic — `restart_basic` and
`restart_master_reset` return as soon as `disconnect_after_restart`'s
mandatory wait elapses, without consulting either value.

Nothing in this crate calls `recovery_wait` and then waits, and nothing
computes or waits out `restart_basic_t1` either. Both are missing pieces
of the same absent behaviour: MP §3.7.1.2.2, p. 81's next clause — *"call
the failed service one last time"* after the Process Time expires, before
declaring the Configuration Procedure failed — is an **obligation this
project has not implemented**, not merely a timing value kept in reserve.
There is, at the time of writing, no Configuration Procedure in this
project that runs a service, hits a failure, restarts the device to
recover, and then needs to retry that same service once the device is
responsive again; MP §2.3's own restart step (step 4) restarts the device
as the *last* thing a procedure does, with nothing afterwards to retry.

**Cause.** Building a generic "wait, then call this one more time" retry
combinator with no real caller to attach it to would be exactly the kind
of speculative abstraction this project's own engineering rules warn
against, and an untested combinator is also exactly the kind of
unverifiable behavioural claim this commissioning plan keeps finding and
removing elsewhere. `recovery_wait` and `restart_basic_t1` were written as
small, directly tested pure values instead, so that the arithmetic MP
§3.7.1.2.2 and §3.7.1.1.2 require is proven correct today, and wiring
either to an actual wait-and-retry becomes a caller's problem once a
caller exists.

**Impact.** A future Configuration Procedure that restarts a device via
Basic Restart or Master Reset mid-procedure and needs to resume afterwards
must itself wait out `restart_basic_t1` or `recovery_wait` and perform the
"one last time" retry MP §3.7.1.2.2 requires — none of that happens
automatically today, and no code path in this crate implements that retry
obligation at all. The timing values themselves are correct and tested
(`recovery_wait_never_goes_below_the_configured_floor`,
`default_restart_timings_match_mp_section_3_7_verbatim`); only the
waiting and the retry are absent.

**Updated, 2026-09-27.** `restart_basic_t1` now has one reader:
`individual_address_write`'s step 4 waits it out once before retrying a
`T_Connect` to a freshly addressed device (§7, "Three limitations this
first write exposed", item 1). That use is a **borrowing**, not the MP
§3.7.1.1.2 obligation this section describes — the retry happens *before*
the restart, not after it — so the "call the failed service one last time
after a restart" behaviour is still unimplemented, and `recovery_wait`
still has no reader.

**Lifted when.** A Configuration Procedure exists in this project whose
recovery from a Basic Restart or Master Reset needs more than "restart,
then let a fresh session reconnect" — at which point that procedure's own
retry loop, not a speculative one built ahead of it, waits out the
relevant timing value and implements MP §3.7.1.2.2's "one last time" retry.

**Accepted boundary, 2026-09-28 (goal-commission K8).** User decision
2026-09-28, asked for each K8 entry to fix it or accept it: *"das was am
sinnvollsten ist"* (whatever makes the most sense), i.e. the recommendation.
The live K2 traces showed the fresh-session reconnect is enough on the v1
device (about 38 s unreachable, then readable); no procedure needs the
computed waits yet.
<!-- END retained catalogue record -->
