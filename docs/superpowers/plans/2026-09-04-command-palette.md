# Command Palette Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `Ctrl+Shift+P` command palette to `knx-desktop` that lists the
app's seven existing actions (Open project, Open .knxdb, Save, Save As,
Undo, Redo, Search) as a filterable, keyboard-driven list.

**Architecture:** Frontend-only. A static command registry
(`commandRegistry.ts`) maps each existing `App.tsx` action to a
`PaletteCommand` with a label, an enablement predicate, and a run
function. `CommandPalette.tsx` reuses `Search.tsx`'s modal-overlay
structure (autofocused input, arrow-key highlight, click-outside-to-close)
against that static list instead of a fuzzy-matched entity index.
`App.tsx` wires the `Ctrl+Shift+P` shortcut and keeps the palette and
search overlays mutually exclusive.

**Tech Stack:** React 19, TypeScript, Vitest (`environment: "node"`, no
component-testing library — matches the existing project convention of
unit-testing pure logic modules only, not React components).

**Spec:** `docs/superpowers/specs/2026-09-04-command-palette-design.md`

## Global Constraints

- No backend change. No new Tauri command, no new `Command` variant.
- Filtering is a plain case-insensitive substring match — no fuzzy
  ranking (`matchEntries`/`searchMatch.ts` are not touched or reused).
- Disabled commands render greyed out, never hidden. Arrow-key
  navigation skips them; click/Enter on one is a no-op.
- `Ctrl+K` (search) and `Ctrl+Shift+P` (palette) are mutually exclusive —
  opening one closes the other.
- The palette works with no project loaded (`tree === null`) — the two
  "Open…" commands must stay enabled in that state, unlike `Search`
  which requires a tree to exist at all before it can open.
- Deviation from the spec's literal wording: the spec describes the
  palette's `CommandContext` as built via `useMemo`. Nothing in `App.tsx`
  today wraps its handler functions in `useCallback` (they're plain
  function declarations, recreated every render, exactly like the props
  already passed to `<Search>`/`<Inspector>` on every render) — memoizing
  only `ctx` while every other prop in this file is a fresh object each
  render would be a one-off inconsistency for no measured benefit. Build
  `ctx` as a plain object literal in the render body instead, matching
  the rest of the file.

---

## File Structure

- Create: `apps/knx-desktop/src/commandRegistry.ts` — `CommandContext`,
  `PaletteCommand`, the static `COMMANDS` array, `filterCommands`.
- Create: `apps/knx-desktop/src/commandRegistry.test.ts` — Vitest coverage
  for `filterCommands` and every command's `isEnabled`.
- Create: `apps/knx-desktop/src/CommandPalette.tsx` — the overlay
  component.
- Modify: `apps/knx-desktop/src/App.tsx` — new `paletteOpen` state, the
  `Ctrl+Shift+P` keydown branch, mutual exclusivity with `Ctrl+K`, the
  `ctx` object, rendering `<CommandPalette>`.
