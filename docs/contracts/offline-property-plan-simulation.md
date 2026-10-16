# Offline property-plan simulation: P3 boundary

Status: integrated and published as `7632fb1a` on 16 October 2026; exact local,
tracking and live remote main equality verified. Fifteen branch and sixteen actual
merged stages pass: Rust4639/0/207ignored, Vitest3626/263files, workspace
Clippy/build, frontend build and documentation/tooling/repository checks. Merged
acceptance includes isolated corpus152/152 in33targets and2899unchanged source
paths. [Receipt](../evidence/offline-property-plan-simulation-2026-10-16.json).
This is not a System-2 source translator, public programming route or recovery
implementation.

## Independent normalized boundary

Core owns immutable ordered property operations, not product XML or networking.
The initial bounded plan targets mask0025, explicit resource objects1–3 and the
existing ten-octet property encoders. An object index in this API is already
resolved; it is never an inferred XML LsmIdx mapping. Every zero-based source
position is accounted for, including explicit tool metadata and data writes at
the same position as their allocation. Unknown controls, gaps/reordering,
unsafe load ordering and memory outside declared absolute allocations refuse.
A hardware-type comparison precedes every mutating load operation.

This does not yet consume ProductDB LoadProcedures. In particular, modern XML
SegFlags polarity and target-specific resource admission require their own
source contract. A normalized fixture cannot certify the real Siemens image or
convert its unresolved16-bit source encoding into success. The source adapter
must eventually account for every source instruction and every image segment;
unsupported constructs may not be skipped.

## Simulator-only execution

Reuse ManagementSession and the existing SimulatedDevice rather than creating
another transport. The entry point accepts the concrete simulator, not a generic
ManagementTransport: physical transports cannot be passed to it. Require matching
explicit simulator Download authorization and target before connecting. Connect
without Verify Mode first; read mask/manufacturer and hardware type, then perform
normal Verify Mode setup, load-state polling and verified memory writes. Failure
always attempts disconnect and retains the failing source position. There is no
automatic retry/rollback or public HTTP/CLI/UI integration. Dropping the future
has no cancellation/abort or restore guarantee and may leave simulated resources
in Loading; no such safety claim may be transferred to hardware.

Absolute simulator allocations decode independent raw field offsets. They must
not reuse the encoder to manufacture expectations or return the generic relative
allocation address. An allocation changing a reference is not evidence of all
real allocation side effects.

## Recovery is not memory-only

The plan inventories connection authorization, Verify Mode setup, load-state/
property controls, absolute memory writes and task/allocator effects separately.
Restart is deliberately not representable in this first normalized subset; a
source adapter must refuse it until its lifecycle and authorization contract is
implemented. This inventory is
not a durable backup. Before any real path can be admitted, an identity-bound,
versioned, persisted/read-back receipt must cover every direct and indirect
mutation and an independently exercised restore method. Existing MemoryDownload
backup alone cannot meet that requirement. Missing property/allocator semantics
keep hardware refused, even when a synthetic simulator sequence passes.

## Evidence

Original Management Procedures v02.01.02 §3.31.3.4 pp139–141 defines the property
records; Resources load states constrain their ordering. Absolute data/stack
records keep Memory Type at byte7 (codes1–3) separate from Memory Attributes at
byte8 (bit7 disables checksum control); byte9 is reserved zero. Neither a
combined byte nor a zero-based0–2 type table is correct for these records.
Profiles v02.01.01
AnnexA.2.4.1 pp144–145 distinguishes global and resource-specific support.
Original Cookbook Load Controls v01.00.02 pp4–8 distinguishes tool instructions
from wire events, not modern XML flag mappings. See the existing
[prerequisites](system2-download-prerequisites.md),
[source-image contract](offline-download-system2-image.md) and
[whole-goal scope](offline-download-expansion.md).
