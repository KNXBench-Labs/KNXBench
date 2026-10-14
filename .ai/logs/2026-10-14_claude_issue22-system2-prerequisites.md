# Issue #22 — System-2 offline prerequisites

Reporter device: Siemens UP 258D12 (5WG1 258-2DB12), application
`M-0001_A-9205-04-9270`, mask `MV-0025` (BCU 2 / System 2). The download planner
refuses it as `not-memory-mapped` because it only admits 070nh / BIM M112 masks.

## Delivered (main 171a63be, feature eb0b277b)

- `LoadStep::TaskCtrl2 { lsm, callback, address, segment_0, segment_1 }`; only the
  exact five-attribute shape without children is typed, everything else stays
  `Unmodelled`.
- `knx_core::commissioning::load_control::{TaskControl2, task_control_2}`: ten-octet
  property record `03 05` + four big-endian words. No mask admission, no I/O.
- Memory planner refuses typed TaskCtrl2 (`Unmodelled("LdCtrlTaskCtrl2")`); mask
  gate unchanged.
- Contract: `docs/contracts/system2-download-prerequisites.md`.

## Evidence

TDD red→green for parser and codec; two mutants killed and reverted. Frozen merged
gate: 17/17 stages, Rust 4495/0, Vitest 3559/0, corpus 146/146 (includes the
ignored exact Siemens fixture run in release mode against the private corpus).

## Open

System-2 table/image contract, full product procedure (21 steps, machine 5,
property-78 compare), 0025 allocation profile, property+memory recovery, offline
acceptance, then authorised hardware validation with confirmed firmware.
