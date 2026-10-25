# Backup application identity acceptance — 25 October 2026

## Published source

`08db292f3` is published on `main`, with exact remote and tracking-ref readback.
Its main-first parents are upstream `4b77a6cea` and feature `ea18f289f`.
Upstream README and community updates are preserved. Reviewed runtime inputs
match the integrated source; the integration also updates the handbook,
contract index and status descriptions.

The [contract](../contracts/backup-application-identity.md) defines one bounded
restore safeguard. A backed-up execution records the actual pre-write
application identity, not the desired plan identity. The lowest restore
planner refuses different, unknown or unrecorded evidence unless the actual
plan retains the exact pre-mutation identity precondition used by the legacy
partial route. Recorded conflicting evidence always refuses. Complete
downloads remain possible over a different or unreadable old application.

## Review and regression evidence

Independent persistence/consumer review approved the source. Core review
requested a tighter position check in the public restore derivation, corrected
read-traffic documentation, and error/ordering coverage. The sole network
executor already rejected late checks before any traffic; the correction
closes a weaker planning contract, not a demonstrated bus-write bypass.
Finding-specific rereview and the main-first join review approved the result.

The original regression demonstrated old application X's bytes being accepted
under new application Y's load procedure. Its original test, failed output and
executable digest are retained; that historical executable was not preserved,
and a later binary is not substituted. The separate late-check correction has
retained, digest-verified RED and GREEN executables. Transport and lagged errors
are tested through a simulated read path; encode-error classification is a
pure-helper test, not a claim of an exercised wire fault. Source-reasoned wrong
variants in the review are not credited as executed mutation tests.

## Final integrated acceptance on `08db292f3`

All 16 stages selected by the maintained package runner passed in their exact
planned order. Start/end source commitments match; touched and unchecked
user-data lists are both empty.

- Rust selection: 6,556 passed, zero failed, 225 ignored across 49 result blocks.
  This is the change-scoped owning/reverse-dependent selection, not a new full
  release gate. Focused predecessor runs overlap and are not added to it.
- Private corpus: 155/155 passed across nine selected targets in a loopback-only
  network namespace. The CLI binary ran all nine download cases; the server
  integration target ran 49/49, including all 29 HTTP download cases. Those
  actual producer tests inspect the durable backup evidence, rather than
  merely compiling ignored assertions.
- The real CLI restore tests ran, including same-application plan display and
  other/unrecorded-application refusal before the supplied loopback connector.
- Formatting, Clippy, documentation, local-information, layering, headers,
  anchors, ledger, corpus registration, tooling, generated-binding freshness
  and final diff checks passed. The selected desktop build prerequisite was
  present; this is not interactive native-desktop acceptance.
- Browser and web execution were not selected by this package's maintained
  plan; no new browser or visual acceptance is claimed.

## Compatibility and remaining boundaries

New backups use strict format 2 with an explicit nullable prior identity.
Format 1 remains readable without rewriting or inventing evidence. Complete
backups without evidence and without an exact executable identity precondition
refuse restore; legacy partial backups retain their existing exact check.
A format-1-only build refuses format 2, so downgrade is not bidirectional
compatibility. A format-2 `null` likewise grants no restore evidence. The
[handbook](../manual/user-guide/10-command-line.md#other-device-commands) and
contract describe these operator-visible limits.

The safeguard does not prove allocation compatibility, physical-exemplar
identity, the restored final application identity or whole-device recovery.
Existing family/recovery admission and partial-to-full refusal stay in force.
No real bus operation, hardware acceptance, release or deployment occurred.
KNX Secure remains excluded. [PRG-01](../open-work/PRG-01-programming-gaps.md)
stays open for the broader programming gaps.
