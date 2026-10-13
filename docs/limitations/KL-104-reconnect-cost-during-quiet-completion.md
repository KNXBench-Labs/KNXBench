# Reconnect cost during quiet completion

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-104)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->
## 104. A device that goes offline mid-`LoadCompleting` now costs a full reconnect per quiet poll

**Limitation.** Since C19, a connected request whose four transmissions
(`MAX_TRANSMISSIONS`, TL §3, p. 15) all go unacknowledged releases the
Transport Layer connection, per TL §3.9, p. 15 and the state machine's `E18`
→ `A6` cell (§5.2 p. 19, §5.4.1's table p. 22, §5.3 p. 19).
`wait_for_load_state` tolerates that release while the device's last reported
state is one RES Table 94 permits silence in — but the next poll may no
longer reuse the connection, so it re-establishes first. That
re-establishment is `connect()` in full: `T_Connect` and its confirmation,
`A_Authorize_Request` — and, on a write-capable session, the Verify Mode
read-modify-write as well, which is gated on the session holding a write
authorisation and not on which `AuthorisationPlan` it uses. One quiet poll
therefore costs one whole reconnect.

**Cause.** The Standard says the connection is gone and the Standard says to
carry on polling; it does not say the polls become cheaper. RES §4.23.2.4.1
asks the MaC to *"try to re-establish the connection periodically during the
maximum transition time"*, and RES's NOTE 86 expects exactly this device:
*"A device may be offline during state LoadCompleting."* The previous
behaviour was cheaper only because it was wrong — it kept polling a
connection the peer had already forgotten, and read whatever came back as if
it meant something.

**Impact.** Latency and bus traffic, not correctness, and it compounds with
entry 101's accounting of the same wait loop. With `SessionTiming::default()`
each quiet poll is `4 × 3 s = 12 s` of unacknowledged transmissions plus the
re-establishment (up to `connection_timeout` = 6 s for the `T_Connect`
confirmation alone, more with a key), where before C19 it was the 12 s and
nothing else. With the default `max_transition` of 30 s that is room for
roughly one or two such polls before the wait gives up, not an open-ended
series — the cost is a slower failure, not a longer one. Against a device that never comes back the wait still ends in
`SessionError::TransitionTimedOut` with the last state read, as it did
before; the failed re-establishments are tolerated rather than reported,
because a single failure is not what *"periodically"* means.

**Not a regression in what is reported.** A re-establishment that fails for a
reason other than the connection failing to come up — a refused property, a
mismatched read-back, a Verify Mode the device will not take — is still
returned to the caller unchanged (`reestablishment_may_be_retried` in
`crates/knx-net/src/commissioning.rs` lists exactly the four
connection-shaped errors it swallows, and since C19's second fix round each
of the four has a test that fails when it is removed from that list).
`SessionError::Lagged` — raised when a session falls behind the event
broadcast channel it reads from (`crates/knx-net/src/commissioning.rs`) — is
the deliberate fifth case `reestablishment_may_be_retried` does not match:
falling behind a broadcast channel says nothing about whether the Transport
Layer connection itself is still there, so retrying on it would be a guess.
That exclusion is reasoned about in the match arm's shape, not tested; no
test in `commissioning.rs` constructs a `Lagged` error at all, so nothing
would fail today if a future edit folded it into the retryable set by
mistake.

**Lifted when.** A measurement on real hardware says the reconnect cost
matters. The cheaper alternative — keeping a released connection and hoping
the peer still honours it — is not available: it is the defect C19 fixed.

**Deferred to hardware, 2026-09-28 (goal-commission K8).** User decision
2026-09-28, asked for each K8 entry to fix it or accept it: *"das was am
sinnvollsten ist"* (whatever makes the most sense), i.e. the recommendation.
The lifting condition is itself a hardware measurement; taken along in the
K7 live session if the device goes quiet mid-`LoadCompleting`, otherwise
accepted as is. **K7, 2026-09-29: it did not go quiet.** No
`LoadCompleting` silence and no mid-download drop in three live downloads
(RESEARCH §19). Accepted as is.
<!-- END retained catalogue record -->
