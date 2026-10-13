# 2026-10-13 — Claude — KL-170 base-image warning, issue #10 probe request

Branch `feature/kl170-base-image-warning` (worktree `kl170-issue10`).

## Owner decisions (2026-10-13)

- Ask issue #10's reporter now to run the probe, with build-from-main steps.
- Named warning in the download plan for BigEndian/undeclared programs whose base
  images look little-endian; downloads stay allowed.
- Tester verification flow in the device download tab; reports via issue or
  issue@knxbench.com (working mailbox, tester reports only); reports include
  parameter values, no project/device names, no address.

## Issue #10

Flow verified locally before posting: `cargo build --release -p knx-cli`
(the Docker image carries only the server), `knx import` of a project with
LittleEndian devices (isolated HOME/XDG; default product DB), then
`knx device byte-order-probe` planned and stopped at the absent gateway
(exit 1, "could not connect"). 0.2.23 is not on the exclusion list (only
1.1.220). Comment: issuecomment-6138026344. Pitfall: exporting HOME/XDG in the
shared terminal session broke the `gh` wrapper until reset.

## Warning (ADR-0120)

- Measured on 929 packages (105 local, 824 public < 6 MB), release build:
  1,623 programs, 519 image builds at defaults, 181 memory-downloadable;
  32 warned (ABB 23, BJ 9, all 0701h), 0 MDT/Siemens/other. 338 "unchecked"
  results were all non-memory masks that `prepare_device_download` refuses
  first, so the measurement selection, not the rule, produced them.
- Verified MDT program: 1 discriminating default, high octet first.
- 11.8 s for 519 checks, max 2.0 s (one large program).
- Mutants 4/4 killed. First gate: inputs fingerprint changed because
  origin/main moved during the run (another session pushed); recomputed
  against the fixed base it matched, then rebased (IMPLEMENTATION_STATUS
  conflict: both entries kept) and gated again on c74bbc8c, fingerprint now
  against HEAD.
