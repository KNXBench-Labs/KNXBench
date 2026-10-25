# Memory-region read acceptance — 25 October 2026

## Published source

`a4a4ebcf1` is published on `main` with matching live and tracking-ref readback.
Its main-first parents are upstream `82a4a5d34` and feature `007dea532`.
Upstream community changes and the separate System-2 reservation are preserved.
The [contract](../contracts/memory-region-read-service.md) defines the correction.

Backup, comparison and byte-order-probe reads select one service for the entire
requested region and keep user-memory chunks within their 15-octet wire count.
The existing exclusive-end service policy and standard-memory budget stay intact.
There is no mutation resend, fallback, device-family or path-budget promotion.

## Regression and review evidence

Independent source review approved the two source/test paths. The parent
replayed the retained original and corrected binaries with identical eleven
test-source methods: original three passed/eight failed (exit 101); corrected
eleven passed (exit 0). The original failure includes an actual cEMI encoder
rejection of count16 and a literal default12 request-service mismatch.

The synthetic transport wrapper runs every outgoing request through the real
encoder before forwarding it; the simulator alone bypasses encoding. Tests pin
literal schedules, patterned returned bytes, short/refused responses, empty
runs, boundary comparison and the backup keeper barrier before WriteMemory.
They do not prove complete product-source preparation, a full application load
procedure, durable backup file persistence or hardware address-space aliasing.

## Integrated acceptance on `a4a4ebcf1`

All 14 maintained change-scoped package stages passed in the selected order;
start/end source commitments agree and both user-data guard lists are empty.

- Rust: 3,145 passed, zero failed, 131 ignored across 17 result blocks, including
  all eleven named region-read regressions. Predecessor focused counts overlap
  and are not added to this total.
- Private corpus: 91/91 across five selected targets in a loopback-only namespace,
  including all 29 HTTP download cases and all nine CLI download cases.
- Formatting, Clippy, documentation/local-information, layering, headers,
  anchors, ledger, corpus registration, tooling and final diff checks passed.
- Browser/web and generated-binding execution were not selected. The desktop
  build prerequisite was present; no interactive desktop acceptance is claimed.

## Boundaries

No real device or bus was contacted. The existing family/recovery gates and
partial-to-full refusal remain. Interface/coupler budget discovery, arbitrary
20-bit-run admission and hardware validation are not supplied by this fix.
KNX Secure stays excluded. [PRG-01](../open-work/PRG-01-programming-gaps.md) and
[KL-007](../limitations/KL-007-bounded-commissioning-support.md) remain open.
