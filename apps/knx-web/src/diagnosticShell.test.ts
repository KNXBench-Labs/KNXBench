// Guards that the diagnostic panels stay inside the shell's class, theme and density system.
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { describe, expect, it } from "vitest";

/**
 * Two defects motivate this file, both found by the stage-5 audit and both
 * invisible to every other test:
 *
 * 1. `bus-monitor-row-new` was set on fresh telegram rows and asserted by
 *    nine expectations in `BusMonitorPanel.test.tsx`, while the rule that
 *    drew it had been deleted from `styles.css` — the class was live in
 *    the DOM and dead on screen. Class names are strings on both sides, so
 *    neither `tsc` nor vitest noticed.
 * 2. `.bus-monitor-table th, td` carried its own `padding`, at a
 *    specificity that beat the shell's `th, td { padding:
 *    var(--knx-cell-padding); }` — the density setting moved every table
 *    except the telegram one.
 */

const SRC_DIR = dirname(fileURLToPath(import.meta.url));
const PANEL_FILES = ["LogPanel.tsx", "BusMonitorPanel.tsx", "BusComposeForm.tsx"];

/**
 * Class names owned by the diagnostic panels. Anything outside these
 * prefixes (`workspace-heading`, `eyebrow`, `field-error`, …) belongs to
 * the shell and is checked where the shell is.
 */
const OWNED_PREFIXES = ["log-", "bus-", "monitor-", "telegram-"];

/**
 * Class names that intentionally draw nothing: query hooks for tests, or
 * modifiers that sit on an element already carrying a styled class. Adding
 * to this list is the conscious decision the guard exists to force.
 */
const UNSTYLED_BY_DESIGN = new Set([
  "log-entry-message", // test hook; the entry's own card supplies the look
  "bus-compose-destination", // inputs, styled as inputs by the shell
  "bus-compose-dpt",
  "bus-compose-value",
  "bus-compose-closed-hint", // always paired with .bus-compose-hint
  "bus-compose-error", // always paired with .field-error
]);

/**
 * Every whole, literal class-shaped token under an owned prefix. Tokens
 * built by interpolation (`` `log-entry-${severity}` ``) end at a `$` and
 * are skipped deliberately: their full set is not knowable from the text.
 */
function ownedTokens(source: string): string[] {
  const found = new Set<string>();
  for (const match of source.matchAll(/(?<![\w$-])([a-z][a-z0-9]*(?:-[a-z0-9]+)+)(?![\w$-])/g)) {
    const token = match[1];
    if (OWNED_PREFIXES.some((prefix) => token.startsWith(prefix))) found.add(token);
  }
  return [...found].sort();
}

function definedClasses(css: string): Set<string> {
  const defined = new Set<string>();
  for (const match of css.matchAll(/\.([a-zA-Z][\w-]*)/g)) defined.add(match[1]);
  return defined;
}

describe("diagnostic panels stay inside the shell", () => {
  const css = readFileSync(join(SRC_DIR, "styles.css"), "utf-8");

  it("has a stylesheet rule for every class name the panels apply", () => {
    const defined = definedClasses(css);
    const orphans: string[] = [];
    for (const file of PANEL_FILES) {
      const source = readFileSync(join(SRC_DIR, file), "utf-8");
      for (const token of ownedTokens(source)) {
        if (!defined.has(token) && !UNSTYLED_BY_DESIGN.has(token)) orphans.push(`${file}: ${token}`);
      }
    }
    expect(orphans, `class names with no rule in styles.css: ${orphans.join(", ")}`).toEqual([]);

    // Sanity check: the scan must actually be finding tokens, or an empty
    // `orphans` would mean nothing rather than everything is fine.
    expect(ownedTokens(readFileSync(join(SRC_DIR, "BusMonitorPanel.tsx"), "utf-8"))).toContain(
      "bus-monitor-row-new",
    );
  });

  it("leaves telegram-table cell padding to the density tokens", () => {
    const block = css.match(/\.bus-monitor-table th,\s*\.bus-monitor-table td \{([^}]*)\}/);
    expect(block, "the .bus-monitor-table cell rule moved or was renamed").not.toBeNull();
    expect(block![1]).not.toMatch(/padding/);
  });
});
