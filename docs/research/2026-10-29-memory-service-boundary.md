# Memory service at the `FFFFh` boundary (DIAG-03 B) — 29 October 2026

Which memory service carries an allocation's writes, prestate reads and verification
reads, where the code chose it before this change, and what changed. Simulator and
in-process tests only; no device, bus or gateway was contacted.

## Spec evidence (from the work package, not re-derived here)

- **03_05_03 Configuration Procedures v02.01.01**, §3.5.2 (printed p43, repeated in
  §3.5.3 from p45): the service is chosen from base address plus *allocated* segment
  size; lower than `FFFFh` is `A_Memory_Write`, higher is `A_UserMemory_Write`; exactly
  `FFFFh` is not defined. §3.5 is a System B procedure.
- Same document, §3.5.1.4 Table 5 (p42): a client shall not use `A_UserMemory_Write`
  towards a device without memory above 64 k.
- **06 Profiles v02.01.01**, §4.2 matrix (printed p37): "DMA on User Memory" is not
  marked for profile class System 2 (mask `0025h`, Table 1 p13). No mask-`0025h`
  procedure section exists in this edition.

## Findings before the change (line numbers at `5cc4caf66`)

- `crates/knx-core/src/commissioning/memory.rs:235` `service_for` decides on
  `base + length` and returns `A_UserMemory_*` for anything reaching `FFFFh`, including
  the undefined equality. `chunks` (`:304`) calls it once per region, so one *region*
  never switches service. Session reads (`crates/knx-net/src/commissioning.rs:2441`) and
  region writes (`:3021`) inherit that per-region choice.
- Ordinary System B download (`crates/knx-net/src/commissioning/download.rs:1433`,
  `load_one_part`): the data write is the whole allocation, and every
  chunk's read-back uses the chunk's service. Write and verification therefore already
  shared one service per allocation; the equality case went out as `A_UserMemory_*`.
- System 2 (`crates/knx-net/src/commissioning/property_download.rs:256`): each
  `WriteMemory` step chose its service from *that write's* range, while the prestate
  capture (`property_recovery.rs:789`) and the final verification (`:496`), both through
  `read_region` (`:664`), chose it from the whole allocation. An allocation such as `FFF0h` + 16 therefore wrote with `A_Memory_Write`
  and verified with `A_UserMemory_Read` — two address spaces for one allocation, and a
  service mask `0025h` does not profile. The plan validator admits allocations ending
  at `10000h` (`crates/knx-core/src/commissioning/property_download.rs:206`, writes `:248`).
- Memory download of mask `070nh` (`crates/knx-net/src/commissioning/memory_download.rs`):
  steps are absolute regions, not allocations; write (`:1296`) and compare (`:940`) use
  the same region, hence the same service. Read-only probes behave the same and are pinned by
  `memory_download/tests/region_reads.rs` (`a_region_ending_exactly_at_ffff_is_user_memory_all_the_way`,
  `a_region_ending_at_fffe_stays_in_the_memory_service`).
- Declared-range diagnostic reads (`crates/knx-core/src/commissioning/declared_memory.rs:148`)
  already refuse any range whose end reaches `FFFFh` and use `A_Memory_Read` only.

## Change (line numbers at the commit introducing it)

- `crates/knx-core/src/commissioning/memory.rs:250` `allocation_service(base, allocated)`
  returns the service for one whole allocation, or `None` for base plus allocated size exactly `FFFFh`.
  `service_for`/`chunks` are unchanged.
- `download.rs:1446`: after the allocation and its base read-back, a part whose base plus
  length is exactly `FFFFh` is refused with `DownloadError::UndefinedMemoryService`
  before its first memory write. The base is device-assigned, so the allocation and
  start-loading writes necessarily precede the check. Below and above the boundary the
  behaviour is unchanged (above it the existing path already writes `A_UserMemory_*`).
- System 2: `property_download.rs:509` calls `property_recovery.rs:845`
  `refuse_beyond_memory_service`, which refuses any absolute allocation whose base plus
  length is not below `FFFFh` (`PropertySimulationFailure::MemoryServiceBoundary`),
  before the session opens. No frame is sent and no journal event is emitted. Every
  remaining allocation, and every write inside it, is then `A_Memory_*` for write,
  prestate read and verification alike.

## Tests

- `knx-core` `memory::tests::an_allocation_ending_exactly_at_ffff_has_no_defined_service`.
- `knx-net` `download::tests::one_allocation_keeps_one_service_on_both_sides_of_ffffh_and_refuses_it_exactly`:
  base `F000h` with lengths ending at `FFFEh` (all `A_Memory_*`), `FFFFh` (refused, no
  memory frame) and `10000h` (all `A_UserMemory_*`).
- `knx-net` `it::system2_memory_boundary`: ending at `FFFEh` runs prestate read, write and
  verification with `A_Memory_*` only; ending at `FFFFh` or `10000h` is refused with the
  simulator recording no frame at all, for the experimental and the plain simulator entry.

## Open

- Table 5's "no memory above 64 k" condition is not checked by the System B path; it
  needs a device capability source this code does not have. Not invented here.
- The `070nh` memory download keeps `service_for`'s answer for a region ending exactly at
  `FFFFh`; §3.5.2 is a System B clause and was not applied to that mask without its own
  evidence.
