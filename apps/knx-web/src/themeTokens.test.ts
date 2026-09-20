/** Holds every theme in the app's stylesheets to ADR-0022's token boundary. */
import { readFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join, relative } from "node:path";
import { describe, expect, it } from "vitest";
import { ACCENTS } from "./appearance";
import { THEMES, loadThemeId, resolveThemeId } from "./theme";
import type { CssRule, ThemeBlock } from "./themeTokens";
import {
  COMPONENT_LAYER_TOKENS,
  THEME_BLOCK_PLAIN_PROPERTIES,
  blockPlainProperties,
  blockTokens,
  componentColourLiterals,
  declaredTokens,
  illegalTokenNames,
  isThemeLayerRule,
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

  it("carries the enclosing selector chain, outermost first", () => {
    const rules = parseRules("@media print {\n  @supports (display: grid) {\n    .a { color: red; }\n  }\n}\n");
    expect(rules.map((r) => r.ancestors)).toEqual([
      [],
      ["@media print"],
      ["@media print", "@supports (display: grid)"],
    ]);
    expect(rules.map((r) => r.depth)).toEqual([0, 1, 2]);
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

  // The failure ADR-0022 names as its reason to exist, in the one shape
  // that used to escape every check in this file: `themeBlocks()` collects
  // depth-0 blocks only, so a theme block inside a media query was neither
  // held to the boundary nor reported. It could therefore redefine
  // `--knx-transition-duration` — the user's motion setting — and pass.
  it("catches a legal-looking theme block nested inside a media query", () => {
    const rules = parseRules(
      "@media (min-width: 40em) {\n" +
        '  :root[data-theme="x"] { --knx-transition-duration: 900ms; }\n' +
        "}\n",
    );
    expect(themeBlocks(rules)).toEqual([]);
    expect(themeSelectorViolations(rules).map((r) => r.selector)).toEqual([':root[data-theme="x"]']);
  });

  it("catches a nested accent variation too", () => {
    const rules = parseRules(
      "@supports (color: color-mix(in srgb, red, blue)) {\n" +
        '  :root[data-theme="x"][data-accent="mint"] { --knx-accent: green; }\n' +
        "}\n",
    );
    expect(themeVariationBlocks(rules)).toEqual([]);
    expect(themeSelectorViolations(rules)).toHaveLength(1);
  });

  it("catches a rule nested inside a theme block", () => {
    const rules = parseRules(':root[data-theme="x"] {\n  .card { background: red; }\n}\n');
    expect(themeSelectorViolations(rules).map((r) => r.selector)).toEqual([".card"]);
  });
});

describe("illegalTokenNames", () => {
  // The old `var\(\s*(--knx-[a-z0-9-]+)` truncated `--knx-fooBar` to
  // `--knx-foo` and handed that on as a real token, so every theme failed
  // for a token nobody had written. The name is now read whole and
  // rejected once, by its actual name.
  it("reads a camelCase token whole instead of truncating it", () => {
    expect(referencedTokens(".a { color: var(--knx-fooBar); }")).toEqual(["--knx-fooBar"]);
    expect(requiredThemeTokens(".a { color: var(--knx-fooBar); }")).toEqual(["--knx-fooBar"]);
  });

  it("rejects the names the naming rule does not allow", () => {
    expect(illegalTokenNames(["--knx-fooBar", "--knx-foo_bar", "--knx-Foo", "--knx-"])).toEqual([
      "--knx-",
      "--knx-Foo",
      "--knx-fooBar",
      "--knx-foo_bar",
    ]);
  });

  it("allows the shape every real token has", () => {
    expect(illegalTokenNames(["--knx-accent", "--knx-on-accent", "--knx-radius-card"])).toEqual([]);
  });
});

describe("componentColourLiterals", () => {
  it("catches a hex literal in a component rule", () => {
    const rules = parseRules(".badge { color: #ff00aa; }\n");
    expect(componentColourLiterals(rules).map((f) => f.literal)).toEqual(["#ff00aa"]);
  });

  it("catches a named colour and a colour function", () => {
    const rules = parseRules(".a { border: 1px solid red; }\n.b { background: rgba(0, 0, 0, 0.4); }\n");
    expect(componentColourLiterals(rules).map((f) => f.literal)).toEqual(["red", "rgba()"]);
  });

  it("leaves the theme layer alone — literals are what a theme block is for", () => {
    const rules = parseRules(
      ':root[data-theme="x"] { --knx-accent: #ff00aa; color-scheme: light; }\n' +
        ':root[data-theme="x"][data-accent="mint"] { --knx-accent: darkseagreen; }\n',
    );
    expect(componentColourLiterals(rules)).toEqual([]);
    expect(rules.every(isThemeLayerRule)).toBe(true);
  });

  it("allows transparent, currentColor, var() and anything inside url()", () => {
    const rules = parseRules(
      ".a { border: 1px solid transparent; }\n" +
        ".b { fill: currentColor; }\n" +
        ".c { background: color-mix(in srgb, var(--knx-accent) 8%, var(--knx-surface)); }\n" +
        '.d { background-image: url("data:image/svg+xml;base64,YWJjZGVm"); }\n',
    );
    expect(componentColourLiterals(rules)).toEqual([]);
  });

  it("reports where the literal is, not just that there is one", () => {
    const rules = parseRules(".a {\n  color: red;\n}\n");
    expect(componentColourLiterals(rules)[0]).toEqual({
      line: 2,
      property: "color",
      value: "red",
      literal: "red",
    });
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
const APP_ROOT = join(HERE, "..");

/** Directories that hold no source of ours. */
const NOT_OURS = new Set(["node_modules", "dist", ".git", "coverage", "test-results"]);

/** Every `.css` file the app ships, path-relative to the app root.
 *
 * Walked rather than named. This file used to read exactly one hard-coded
 * path, so the day someone split the stylesheet — a second `.css` next to
 * the first, or a per-component sheet — that file would have sat outside
 * ADR-0022 entirely: its theme blocks unchecked, its `var()` reads missing
 * from the boundary, its literal colours unseen. An empty list is itself a
 * failure below, so a rename cannot make this test vacuously pass either. */
function findStylesheets(dir: string): string[] {
  const found: string[] = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.name.startsWith(".") || NOT_OURS.has(entry.name)) continue;
    const full = join(dir, entry.name);
    if (entry.isDirectory()) found.push(...findStylesheets(full));
    else if (entry.name.endsWith(".css")) found.push(full);
  }
  return found.sort();
}

interface Stylesheet {
  /** Path relative to `apps/knx-web`, so a failure says which file. */
  name: string;
  css: string;
  rules: CssRule[];
}

const stylesheets: Stylesheet[] = findStylesheets(APP_ROOT).map((path) => {
  const text = readFileSync(path, "utf-8");
  return { name: relative(APP_ROOT, path), css: text, rules: parseRules(text) };
});

// `var()` reads and theme blocks are questions about the app's whole CSS,
// not about one file, so both are answered over the concatenation. Line
// numbers stay per-file — every failure message below names its sheet.
const css = stylesheets.map((sheet) => sheet.css).join("\n");
const rules = stylesheets.flatMap((sheet) => sheet.rules);
const required = requiredThemeTokens(css);
const blocks = themeBlocks(rules);
const paletteThemeIds = THEMES.filter((theme) => theme.id !== "system").map((theme) => theme.id);

describe("the stylesheet set this file judges", () => {
  it("is discovered, not hard-coded, and is not empty", () => {
    expect(stylesheets.map((sheet) => sheet.name)).toContain("src/styles.css");
    expect(stylesheets.length).toBeGreaterThan(0);
  });
});

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

  it.each(stylesheets.map((sheet) => sheet.name))(
    "%s defines no theme by negation, nests none, and lets none style an element",
    (name) => {
      const sheet = stylesheets.find((candidate) => candidate.name === name)!;
      const violations = themeSelectorViolations(sheet.rules).map(
        (rule) => `${name}:${rule.line}: ${rule.selector}`,
      );
      expect(
        violations,
        `selectors outside the boundary's two shapes:\n${violations.join("\n")}`,
      ).toEqual([]);
    },
  );

  it.each(stylesheets.map((sheet) => sheet.name))(
    "%s writes no literal colour outside the theme layer",
    (name) => {
      const sheet = stylesheets.find((candidate) => candidate.name === name)!;
      const literals = componentColourLiterals(sheet.rules).map(
        (found) => `${name}:${found.line}: ${found.property}: ${found.value}  (${found.literal})`,
      );
      expect(
        literals,
        "ADR-0022: a component rule paints through a theme token, never a literal. " +
          `Use var(--knx-…), or give the theme layer a token for it:\n${literals.join("\n")}`,
      ).toEqual([]);
    },
  );

  it("names every token it reads or declares legally", () => {
    const illegal = illegalTokenNames([...referencedTokens(css), ...declaredTokens(rules)]);
    expect(illegal, `--knx-* names outside [a-z0-9-]: ${illegal.join(", ")}`).toEqual([]);
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

  // The bootstrap sets data-theme before the first paint, so the markup's
  // own attribute is unreachable in a working build. It is there for the
  // build that is not working: a CSP that drops the inline script, a
  // localStorage access that throws before the attribute is set. Without
  // it the document matches no theme block at all and renders as Times New
  // Roman on transparent, because ADR-0022 deliberately left no
  // no-attribute default.
  it("ships a data-theme in the markup for the bootstrap to overwrite", () => {
    const match = /<html\b[^>]*\sdata-theme="([a-z0-9-]+)"/.exec(html);
    expect(match, "index.html's <html> carries no data-theme fallback").not.toBeNull();
    expect(paletteThemeIds).toContain(match![1]);
    expect(match![1]).toBe(resolveThemeId("system", false));
  });

  it("falls back to the same system default resolveThemeId does", () => {
    const match = /matches \? "([a-z0-9-]+)" : "([a-z0-9-]+)"/.exec(html);
    expect(match, "index.html no longer resolves the system default inline").not.toBeNull();
    const [, darkDefault, lightDefault] = match!;
    expect(darkDefault).toBe(resolveThemeId("system", true));
    expect(lightDefault).toBe(resolveThemeId("system", false));
  });
});