- Modify: `apps/knx-desktop/src/styles.css` — `.search-result` gains
  `display: flex; justify-content: space-between; align-items: center;`
  (harmless for `Search.tsx`'s single-text-node rows) and a new
  `.search-result.disabled` rule. The shortcut-hint pill reuses the
  existing `.provenance-badge` class — no new class needed for it.

---

### Task 1: Command registry

**Files:**
- Create: `apps/knx-desktop/src/commandRegistry.ts`
- Test: `apps/knx-desktop/src/commandRegistry.test.ts`

**Interfaces:**
- Consumes: `ProjectTree` type from `./bindings/ProjectTree` (only
  `tree.can_undo` / `tree.can_redo` are read).
- Produces: `CommandContext` (fields: `tree: ProjectTree | null`,
  `pickProject: () => void`, `openNativeProject: () => void`,
  `saveProject: () => void`, `saveProjectAs: () => void`,
  `undo: () => void`, `redo: () => void`, `openSearch: () => void`),
  `PaletteCommand` (fields: `id: string`, `label: string`,
  `shortcutHint?: string`, `isEnabled: (ctx: CommandContext) => boolean`,
  `run: (ctx: CommandContext) => void`), `COMMANDS: PaletteCommand[]`,
  `filterCommands(commands: PaletteCommand[], query: string): PaletteCommand[]`.
  Task 2 (`CommandPalette.tsx`) and Task 3 (`App.tsx`) both import these
  exact names.

- [ ] **Step 1: Write the failing test**

```ts
// apps/knx-desktop/src/commandRegistry.test.ts
import { describe, expect, it } from "vitest";
import { COMMANDS, filterCommands } from "./commandRegistry";
import type { CommandContext } from "./commandRegistry";
import type { ProjectTree } from "./bindings/ProjectTree";

function fakeTree(can_undo: boolean, can_redo: boolean): ProjectTree {
  return { can_undo, can_redo } as unknown as ProjectTree;
}

function noopCtx(overrides: Partial<CommandContext> = {}): CommandContext {
  return {
    tree: null,
    pickProject: () => {},
    openNativeProject: () => {},
    saveProject: () => {},
    saveProjectAs: () => {},
    undo: () => {},
    redo: () => {},
    openSearch: () => {},
    ...overrides,
  };
}

describe("filterCommands", () => {
  it("returns every command for an empty or blank query", () => {
    expect(filterCommands(COMMANDS, "")).toEqual(COMMANDS);
    expect(filterCommands(COMMANDS, "   ")).toEqual(COMMANDS);
  });

  it("matches case-insensitively on a label substring", () => {
    const result = filterCommands(COMMANDS, "SAVE");
    expect(result.map((c) => c.id).sort()).toEqual(["save", "save-as"]);
  });

  it("returns nothing for a query matching no label", () => {
    expect(filterCommands(COMMANDS, "xyzzy")).toEqual([]);
  });
});

describe("command enablement", () => {
  it("keeps both Open actions enabled with no project loaded", () => {
    const ctx = noopCtx({ tree: null });
    const open = COMMANDS.find((c) => c.id === "open-project")!;
    const openNative = COMMANDS.find((c) => c.id === "open-native")!;
    expect(open.isEnabled(ctx)).toBe(true);
    expect(openNative.isEnabled(ctx)).toBe(true);
  });

  it("disables Save/Save As/Undo/Redo/Search with no project loaded", () => {
    const ctx = noopCtx({ tree: null });
    for (const id of ["save", "save-as", "undo", "redo", "search"]) {
      const cmd = COMMANDS.find((c) => c.id === id)!;
      expect(cmd.isEnabled(ctx)).toBe(false);
    }
  });

  it("enables Save/Save As/Search but not Undo/Redo with a fresh project and empty history", () => {
    const ctx = noopCtx({ tree: fakeTree(false, false) });
    for (const id of ["save", "save-as", "search"]) {
      expect(COMMANDS.find((c) => c.id === id)!.isEnabled(ctx)).toBe(true);
    }
    for (const id of ["undo", "redo"]) {
      expect(COMMANDS.find((c) => c.id === id)!.isEnabled(ctx)).toBe(false);
    }
  });

  it("enables Undo/Redo once history exists", () => {
    const ctx = noopCtx({ tree: fakeTree(true, true) });
    expect(COMMANDS.find((c) => c.id === "undo")!.isEnabled(ctx)).toBe(true);
    expect(COMMANDS.find((c) => c.id === "redo")!.isEnabled(ctx)).toBe(true);
  });

  it("calls the matching context function when run", () => {
    let called = false;
    const ctx = noopCtx({ saveProject: () => (called = true) });
    COMMANDS.find((c) => c.id === "save")!.run(ctx);
    expect(called).toBe(true);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd apps/knx-desktop && npm test -- commandRegistry`
Expected: FAIL — `Cannot find module './commandRegistry'`

- [ ] **Step 3: Write minimal implementation**

```ts
// apps/knx-desktop/src/commandRegistry.ts
import type { ProjectTree } from "./bindings/ProjectTree";

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
  {
    id: "open-project",
    label: "Open project…",
    isEnabled: () => true,
    run: (ctx) => ctx.pickProject(),
  },
  {
    id: "open-native",
    label: "Open (.knxdb)…",
    isEnabled: () => true,
    run: (ctx) => ctx.openNativeProject(),
  },
  {
    id: "save",
    label: "Save",
    isEnabled: (ctx) => ctx.tree !== null,
    run: (ctx) => ctx.saveProject(),
  },
  {
    id: "save-as",
    label: "Save As…",
    isEnabled: (ctx) => ctx.tree !== null,
    run: (ctx) => ctx.saveProjectAs(),
  },
  {
    id: "undo",
    label: "Undo",
    shortcutHint: "Ctrl+Z",
    isEnabled: (ctx) => ctx.tree?.can_undo === true,
    run: (ctx) => ctx.undo(),
  },
  {
    id: "redo",
    label: "Redo",
    shortcutHint: "Ctrl+Shift+Z",
    isEnabled: (ctx) => ctx.tree?.can_redo === true,
    run: (ctx) => ctx.redo(),
  },
  {
    id: "search",
    label: "Search…",
    shortcutHint: "Ctrl+K",
    isEnabled: (ctx) => ctx.tree !== null,
    run: (ctx) => ctx.openSearch(),
  },
];

/**
 * Case-insensitive substring match against each command's label. Unlike
 * `matchEntries` (search overlay), an empty query returns every command —
 * the palette is a browsable list on open, not a search-only box — and
 * there is no ranking: seven static entries need no scoring algorithm.
 */
export function filterCommands(commands: PaletteCommand[], query: string): PaletteCommand[] {
  const needle = query.trim().toLowerCase();
  if (needle === "") return commands;
  return commands.filter((cmd) => cmd.label.toLowerCase().includes(needle));
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd apps/knx-desktop && npm test -- commandRegistry`
Expected: PASS, 7 tests

- [ ] **Step 5: Commit**

```bash
git add apps/knx-desktop/src/commandRegistry.ts apps/knx-desktop/src/commandRegistry.test.ts
git commit -m "feat(knx-desktop): add command palette registry"
```

---

### Task 2: `CommandPalette.tsx` component

**Files:**
- Create: `apps/knx-desktop/src/CommandPalette.tsx`
- Modify: `apps/knx-desktop/src/styles.css`

**Interfaces:**
- Consumes: `CommandContext`, `PaletteCommand`, `COMMANDS`,
  `filterCommands` from `./commandRegistry` (Task 1).
- Produces: default export `CommandPalette(props: { ctx: CommandContext;
  onClose: () => void })`. Task 3 (`App.tsx`) renders
  `<CommandPalette ctx={ctx} onClose={...} />`.

No component-testing library exists in this project (`vitest.config.ts`
sets `environment: "node"`; `Search.tsx` itself has no direct test either
— only the pure logic underneath it, `searchMatch.ts`, is unit-tested).
This task is verified by type-checking and the manual smoke check noted
in Task 3, not by an automated test.

- [ ] **Step 1: Write the component**

```tsx
// apps/knx-desktop/src/CommandPalette.tsx
import { useEffect, useMemo, useState } from "react";
import type { KeyboardEvent } from "react";
import type { CommandContext, PaletteCommand } from "./commandRegistry";
import { COMMANDS, filterCommands } from "./commandRegistry";

function firstEnabledIndex(commands: PaletteCommand[], ctx: CommandContext): number {
  return commands.findIndex((cmd) => cmd.isEnabled(ctx));
}

export default function CommandPalette(props: { ctx: CommandContext; onClose: () => void }) {
  const { ctx, onClose } = props;
  const [query, setQuery] = useState("");
  const [highlight, setHighlight] = useState(0);
  const results = useMemo(() => filterCommands(COMMANDS, query), [query]);

  useEffect(() => {
    setHighlight(firstEnabledIndex(results, ctx));
  }, [results, ctx]);

  function runCommand(cmd: PaletteCommand) {
    cmd.run(ctx);
    onClose();
  }

  // Moves the highlight to the next/previous *enabled* row, stopping at
  // the first/last enabled row rather than wrapping — disabled rows are
  // visible (never hidden, per design) but never land the highlight.
  function moveHighlight(delta: number) {
    setHighlight((current) => {
      let next = current;
      for (let step = 0; step < results.length; step++) {
        next += delta;
        if (next < 0 || next >= results.length) return current;
        if (results[next].isEnabled(ctx)) return next;
      }
      return current;
    });
  }

  function handleKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === "Escape") {
      onClose();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      moveHighlight(1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      moveHighlight(-1);
    } else if (e.key === "Enter") {
      const cmd = results[highlight];
      if (cmd && cmd.isEnabled(ctx)) runCommand(cmd);
    }
  }

  return (
    <div className="search-overlay" onClick={onClose}>
      <div className="search-panel" onClick={(e) => e.stopPropagation()}>
        <input
          autoFocus
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Type a command…"
        />
        {results.length === 0 && <p className="search-empty">No matching commands.</p>}
        <ul className="search-results">
          {results.map((cmd, i) => {
            const enabled = cmd.isEnabled(ctx);
            const classes = ["search-result"];
            if (i === highlight) classes.push("selected");
            if (!enabled) classes.push("disabled");
            return (
              <li
                key={cmd.id}
                className={classes.join(" ")}
                aria-disabled={!enabled}
                onClick={() => enabled && runCommand(cmd)}
              >
                <span>{cmd.label}</span>
                {cmd.shortcutHint && <span className="provenance-badge">{cmd.shortcutHint}</span>}
              </li>
            );
          })}
        </ul>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Add the disabled-row style**

```css
/* apps/knx-desktop/src/styles.css — extend the existing .search-result rule */
.search-result {
  padding: 0.35rem 0.5rem;
  cursor: pointer;
  border-radius: 3px;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.search-result.disabled {
  opacity: 0.4;
  cursor: default;
}
```

Replace the existing `.search-result` rule (`styles.css:185-189`) with
the block above — it adds `display: flex; justify-content:
space-between; align-items: center;` to the existing declarations
(harmless for `Search.tsx`, whose rows are a single text node), then add
the new `.search-result.disabled` rule directly after it, before
`.search-result.selected`.

- [ ] **Step 3: Type-check**

Run: `cd apps/knx-desktop && npx tsc --noEmit`
Expected: no errors

- [ ] **Step 4: Commit**

```bash
git add apps/knx-desktop/src/CommandPalette.tsx apps/knx-desktop/src/styles.css
git commit -m "feat(knx-desktop): add command palette overlay component"
```

---

### Task 3: Wire the palette into `App.tsx`

**Files:**
- Modify: `apps/knx-desktop/src/App.tsx`

**Interfaces:**
- Consumes: `CommandPalette` (default export, Task 2), `CommandContext`
  (type, Task 1).
- Produces: nothing new for later tasks — this is the integration point.

- [ ] **Step 1: Import the new module**

In `apps/knx-desktop/src/App.tsx`, add alongside the existing imports
(after the `Search` import on line 9):

```ts
import CommandPalette from "./CommandPalette";
import type { CommandContext } from "./commandRegistry";
```

- [ ] **Step 2: Add palette state**

Add next to the existing `searchOpen` state (line 27):

```ts
const [paletteOpen, setPaletteOpen] = useState(false);
```

- [ ] **Step 3: Wire the keyboard shortcuts**

Replace the `keydown` handler's `Ctrl+K` branch (lines 35-40) and insert
a new `Ctrl+Shift+P` branch immediately after it, before the existing
`z`-key handling:

```ts
function handleKeyDown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
    e.preventDefault();
    setPaletteOpen(false);
    if (tree) setSearchOpen(true);
    return;
  }
  if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === "p") {
    e.preventDefault();
    setSearchOpen(false);
    setPaletteOpen(true);
    return;
  }
  if (!(e.ctrlKey || e.metaKey) || e.key.toLowerCase() !== "z") return;
  e.preventDefault();
  if (e.shiftKey) {
    if (tree?.can_redo) void redo();
  } else {
    if (tree?.can_undo) void undo();
  }
}
```

The palette's own shortcut check comes before the `z`-key branch's early
`return` on any non-`z` key, so it isn't skipped. Unlike the `Ctrl+K`
branch, the new branch does not gate on `tree` — the palette must open
with no project loaded, since "Open project…"/"Open (.knxdb)…" are its
only enabled entries in that state.

- [ ] **Step 4: Build the command context and render the palette**

Add just before the `return (` in the component body (after the
`redo` function, before line 178):

```ts
const ctx: CommandContext = {
  tree,
  pickProject,
  openNativeProject,
  saveProject,
  saveProjectAs,
  undo,
  redo,
  openSearch: () => setSearchOpen(true),
};
```

Add the palette's render alongside the existing `Search` render (after
the `{tree && searchOpen && (...)}` block, still inside `<main>`, but
**not** nested inside any `tree &&` guard — the palette must render with
no project loaded):

```tsx
{paletteOpen && <CommandPalette ctx={ctx} onClose={() => setPaletteOpen(false)} />}
```

- [ ] **Step 5: Type-check and run the full test suite**

Run: `cd apps/knx-desktop && npx tsc --noEmit && npm test`
Expected: no type errors; all existing tests plus Task 1's
`commandRegistry.test.ts` pass.

- [ ] **Step 6: Manual smoke check (leave documented, not performed here)**

This step cannot be executed in this environment — no display is
available for a Tauri GUI session, the same limitation cycles 4 and 5
recorded in `docs/IMPLEMENTATION_STATUS.md`. Before or at merge, a
session with a display should:

1. Launch the app with no project loaded, press `Ctrl+Shift+P`: palette
   opens, "Open project…"/"Open (.knxdb)…" are clickable, the other five
   rows are greyed out.
2. Type `save`: only "Save"/"Save As…" rows remain, both greyed out (no
   project loaded).
3. Load a project, press `Ctrl+Shift+P` again, arrow down past "Save":
   highlight lands on the next enabled row, never on a greyed-out one.
4. Press `Ctrl+K` while the palette is open: palette closes, search
   opens. Press `Ctrl+Shift+P` while search is open: search closes,
   palette opens.
5. Highlight "Undo" (with a pending edit) and press `Enter`: the edit is
   undone and the palette closes.

- [ ] **Step 7: Commit**

```bash
git add apps/knx-desktop/src/App.tsx
git commit -m "feat(knx-desktop): wire command palette into App.tsx"
```

---

## Post-implementation documentation

After Task 3 is committed and the manual smoke check (Step 6) has been
run on a machine with a display:

- Update `docs/ROADMAP.md`'s Session 5 section: add a "Cycle 6" paragraph
  in the same style as cycles 1-5, and move "command palette" out of the
  "remain not yet scheduled" trailer sentences (cycles 4 and 5's
  paragraphs, and the Cycle 6+ candidates paragraph, all currently list
  it).
- Update `docs/IMPLEMENTATION_STATUS.md`: record the command palette as
  shipped, and note whether Step 6's manual smoke check was actually
  performed (cycles 4 and 5's entries are the template for this — both
  recorded their unperformed manual checks explicitly rather than
  silently).
- If Step 6 surfaces a real bug (not a docs gap), fix it under
  `superpowers:systematic-debugging` before writing these docs updates,
  not after.
