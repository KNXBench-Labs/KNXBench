/** Holds every theme in styles.css to ADR-0022's token boundary. */
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { describe, expect, it } from "vitest";
import { ACCENTS } from "./appearance";
import { THEMES, loadThemeId, resolveThemeId } from "./theme";
import type { ThemeBlock } from "./themeTokens";
import {
  COMPONENT_LAYER_TOKENS,
  THEME_BLOCK_PLAIN_PROPERTIES,
  blockPlainProperties,
  blockTokens,
  declaredTokens,
  parseRules,
  referencedTokens,
  requiredThemeTokens,
  themeBlocks,
  themeSelectorViolations,
  themeVariationBlocks,
} from "./themeTokens";

describe("parseRules", () => {
  it("attributes declarations to the block they are written in", () => {
    const rules = parseRules(".a {\n  color: red;\n}\n@media print {\n  .b {\n    color: blue;\n  }\n}\n");
    expect(rules.map((r) => r.selector)).toEqual([".a", "@media print", ".b"]);
    expect(rules[0].declarations).toEqual([{ property: "color", value: "red", line: 2 }]);
    expect(rules[1].declarations).toEqual([]);
    expect(rules[2].declarations).toEqual([{ property: "color", value: "blue", line: 6 }]);
    expect(rules[2].depth).toBe(1);
  });

  it("ignores braces and semicolons inside comments", () => {
    const rules = parseRules("/* .ghost { color: red; } */\n.a {\n  color: red;\n}\n");
    expect(rules.map((r) => r.selector)).toEqual([".a"]);
    expect(rules[0].line).toBe(2);
  });

  it("keeps a multi-value declaration whole", () => {
    const rules = parseRules(":root {\n  --knx-backdrop-size: auto, 48px 48px;\n}\n");
    expect(rules[0].declarations[0].value).toBe("auto, 48px 48px");
  });
});

describe("themeSelectorViolations", () => {
  it("catches the accent-by-negation trap", () => {
    const rules = parseRules(':root:not([data-theme="x"]) { --knx-accent: red; }\n');
    expect(themeSelectorViolations(rules).map((r) => r.selector)).toEqual([
      ':root:not([data-theme="x"])',
    ]);
  });

  it("catches a theme styling an element", () => {
    const rules = parseRules(':root[data-theme="x"] body::before { background: red; }\n');
    expect(themeSelectorViolations(rules)).toHaveLength(1);
  });

  it("allows the two shapes the boundary defines", () => {
    const rules = parseRules(
      ':root[data-theme="x"] { --knx-accent: red; }\n' +
        ':root[data-theme="x"][data-accent="mint"] { --knx-accent: green; }\n',
    );
    expect(themeSelectorViolations(rules)).toEqual([]);
  });
});

describe("requiredThemeTokens", () => {
  it("is what the component layer reads, minus what a setting owns", () => {
    const css = ".a { color: var(--knx-foreground); transition-duration: var(--knx-transition-duration); }";
    expect(requiredThemeTokens(css)).toEqual(["--knx-foreground"]);
  });
});

describe("blockPlainProperties", () => {
  it("catches a non-knx custom property, not just knx tokens and color-scheme", () => {
    const rules = parseRules(':root[data-theme="x"] { --foo: red; color-scheme: light; }\n');
    expect(blockPlainProperties(rules[0])).toEqual(["--foo", "color-scheme"]);
  });
});

const HERE = dirname(fileURLToPath(import.meta.url));
const css = readFileSync(join(HERE, "styles.css"), "utf-8");
const rules = parseRules(css);
const required = requiredThemeTokens(css);
const blocks = themeBlocks(rules);
const paletteThemeIds = THEMES.filter((theme) => theme.id !== "system").map((theme) => theme.id);

/** The one `:root[data-theme="<id>"]` block for `id`, or a named failure
 * instead of `blocks.find(...)!`'s `TypeError` when a registered theme has
 * none. */
function requireBlock(id: string): ThemeBlock {
  const block = blocks.find((candidate) => candidate.id === id);
  expect(block, `no :root[data-theme="${id}"] block in styles.css`).toBeDefined();
  return block!;
}

