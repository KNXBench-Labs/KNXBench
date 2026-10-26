# ADR 0152: Gated hardware release of the System-2 (0025h) download

Date: 2026-10-22
Status: Accepted
Session: 7 (integration/hardening)

## Context

The source-bound System-2 candidate (ADR-0149, contract
`up258-system2-validation.md`) is delivered as STRONG HYPOTHESIS / SIMULATOR
TESTED with physical admission deliberately refused. The refusal is two-layered
inside `knx-net::commissioning::property_download::run_property_plan_inner`:

1. a hard `return Err("physical source/recovery admission is unavailable …")`
   whenever the transport is `TargetKind::Hardware`, and
2. `ExperimentalPropertyPreconditions::require_full_hardware_write_coverage`,
   which refuses any allocation whose bytes are not fully written.

The System-2 plan deliberately leaves 754 octets unwritten — 338 masked bytes
(no source write) plus 416 octets of declared address/association-table capacity
beyond the computed tables — so the full-coverage requirement can never hold for
this plan. (**Correction, 26 October 2026:** 15 of those 416 octets are
independent source data, not table capacity; the current defaults profile v2
writes them and leaves 739 = 338 + 401 unwritten. New physical execution with a
v1 bundle refuses; see the
[contract](../contracts/up258-system2-validation.md).) The open points 1–4
(masked-byte survival, the master-schema
pointer/flag transforms, physical read-back, parameter-encoding authority) remain
blocked on defining sources that are not in the authorized corpus and are not
obtainable without manufacturer/KNX-Association contact.

One empirical observation (issue #22 reporter capture, documented in
`research/2026-10-22-system2-ets-download-procedure.md`) shows that Unload does
**not** physically erase memory on that device — before/after dumps are
byte-identical. This is a single observation on one device, not a universal
rule and not a substitute for the blocked defining sources.

The owner decided on 22 October 2026 to release the download. The release is
explicitly **gated**, not a removal of the safety checks, and offers **both**
recovery tracks (external ETS, or KNXBench best-effort).

## Decision

Release the System-2 hardware download as a consent-gated, best-effort path.

- Add a dedicated hardware admission value (separate from the existing
  `WriteAuthorisation::for_hardware`) that can only be constructed from a
  typed, device- and operation-specific confirmation phrase which names the
  best-effort nature and the unwritten masked bytes. The value is opaque (no
  `Default`, no `Copy`), so a caller cannot produce it by accident and a
  function that does not take it cannot reach hardware.
- Replace the unconditional hardware refusal in
  `run_experimental_property_plan` / `restore_experimental_property_plan` with:
  hardware is permitted only when a valid admission is supplied; the simulator
  path is unchanged and still needs no admission.
- Replace `require_full_hardware_write_coverage` with a consent-gated check:
  without admission the full-coverage requirement stays exactly as today; with
  admission, unwritten/masked bytes are permitted, but the pre-dump
  (`property_recovery::acquire`) must still capture the full declared
  allocation intervals, and recovery is labelled best-effort.
- Offer both recovery tracks at the CLI/App layer: (a) external ETS
  (advisory — the pre-dump is still captured unconditionally), and (b)
  KNXBench best-effort (replay the captured old bytes via the existing
  restore path). Both tracks capture the durable pre-state.
- Label the plan, the report and the docs honestly: unwritten masked bytes,
  best-effort recovery, no guarantee that a different device/firmware retains
  the masked bytes, and no ETS-equivalence or verified-device claim.

The physical-admission wording in ADR-0149 and the validation contract is
amended to describe this gated path, not a blanket refusal.

## 26 October 2026 amendment: explicit configured pointer-width policy

The candidate correction for issue #30 adds a versioned policy only to newly
observed, exact-UP258 configured hardware records. It does not reinterpret old
operations or edit shipped profile bytes. The policy accepts one two- or four-
octet PID 7 element on objects 1/2/3, preserving the raw representation through
private capture, durable reopening, observations and explicit recovery.

The [independently retrieved public package](../research/2026-10-26-up258-pid7-width.md)
declares both property datatypes and the three two-octet resources; the reporter
measured two octets on object 1 only. Objects 2/3 remain source-bound experimental
hypotheses and must be labelled as such in the offline preview. Numeric equality
with a source pointer is not evidence of allocator history or whole-device
rollback. The ordinary hardware gates, best-effort consent and hard final checks
remain unchanged; masked/unwritten-byte survival is still unresolved.

A new hardware-record revision binds the append-only policy identifier into the
journal digest. Old records retain their exact grammar, digests and four-octet
behaviour; new readers validate policy/representation consistency, and older
readers reject new fields/widths rather than drop them. Review and acceptance of
the candidate implementation are recorded separately, not inferred from this
architectural decision.

## Alternatives considered

- **Remove the refusal outright (route MV-0025 through the ordinary planner).**
  Rejected: the ordinary planner encodes a different (070nh/System-B) procedure;
  routing 0025h through it would silently claim an unverified load sequence and
  drop the property-based controls. This is exactly the bypass the original
  triage refused.
- **Keep the refusal until open points 1–4 resolve.** Rejected by the owner's
  explicit decision; the gated path preserves the refusal for non-consented
  hardware while admitting a consented, clearly-labelled best-effort run.
- **Full-coverage-only release (write the masked bytes with a guessed value).**
  Rejected: there is no source value for the 338 masked bytes, and inventing one
  would violate data integrity.

## Consequences

- A real System-2 download becomes reachable only through a typed, explicit
  consent that names the target and the best-effort/masked-byte nature; the
  compiler, not convention, prevents a hardware run without it.
- The residual risk is explicit and documented: the 338 masked bytes and the
  416 capacity bytes are not written (401 with profile v2; see the correction
  under Context); recovery is best-effort and depends on the
  pre-dump plus (for KNXBench-only runs) the device retaining those bytes across
  Unload/reallocation. A failed or interrupted run reports `RestoreFailed`
  rather than `Restored` when full verification does not hold.
- The existing destructive-Unload counter-hypothesis test stays correct; the
  simulator is not relabelled as hardware proof.
- New tests must cover: hardware refused without admission; hardware admitted
  only with the exact phrase (near-miss refused); the simulator path unchanged;
  both recovery-track selections; and mutation checks that the admission cannot
  be forged (default/constructed out of thin air).

This decision releases the download path only; it does not close open points
1–4, does not claim ETS equivalence, and does not relax the ordinary download
planner for any other mask.
