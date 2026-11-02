# Restart snapshot

## WP-26 controller/footer (ACTIVE; owner WebUI task queue; observed 2026-11-02 17:03 CET)
- WP-26 controller/footer: taken by WebUI, planned branch/worktree
  `feat/wp26-controller-footer-20261102` after this claim is published.
- Backend A published `038845ef8`; review/re-review APPROVE, both14-stage gates
  accepted (Rust1673/72ignored, corpus51/51, frozen inputs, user data untouched).
  Acceptance: `docs/status/2026-11-02-wp26-nonqueueing-start.md`.
- Current branch only closes backend docs and claims next scope; no UI worker yet.
  NEXT: docs gate/publication, then create the next worktree and implement one
  end-to-end bound-stop/controller/footer package, review and gate its actual join.
- Decisions: `docs/open-work/WP-26-status-bar-bus-connection.md`. Shared session,
  strict id+server-bound nonqueueing stop for new callers; legacy stop separate.
  Preserve panel publication ordering; no stale connected state or implicit starts.
- Active backend evidence `<profile-scratch>/wp26-bus-connection-20261102/`;
  next contract `<profile-scratch>/wp26-controller-footer-20261102/admitted-contract.md`.
- Pure selection; no auto retry/reconnect, USB, extra consent or real bus Go.
  No release/deployment. Retire only owned backend trees/caches after archiving.

## Retained boundaries (STANDING; observed 2026-11-02 17:03 CET)
- System-2 acceptance: `docs/status/2026-11-02-system2-selective-settlement.md`.
- Root intentionally stale/dirty: preserve private/untracked files and other edits.
- Foreign simulator-control-records worktree remains untouched; Alpha16 belongs
  to another track. Prior WP-03, issue31 and WP-08 evidence remains in docs/Git.
- Private hardware-research is not owned here; root's parked private-track notes
  remain authoritative for its separate owner/consent. Private bytes stay private.
- WP-11 Properties pane/wider native matrix and KL-130 stay open.
