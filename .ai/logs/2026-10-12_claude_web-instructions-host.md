# 2026-10-12 — Claude — installation instructions tab (web integration package 2)

## What changed
- `DeviceInstructionsPanel.tsx`: hosts `InstructionEditor` as the sixth device tab
  (lazy first mount, then hidden); refresh after every Apply through App's publication;
  shows "applied, reload failed" itself because the leaf does not await `onApplied`.
- `snapshotIdentity.ts`: shared explicit snake_case → camelCase identity mapping (notes
  host now uses it too).
- Leaf: `context: null` read-only mode (no toolbar/preview/reset, readOnly textarea).
- App: one draft registry (`draftsRef` notes + instructions) behind the existing guard;
  guard copy generalized (`draftGuard.*`).
- Both leaves: rebase a dirty draft across a same-project revision change when the
  server content is unchanged (Save bumps the revision); real changes still conflict.

## Lessons
- Real-server smokes find what intercepted fixtures cannot: Save changing
  `snapshot_revision` was invisible until a draft was typed before saving.
- A flaky-looking probe failure was the defect itself; the 300 ms "settle" that made it
  pass would have hidden it. Remove such delays once the cause is fixed.
- Two specs writing the same frame names into one capture dir overwrite each other;
  use one directory per spec.
