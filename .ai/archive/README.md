# Handover archives

[Current per-track state](../CURRENT_STATE.md).

The current file contains active/waiting/parked tracks, decisions and coordination,
not the entire delivery journal. Older next-step instructions and locks are
historical; do not execute or re-acquire them automatically.

- [Published snapshot, 13 October 2026](CURRENT_STATE_main_2026-10-13.md): full
  journal from main `4c0bea47`, with only reversible link-destination relocation.
- Existing earlier archives stay at their own paths and are not duplicated.

At the next authorized checkpoint capture the complete original, archive it
before shortening anything, and reconcile current tracks against the latest
owner records. Never discard active work solely because its timestamp is old.
Preserve unknown and parked decisions instead of interpreting them as permission.
Keep source offsets/digests and check original-source reconstruction; a historical
byte-exact suffix need not remain in the short main file if it is preserved in the
verified archive. Concurrent newer entries must be preserved separately and folded
into their actual track only after their owner disposition has been read.

Local-only journals/private research **must not be published** as part of a public
archive. Keep those histories in a separate ignored local archive, preserve owner
park/refusal decisions in the local current summary, and leave all source research,
planning files, stashes and other sessions' artifacts untouched.
