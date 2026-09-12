import { useEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import type { CommandContext, ResolvedPaletteCommand } from "./commandRegistry";
import { COMMANDS, filterCommands } from "./commandRegistry";
import { useTranslate } from "./i18n";
import type { Translate } from "./i18n";
import type { MessageKey } from "./messages/en";
import Overlay from "./Overlay";

function firstEnabledIndex(commands: ResolvedPaletteCommand[], ctx: CommandContext): number {
  return commands.findIndex((cmd) => cmd.isEnabled(ctx));
}

// `commandRegistry.ts`'s `shortcutHint` strings (e.g. `"Ctrl+Z"`) are not
// display text — they're the *keys* this lookup translates from, following
// German ETS convention (Strg for Ctrl, Umschalt for Shift, Alt stays Alt,
// Entf for Delete — no Alt/Delete hint exists in the registry today, but
// the convention is noted here for whoever adds the next one). A hint this
// map has never heard of renders verbatim, same fallback shape as
// `BUILDING_PART_KIND_KEYS` in Inspector.tsx.
const SHORTCUT_HINT_KEYS: Record<string, MessageKey> = {
  "Ctrl+Z": "command.shortcutHint.undo",
  "Ctrl+Shift+Z": "command.shortcutHint.redo",
  "Ctrl+K": "command.shortcutHint.search",
};

function shortcutHintLabel(t: Translate, hint: string): string {
  const key = SHORTCUT_HINT_KEYS[hint];
  return key ? t(key) : hint;
}

export default function CommandPalette(props: { ctx: CommandContext; onClose: () => void }) {
  const { ctx, onClose } = props;
  const inputRef = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState("");
  const [highlight, setHighlight] = useState(0);
  const t = useTranslate();
  // `COMMANDS` holds `labelKey`s, not display text (see commandRegistry.ts)
  // — resolved here, at render time, so a UI language switch (which
  // changes `t`'s identity, per `useTranslate()`) re-resolves every label
  // instead of leaving stale text from whichever language was active when
  // the module first loaded.
  const resolvedCommands = useMemo<ResolvedPaletteCommand[]>(
    () => COMMANDS.map(({ labelKey, ...cmd }) => ({ ...cmd, label: t(labelKey) })),
    [t],
  );
  const results = useMemo(() => filterCommands(resolvedCommands, query), [resolvedCommands, query]);

  useEffect(() => {
    setHighlight(firstEnabledIndex(results, ctx));
  }, [results, ctx]);

  function runCommand(cmd: ResolvedPaletteCommand) {
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
    if (e.key === "ArrowDown") {
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
    <Overlay label={t("command.overlayLabel")} onClose={onClose} initialFocusRef={inputRef}>
      <input
        ref={inputRef}
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder={t("command.placeholder")}
        role="combobox"
        aria-haspopup="listbox"
        aria-expanded={results.length > 0}
        aria-controls="palette-results"
        aria-activedescendant={results[highlight] ? `palette-option-${highlight}` : undefined}
      />
      {results.length === 0 && <p className="search-empty">{t("command.noMatches")}</p>}
      <ul className="search-results" id="palette-results" role="listbox">
        {results.map((cmd, i) => {
          const enabled = cmd.isEnabled(ctx);
          const classes = ["search-result"];
          if (i === highlight) classes.push("selected");
          if (!enabled) classes.push("disabled");
          return (
            <li
              key={cmd.id}
              id={`palette-option-${i}`}
              role="option"
              aria-selected={i === highlight}
              className={classes.join(" ")}
              aria-disabled={!enabled}
              onClick={() => enabled && runCommand(cmd)}
            >
              <span>{cmd.label}</span>
              {cmd.shortcutHint && (
                <span className="provenance-badge">{shortcutHintLabel(t, cmd.shortcutHint)}</span>
              )}
            </li>
          );
        })}
      </ul>
    </Overlay>
  );
}
