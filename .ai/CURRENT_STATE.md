# Restart snapshot

## WP-25 import findings after reopening (ACTIVE; observed 2026-11-02 11:07 CET)
- Owner Go: 2 November 2026; branch `fix/wp25-import-findings`, isolated sibling worktree.
- Whole-feature review and focused guard-fix re-review APPROVE, no blockers;
  16 independent server controls pass, three targeted mutants fail as intended.
- Initial base `ec9d9bd7f`; first branch gate on `b36060577` failed two Web guard
  inventories. Corrected only the exact read-only graph/query-hook inventories;
  companion no-report/no-mutation runtime assertions retained. Full Vitest 4897 green.
- Current source unchanged by review; EN/DE production/dev 1440/400 proofs green.
- NEXT: rebase unpublished branch onto current main, recreate main-first merge;
  require identical feature/integration trees, then one complete scoped gate for both.
  Prior failed attempt is superseded, never credited to the corrected tree.
- No bus operations, release, deployment or manufacturer contact.

## Retained boundaries (STANDING; observed 2026-11-02 10:48 CET)
- Root remains stale/dirty and protected; no root index or user-data edits authorized.
- Prior WP-03, issue31 and WP-08 evidence stays in maintained docs/Git history.
- Foreign parked/release trees remain untouched. Alpha16 belongs to another track.
- Private hardware-research remains parked; private evidence/bytes stay private.
  Further hardware work requires its own authorization; no automatic scope expansion.
- WP-11 Properties pane/wider native matrix and KL-130 stay open.
