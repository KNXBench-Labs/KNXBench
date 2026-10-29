# Restart snapshot

## Alpha 18 release freeze (ACTIVE, observed 2026-10-29 00:19 CET)
- Release owner: this WebUI; source cutoff `bb2d8e51efcbd029034c694ab4da9638f80961ee`.
- Freeze `0b8d80278` is published; prerequisite delivery and local retirement verified.
- Candidate branch `release/a18-prep-20261028`, tree `<sources>/knxbench-a18-release-20261028`;
  app versions `0.1.0-alpha.18`; native/model16 unchanged, productdb28->30.
- `main` stays release-frozen for code until the tag is published. Other packages stay local;
  no branch/main pushes by other tracks. Later work belongs to the next release.
- Reader/version review found one stale Docker pin and two disclosure/date defects; corrected.
- Tooling/anchors preflight green; full acceptance has not run. No hardware/real-bus access.
- Historical111-file restoration permission unanswered: no copy restored; two pinned opt-ins
  NOT RUN. Other acceptance remains GO; do not call omissions a complete release gate.
- Next: commit corrected candidate, full repository gate plus release extras, packaging dry runs.
- STOP before tag, GitHub release or Docker publication: explicit owner GO still required.
- Evidence `<profile>/evidence/a18-release-20261028/`; root/user data preserved.

## Alpha 18 follow-ups 1 + 2 (DELIVERED, RETIRED; observed 2026-10-28 23:01 CET)
- Owner's root checkpoint records final closeout; source delivery is verified on the cutoff.
- Runtime feature `96f41c7f1`, main-first `309384726`, result docs `bb2d8e51e`.
- Review APPROVE0/0/0; branch15/integrated15 at `8bdc42c7e`: each Rust3530/143ignored,
  Vitest4654, browser940 +production57, selected corpus91/91 (overlapping runs).
- Built server at `8bdc42c7e`: isolated startup/frontend/status smoke, ordinary exit0;
  later docs-only revisions are not relabelled as runtime execution.
- Native `923b34599`, native12 cells/42drags/20restorations; placement rules until reload.
- All four own native/runtime trees, refs and caches retired after idle/ancestry and
 551/1076-member native/runtime archive checks; executable/frontend retained.
- Evidence `<profile>/evidence/alpha18-followups-20261028/`; no source/desktop reservation.
- #30/#22 allocation cause, per-part equivalence and wider WP-11/KL-130 stay open.

## Retained boundaries
- Prior WP-03, issue31 and WP-08 delivery evidence stays in maintained docs/Git history;
  old checkpoint wording is not proof of current activity. Root coordination is preserved.
- Root stays stale/dirty and protected. Foreign parked/release trees remain untouched.
- WP-11 Properties pane/wider native matrix, KL-130 and #30/#22 allocation cause stay open.
