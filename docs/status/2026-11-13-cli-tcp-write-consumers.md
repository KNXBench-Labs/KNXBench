# CLI TCP service-control, ordinary download and restore — 13 November 2026

## Scope and state

Three bounded source changes enable explicit `--tunnel-transport udp|tcp` on:

1. `device service-control`: existing read and confirmed bit-2 write.
2. Ordinary `device download`: existing prepared memory-download procedure.
3. `device restore`: existing serial-bound backup restore.

The source candidate `9209f0fe1` is **software-accepted** after independent
Standards and Spec reviews (both APPROVE, no blocking findings) and the complete
selected 13-stage package gate. The gate passed 710 Rust tests (22 ignored) and
17/17 separately selected private-corpus tests, with frozen inputs and no touched
or unchecked user data. Web/browser execution was not selected for this CLI-only
change. This is not a release or hardware-interoperability result.
WP-29 remains open. No real bus/device, System-2 validation/preparation,
server/Web programming, other programming command or protocol algorithm changed.

## Behaviour

Omission stays UDP. Strict shared parsing refuses invalid, missing or repeated
transport values. Explicit gateways require unicast IPv4 and nonzero port before
input/key/history/runtime I/O, including plans. A plan-only invocation refuses
explicit transport selection rather than silently ignoring it; omit the option
to inspect the plan, including `download --changes`.

The existing client transport selector precedes the diagnostic tap; operation
kinds remain `ServiceControl` and `Download`. Ordinary stderr names the opened
transport and assigned address. Positive skipped TCP-frame counts are sampled
before consuming disconnect and disclosed exactly once on success or failure,
without recording. Zero stays silent; skips are not lost-telegram/final-wire
counts. Explicit TCP never falls back, reconnects or replays. Existing exact
confirmation/untested phrases, exclusions, key/history admission,
backup-before-mutation/readback, report/result/exit and cleanup policies remain.
Download rerun reconstruction retains explicit transport when supplied.

## Executed local checks

Package-batch implementation preceded execution. All network checks used only
loopback inside an isolated network namespace, through the built public CLI.
The maintained bounded peer reuses the production cEMI codecs and management
simulator; no copied protocol encoder or real-device behaviour is claimed.

Thirteen distinct focused test leaves have observed passes by source-bound
composition: ten unchanged leaves are carried from the corrected multi-consumer
run, and three complete-acquisition leaves passed against the final coherent
caller/peer source. The earlier enclosing run remains **failed** (11 passed /
2 failed), not relabelled green. Its last failures were inaccurate history
cleanup expectations (`confirmed` versus the producer's `returnedOk`), after
actual completed writes. The affected-only correction passed both completion
leaves and added literal load-control/final Loaded assertions. A final
self-review strengthened the peer's paired management-session teardown and
unexpected-mutation checks; all three affected acquisition leaves then passed,
including both zero/positive-skip rows and the remaining backup/no-retry/input
integrity assertions. No production behaviour changed in these test repairs.
The earlier oracle-only rerun used a byte-identical CLI producer; the final
commit-bound build additionally changes embedded revision metadata, with
unchanged production source.

The focused inventory covers:

- Complete service-control TCP reads and confirmed writes, each with zero and
  positive skips; exact original recovery properties, bit-preserving writes,
  Verify Mode bytes and reopened write history.
- Complete ordinary TCP download and restore with synthetic saved product/project
  and admitted backup inputs, each with zero and positive skips. Literal payload
  chunks/control records, original/current backup bytes, final memory/Loaded
  state, reopened history, orderly management/outer disconnect and peer EOF.
- EOF before acquisition for all three consumers (both service-control modes),
  positive skip disclosure once, failure/no-write outcomes and no replay.
- Refused TCP connections with retained bound sockets and UDP fallback traps.
  Default and explicit UDP controls deliberately refuse at CONNECT_RESPONSE:
  **handshake selection only, not completed UDP operations**.
- Invalid/missing/duplicate transport, invalid gateways before missing-input I/O,
  explicit plan refusal, confirmation/untested admission and exclusions obtained
  from the shared exclusion constant. Remaining address-programming consumers
  still refuse the option.

`cargo clippy --offline --locked -p knx-cli --all-targets -- -D warnings` passed,
as did `cargo fmt --all --check`. Commands used private Cargo target/scratch
homes, scalar Cargo force/value overrides and the shared workspace build lease.
The complete owning CLI suite and independent maintained package gate were not
run by the author, to avoid duplicating the integrating owner's acceptance.

Earlier setup/fixture failures are retained separately: simulator-trace enum
misuse (compilation), comparison-only fixture without unload/start/completion
controls, inaccurate exclusion/history text, and a malformed assertion that
prevented formatting before execution. No production safety gate was weakened.
Private raw logs retain argv, stdout/stderr, direct exits, source hashes,
qualified inventories and copies/digests of the executed CLI and test binaries.

## Independent review and complete selected acceptance

Both independent axes reviewed `35605e939...9209f0fe1` and returned APPROVE
without blocking findings. A private-archive discriminator removed only the
service-control skipped-frame report: the maintained EOF regression compiled
and failed at its expected warning assertion (0 versus 1). Original and exact
restored controls each passed. The mutant stopped in the read row; its later
write row is not independently credited as killed.

The maintained change-scoped package gate on `9209f0fe1` completed all 13
selected stages: formatting, documentation, browser-lane declarations, five
xtask checks, tool tests, all-target Clippy, complete CLI/xtask Rust suites,
selected private corpus and whitespace. All stages exited zero with valid
proofs. Rust: 710 passed, zero failed, 22 ignored. Separate corpus: 17/17.
Inputs remained frozen; user-data checks found no touched or unchecked paths.
No web/browser execution, workspace-wide Rust suite, bindings generation,
hardware access, release or deployment was selected or claimed. The earlier
failed author attempts remain failed and distinct from this successful gate.
Final result-only documentation is accepted separately against this gated head;
no product inputs change in that delta.

## Remaining limits

This adds transport selection to already-authorised software workflows, not new
write authority or proof that a gateway/device supports TCP. Timing, retry,
download and restore algorithms are unchanged. No general physical recovery,
KNX IP Secure, real-gateway compatibility, release or deployment claim follows.
Source-level software acceptance does not lift any of those remaining boundaries.
