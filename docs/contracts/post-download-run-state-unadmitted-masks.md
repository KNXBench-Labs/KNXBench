# Post-download run-state — unadmitted masks (PRG-01-N4)

Bounded contract; extends [post-download-run-state](post-download-run-state.md).
Evidence: [research note](../research/2026-11-21-run-state-remaining-masks-and-layouts.md);
decision: [ADR-0182](../adr/0182-unadmitted-run-state-masks-stay-refused.md).

## Admitted masks

Only exact `0700h` (descriptor-qualified), `0701h`, `0705h` reach the run-state continuation.

## Rule

For every other mask the observation returns
`NotRead(UnsupportedMask)` after the existing identity/load-state reads:

1. Exactly one object-3 property read (the identity read); no PID 6 read, no
   description read, no second object index, no scan, no memory fallback.
2. The mask refusal outranks load-read classification, so a refused mask is
   never reported as a transient load failure.
3. The session is read-only and closed; no write frame is sent.

Covered by simulator tests: `0703h`, `0704h`, `0706h`–`070Eh`.

Masks outside the `070nh` family (`0020h`, `0101h`, `0102h`, `5705h`, `0710h`) never
reach this stage: the observation is `NotPerformed(UnsupportedMask)` decided
offline, and no session is opened and no frame is sent.

## Evidence needed to admit a further mask

A named source approving or profiling the mask, a datatype rule for its PID 6
(PDT_CONTROL vs legacy), and a simulator schedule derived from them. Memory-mapped
run state additionally needs a source for the encoding. Absent that, refuse.
