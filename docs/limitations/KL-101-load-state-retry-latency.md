# Load-state retry latency

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-101)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 101. RES §4.23.2.4.1's "once more" attempt can roughly triple a load-state wait's worst-case latency

**Limitation.** C5 (`wait_for_load_state`, `crates/knx-net/src/commissioning.rs`)
correctly grants exactly one extra poll past `max_transition`, per RES
§4.23.2.4.1's *"once more when the maximum transition time has passed"*. The
flag that gates it latches *after* the read that discovers the deadline has
passed and *before* the loop's next `tokio::time::sleep`, so the extra
"attempt" is a complete loop body: a full `poll_interval` sleep, an optional
`reconnect()`, and a full `read_load_state` — which, since Task C4, retries
up to `MAX_TRANSMISSIONS` (4, not 3) times before giving up, at
`response_timeout` (`ACKNOWLEDGE_TIMEOUT` = 3 s) each. With
`SessionTiming::default()` (`connection_timeout` 6 s = `CONNECTION_TIMEOUT`,
`response_timeout` 3 s, `poll_interval` 3 s, `max_transition` 30 s) and
`AuthorisationPlan::Skip`, the worst case for that one extra attempt —
device stays in a state Table 94 permits silence in, so `NoAnswer` is
swallowed rather than returned — is:

```
poll_interval sleep (3 s) + read_load_state (4 × 3 s = 12 s) = 15 s
```

with no reconnect needed, since the connection never dropped. The read that
*first notices* the deadline has passed can itself have taken up to 12 s
(the same 4-attempt ladder, if that poll also went unanswered), so a caller
who budgeted `max_transition = 30 s` can see this call block up to
`30 + 12 (last pre-deadline read) + 15 (the one extra attempt) = 57 s` in the
worst case, against a pre-C5-fix bound (C4 already applied) of
`30 + 12 = 42 s`. If the extra attempt's `reconnect()` is also needed (the
connection *did* drop), `reconnect()` re-runs `connect()`, adding up to
`connection_timeout` (6 s) for `AuthorisationPlan::Skip`; the total for that
case is `30 + 12 + 3 + 6 + 12 = 63 s`. `AuthorisationPlan::WithKey` is worse
still: `reconnect()` → `connect()` also calls `authorise()` (`exchange`-based,
so up to another `MAX_TRANSMISSIONS × response_timeout` = 12 s) and, when a
session scope is set, `assert_verify_mode()` (at least one more such
exchange, possibly a read-modify-write pair) — each one an additional
worst-case delay this document does not attempt to total exactly, since
`assert_verify_mode`'s own exchange count was not fully enumerated for this
entry.

**Cause.** The Standard authorises the one extra attempt but does not bound
what an "attempt" is allowed to cost, and this project's attempts are not
free: C4 (Task C4, same review cycle) independently widened every exhausted
exchange from 9 s (`MAX_REP_COUNT` = 3 attempts × 3 s) to 12 s
(`MAX_TRANSMISSIONS` = 4 × 3 s), which compounds with C5's extra attempt
rather than being independent of it.

**Impact.** Latency only, not correctness: `SessionError::TransitionTimedOut`'s
`waited` field reports the true elapsed time, not `max_transition`, so
nothing misreports how long the call actually took. A commissioning UI that
shows a progress indicator keyed to `max_transition` (30 s by default) can
appear to hang for up to roughly twice that, worse still under
`AuthorisationPlan::WithKey`, before the call returns.

**Lifted when.** A UI surface actually renders `SessionTiming::max_transition`
as a hard progress bound (none does yet, per entry 97) — at which point
either the bound must be widened to reflect the true worst case, or the
extra attempt's own cost must be capped independently of `MAX_TRANSMISSIONS`
and `response_timeout`.

**Status 2026-09-28 (K5).** Still open, now with a bound on where it shows.
The first UI that runs a download, the **Download to device** tab
(`DeviceDownloadPanel`), renders no time bound at all: its progress is
steps started out of the plan's steps, and octets read back out of the
plan's octets, both counted from events the executor actually reported. A
load-state wait that runs the extra attempt therefore shows as one step
that takes longer. Nothing claims to finish at `max_transition`, so nothing
appears to hang against a promise. The limitation still applies to any
future surface that shows a countdown.

**Deferred to hardware, 2026-09-28 (goal-commission K8).** User decision
2026-09-28, asked for each K8 entry to fix it or accept it: *"das was am
sinnvollsten ist"* (whatever makes the most sense), i.e. the recommendation.
Only a real device can show whether the extra attempt matters; no surface
renders the bound (K5 status above). Measured opportunistically in the K7
live session, otherwise accepted as is. **K7, 2026-09-29: nothing to
measure.** In all three live downloads each load-state wait was answered
at once (`Loading` → `Loaded`, no extra attempt; RESEARCH §19). Accepted
as is.
<!-- END retained catalogue record -->
