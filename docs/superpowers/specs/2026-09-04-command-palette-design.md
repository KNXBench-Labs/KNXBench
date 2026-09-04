# Session 5, cycle 6: command palette — design

**Status.** Approved, ready for implementation planning.

## Context

Cycle 5 (`docs/superpowers/specs/2026-09-04-search-design.md`) delivered
`Ctrl+K` search across devices, group addresses, and building parts.
ROADMAP.md and CLAUDE.md's UI/UX section still list two deliverables not
yet scheduled: Command Palette and dark/light mode. This cycle is the
Command Palette.

**Current state** (`apps/knx-desktop`):

- `App.tsx` exposes seven actions, all as always-visible toolbar buttons:
  Open project…, Open (.knxdb)…, Save, Save As…, Undo, Redo, Search…
  (Ctrl+K). Every one is already a plain function in `App.tsx`
  (`pickProject`, `openNativeProject`, `saveProject`, `saveProjectAs`,
  `undo`, `redo`, opening the `Search` overlay), each already guarded by
  its own enablement condition (`disabled={!tree}`,
  `disabled={!tree?.can_undo}`, etc.) and its own `try`/`catch` ->
  `setError`.
- `Search.tsx` established the modal-overlay pattern this cycle reuses:
  autofocused input, `ArrowUp`/`ArrowDown`/`Enter`/`Escape` handling,
  click-outside-to-close, a `highlight` index into a filtered list.
- `searchMatch.ts`'s `matchEntries` is a fuzzy ranker built for a
  hundreds-of-entries index (devices/group addresses/building parts). The
  command palette's list is seven static entries — fuzzy ranking has
  nothing to rank; generalizing `matchEntries` for a second, structurally
  different caller here would be speculative abstraction for a cycle
  scoped to app actions only.

**Scope decisions:**

- The palette lists **app-level actions only** — the seven existing
  toolbar actions — not entity navigation. Entity navigation (jump to a
  device/group address/building part) is Search's job already; folding
  both into one overlay would blur two overlays with different data
  sources and different lifetimes into one, for no benefit ROADMAP asks
  for.
- Filtering is a plain case-insensitive substring match on each action's
  label, not a fuzzy/ranked match. Seven static entries need no scoring
  algorithm.
- Actions that don't currently apply (e.g. Save with no project loaded,
  Undo with empty history) are rendered **greyed out, not hidden** — the
  palette's contents stay constant so a user learns them once. Arrow-key
  navigation skips greyed-out entries; clicking or pressing Enter on one
  is a no-op.
- Opening either overlay closes the other — `Ctrl+K` and `Ctrl+Shift+P`
  are mutually exclusive, never both open at once.
- No new domain `Command` variant, no new Tauri command, no backend
  change at all — every palette action is a call to a function that
  already exists in `App.tsx`.

## Backend

None. This cycle is frontend-only.

## Frontend

### Command registry (`commandRegistry.ts`, new)

```ts
export interface CommandContext {
  tree: ProjectTree | null;
  pickProject: () => void;
  openNativeProject: () => void;
  saveProject: () => void;
  saveProjectAs: () => void;
  undo: () => void;
  redo: () => void;
  openSearch: () => void;
}

export interface PaletteCommand {
  id: string;
  label: string;
  shortcutHint?: string;
  isEnabled: (ctx: CommandContext) => boolean;
  run: (ctx: CommandContext) => void;
}

export const COMMANDS: PaletteCommand[] = [
  { id: "open-project", label: "Open project…", isEnabled: () => true, run: (c) => c.pickProject() },
  { id: "open-native", label: "Open (.knxdb)…", isEnabled: () => true, run: (c) => c.openNativeProject() },
  { id: "save", label: "Save", isEnabled: (c) => !!c.tree, run: (c) => c.saveProject() },
  { id: "save-as", label: "Save As…", isEnabled: (c) => !!c.tree, run: (c) => c.saveProjectAs() },
  { id: "undo", label: "Undo", shortcutHint: "Ctrl+Z", isEnabled: (c) => !!c.tree?.can_undo, run: (c) => c.undo() },
  { id: "redo", label: "Redo", shortcutHint: "Ctrl+Shift+Z", isEnabled: (c) => !!c.tree?.can_redo, run: (c) => c.redo() },
  { id: "search", label: "Search…", shortcutHint: "Ctrl+K", isEnabled: (c) => !!c.tree, run: (c) => c.openSearch() },
];

export function filterCommands(commands: PaletteCommand[], query: string): PaletteCommand[] {
  const q = query.trim().toLowerCase();
  if (q === "") return commands;
  return commands.filter((cmd) => cmd.label.toLowerCase().includes(q));
}
```

