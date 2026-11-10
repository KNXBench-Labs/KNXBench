# Server/Web TCP line scans — main-only software acceptance

Date: 10 November 2026. Runtime-tested and published revision: `ebee02cdf20f1e175ff0890b5b47c64092fceb3b`.
Source feature `12e4fd5ac6a957c8c5d9c3bdac9ea55f8ba49493` was integrated main-first
as `7fd0153d1e2c801084e4086dc00caea9dd7f273e` over `7e5612b62`. The runtime-tested
successor additionally corrects checkpoint vocabulary and a deterministic tooling
fixture; no scan/product logic changed after independent source closure.
The exact main ref was freshly read back after a normal fast-forward push.
This is not part of Alpha 20 and is not a release, deployment, real-bus operation
or real-gateway interoperability claim. Broader WP-29 remains open.

## Accepted scope and contracts

- Explicit scan-local TCP reaches the recorded transport and existing DD0 scanner,
  exclusions, deadlines and six outcomes. UDP remains the default independently
  of the monitor preference. Choosing TCP invalidates the estimate, not the bus.
- `start-tcp` is distinct from legacy UDP Start. Unsupported strict routes never
  retry UDP; actual tunnel provenance is checked before probing.
- Start/results/cancel require session ID, server incarnation, actual transport and
  skipped-frame count: measured integer for TCP, null for UDP. Bound cancellation,
  comparison and deliberate reconciliation retain the same exact identity;
  reconciliation still uses existing commands and undo.
- Server-owned tasks retain pending connect, joins and consuming cleanup despite
  dropped requests. Held reservations refuse rather than queue. Running includes
  cleanup; terminal publication follows disconnect settlement. Probe errors and
  cleanup uncertainty remain independent.
- Live skips come from the actual tunnel. Terminal counts are pre-disconnect
  samples, not final-wire totals, telegram loss or occupancy. Unknown/over-cap
  skipped bodies are not exported. Failed cleanup is not a confirmed disconnect.
- Navigation leaves scans server-owned. Explicit Cancel during pending Start
  targets only its eventual session/process pair. Late callbacks are fenced;
  malformed metadata locks actions, and a lost Start reply never authorizes retry.

## Independent review and calibrated privacy oracles

Both initial source-review axes blocked the unpublished WIP. Their original
reports remain historical BLOCKs. Corrections closed the post-ack/first-poll lost
Cancel window, hidden download-holder cleanup uncertainty, status coercion,
successor Start overlapping reconciliation and editing locked after a definite
no-effect refusal. Both independent bounded correction reviews passed source
closure; runtime acceptance is supplied by the composed native proofs below.

The real maintained HTTP/RealConnector/TCP/DD0 leaf passed on an isolated complete
copy of the merged source. Its test/plan/probe/counter bytes stayed fixed through
exactly two synthetic serializer-leak variants: a short `raw_payload` sentinel
failed the short-body assertion; an empty `raw_payload` failed the exact
metadata-field assertion. Each variant compiled with exactly one selected test,
reached its named native assertion, and was followed by a byte-restored passing
control. The first completed scenario fails in each mutant; its later cancellation
loop is unexecuted. These are serializer-oracle calibrations, not decoder-retention
bugs or hardware/device outcomes. All five executed binaries were retained.

## Integrated execution

The maintained change-scoped plan against fresh upstream `7e5612b62` has
**15/15 actual native stage proofs** by explicit source-bound composition:
13 successful prefix stages from the integrated attempt plus the corrected
native corpus and remaining diff-check tail. The original integrated receipt
remains failed/incomplete at corpus admission (41/51); it is not rewritten or
claimed as one all-green execution. Every credited native exit is zero with a
valid proof; source/HEAD and Web bundle identities are unchanged across the
prefix and tail, inputs are frozen, and checked real user data is unchanged.
The owning privacy/UI join was not substituted for this complete applicable plan.

