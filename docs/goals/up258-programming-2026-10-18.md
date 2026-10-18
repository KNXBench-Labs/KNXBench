# UP 258D12: offline work toward a reporter-run programming test

## Owner decision (18 October 2026)

Continue the exact-device offline path, not another mask-family expansion.
Target: Siemens UP 258D12, application `M-0001_A-9205-04-9270`.
The reporter executes every eventual hardware step with separate explicit
consent. No bus operation, deployment or purchase is authorised by this goal.

Recovery is mixed: an independently established ETS recovery route may be
available; without ETS, KNXBench must supply an independently exercised restore
path. Neither an ETS project nor an identity ZIP nor selected memory bytes
alone establish whole-device recovery. Do not promise that a full ETS download
can repair every interrupted allocator, access-control or firmware state.

## Sequence and acceptance

1. Reuse the existing byte-order probe and diagnostic collector rather than
   adding a second parameter evaluator. Prepare a bounded, explicit-field
   comparison for the exact source, previewed offline before any read consent.
   Keep unavailable reads distinct from zeros; no generic dump or implicit
   source-address-to-runtime admission. An offline plan is not a backup.
2. Complete image construction and all source-step translation only where the
   source semantics are established. Retain the current refusal for unresolved
   `LegacyPatchAlways`, encoding and allocation transforms; do not suppress it
   just to reach a simulator success.
3. Exercise execution and whole-mutation recovery offline, including setup,
   partial execution and cancellation. A memory-only receipt cannot authorise
   unloading, allocator changes or task-pointer writes. No physical executor
   becomes reachable solely because its simulator passes.
4. At the real hardware boundary, name the accepted build, exact identity,
   plan and recovery method; the reporter separately approves each operation.
   Readback and human presence/brightness checks are both required.

### Write ladder (owner go, 18 October 2026)

The owner asked for a routine that writes more and more until the device is
fully programmed. Accepted in this staged form, because the memory stages
can grow step by step but the load procedure cannot (after Unload the device
has no valid application until LoadCompleted):

| Stage | Action | Writes |
| --- | --- | --- |
| 0 | Read the source-declared ranges only; compare offline with the ETS-programmed state | no |
| 1 | Write back 3 identical octets, with own receipt and pre-Verify-Mode setup state | minimal |
| 2 | Change one parameter, human function check, restore | yes |
| 3 | Whole parameter segment | yes |
| 4 | Tables (address, association, objects) | yes |
| 5 | Complete load procedure as one non-incremental operation | all |

Within one consented stage a routine may proceed automatically (receipt →
write → readback → next block) and stops on any mismatch, timeout or error.
It never moves to a higher stage by itself and never crosses into stage 5.
A human function check sits between stages; a byte comparison does not show
that the detector still works. Readback proves transfer, not correct content:
stage 0's comparison against the device's existing ETS-programmed state is the
content check. Repeated passes cost EEPROM write cycles; no unbounded loops.
Stages 1–5 stay blocked by the boundaries in the sequence above.

Continue every implementable item without asking again for offline permission.
When a step needs unavailable defining evidence, record the exact dependency,
finish independent work and report the actual boundary rather than claiming
programming support. A single field comparison cannot establish all source
encoding, Union activation, firmware behaviour or recovery semantics.

## Ownership and current state

Owned by the `feat/up258-offline-20261018` workstream. UI feedback work remains
with its existing owner; no web UI changes belong to this package.

Initial inspection: the existing probe is coupled to memory-download mask
admission, its CLI connects immediately, and System-2 image construction and
property execution/recovery are separate missing links. This goal begins with
an executable offline preparation path and explicit read-scope evidence.
The offline source preview is implemented: ProductDB → App → built CLI,
with explicit field selection and no network client. Synthetic and exact-package
acceptance are recorded in the [contract](../contracts/system2-source-probe-preview.md).
This is a completed preparation slice, not completion of the programming goal.
Image/procedure/recovery and separately authorised hardware acquisition remain
pending. A first write-back of identical bytes does not prove no side effects
or the full download path; it needs its own scope and setup/recovery contract.
