# 2026-10-12 — Claude — checked parameter editors (web integration package 3)

## What changed
- `parameterSourceHost.ts`: field-by-field parse of `parameter-editor-source`
  answers; "malformed" and "mismatch" (device/ref/program/snapshot) never become
  evidence; revision and selection keys from authoritative identity only.
- `ParameterSourceEditor.tsx`: opt-in host per row; re-reads per revision while
  keeping the previous answer; writes through `setParameterValue` and publishes
  inside `onApply` (survives unmount/revision change).
- `ParameterPanel.tsx`: **Checked editor** button for Time/Number/Color/Picture in
  the parameters view with a complete identity; the editor replaces the row (no
  controls inside the `<label>`); `useDeviceParameters` blanks only on device or
  product-language change.
- Leaf: rebase across Save, settle on the applied value, clean editor follows Undo,
  a revision arriving during the editor's own write is never adopted.

## Lessons
- Run the real-server smoke before declaring a leaf "done": Undo after Apply only
  showed up there.
- `.knxproj` project XML must be schema 23 (or another supported one); the product
  files inside may be schema 20. Schema-20 project XML fails at detectSchema.
- A shared hook that blanks state on every refresh silently defeats every
  draft-preserving rule below it; check the parents, not just the leaf.
