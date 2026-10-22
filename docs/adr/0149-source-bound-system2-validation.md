# ADR-0149: Isolate the source-bound System-2 validation candidate

- Status: Accepted for simulator-only validation
- Date: 2026-10-20

## Context

The ordinary System-2 image/admission path deliberately refuses indispensable
unresolved source semantics. UP258D12 application `M-0001_A-9205-04-9270` needs
legacy parameter/reference handling, Background/Union precedence and ordered
absolute property loading. Existing memory-control records are not the property
wire grammar. A synthetic partial plan is not a complete product download.

## Decision

Add a separate, explicitly experimental image entry with mandatory versioned
interpretation names and exact program/namespace/source/package binding. It
reuses the existing parameter evaluator, resolver, image/table codecs and the
existing management-session executor; it does not relax ordinary admission.
All source parameters receive a disposition. Unknown required rules refuse.
Manufacturer-specific identities/digests live in versioned data, not branches in
Core or generic application logic. Initial configuration is complete defaults,
no group links, no project overrides.

Translate every retained source-procedure position in order into a normalized
Core property plan. Tool-only metadata stays observable with zero wire effect.
Use the ten-octet absolute property grammar independently of memory-mapped
controls. No allocation-size increment is inferred from a feature name.

Before Verify Mode or a destructive resource event, Net acquires complete
allocation intervals, write regions, stable load states, source-applicable
pointers and target/application identity. App owns private durable immutable
checkpoints with sync/readback, plan binding and explicit recovery. Record an
upcoming mutation before sending it. Reopening an unfinished operation yields
interrupted, not success. Recovery replays the complete original procedure with
old write bytes, once and with a different confirmation; it never runs
implicitly after an error. This is same-application/procedure/allocation-shape
reinstallation, not arbitrary allocator/task-state rollback.

The simulator separately exposes allocation ownership, raw attributes, task
identity/setup and four TaskCtrl2 words. Independent OLD fixtures and full
persistent-state comparisons prevent memory-only assertions from disguising
missing resource reconstruction. A separately named destructive-Unload fixture
is a counter-hypothesis, not undocumented firmware behavior.

## Boundaries and consequences

All candidate interpretations remain **STRONG HYPOTHESIS / SIMULATOR TESTED**.
The exact source reserves 1,514 bytes and writes 760, leaving 754 bytes
unwritten: 338 masked (no source write) plus 416 octets of declared
address/association-table capacity beyond the computed tables.
Their survival through real Unload/reallocation is unresolved. The CLI has no
physical transport; even explicit consent plus `--gateway` refuses before a
socket, activity lease or operation receipt. The generic experimental Net entry
also refuses every hardware-kind execution before connection; complete write
coverage and ordinary Download authorization never substitute for source/recovery
admission. The earlier incomplete-coverage diagnostic remains explicit.
Separate user approval does not override that missing technical prerequisite.

Historical Schema10 prose and a revision-bound third-party namespace20 schema
mirror inform explicit candidate rules; neither proves that every semantic is
unchanged in namespace20 or confirms this device firmware. Provenance and
contradictions remain visible in the [research](../research/2026-10-20-system2-validation-rules.md).
No manufacturer contact, proprietary executable inspection or copied owner
research is needed or claimed. No new storage/model migration or ETS-equivalence
claim follows.

See the [behavioral contract and executable validation](../contracts/up258-system2-validation.md).

## Amendment — 21 October 2026: masked-byte rule characterized

The 338 masked bytes have no source-cited survival rule. The standard defines
logical invalidation on Unload (Management Procedures `03_05_02` v02.01.02
§3.31.3.1, printed p. 138) and clears only the PID 7 pointer (Resources
`03_05_01` v01.10.01 §4.2.7, printed p. 31); the sole "keep existing memory
contents unchanged" clause belongs to the relative data allocation that mask
`0025h` does not use (Profiles `06` v02.01.01 §A.2.4.1, printed p. 144). See the
[open-point dispositions](../contracts/up258-system2-validation.md#open-point-dispositions-21-october-2026)
and the [source audit](../research/2026-10-21-up258-masked-byte-and-recovery-rules.md).
The refusal stands; the destructive-Unload counter-hypothesis test remains the
correct behaviour.

## Amendment — 22 October 2026: gated hardware release

The owner released the System-2 download as a consent-gated, best-effort path.
See [ADR-0152](0152-gated-hardware-system2-download.md): the unconditional
hardware refusal is replaced by an admission that only a typed, device- and
operation-specific [`HardwareRelease`](../../crates/knx-net/src/commissioning/hardware_release.rs)
value can grant, and unwritten (masked) bytes are permitted only under that
consent. The simulator path and the non-consented hardware refusal are
unchanged; open points 1–4 remain blocked.
