# Restart snapshot

- **Last Agent:** codex
- **Timestamp:** 2026-10-24 23:19 CEST (coordination observation, not independent gate acceptance)
- **Completed:** Handover policy changed to a compact snapshot; the prior journal is preserved byte-for-byte in [the archive](archive/CURRENT_STATE_2026-10-24_pre-compact.md). No product code or gate inputs changed in this task.
- **Pending/Next Steps:** Preserve the active WP-06 track below. The owner will forward [the session notice](logs/2026-10-24_handover-notice.md); no live delivery or acknowledgement is claimed.
- **Notes for Codex oder Claude:** These model-specific labels are transition compatibility, not a requirement of the new repository policy. Replace the snapshot at relevant checkpoints, do not prepend entries; aim for at most 50 lines.

## Active track: saved-project-folder grouping

- Owner: existing implementation/controller session; this documentation session does not take over.
- Branch/worktree: `integrate/wp06-folder-grouping-20261024`, `<sources>/knxbench-wp06-folder-grouping-20261024`.
- Gate candidate observed: `e6f227c4eee8a3072dcc176427a53604f5aaf3f1`.
- Receipt: `<profile-scratch>/wp06-folder-claude-20261024/integration/gate/receipt.json`; now reports `complete=true`, `success=true` for that candidate. Independent acceptance/publication remain with its owner.
- Scope: read-only folder view using existing opaque keys; channel/flat views, core/API/schema and write semantics unchanged. The owner's current contract/source is authoritative.
- Next: owner validates the completed receipt and handles publication. Do not edit its tree, replay tests or infer published delivery from this observation.

## Parked tracks and safety

- The earlier post-Alpha-16 controller's finish-only STOP remains scoped to that track; this housekeeping task grants no product-work continuation.
- Preserve the separate Alpha 16 release worktree and selected-root private/untracked research. Do not retire another owner's evidence or worktrees.
- No bus/device operations, vendor contact, model downloads or release/deployment authorised by this task. The globally excluded device remains off-limits, including reads and scan contact.
- Private closeout evidence: `<profile>/release-evidence/post16-20261024`; details: [post-16 closeout](logs/2026-10-24_codex_post16-closeout.md).

## Sources of truth

- Project status/backlog: `docs/OPEN_WORK.md`, `docs/status/LEDGER.md`, maintained contracts/ADRs.
- Durable decisions/pitfalls: external knowledge vault; conversation history: Hermes sessions.
- Old journal instructions are historical, not current authority. Revalidate ownership, consent, Git state and receipts before acting.
