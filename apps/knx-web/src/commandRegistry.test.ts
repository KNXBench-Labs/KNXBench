/** Tests for command filtering and enablement in the command registry. */
// @vitest-environment happy-dom
//
// happy-dom, not node: `translate()` (via `getActiveUiLanguage()`) reads
// `window.localStorage`/`window.navigator` on its first call, the same
// reasoning `uiLanguage.test.tsx` and `CommandPalette.test.tsx` document
// for the same store.
import { describe, expect, it } from "vitest";
import { COMMANDS, filterCommands } from "./commandRegistry";
import type { CommandContext, ResolvedPaletteCommand } from "./commandRegistry";
import type { ProjectTree } from "./bindings/ProjectTree";
import { translate } from "./i18n";

// `COMMANDS` holds `labelKey`s now (see commandRegistry.ts), not resolved
// text — `filterCommands` itself only ever sees resolved labels
// (`CommandPalette` is the real caller, via `useTranslate()`), so this
// file's own `filterCommands` tests resolve the same way with the
// non-hook `translate()` accessor, at the module's active language
// (English by default in a vitest run — nothing here switches it).
const RESOLVED_COMMANDS: ResolvedPaletteCommand[] = COMMANDS.map(({ labelKey, ...cmd }) => ({
  ...cmd,
  label: translate(labelKey),
}));

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
    expect(filterCommands(RESOLVED_COMMANDS, "")).toEqual(RESOLVED_COMMANDS);
    expect(filterCommands(RESOLVED_COMMANDS, "   ")).toEqual(RESOLVED_COMMANDS);
  });

  it("matches case-insensitively on a label substring", () => {
    const result = filterCommands(RESOLVED_COMMANDS, "SAVE");
    expect(result.map((c) => c.id).sort()).toEqual(["save", "save-as"]);
  });

  it("returns nothing for a query matching no label", () => {
    expect(filterCommands(RESOLVED_COMMANDS, "xyzzy")).toEqual([]);
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

  it("lists all seven commands in palette order, with an unconditionally enabled first entry", () => {
    expect(COMMANDS.map((c) => c.id)).toEqual([
      "open-project", "open-native", "save", "save-as", "undo", "redo", "search",
    ]);
    expect(COMMANDS[0].isEnabled(noopCtx({ tree: null }))).toBe(true);
  });
});
