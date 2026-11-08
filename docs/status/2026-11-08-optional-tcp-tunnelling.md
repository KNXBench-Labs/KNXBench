# Optional TCP tunnelling: bounded software acceptance, 8 November 2026

## Outcome and scope

Main-only optional TCP is implemented for the library, `knx bus monitor`,
`knx bus write`, and the shared server/Web monitor with its bound group
read/write actions. UDP remains the default. Transport selection is explicit;
refusal does not silently fall back to UDP. Session status identifies observed
transport rather than merely repeating a saved preference.

This closes this bounded software increment, **not the whole WP-29**. Device
programming, scans, discovery and routing retain their existing UDP paths.
Plain TCP is not KNX IP Secure, proof of an S2-04 fix, or verified physical
interoperability with any gateway. No live bus, hardware, tag, release or
 deployment operation was performed for this acceptance.

## Source and independent review

Feature `c6c44f2e3` was integrated main-first on `f9cfa27be`, followed by the
browser-lane metadata correction `7d0161f31` and scoped Web closure `6b27d5b1a`.
The independent execution-backed review approved integration after targeted
compiled semantic mutations, baseline/restored controls and an inert control.
The parent verified the frozen 4,317-path source manifest and ten binary digests.

The original MUT6 review receipt has a failing text oracle. It is retained,
not rewritten as passing: the parent adjudication inspected the intended
assertion failures and restored control separately. Nine targeted semantic
mutations were killed; compiler failures and inert changes were not counted
as semantic kills.

## Maintained gate actually executed

The change-scoped native package gate ran on `6b27d5b1a` against the explicit
upstream base `f9cfa27be`, completing on 8 November at 18:51 CET:

- All 15 selected stages completed with direct zero exits and valid proofs.
- Inputs remained frozen; checked user data was untouched, with none unchecked.
- Nested Web gate: 37 stages, 5,402 passing unit tests, the selected default and
  production communication browser lanes, build and TypeScript checks.
- Selected Rust suites: 4,553 passed, zero failed, 153 ignored across 17 summary
  rows. This is **not a full-workspace Rust claim**.
- Separate private corpus run: 91/91 passed in five targets; not part of the
  preceding Rust pass count.
- Tool suite: 560 tests, including 16 explicit skips; no skipped test is claimed
  as executed coverage.
- Desktop build check, strict Clippy, formatting, layering, headers, anchors,
  ledger/corpus contracts and whitespace checks passed.

The actual CLI and server executables both returned exit zero for `--version`,
identifying `0.1.0-alpha.19+g6b27d5b1a`. Their executed bytes were retained and
hashed separately. This development version does not create a new release.

## Moving-main and documentation closure

Upstream then advanced only by community commit `604eb50b3`: the generated
README scoreboard and its two `docs/community` records. Its actual patch was
inspected. Main-first recomposition `9fbdefbe7` has first parent `604eb50b3` and
retains the accepted source as its other parent. An alternate index applied
the raw upstream patch to the accepted tree and produced exactly the candidate
tree; the deliberately mismatching accepted-only tree was rejected.

The result-only documentation successor changes no runtime, build, dependency
or asset input from the accepted source. Its own selected delta gate is required
before conditional publication; the immutable full gate above is not relabelled
as a run on that successor. Private raw receipts, patches, reviews and retention
records live under `<profile>/evidence/2026-11-08-wp29-tcp/`.

## Failed attempts retained

- Earlier HTTP test compilation, empty admission selection and strict-lint
  failures were corrected and followed by nonempty successful checks.
- The first private TCP browser run used an incorrect button locator. The next
  run passed 16 development and 16 production cases but its outer supervisor
  failed to report through an unopened pipe descriptor. Product results and the
  outer failure remain separate; later native Web acceptance passed normally.
- Native attempt 1 failed because the real-time TCP cases lacked exclusive-lane
  tags. Native attempt 2 failed two Web unit checks: companion API inventory and
  notice/CSS coverage. The scoped three-file correction passed its 37 targeted
  tests; native attempt 3 is the successful full selected gate above.

## Boundaries that remain

TCP socket acceptance is not KNX device delivery. TCP has no tunnelling ACK;
heartbeat monitoring is retained. Unknown/oversized frames are drained with
separate skip accounting, not silently equated to lost telegrams or buffered-row
drops. Diagnostic review pins its skip count so later traffic does not change
an earlier reviewed text/ZIP result. No automatic reconnect or replay is added.

The retained KNX IP Secure limitation and source-ID dispositions are unchanged.
Real-gateway validation requires a separately authorised activity. The owner
requested completion of this package followed by a stop for release preparation;
this record grants no automatic next-package or release authority.

[Contract](../contracts/tcp-tunnelling.md) ·
[Independent source basis](../research/2026-11-08-knxnet-ip-tcp-tunnelling.md) ·
[Remaining WP-29](../open-work/WP-29-knxnet-ip-tcp-tunnelling.md)
