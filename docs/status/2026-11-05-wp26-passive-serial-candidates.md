# WP-26 passive USB-serial candidates — 5 November 2026

## Delivered capability and boundaries

Published main-first successor **`edc6c90d5`** adds an explicit, passive Linux
server/container-local USB-serial candidate list to the existing shared settings,
status-bar and monitor transport fields. `GET /api/bus/serial-candidates` inspects
bounded by-id symlinks and metadata, never opens/probes a port or discovers IP
interfaces. Only an explicit row action changes the existing path preference;
source, mode, transport and session are retained and Connect remains separate.

These are unverified candidates, not confirmed KNX adapters, permission or
availability evidence, physical-device identity or a complete USB census. Missing,
unreadable, unsupported and successful-empty states remain distinct. Manual paths
remain supported. No real host enumeration, hardware, bus, native GUI, release or
deployment action was performed for this package; earlier C6 witnesses are separate.
[Exact contract and primary source](../research/2026-11-05-passive-serial-candidates.md).

## Source-bound acceptance, not a relabelled failed run

The selected package requires 15 stages. Complete Web acceptance ran on main-first
`b3d1a0a7c`; that package later failed Clippy on four missing test-fixture fields and
one new test's initializer style. Its failed package outcome remains failed.

`edc6c90d5` changes exactly five Rust test files relative to that merge. Four
explicit fixture insertions are byte-authenticated outside the inserted field;
the fifth is an initializer-style correction. No assertions or production source
changed. Every other tracked blob/mode, applicable Web input, tool identity and
retained frontend build is unchanged. The final maintained 14-stage native delta
passed on `edc6c90d5`; together with the unchanged complete Web proof it covers all
15 required stages. Both completed phases are frozen and their owning user-data
checks are clear. The original frontend build keeps its `b3d1a0a7c` identity; it is
not represented as a new frontend build at the Rust-test-only successor.

Executed populations, not additive across overlapping runs/lanes:

- Scoped `knx-server`/`knx-desktop`: **1,748 passed, 0 failed, 72 routinely ignored**.
- Separate selected corpus: **51/51**, in two `knx-server` target groups (2 and 49).
  This is not a full-workspace corpus run or execution of all 72 ignored cases.
- Full Vitest population: **5,169/5,169**.
- Selected development browser inventory: **1,021 passed**, no unexpected/flaky/skipped cases.
- Selected production-communication inventory: **95 passed**, no unexpected/flaky/skipped cases.
- The final server was actually built under the shared lease and its retained
  original executed with `--version`: **`knx-server 0.1.0-alpha.19+gedc6c90d5`**.

Retained executed-server SHA-256:
`1ca0f7ded548630d1536a05c61275f2d731de5d8a8d68bac68f70da68c17073b`.
Raw receipts, source/argument bindings, reports and original executables stay private.

## Review and regression closure

Whole-feature review found the focused button's native-loading-disable dismissing
the footer and an unhandled legacy-password fixture request. Both were corrected
and independently re-reviewed. A synchronous in-flight guard preserves focus and
one request during pending activation; real external busy disabling is retained.
Focused EN/DE 400/1440 development and production cases passed on their original
corrected-source binding and are not added to the final populations above.

The shared read-only companion graph/API admission was independently reviewed;
an injected project `undo()` still failed the exact mutation census, with green
harmless/restored controls. The final five mechanical Rust test-only corrections
were parent-reviewed, not represented as a new independent whole-feature review.
The composed-receipt guard passed a real completed positive control and rejected
seven explicitly synthetic negative receipt fixtures. Earlier branch, browser and
Clippy failures remain retained as failures, not acceptance of later sources.

This closes the bounded passive-list successor only. Wider USB protocols, complete
enumeration, firmware/physical compatibility and broader WP-26 hardware acceptance
remain outside it. There is no new release, deployment or real-installation authority.
