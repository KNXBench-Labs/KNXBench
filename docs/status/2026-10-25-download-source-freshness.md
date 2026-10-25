# Download source-freshness acceptance — 25 October 2026

## Published source

`fa4c18213` is published on `main`, with matching live and tracking-ref readback.
The main-first parents are upstream `ec766a9b2` and feature `33551a563`.
Upstream issue #30 web changes and community updates are preserved unchanged.
The [contract](../contracts/download-settlement-source-freshness.md) defines the
bounded guard; [PRG-01](../open-work/PRG-01-programming-gaps.md) remains open.

A completed regular memory download replans its original written request with
its selected scope using the current catalogue, under the same read lease as
the current project request. Only an identical executable plan can credit the
written memory parts. A changed selected plan or unavailable/refused source
leaves them pending and forgets their undo/redo programming claims. Ordinary
project edits retain existing per-part settlement. No partial-to-full fallback,
new device-family admission, schema, endpoint or bus operation was added.

## Review and regression evidence

Independent logic, persistence and main-first integration reviews approved the
source. Small test/documentation findings were closed without runtime changes:
unreadable owners assert no address or memory credit, reopened partial undo
explicitly preserves unselected claims, and executable-plan/snapshot limits
are stated without confusing activity history with undo history.

The retained original RED executable was replayed by the parent: the actual
server completion callback incorrectly credited all parts after a source
replacement producing a different plan for the same request. It failed the
intended assertion (exit 101). Retained final GREEN execution passed all 32
programming-state tests, including 29 source-freshness cases. The final merged
gate executes those 29 named cases again; overlapping predecessor runs are
not added to its counts. An earlier no-forget experiment lacks a retained
mutant binding and is not credited as source-bound mutation proof.

Persistence tests use the real save-as, settlement working-journal write,
fresh native-project open and undo/redo routes. They do not claim an explicit
save or hardware state. Source fixtures model admitted preparation using a
synthetic program and direct hardware mapping, not package-ingestion validity.

## Integrated acceptance on `fa4c18213`

All 14 stages selected by the maintained change-scoped package runner passed
in the planned order. Start/end source commitments agree; touched and unchecked
user-data lists are empty.

- Rust: 2,567 passed, zero failed, 122 ignored across 14 result blocks.
- Private corpus: 91/91 across five selected targets, loopback-only network
  namespace, including all 29 HTTP download cases and all nine CLI download
  cases. Other ignored tests are not silently counted as executed.
- Formatting, Clippy, documentation/local-information, layering, headers,
  anchors, ledger, corpus registration, tooling and final diff checks passed.
  The selected desktop build prerequisite was present; this is not interactive
  native-desktop acceptance.
- Browser, web and generated-binding execution were not selected by this
  maintained plan. No new visual, release or hardware acceptance is claimed.

## Remaining boundaries

This is a settlement snapshot, not a subscription to later catalogue changes.
Parts not written by this run keep their marks. Equality concerns the selected
executable plan, not inference annotations, warnings or identity outside it.
It is not device-byte, runtime, physical-exemplar or complete-recovery evidence.
An unreadable catalogue cannot supply even the current request and credits no
part. Transfer/restart/post-check and activity outcomes remain separate facts.
No real bus operation, deployment or release occurred. KNX Secure is excluded;
parameter policies, comparison exclusions and broader recovery remain open.
