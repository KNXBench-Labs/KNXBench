# Restart snapshot

Observed: 2026-10-28 17:06 CET. Operational snapshot; maintained docs own status.

## WP-03 legacy enum binary retention — reviewed, acceptance GO
- Own `<sources>/knxbench-wp03-enum-mapping-20261028`, branch
  `research/wp03-enum-mapping-20261028`, base `0154b8ae9`, implementation `68f9da61e`.
- Productdb/migration reservation belongs here; v30 is data-only. Strict Atomic5
  binary retention plus authenticated retained-source backfill; Atomic4 remains unknown.
- Whole-feature Opus review found no production bug; three mutation-proven test gaps
  and two doc findings closed. Independent correction review APPROVE, no open findings.
- Focused17/0failed/0ignored and all-target Clippy green; earlier full Productdb1684/
  33ignored, separate corrected private legacy corpus1 green. Scopes overlap.
- Next: freeze/commit reviewed corrections/docs, run branch14, main-first integrated14,
  conditional publication/readback and owned cleanup. No package acceptance claimed yet.
- Evidence `<profile>/evidence/wp03-{enum-mapping,implementation,review}-20261028/`.
- No bus, VM, user catalogue, hardware, release or deployment. Failed backfill retry,
  unmatched conversion equivalence and large-payload cost remain bounded.

## Foreign Issue #31 / #30 System-2 result integration — preserve owner
- Shared root observation 2026-10-28 17:01 CET: GO, own branch
  `fix/issue31-system2-status-20261028`, base `0154b8ae9`; no takeover.
- Its server/web/application result-status scope excludes productdb/migrations,
  protocol and device writes. Correction2 running after review2; final acceptance pending.
- Preserve its tree and evidence `<profile-scratch>/issue31-evidence/`.

## Prerequisite and retained boundaries
- WP-08 source `4beabe531`, result docs `1329fb8e8` published; its productdb
  reservation is released. Date image/native editor and historical repair remain open.
- Native WP-11 re-check needs fresh desktop consent. Prior deliveries are in maintained
  status docs and private evidence, not active work inferred from old entries.
- Root intentionally stale/dirty and preserved. No release/deployment/bus authority.