`CommandContext` carries only `tree` plus the seven callbacks — no
`hasStorePath`, since no command's `isEnabled`/`run` reads it. Save's
dialog-vs-overwrite branching already lives inside `saveProject` itself
(`App.tsx`, unchanged); the palette only needs to know whether it may
call it at all, which `tree` alone answers.

### `CommandPalette.tsx` (new)

Same structural shape as `Search.tsx`: a click-outside-closes overlay
div wrapping a panel with an autofocused input and a result list.
Differences from `Search.tsx`:

- Input filters `COMMANDS` via `filterCommands`, not `matchEntries`.
- No grouping by kind — one flat list.
- Each row renders `label` and, if present, `shortcutHint` right-aligned.
- Rows where `isEnabled(ctx)` is false get a `disabled` class and
  `aria-disabled="true"`; they render but are excluded from the
  `highlight` index's traversal (`ArrowUp`/`ArrowDown` skip over them)
  and from click/`Enter` handling (no-op if the resolved row is
  disabled).
- Picking an enabled row calls `command.run(ctx)` then `onClose()`,
  unconditionally — same as `Search.tsx`'s `pick()` always closing
  regardless of what the selection callback does. Failures inside
  `run` surface exactly as they do today (each wrapped function already
  sets `error` via its own `try`/`catch`); the palette itself does not
  add error handling.

### `App.tsx` wiring

- New `paletteOpen` state, mirroring `searchOpen`.
- `keydown` handler gains a `Ctrl+Shift+P` (or `Cmd+Shift+P`) branch:
  `preventDefault()`, `setSearchOpen(false)`, `setPaletteOpen(true)`.
  The existing `Ctrl+K` branch gains `setPaletteOpen(false)` alongside
  its `setSearchOpen(true)`, so the two overlays stay mutually exclusive
  in both directions.
- `ctx: CommandContext` built with `useMemo`, keyed on `tree` and the
  seven callback identities (all stable across renders already, since
  none of them close over renamed local state beyond what `tree`/
  `hasStorePath` already trigger a re-render for).
- Toolbar buttons are unchanged — the palette is an additional way to
  invoke the same actions, not a replacement for the buttons. (Removing
  the buttons in favor of the palette alone is not this cycle's job;
  ROADMAP lists no such deprecation.)

## Testing

`commandRegistry.test.ts` (Vitest, alongside `searchMatch.test.ts`):

- `filterCommands`: empty query returns all seven; a query matching one
  label's substring (case-insensitive) returns only that one; a query
  matching nothing returns an empty array.
- `isEnabled` per command against a table of contexts: no `tree` (Save/
  Save As/Undo/Redo/Search disabled, Open actions enabled), a `tree`
  with `can_undo: false`/`can_redo: false` (Undo/Redo disabled, rest
  enabled), a `tree` with both `true` (all seven enabled).

Manual smoke check — open the palette (`Ctrl+Shift+P`), type a filter,
navigate with arrow keys skipping a disabled entry, invoke one with
Enter — is left unperformed in this environment for the same reason
cycles 4 and 5 left theirs unperformed: no display available here for a
Tauri GUI session. Should be run before, or at, merge.