| Owning evidence | Actual result |
| --- | --- |
| Rust selected packages (`knx-server`, `knx-desktop`) | 1841 passed, 0 failed, 72 ignored; 7 native result summaries |
| Private corpus, separate from ordinary Rust | 51/51 selected passed |
| Python tooling | unittest summary 563 tests, 18 optional/privilege skips, no failures |
| Nested Web gate | 46/46 planned stages complete |
| Web typechecks | app in build plus 27 derived fixture/config projects |
| Vitest | 5532 passed, no failed/pending tests |
| Default browser | 1168 total (1071 functional + 97 exclusive) |
| Production `communication` browser | 144 total (115 functional + 29 exclusive) |
| Production `device-download-support` browser | 78 total (78 functional + 0 exclusive) |
| Production `devices` browser | 71 total (70 functional + 1 exclusive) |
| Production `table-columns` browser | 180 total (176 functional + 4 exclusive) |

Browser lanes/configurations overlap and are not additive coverage. Nonempty
browser cases passed once, without retry/flaky/skip credit. Inventory-empty lanes
are disclosed by their native receipts, not described as executed tests. Ordinary
Rust ignored tests and optional tooling skips are not hardware acceptance.

A separate merged-source join used two actual backend capture files, each holding
`start`, `poll` and `cancel`, from the real HTTP listener through production app /
RealConnector to an isolated scripted TCP gateway and same-port UDP trap. The
byte-restored baseline provides the captures: two decoder skips, exact DD0 mask
`0x07b0`, addresses 5/7 with 6 excluded, bound cleanup and observed EOF. The actual
captured JSON entered API admission and UI rendering under both Default and
Communication selectors: 20/20 per selector. EN/DE, desktop/narrow, remount and
exact Cancel passed; fresh DE400 and EN desktop frames were visually inspected.
Authored negative controls are recorded separately from actual producer JSON.

The copy and merged candidate had complete tracked-source and ignored-bundle
identity at calibration/join time. The later checkpoint and tooling-fixture
corrections leave all product/backend/frontend inputs unchanged; the composed
acceptance plan covered their actual successor revision. This documentation-only successor
leaves the complete non-documentation tracked path set, modes and bytes unchanged
against the runtime-tested revision. It does not claim a new runtime binary build.

## Preserved failures and limits

- The first integrated gate stopped after format at documentation admission:
  `INTEGRATING` was not a valid checkpoint status. No later stage executed.
- The second passed static stages, then a tooling fixture failed because the
  scratch HOME correctly produced both home-directory and agent-profile privacy
  findings while the fixture assumed one. The checker was not weakened: only
  that test's synthetic home was pinned. Its owning 15 tests then passed under
  the same isolation. Web/Rust/corpus tails of that failed gate were unexecuted.
- The next integrated attempt passed all 13 prefix stages, including every Web
  lane, Clippy and ordinary Rust, but corpus admission failed 41/51. Retained
  failed executables and the helper library proved that a reused cache embedded
  the calibration copy's `CARGO_MANIFEST_DIR`; the runner correctly linked the
  corpus into integration, not that copy. The helper was rebuilt from integration
  without source/test/guard changes. Its actual maintained corpus targets then
  passed 51/51 and diff-check passed, with nonempty inventory, frozen source/HEAD,
  unchanged bundle/corpus inputs and checked real user data. These form the
  explicit additive acceptance above; the failed original remains failed.
- Earlier focused preparation retains its formatting-only refusal and a private
  capture-validator failure after the successful wire witness. The real producer
  emits two files containing three response objects each, not three files.
- The authenticated pre-correction replay was unpublished WIP, not published
  original-source RED. Initial failures, corrected checks, calibration variants
  and final integrated acceptance remain separate source-bound attempts.
- No settings/schema/dependency, protocol codec, programming/download/restore,
  serial, discovery/routing or KNX IP Secure behavior is widened. No fallback,
  reconnect, gateway-support, certification or ETS-equivalence claim follows.
- All socket witnesses and browser servers used loopback-only network namespaces;
  hardware environment selectors were cleared. Corpus content stays private.
  Release-cutoff and prior acceptance records remain unchanged. Parent owns final
  independent verification, private retention and owned-worktree cleanup; the
  shared checkout is deliberately stale/dirty and was not synchronized.

[WP-29](../open-work/WP-29-knxnet-ip-tcp-tunnelling.md) ·
[TCP contract](../contracts/tcp-tunnelling.md)
