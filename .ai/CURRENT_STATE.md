# Restart snapshot

## Alpha 18 release freeze (ACTIVE, observed 2026-10-28 23:05 CET)
- Release owner: this WebUI; source cutoff `bb2d8e51efcbd029034c694ab4da9638f80961ee`.
- Prerequisite delivered: feature `96f41c7f1`, main `309384726`, result docs `bb2d8e51e`;
  both runtime trees/refs retired; no matching cwd worker or package gate remains.
- Candidate branch `release/a18-prep-20261028`, tree `<sources>/knxbench-a18-release-20261028`.
- `main` stays release-frozen for code until the tag is published. Other packages stay local;
  no branch/main pushes by other tracks. Later work belongs to the next release.
- GO: finish notes/docs/version bump, full local release gate and packaging dry runs.
- STOP before tag, GitHub release or Docker publication: explicit owner GO still required.
- Draft reviewed; project/core16 unchanged, productdb28->30; distribution requires public readback.
- Evidence `<profile>/evidence/a18-release-20261028/`; root/user data preserved.
- Next: commit the candidate on the freeze, run full gate and nonpublishing packaging dry runs.

## Alpha 18 follow-ups 1 + 2 — source PUBLISHED, closeout GO
- Publication verified 2026-10-28 22:44 CET. Maintained docs own feature status.
- Native point 2 published `923b34599`; independent review, branch10/integrated10 accepted;
  native12 cells/42drags/20restorations, app/children exit0. Both trees/refs/caches retired
  after551-member archive checks. Placement rules remain until compositor reload.
- Runtime feature `96f41c7f1`, reviewed tree `7a1b3831f`: correction re-review approved0/0/0.
  Branch15 + integrated15 accepted at `8bdc42c7e`: each Rust3530/143ignored, Vitest4654,
  browser940 + production-devices57, selected corpus91/91; overlapping, not additive runs.
- Source published main-first `309384726`; exact upstream docs delta on `8bbd8ea76`
  separately accepted10/10. No runtime/test/build/gate source changed after accepted `8bdc42c7e`.
  Parent verified local/tracking/live equality. Router-byte EN/DE UI/report join separate.
- Built server at `8bdc42c7e`: isolated loopback health/frontend/version/empty-status smoke,
  ordinary termination exit0/no listener; executable/frontend retained, not relabelled.
- Own runtime feature tree `<sources>/knxbench-system2-final-diagnostic-20261028`, branch
  `feat/system2-final-diagnostic-20261028`; integration tree
  `<sources>/knxbench-alpha18-system2-integration-20261028`, branch
  `integration/alpha18-system2-final-diagnostic-20261028` retained for result-doc closeout.
- Next: accept/publish result docs, archive and verify owned source/evidence, idle/ancestry
  checks, retire only both runtime trees/refs/caches, then mark root coordination retired.
- Evidence `<profile>/evidence/alpha18-followups-20261028/`. No new source reservation.
  No bus/device/VM/user data, release/tag/deployment, hardware-cause or ETS-parity claim.

## Retained boundaries
- Prior WP-03, issue31 and WP-08 delivery evidence stays in maintained docs/Git history;
  old checkpoint wording is not proof of current activity. Root coordination is preserved.
- Root stays stale/dirty and protected. Foreign parked/release trees remain untouched.
- WP-11 Properties pane/wider native matrix, KL-130 and #30/#22 allocation cause stay open.
