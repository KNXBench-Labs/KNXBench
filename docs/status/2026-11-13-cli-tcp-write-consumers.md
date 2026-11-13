# CLI TCP service-control, ordinary download and restore — 13 November 2026

## Scope and state

Three bounded source changes enable explicit `--tunnel-transport udp|tcp` on:

1. `device service-control`: existing read and confirmed bit-2 write.
2. Ordinary `device download`: existing prepared memory-download procedure.
3. `device restore`: existing serial-bound backup restore.

This is author-side local verification, **not independent package acceptance**,
publication, a release or hardware interoperability. The integrating owner owns
independent review, the complete owning CLI suite and final package acceptance.
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

## Remaining limits

This adds transport selection to already-authorised software workflows, not new
write authority or proof that a gateway/device supports TCP. Timing, retry,
download and restore algorithms are unchanged. No general physical recovery,
KNX IP Secure, real-gateway compatibility, release or deployment claim follows.
Independent review and the integrating owner's final acceptance remain required.
