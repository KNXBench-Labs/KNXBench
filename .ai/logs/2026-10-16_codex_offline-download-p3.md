# Offline download P3: property execution prerequisites

Checkpoint: 2026-10-16 00:26. Test-first preparation only; implementation is not accepted.

- Isolated branch starts at P2 featurebe01a228; actual P2 merge79156a5d is separately gated.
- The queued initial Core property-plan test has no execution result yet. Shared execution leases remain in force; P2 source is frozen.
- Original MP§3.31.3.4 pp139–140 defines ten-byte absolute data/stack/task property records; raw product SegFlags is a separate mapping question. Property bit7 disables checksum control, unlike the memory-record API.
- Original Cookbook Load Controls v01.00.02 pp4–8 distinguishes tool instructions from wire/state-machine events. It does not define modern XML SegFlags or a universal LsmIdx-to-interface-object conversion.
- Original Profiles v02.01.01 AnnexA.2.4.1 pp144–145 makes global subtype support distinct from resource-specific support; its matrix cannot substitute for resource mapping.
- Existing core memory_download::property_matches already retains a disclosed source inference for zero-padded inline comparisons. Reuse with explicit provenance instead of inventing a normative ten-to-six-byte rule.
- Planned scope must resolve every source position, data write and mutation/recovery effect; real-fixture encoding refusal remains open. No hardware admission or public route change is authorised by this checkpoint.

## P3a normalized simulation implementation

P2 is now delivered as merge79156a5d and documentation19e2827d; exact main
readback and owned retirement are recorded in the durable private P2 archive.
P3 was fast-forwarded onto19e2827d before acceptance; owned runtime bytes were
preserved. A rejected whole-file handover write was repaired with a targeted
prepend, preserving prior history.

Core now validates immutable normalized operations, contiguous source positions,
explicit object indices, exact identity guards, load lifecycle, absolute bounds
and non-overlapping writes. Possible setup, property and indirect allocation
mutations remain distinct from memory writes and from recovery proof. Net's
entry accepts only concrete SimulatedDevice, never an arbitrary transport;
identity checks precede Verify Mode, writes reuse ManagementSession, and failures
retain source position/mutation ambiguity without pretending to restore.

Observed test-first failures: absent Core/Net module E0432; new validator initially
conflated memory-type/attribute positions and then rejected EEPROM type3. Original
MP139 and the existing explicit enum exposed the cause; independent raw fixtures
now distinguish type1/2/3, separate attribute bit7 and reserved octets. The old
simulator returned generic relative7000 instead of absolute3C00; its independently
specified end-to-end assertion failed, then passed after raw-offset decoding.
Explicit setup-effect omission and wrong simulator-instance guard also went RED
then GREEN. Header preflight initially rejected one104-column summary; shortened
only that owned header. Historical failures are retained, not reclassified.

Latest actual focused result: Core8/8 plus Net7/7; the compile-fail example emits
E0308 specifically for expected &SimulatedDevice versus &T. Owning-crate all-target
warnings-denied Clippy and documentation links pass. Full branch acceptance is
still pending. Separate in-session self-review is not independent approval.

This is deliberately P3a, NOT completion of P3/the owner goal. Modern XML SegFlags,
source resource admission/mapping, restart, real-source16-bit encoding and durable
whole-mutation recovery remain separate blockers/work. No public route, hardware
support status, persistence schema, new dependency or original evidence export.
The helper has no future-drop cancellation/abort/restore guarantee.

## Observed branch acceptance

Fifteen actual stages passed on frozen19e2827d plus the complete owned delta:
Rust4639/0/207ignored, Vitest3626/263files, workspace Clippy/build, real frontend
build, documentation/tooling/five repository checks and explicit Release
System2 source-refusal regression. All2898source paths agree at start/end/current
mode/content. Runtime source unchanged by the observed evidence/docs closure.
Actual merged/corpus acceptance and publication remain pending.
