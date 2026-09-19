/** Registry of command-palette commands, their enablement rules, and keyboard-shortcut hints. */
import type { ProjectTree } from "./bindings/ProjectTree";
import type { MessageKey } from "./messages/en";

export interface CommandContext {
  tree: ProjectTree | null;
  newProject: () => void;
  pickProject: () => void;
  openNativeProject: () => void;
  saveProject: () => void;
  saveProjectAs: () => void;
  undo: () => void;
  redo: () => void;
  openSearch: () => void;
  openLog: () => void;
  openBusMonitor: () => void;
  openSettings: () => void;
  openCompanion: () => void;
  openHelp: () => void;
}

/**
 * `COMMANDS` below is a module-level array literal, built once at import
 * time — so `labelKey` names a catalogue entry rather than holding
 * resolved text directly, the same way `App.tsx`'s JSX calls `t(key)` at
 * render time instead of baking English into a constant. A `label:
 * string` resolved here could never update after a UI language switch,
 * since nothing re-executes this array literal; `CommandPalette` resolves
 * `labelKey` via `useTranslate()` on every render instead (see its own
 * comment on `resolvedCommands`).
 */
export interface PaletteCommand {
  id: string;
  labelKey: MessageKey;
  shortcutHint?: string;
  isEnabled: (ctx: CommandContext) => boolean;
  run: (ctx: CommandContext) => void;
}

/** A `PaletteCommand` with `labelKey` resolved to display text for the
 * currently active UI language — what `filterCommands` and the rendered
 * list actually work with, so a German query matches a German label
 * instead of the (English-ish) catalogue key underneath it. */
export interface ResolvedPaletteCommand extends Omit<PaletteCommand, "labelKey"> {
  label: string;
}

// `isEnabled` on each command below mirrors the `disabled` condition on the
// matching toolbar button in `App.tsx` (`!tree`, `!tree?.can_undo`,
// `!tree?.can_redo`) — the toolbar buttons are not derived from this
// registry, so the two must be kept in sync by hand if either changes.
//
// `labelKey` reuses `App.tsx`'s own toolbar keys wherever the text is
// identical (`toolbar.save`, `toolbar.undo`, …) rather than duplicating
// the catalogue entry — "search" is the one exception, since the toolbar
// button's `toolbar.search` carries a "(Ctrl+K)" suffix this palette row
// already renders separately via `shortcutHint`.
export const COMMANDS: PaletteCommand[] = [
  // First, and enabled with nothing loaded: it is the only File action
  // that does not require the user to already own a file.
  {
    id: "new-project",
    labelKey: "toolbar.newProject",
    isEnabled: () => true,
    run: (ctx) => ctx.newProject(),
  },
  {
    id: "open-project",
    labelKey: "toolbar.openProject",
    isEnabled: () => true,
    run: (ctx) => ctx.pickProject(),
  },
  {
    id: "open-native",
    labelKey: "toolbar.openNativeProject",
    isEnabled: () => true,
    run: (ctx) => ctx.openNativeProject(),
  },
  {
    id: "save",
    labelKey: "toolbar.save",
    isEnabled: (ctx) => ctx.tree !== null,
    run: (ctx) => ctx.saveProject(),
  },
  {
    id: "save-as",
    labelKey: "toolbar.saveAs",
    isEnabled: (ctx) => ctx.tree !== null,
    run: (ctx) => ctx.saveProjectAs(),
  },
  {
    id: "undo",
    labelKey: "toolbar.undo",
    shortcutHint: "Ctrl+Z",
    isEnabled: (ctx) => ctx.tree?.can_undo === true,
    run: (ctx) => ctx.undo(),
  },
  {
    id: "redo",
    labelKey: "toolbar.redo",
    shortcutHint: "Ctrl+Shift+Z",
    isEnabled: (ctx) => ctx.tree?.can_redo === true,
    run: (ctx) => ctx.redo(),
  },
  {
    id: "search",
    labelKey: "command.search",
    shortcutHint: "Ctrl+K",
    isEnabled: (ctx) => ctx.tree !== null,
    run: (ctx) => ctx.openSearch(),
  },
  // The three entries below are the *only* device-independent way to reach
  // the log, the bus monitor and the settings dialog: their navigation
  // buttons live inside `App.tsx`'s left `ResizablePane`, which the
  // navigation toggle can collapse. `LogPanel` renders with `tree === null`
  // (it reports import diagnostics, which exist before a project opens) and
  // `BusMonitorPanel` takes `projectOpen` rather than requiring a project,
  // so neither is gated on `ctx.tree`.
  {
    id: "open-log",
    labelKey: "toolbar.log",
    isEnabled: () => true,
    run: (ctx) => ctx.openLog(),
  },
  {
    id: "open-bus-monitor",
    labelKey: "toolbar.busMonitor",
    isEnabled: () => true,
    run: (ctx) => ctx.openBusMonitor(),
  },
  {
    id: "open-settings",
    labelKey: "toolbar.settings",
    isEnabled: () => true,
    run: (ctx) => ctx.openSettings(),
  },
  // Same reason as the three above, one step further: the companion
  // window's only button is in that same collapsible pane, and a window
  // that can only be opened with a mouse is not reachable.
  {
    id: "open-diagnostics-window",
    labelKey: "companion.open",
    isEnabled: () => true,
    run: (ctx) => ctx.openCompanion(),
  },
  // Last, and enabled with nothing loaded. F1 is the shortcut people who
  // already know it will use; this row is for everyone else, and it is
  // the reason `shortcutHint` says F1 at all (ADR-0024).
  {
    id: "open-help",
    labelKey: "toolbar.help",
    shortcutHint: "F1",
    isEnabled: () => true,
    run: (ctx) => ctx.openHelp(),
  },
];

/**
 * Case-insensitive substring match against each command's *resolved*
 * label — never the `labelKey`, so a German user typing a German word
 * still finds the command (`CommandPalette` is what resolves `COMMANDS`
 * into this shape before calling here; see its `resolvedCommands`). Unlike
 * `matchEntries` (search overlay), an empty query returns every command —
 * the palette is a browsable list on open, not a search-only box — and
 * there is no ranking: thirteen static entries need no scoring algorithm.
 */
export function filterCommands(
  commands: ResolvedPaletteCommand[],
  query: string,
): ResolvedPaletteCommand[] {
  const needle = query.trim().toLowerCase();
  if (needle === "") return [...commands];
  return commands.filter((cmd) => cmd.label.toLowerCase().includes(needle));
}
