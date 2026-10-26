# UP258 System-2 image differential delivery

Observed: 26 October 2026.

## Delivered boundary

Published main-first merge `678082047` on base `9a2d880f1` with two feature merges:

- `c3075dfa0`: the System-2 CLI hardware execute/recover and observation commands
  use a small multi-thread tunnel runtime, so tunnel acknowledgements continue
  during synchronous checkpoint persistence (Tunnelling v01.07.01 §2.6.1, printed
  p. 9). A loopback regression test fails on the former current-thread runtime.
- `609f8041e`: UP258 defaults profile v2 writes the 15 independent source octets
  beyond the declared association-table capacity (775 written, 739 not written);
  profile v1 stays readable for simulator reproduction and recovery, and new
  physical execution with a v1 bundle refuses before any socket or checkpoint.
- `1d9fc4c4f`: [classification](../research/2026-10-26-up258-image-differential.md)
  of the remaining differences and the new open item
  [S2-03](../open-work/S2-03-up258-image-interpretation.md).

## Acceptance

- Independent cross-vendor review of the integrated change: accept, no findings.
  Its tunnel test could not bind loopback in the reviewer sandbox; that test ran in
  the gates below.
- Mutation controls: reverting the policy selection, removing the executor refusal
  or dropping the defaults-v2 id guard each fails named tests; the tunnel test fails
  when the runtime reverts to current-thread (20/20 green otherwise).
- Maintained complete package gate on the published merge: all 10 stages green,
  inputs frozen, protected user data untouched; Rust 6,924 passed with 230 ignored
  (not an execution of every ignored test), selected private corpus 155/155,
  browser 850/850.
- The same gate on an earlier merge (before `main` moved) was also green. A
  second attempt on the published merge failed one mocked-API browser case
  (`system2-download.e2e.ts:47`, five-second wait for the restored status) under
  heavy host load; that case touches no changed code, passed 10/10 in isolation
  and passed in the final complete gate. The failed receipt is retained privately.
- Simulator-only end-to-end check: four configured variants (defaults, one
  multi-octet parameter, added group addresses and associations, changed object
  flags) ran through the CLI and simulator gateway in an isolated loopback network
  namespace. All finished with verified write evidence; transmitted bytes equalled
  each prepared plan and the variant-to-variant changes matched the reference-tool
  oracle except the documented communication-enable policy.

## Boundaries

Simulator only; no bus, hardware, ETS-equivalence, release or deployment claim.
Gateway PID 56/83 values remain assumptions; PID 7 width (S2-01), TaskSegment
identity, the omitted program-version octets and physical recovery are unchanged.
Invisible-parameter precedence and the communication-enable policy stay open in
S2-03, as do the three remaining tunnel commands that persist between frames.
