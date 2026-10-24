# Handover archives

[Current per-track state](../CURRENT_STATE.md).

The current file is a compact, overwrite-in-place restart snapshot (target at most
50 lines), not the entire delivery journal. Keep current assignments, explicit
stop/go per track, ownership, unfinished work, gate/evidence pointers, blockers
and next steps. The checkpoint and concurrent-edit rules live in `AGENTS.md`.
Older next-step instructions and locks are historical; do not execute or
re-acquire them automatically.

- [Pre-compaction journal, 24 October 2026](CURRENT_STATE_2026-10-24_pre-compact.md):
  byte-exact journal from `57bdb354c01d0dd225408cd4f46f0cece59ff3cd`; relative
  links retain their original `.ai/CURRENT_STATE.md` context. Resolve historical
  links at that source revision rather than treating them as current instructions.

- [Published snapshot, 13 October 2026](CURRENT_STATE_main_2026-10-13.md): full
  journal from main `4c0bea47`, with only reversible link-destination relocation.
- Existing earlier archives stay at their own paths and are not duplicated.

When archiving another legacy journal, capture the complete original and archive
it before shortening anything; reconcile tracks against the latest owner records.
Routine snapshot updates replace current state and do not create another archive.
Never discard active work solely because its timestamp is old.
Preserve unknown and parked decisions instead of interpreting them as permission.
Keep source offsets/digests and check original-source reconstruction; a historical
byte-exact suffix need not remain in the short main file if it is preserved in the
verified archive. Concurrent newer entries must be preserved separately and folded
into their actual track only after their owner disposition has been read.

Local-only journals/private research **must not be published** as part of a public
archive. Keep those histories in a separate ignored local archive, preserve owner
park/refusal decisions in the local current summary, and leave all source research,
planning files, stashes and other sessions' artifacts untouched.
