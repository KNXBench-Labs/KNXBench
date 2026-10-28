# Restart snapshot

Observed: 2026-10-28 17:58 CET. Operational snapshot; maintained docs own status.

## WP-03 legacy enum binary retention — PUBLISHED, result-doc closeout GO
- Source `6dbdaf8b9` published main-first on `0154b8ae9`; implementation `68f9da61e`,
  review regressions `895f4f793`, docs/corpus expectations `89a3e5134`.
- Local integration HEAD, tracking and live main readbacks matched. Whole-feature
  and correction Opus reviews complete, no open findings.
- Branch14 and integrated14 accepted: Rust4494/165ignored, selected corpus97/97
  each, overlapping scope; frozen inputs, checked/unchanged user data. Focused17
  and separate legacy corpus1 green. No browser/bindings/hardware claim.
- Productdb write reservation RELEASED. Data-only v30 preserves historical facts;
  Atomic4 semantics, refused-backfill repair/retry and large-payload cost remain bounded.
- Own integration tree `<sources>/knxbench-wp03-integration-20261028`, branch
  `integration/wp03-enum-mapping-20261028`; result-doc gate/publication and cleanup next.
- Evidence `<profile>/evidence/wp03-{enum-mapping,implementation,review}-20261028/`.
  No bus, VM, user catalogue, hardware, release or deployment action.

## Issue #31 / #30 System-2 result integration — acceptance GO
- Observed 2026-10-28 18:41 CET: reviewed feature `d52ee440e`, focused unit104/browser12 green.
- Final correction review APPROVE; no open source blocker. No hardware acceptance inferred.
- Feature gate15 ACCEPTED in `<sources>/knxbench-issue31-system2-status-20261028`, head
  `d52ee440e`: Rust2794/133ignored, Vitest4640, browser947 +production109, corpus91/91;
  inputs frozen, checked user data unchanged. Evidence `<profile-scratch>/issue31-evidence/`.
- Own main-first integration `<sources>/knxbench-issue31-system2-integration-20261028`, branch
  `integration/issue31-system2-result-20261028`, parent `d8f4b3ca0`; source merge prepared.
- Next commit integrated candidate, run selected integrated gate15, then publish only after fresh
  freeze/owner check and exact local/tracking/live readback.
- #30/#22 allocation mismatch, per-part source equivalence and hardware outcome remain open.
- No bus/VM/user data/release/deployment permission; root intentionally not synchronized.

## Retained boundaries
- WP-08 source `4beabe531` / result `1329fb8e8` delivered; reservation released.
- Native WP-11 re-check needs fresh desktop consent. Root intentionally stale/dirty,
  preserved and not synchronized. No release/deployment/bus authority.
