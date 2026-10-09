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
  return { can_undo, can_redo, is_modified: can_undo } as unknown as ProjectTree;
}

function noopCtx(overrides: Partial<CommandContext> = {}): CommandContext {
  return {
    tree: null,
    newProject: () => {},
    pickProject: () => {},
    openNativeProject: () => {},
    saveProject: () => {},
    saveProjectAs: () => {},
    undo: () => {},
    redo: () => {},
    openSearch: () => {},
    openLog: () => {},
    openBusMonitor: () => {},
    openSettings: () => {},
    openCompanion: () => {},
    openHelp: () => {},
    openCatalog: () => {},
    openDevices: () => {},
    addDevice: () => {},
    openIntroduction: () => {},
    openAchievements: () => {},
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
  it("offers the Devices view only with an open project and invokes its navigation callback", () => {
    const command = COMMANDS.find((entry) => entry.id === "open-devices");
    expect(command, "Devices must remain reachable with the navigation collapsed").toBeDefined();
    expect(command!.isEnabled(noopCtx())).toBe(false);
    let opened = false;
    const ctx = { ...noopCtx({ tree: fakeTree(false, false) }), openDevices: () => { opened = true; } };
    expect(command!.isEnabled(ctx)).toBe(true);
    command!.run(ctx);
    expect(opened).toBe(true);
  });

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

  it("lists all seventeen commands in palette order, with an unconditionally enabled first entry", () => {
    expect(COMMANDS.map((c) => c.id)).toEqual([
      "new-project", "open-project", "open-native", "save", "save-as", "undo", "redo", "search",
      "open-log", "open-bus-monitor", "open-settings", "open-diagnostics-window",
      "open-catalog", "open-devices", "add-device", "show-introduction", "open-achievements", "open-help",
    ]);
    expect(COMMANDS[0].isEnabled(noopCtx({ tree: null }))).toBe(true);
  });

  // ADR-0093: the wizard creates into a project, so it needs one open.
  it("enables Add device only with a project open and runs the wizard", () => {
    let ran = 0;
    const cmd = COMMANDS.find((c) => c.id === "add-device")!;
    expect(cmd.isEnabled(noopCtx({ tree: null }))).toBe(false);
    const ctx = noopCtx({ tree: {} as never, addDevice: () => (ran += 1) });
    expect(cmd.isEnabled(ctx)).toBe(true);
    cmd.run(ctx);
    expect(ran).toBe(1);
  });

  // The whole point of the from-scratch launcher: it is the one File
  // action that works when the user owns no project file at all, so a
  // `tree === null` that disabled it would put the feature behind the
  // very thing it exists to avoid needing.
  it("keeps New project runnable with no project open", () => {
    let started = false;
    const ctx = noopCtx({ tree: null, newProject: () => (started = true) });
    const cmd = COMMANDS.find((c) => c.id === "new-project")!;
    expect(cmd.isEnabled(ctx)).toBe(true);
    cmd.run(ctx);
    expect(started).toBe(true);
  });

  // The diagnostic entries exist because the buttons that used to reach
  // these panels now sit inside a collapsible navigation pane; a command
  // that went dark without a project would reintroduce the same gap.
  it("keeps log, bus monitor, settings and the companion window runnable with no project open", () => {
    const opened: string[] = [];
    const ctx = noopCtx({
      tree: null,
      openLog: () => opened.push("log"),
      openBusMonitor: () => opened.push("monitor"),
      openSettings: () => opened.push("settings"),
      openCompanion: () => opened.push("companion"),
    });
    for (const id of ["open-log", "open-bus-monitor", "open-settings", "open-diagnostics-window"]) {
      const cmd = COMMANDS.find((c) => c.id === id)!;
      expect(cmd.isEnabled(ctx)).toBe(true);
      cmd.run(ctx);
    }
    expect(opened).toEqual(["log", "monitor", "settings", "companion"]);
  });

  // The first-run guide's "Add product data" task runs `open-catalog`, and
  // the guide itself is reopened through `show-introduction`; both are
  // used exactly where no project is open yet.
  it("keeps the product catalog and the introduction runnable with no project open", () => {
    const opened: string[] = [];
    const ctx = noopCtx({
      tree: null,
      openCatalog: () => opened.push("catalog"),
      openDevices: () => {},
      addDevice: () => opened.push("add-device"),
      openIntroduction: () => opened.push("introduction"),
    });
    for (const id of ["open-catalog", "show-introduction"]) {
      const cmd = COMMANDS.find((c) => c.id === id)!;
      expect(cmd.isEnabled(ctx)).toBe(true);
      cmd.run(ctx);
    }
    expect(opened).toEqual(["catalog", "introduction"]);
  });

  // ADR-0089: the overview opens with or without a project, and even with
  // achievements switched off — the dialog then says how to switch them on.
  it("keeps the achievements overview runnable with no project open", () => {
    let opened = false;
    const ctx = noopCtx({ tree: null, openAchievements: () => (opened = true) });
    const cmd = COMMANDS.find((c) => c.id === "open-achievements")!;
    expect(cmd.isEnabled(ctx)).toBe(true);
    cmd.run(ctx);
    expect(opened).toBe(true);
  });
});
