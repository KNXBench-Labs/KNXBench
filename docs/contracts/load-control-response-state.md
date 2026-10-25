# Load-control write answers are observations

Published source: `e4f5e7c6f` after independent review and the maintained integrated
package gate ([acceptance](../status/2026-10-25-load-control-response.md)).
Simulator/software acceptance only; no hardware run.

## Rule

An `A_PropertyValue_Write` to `PID_LOAD_STATE_CONTROL` is answered with one
element carrying the load state (Management Procedures v02.01.02,
§3.31.3 DMP_LoadStateMachineWrite_RCo_IO, printed p. 138).
Resources v01.10.01 §4.23.2.3.2, printed p. 294, requires a device going offline
during `LoadCompleting` to report that state in this response. The answer is
therefore an actual state observation, preceding the first verification poll.
The conditional wait/reconnect rules remain those of Resources §4.23.2.4.1,
printed p. 297.

## Behaviour

- The answer is decoded like a poll (one octet, known state). Wrong width or an
  unknown value ends the write with the existing errors before any poll. A
  changed state forbidden by the mask-narrowed transition table is rejected
  immediately; an unchanged state retains the existing pending treatment.
- A state that differs from the one read before the event seeds the existing
  wait loop's last-observed state. Its conditional tolerance of silence and
  connection release (`LoadCompleting` only) applies from the first poll.
- An unchanged state seeds nothing and licenses nothing; the wait stays pending
  under the existing time-outs.
- A settled answer does not return early. The verification poll stays and its
  later observation decides the outcome. If it returns the pre-event state and
  never moves, the existing `StateUnchanged` outcome can replace the earlier
  settled observation; this is not a successful completion.
- The event is never re-sent, nothing is inferred from silence, and the wait
  budget and its one extra attempt are unchanged. A device that stays silent
  after reporting `LoadCompleting` ends in `TransitionTimedOut` holding that
  last observed state.

## Limits

The memory-mapped load path (three-read mechanism) has no write answer and is
untouched. The simulator gained `LoadControlResponse`, a fixture choice for the
write answer; a real device's behaviour is unverified. This change neither
widens device-family admission nor establishes recovery or hardware acceptance.
