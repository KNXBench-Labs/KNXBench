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
