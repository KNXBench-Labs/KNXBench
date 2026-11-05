# Quiet-poll event-loss regression — 5 November 2026

## Bounded requirement

The optional quiet-poll regression in [LS-07](../open-work/LS-07-optional-hardening.md)
is admitted as a test-only increment: local event loss must remain fatal while
waiting through an observed `LoadCompleting`, both on an existing connection
and during re-establishment. It must not become tolerated device silence,
a transition timeout, another poll or another reconnect.

This characterizes existing behavior. It does not change runtime handling,
simulator semantics, retry/timeout policy, protocol support or hardware admission.

## Candidate and witnesses

Source base: `8f1a933f2`. The candidate adds one module registration in
`crates/knx-net/src/commissioning.rs` and
`crates/knx-net/src/commissioning/tests/quiet_poll_lagged.rs`.

Four registered tests share the prefix
`commissioning::tests::quiet_poll_lagged::`:

- `lagged_during_a_quiet_poll_is_fatal_without_another_poll_or_reconnect`
- `a_quiet_poll_without_event_loss_keeps_polling_on_the_same_connection`
- `lagged_during_reestablishment_is_fatal_without_another_poll_or_reconnect`
- `reestablishment_without_event_loss_resumes_polling_after_a_quiet_release`

A test-only adapter delegates each send to the existing synchronous simulator
and forwards its actual events into an eight-slot broadcast channel. At a named
send boundary it adds nine harmless confirmations to cause genuine receiver
lag, or one confirmation for the paired healthy control. No forwarding task,
invented `SessionError`, external socket or live device is used. All scenarios
first read `LoadCompleting`; none supplies that state as a seed. Exact error
classification and transport/simulator counters distinguish fatal loss from
retrying, retransmitting or continuing to poll.

## Acceptance state

Independent whole-source review approved the candidate with no findings.
The actual leased, network-isolated focused run passed all four registered tests
with unchanged source hashes. In a separate source copy, two compiled mutations
were rejected by their intended assertions: tolerating quiet-poll lag failed only
the quiet-overflow test, and making re-establishment lag retryable failed only
the reconnect-overflow test (each 3 passed, 1 failed). The equivalent
`is_some_and` control and restored original each passed 4/4. All copied source
inputs were restored exactly; the feature sources were unchanged.

The maintained package gate is still pending. Its dry run selects the normal
14-stage Rust/dependent-package and corpus closure, with no application browser
or Web unit-test stage. Tool tests still need the installed browser tooling.
No gate selection policy is changed. Raw focused/review/mutation evidence is
retained under `<profile-scratch>/next-work-20261105/` until final retention.

## Limits

The adapter supports these synchronous simulator scenarios, not arbitrary delayed
transport events. Re-establishment coverage targets connection confirmation,
not later keyed authorisation or Verify Mode. This closes neither the rest of
LS-07 nor any physical KNX acceptance. There is no bus, device, VM, release or
deployment action in this package.
