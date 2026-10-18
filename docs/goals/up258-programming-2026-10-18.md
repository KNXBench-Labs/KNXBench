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
