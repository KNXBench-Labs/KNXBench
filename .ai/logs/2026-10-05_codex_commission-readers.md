# CLI serial-lookup read lifecycle — offline candidate

Date: 2026-10-05
Agent: codex
Worktree: `commission-readers-20261005`
Base: `9edad0a010cb6a8a8447571e6a0d39d8048b218c`

## Change

Optional CLI history for both serial-lookup directions, using the existing
shared `serialLookup` lifecycle rather than creating another engine or kind.
Explicit storage failure refuses before the adapter; metadata start is persisted
before connecting. Terminal connection failure is recorded. Cancellation/process
interruption keeps uncertainty, not a guessed outcome. Legacy no-option behavior
and existing disconnect diagnostics are unchanged. Journal failure after a lookup
is reported separately and produces a failing CLI exit.

The format-2 validator requires `serialLookup.address` to be null in both
directions. The first positive attempt exposed the initially incorrect known-address
assumption in the candidate/test; it was refused before the adapter. Canonical
candidate/tests now preserve the existing closed domain/client contract. No new
protocol or Web source was introduced. CLI candidate version is alpha.3.

## Executed evidence

- Parser RED: compile0, exactly one registered test, runtime101 at the intended
  explicit-history acceptance assertion.
- Pre-adapter persistence RED after parser support: compile0, exactly one registered
  test, runtime101 at the intended missing durable start assertion.
- Five-stage branch GREEN: fmt; strict CLI all-target Clippy; selected CLI bin and
  two integration binaries117/0/8 across three result blocks; actual version smoke
  `knx 0.1.0-alpha.3+g9edad0a0`; whitespace. Source manifest stayed unchanged.
- Five new read tests passed: both directions/first request/interruption/privacy;
  foreign-file and directory-history refusal with evidence intact; duplicate option
  refusal; rejected fake loopback connection produces a terminal failed row; usage.
- Permanent summary: `docs/evidence/cli-serial-read-lifecycle-2026-10-05.json`.
  Full local receipts: profile scratch `iaw/commission-readers-20261005/`.

## Sibling read admission review and current GREEN

The existing service-control read caller could continue to a tunnel after its
explicit history failed. A third source-bound RED compiled/listed one test and
failed101 at the unwanted-adapter assertion. Storage admission and persisted-start
status now refuse before adapter acquisition. Its existing terminal warning/exit
policy remains unchanged; serial-lookup late recording failure has its own stricter
exit policy. No write/protocol/Web behavior changed.

Historical branch-green-2 five-stage acceptance is118/0/8 in three blocks, including
new read6/0/0, fmt/strict Clippy/actual alpha3 smoke/whitespace, source frozen.
The earlier117/5-test receipt remains historical, not retagged. That snapshot summary is retained separately; the current permanent
summary: `docs/evidence/cli-read-lifecycle-2026-10-05.json`. Documentation gates
previously passed186 ledger rows and453 links/277 Markdown files against the
correct reader worktree; final post-update documentation checks remain to run.

## In-session review — finding before correction

IMPORTANT R1: `apps/knx-cli/src/main.rs:2989` marks `Ok(None)` as a failed
serialLookup, while `device_serial.rs:337` defines it as a completed "no such
device" observation and `apps/knx-server/src/serial_address_routes.rs:278`
records any `Ok` as finished. The CLI's existing nonzero no-match exit should
remain unchanged, but lifecycle metadata must distinguish a completed empty
result from a transport error. The no-match regression compiled/listed one test and failed101 at its intended
metadata-state assertion in no-match-red-2, then the caller changed only its
completion predicate to is_ok. The preceding compile-only refusal used the wrong
namespace for tunnelling constants; it is not an admitted RED. The direct GREEN
harness with no parent-netns variable refused before Cargo; it is not a test result.
The corrected source-bound branch-green-3 is119/0/8 (read7/0/0) and
workspace-rust-3 is3225/0/177 in178 blocks, all namespace/source checks true.
Both real CLI no-answer text and exit1 are unchanged. R1 is closed by that named
CLI/runtime/persisted-row test; no independent review is claimed.

The additional Rust workspace first refused before tests because its fresh
worktree lacked the required real frontend dist resource. A new attempt ran actual
`npm ci --offline` and Web build, then cargo workspace under distinct verified
loopback-only netns:3224 passed/0 failed/177 ignored in178 blocks. Source/Web
hashes stayed unchanged; generated TS exports landed outside the frontend tree.
This is cached-target Rust branch evidence, not current-main/private/browser or
final acceptance. Earlier failed attempt retained, no placeholder dist fabricated.

## Final corrected-source guard controls

Three unique source controls compiled0/listed exactly1/runtime101 at the named
assertion: no-match terminal misclassification, omitted pending start, and the
service-control explicit-history admission class (both layered guards disabled
together). Tests unchanged; both shared leases held before mutation and throughout
restoration. All canonical source bytes restored; the actual CLI recompiled and
all seven reader tests passed7/0/0 afterward. This is three contract controls,
not a claim that either redundant service admission guard independently fails
the same test. Full branch source hashes still match GREEN3/Workspace3.

## Final current-source branch checks

Final strict workspace all-target Clippy and four fresh-target repository gates
passed: layering; headers507 well-formed/157 without (ceiling157)/17 generated
skipped; anchors453 links/277 Markdown; ledger186. Whitespace passed. The fresh
xtask binary audits this actual reader worktree, not a cached deleted checkout.
Static added-source security scan found no flagged patterns; Web diff empty.
Current branch gate119/0/8 and workspace3225/0/177 in178 blocks are separate
scopes, not summed. Review is in-session, R1 corrected; independent acceptance
and latest-main integration/publication remain open.

## Boundaries and remaining work

This is a local source-hash-bound candidate, not source acceptance of the
base commit itself. A full offline Rust branch workspace passed on the current source, not an
integrated/publication acceptance. No private-corpus or browser gate; no independent
review, integration, publication, hardware, ETS, vendor or power-loss claim.
No-option CLI remains unjournaled; additional callers/long sessions and broader
abort/restore/client contracts remain open. Recovery commit9edad0a0 and original
stash9d14c93c are preserved; root checkout is not synchronized. Main69c2573d was
fetched during preflight: Web lock reported free, but the user prohibition on own
Web source remains binding. Canonical drag-E2E correction/integrated publication
still require owner action or the specifically requested file permission.