describe("the theme layer of styles.css", () => {
  it("has exactly one block per theme in the registry", () => {
    expect([...blocks.map((block) => block.id)].sort()).toEqual([...paletteThemeIds].sort());
  });

  it("ships at least three themes", () => {
    expect(blocks.length).toBeGreaterThanOrEqual(3);
  });

  // The point of this file. A theme that forgets a token does not fall
  // back to something tasteful — it inherits whatever the previous rule
  // happened to leave behind, which is how a half-themed palette ships.
  it.each(paletteThemeIds)("theme %s defines every token in the boundary", (id) => {
    const block = requireBlock(id);
    const defined = new Set(blockTokens(block.rule));
    const missing = required.filter((token) => !defined.has(token));
    expect(missing, `theme "${id}" is missing ${missing.length} token(s)`).toEqual([]);
  });

  it.each(paletteThemeIds)("theme %s sets no token a user setting owns", (id) => {
    const block = requireBlock(id);
    const trespassing = blockTokens(block.rule).filter((token) =>
      COMPONENT_LAYER_TOKENS.includes(token),
    );
    expect(
      trespassing,
      `theme "${id}" sets component-layer token(s); those belong to the motion/density settings`,
    ).toEqual([]);
  });

  it.each(paletteThemeIds)("theme %s declares color-scheme and nothing else plain", (id) => {
    const block = requireBlock(id);
    expect(blockPlainProperties(block.rule)).toEqual([...THEME_BLOCK_PLAIN_PROPERTIES]);
  });

  it("defines no theme by negation and lets no theme style an element", () => {
    const violations = themeSelectorViolations(rules).map((rule) => `${rule.line}: ${rule.selector}`);
    expect(violations, `selectors outside the boundary's two shapes:\n${violations.join("\n")}`).toEqual(
      [],
    );
  });

  it("declares no token nobody reads", () => {
    const read = new Set(referencedTokens(css));
    const dead = declaredTokens(rules).filter((token) => !read.has(token));
    expect(dead, `declared but never read through var(): ${dead.join(", ")}`).toEqual([]);
  });
});

describe("accent variations", () => {
  const variations = themeVariationBlocks(rules);

  it("belong to a registered theme and a registered accent", () => {
    for (const variation of variations) {
      expect(paletteThemeIds, `unknown theme in ${variation.rule.selector}`).toContain(variation.id);
      expect(ACCENTS as readonly string[], `unknown accent in ${variation.rule.selector}`).toContain(
        variation.accent,
      );
    }
  });

  it("re-point the accent pair and nothing else", () => {
    for (const variation of variations) {
      const outside = blockTokens(variation.rule).filter(
        (token) => token !== "--knx-accent" && token !== "--knx-on-accent",
      );
      expect(outside, `${variation.rule.selector} changes more than the accent`).toEqual([]);
      expect(blockPlainProperties(variation.rule)).toEqual([]);
    }
  });

  it("theme.ts's hasAccentVariations agrees with what styles.css declares", () => {
    const themesWithBlocks = new Set(variations.map((variation) => variation.id));
    for (const theme of THEMES.filter((candidate) => candidate.id !== "system")) {
      expect(
        theme.hasAccentVariations,
        `THEMES["${theme.id}"].hasAccentVariations disagrees with styles.css`,
      ).toBe(themesWithBlocks.has(theme.id));
    }
  });
});

describe("index.html's pre-mount bootstrap", () => {
  // It cannot import theme.ts (it runs before any module loads), so the id
  // list, the legacy migration and the system default are all duplicated
  // there. A drift shows up as a flash of the wrong theme, or as a stored
  // theme silently resolving back to the default.
  const html = readFileSync(join(HERE, "..", "index.html"), "utf-8");

  it("lists exactly the registry's palette themes, in the same order", () => {
    const match = /var knownThemes = \[([^\]]*)\]/.exec(html);
    expect(match, "index.html no longer declares `var knownThemes = [...]`").not.toBeNull();
    const listed = match![1]
      .split(",")
      .map((entry) => entry.trim().replace(/^"|"$/g, ""))
      .filter((entry) => entry !== "");
    expect(listed).toEqual(paletteThemeIds);
  });

  it("migrates the legacy \"light\"/\"dark\" values the same way loadThemeId does", () => {
    const light = /if\s*\(t === "light"\)\s*t = "([a-z0-9-]+)"/.exec(html);
    const dark = /if\s*\(t === "dark"\)\s*t = "([a-z0-9-]+)"/.exec(html);
    expect(light, 'index.html no longer migrates the legacy "light" value').not.toBeNull();
    expect(dark, 'index.html no longer migrates the legacy "dark" value').not.toBeNull();
    expect(light![1]).toBe(loadThemeId({ getItem: () => "light" }));
    expect(dark![1]).toBe(loadThemeId({ getItem: () => "dark" }));
  });

  it("falls back to the same system default resolveThemeId does", () => {
    const match = /matches \? "([a-z0-9-]+)" : "([a-z0-9-]+)"/.exec(html);
    expect(match, "index.html no longer resolves the system default inline").not.toBeNull();
    const [, darkDefault, lightDefault] = match!;
    expect(darkDefault).toBe(resolveThemeId("system", true));
    expect(lightDefault).toBe(resolveThemeId("system", false));
  });
});
