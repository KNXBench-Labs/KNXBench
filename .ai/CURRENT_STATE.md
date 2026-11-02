# Restart snapshot

## WP-25 import findings after reopening (ACTIVE; observed 2026-11-02 10:56 CET)
- Owner Go: 2 November 2026; branch `fix/wp25-import-findings`, isolated sibling worktree.
- Implementation and independent whole-feature review settled: APPROVE, no blockers;
  16 independent server controls pass, three targeted mutants fail as intended.
- Base `ec9d9bd7f`; RED: stored counters (1,4), log findings absent; re-projection chosen.
- Sparse-history/overflow/grouping/count/cap/recovery/refusal controls green;
  EN/DE 1440/400 browser proofs pass in production/dev, full download exercised.
- Candidate frozen for scoped branch gate; no running gate claimed by this snapshot.
- NEXT: launch branch gate, main-first integration and scoped merged gate, then
  publication/docs closeout and owned cleanup. No new Owner Go needed.
- No bus operations, release, deployment or manufacturer contact.

## Retained boundaries (STANDING; observed 2026-11-02 10:48 CET)
- Root remains stale/dirty and protected; no root index or user-data edits authorized.
- Prior WP-03, issue31 and WP-08 evidence stays in maintained docs/Git history.
- Foreign parked/release trees remain untouched. Alpha16 belongs to another track.
- Private hardware-research remains parked; private evidence/bytes stay private.
  Further hardware work requires its own authorization; no automatic scope expansion.
- WP-11 Properties pane/wider native matrix and KL-130 stay open.
