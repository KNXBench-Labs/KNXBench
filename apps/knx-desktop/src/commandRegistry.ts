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
