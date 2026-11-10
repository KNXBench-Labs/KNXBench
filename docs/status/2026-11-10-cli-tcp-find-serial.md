# CLI find-serial over explicit TCP — 10 November 2026

Status: bounded software acceptance passed and runtime published on main. This follows the owner's next-open-work request under
[WP-29](../open-work/WP-29-knxnet-ip-tcp-tunnelling.md), not new hardware authority.
It is a main-source successor, not an Alpha 20 release addition.

## Scope and preserved policies

Both existing read-only `knx device find-serial` queries accept
`--tunnel-transport udp|tcp`: serial-to-address broadcast lookup and
`--address`-to-serial PID_SERIAL_NUMBER read. UDP stays the default; no fallback,
reconnect or replay is introduced. Gateway admission requires unicast IPv4 and a
nonzero port before history/runtime/contact. Missing, malformed or duplicate
transport/query options and unsupported write/key/serial flags refuse.

Selection precedes the diagnostic tap, retaining `SerialNumberAddress`. Ordinary
stderr identifies the opened transport and reports positive TCP skipped-frame
counts on success or error, sampled **before** consuming disconnect. Measured
TCP zero and UDP unavailable are distinct; neither invents a warning. Skips are
not asserted to be lost telegrams or final-wire totals. Matching device answers,
not socket-send acceptance, determine results. Metadata-only history, optional
consented/redacted diagnostic bundles, recording cleanup, exit policy and
best-effort tunnel disconnect remain unchanged. SIGINT retains existing process
and interrupted-history behavior, not a new graceful-settlement guarantee.

## Source admission and actual focused evidence

Both independent Standards and Spec source reviews passed without blockers on
the five material CLI source/test files. The optional Standards O1 finding was
adopted: only the rejected-handshake fixture's initial accept now follows its
existing nonblocking/deadline pattern. A separate narrow source closure passed
against all five final hashes; production and assertions are unchanged. These
are source approvals, not runtime/package acceptance.

The original focused attempt compiled successfully, then ran 40 selected
integration tests: **38 passed, two failed** at incorrect EOF-text oracles.
The existing error producer says `the connection was broken down during ...`,
not the expected `connection lost`. Its 14 selected parser/unit tests passed;
CLI all-target Clippy, four maintained Rust-layout tests, format and diff checks
also passed. Only the two text oracles were corrected, and exactly those two
leaves then passed. The source-bound composition covers **54 unique tests**
(40 integration + 14 unit); it is not a fresh 54-test run and does not relabel
the first attempt green. Previously unreached address-mode/history tails and
later skip rows receive credit only from the corrected run.

A separate reporter-only calibration actually returned **0 / 101 / 0** for
control / removed-call mutant / restored control. The mutant failed the exact
`missing ordinary positive skip warning` assertion on its first broadcast/EOF
row; the other three mutant rows were not executed. The reporter consumer,
complete oracle leaf and shared gateway are byte-identical after O1, so this
proof is inherited only for that unchanged context, not for a globally identical
final test file or binary. All original failures and source epochs are retained.

The eleven new maintained TCP leaves exercise real compiled CLI callers and
loopback peers: both read-only wire sequences and answers, management/tunnel
cleanup with independently observed EOF, no-fallback UDP traps, strict no-I/O
admission, refusal/EOF/no-answer/nonmatching replies, ordinary skip reporting
with recording disabled, bundle bytes/redaction/classification/temporary cleanup,
and SIGINT. Explicit and omitted UDP controls preserve the existing path.
Focused execution used isolated scratch homes, offline Cargo and loopback-only
network namespaces under the shared execution lease. No real installation was
contacted. Commands, native exits, frozen inputs, executed binary copies,
source archives and review identities remain private under
`<profile-scratch>/wp29-cli-find-serial-20261110/`.

## Integrated acceptance and publication

The focused source/documentation commit is `64effe8a02d4bae38f317eeae32eaff01f66788b`.
Fresh main-first runtime acceptance ran on `5b20e14a1a9307335b41abd61f1fecb375ff8386` against pinned upstream
`4a5325e383e7fecf88443ed0ebcdfc6315392848`. The actual maintained change-scoped package gate completed
**13/13 selected stages**, all direct exits zero with valid native proof:

`fmt`, `documentation`, `browser-lanes`, `check-layering`, `check-headers`, `check-anchors`, `check-ledger`, `check-corpus-gates`, `tool-tests`, `clippy`, `rust-tests`, `corpus`, `diff-check`.

The complete selected Rust suites for `knx-cli`, `xtask`
reported **659 passed, 0 failed, 22 ignored** across
4 native result blocks; result blocks are not asserted to be executable
counts. The separate selected private corpus ran **17/17**, nonzero,
with no missing-input skip. Tooling ran 563 tooling tests, with 16 existing opt-in/environment skips (not runtime or corpus coverage).
Web/browser, bindings, website and story stages were not selected; no acceptance
of those unselected surfaces is claimed or borrowed from older packages.

The direct gate process exited zero; native `complete`, `success`,
`planCompleted` and `inputsFrozen` are true, with equal start/end fingerprints.
Real user data was checked and unchanged, with no unchecked directories.
Additional before/after file identities and private-input metadata agree.
The gate used a fresh isolated Cargo target, scratch test homes, offline
Cargo and the maintained shared leases. The corpus stage alone linked private
fixtures and ran in a loopback-only network namespace; selected ordinary CLI
socket tests used loopback peers, not a real installation. Executed Cargo test
binaries and the compiled CLI were retained outside build caches with hashes;
source exports, raw native logs/receipts and earlier failed epochs remain private.

A normal, non-force push published runtime `5b20e14a1a9307335b41abd61f1fecb375ff8386`; local,
tracking and live remote main readback matched, including the five reviewed
material source identities. This result-only documentation successor preserves
the entire non-documentation tree (coordination checkpoint excepted), removes
only its completed active checkpoint and requires its own maintained
documentation-only gate before publication, without rerunning unchanged
Rust/corpus suites.
The first integrated gate attempt is retained as failed: eight static stages
passed, then the tooling suite refused its private starter's prematurely
canonicalized deep temp root (14 setup errors among 563 tool tests; 16 existing
opt-in/environment skips). No Clippy, Rust-suite or corpus stage ran in that
attempt. Only the private execution environment was corrected to retain the
approved lexical scratch ancestor; source and the temp guard were unchanged.
An actual formerly failing Chromium-launch case then passed (1/1), followed by
the new complete selected attempt reported above. The first attempt is not
relabelled green.

The definitive private receipts are `delivery/runtime-gate-02/receipt.json`,
`delivery/runtime-publication.json` and the separately bound documentation receipts
under the private evidence root above. No release or deployment was performed.

## Remaining limits

Only read-only find-serial is added. Device programming, address assignment,
download/restore, serial-port transport, discovery/routing, KNX IP Secure,
real-gateway TCP interoperability and broader WP-29 remain outside this package.
Loopback release witnesses do not establish physical device behavior or a new
cleanup exit policy. No real bus/device/LAN/serial/VM, release or deployment
operation is authorised or performed by this delivery.
