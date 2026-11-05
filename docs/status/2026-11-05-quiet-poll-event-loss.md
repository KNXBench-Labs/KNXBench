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

Source commit `b8f4d2ee9` passed the complete selected 14-stage package gate on
main-first integration `a3eea3696` (first parent `eab79cc5b`): every stage exited
zero with valid proof, inputs frozen and user data checked and untouched.
The run passed 4,366 Rust tests with 153 explicitly ignored tests across
17 result blocks, plus a separate selected corpus run of 91/91. Python tool
coverage registered 517 tests with six existing optional skips. Clippy and the
required desktop preparation/build consumers passed. No application browser or
Web unit-test stage was selected, and no gate selection policy changed.

The gate completed on 5 November 2026 at 07:40 CET. Its conditional push correctly
refused because `main` had advanced only by the USB documentation closeout
`bf83332e9`. The main-first successor preserves that documentation byte-for-byte,
removes only this package's finished checkpoint block and carries these Rust and
corpus results through an exact runtime-input equality proof. The composition
check also rejects the old gated tree as a negative control; the successor runs
its own maintained documentation-affected gate rather than replaying Rust.

Raw review, focused/mutation runs, the accepted source gate and composition
proofs are retained under `<profile>/evidence/2026-11-05-quiet-poll-event-loss/`.
The broader LS-07 work remains separate.

## Limits

The adapter supports these synchronous simulator scenarios, not arbitrary delayed
transport events. Re-establishment coverage targets connection confirmation,
not later keyed authorisation or Verify Mode. This closes neither the rest of
LS-07 nor any physical KNX acceptance. There is no bus, device, VM, release or
deployment action in this package.
