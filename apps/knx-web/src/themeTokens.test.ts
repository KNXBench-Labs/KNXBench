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
  contrastRatio,
  declaredTokens,
  evaluateThemeContrast,
  resolveThemeColor,
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

  // The semicolon before `}` is optional in CSS, and a parser that flushes
  // only on `;` cannot see the declaration at all. Every guard in this app
  // reads this one parser, so `.evil { color: #ff00aa }` — legal,
  // browser-honoured, and ADR-0022's opening failure — used to pass the
  // whole suite. One deleted character was the entire exploit.
  it("sees a final declaration written without its semicolon", () => {
    const rules = parseRules(".a {\n  color: red\n}\n");
    expect(rules[0].declarations).toEqual([{ property: "color", value: "red", line: 2 }]);
  });

  it("sees the last of several when only the last loses its semicolon", () => {
    const rules = parseRules(".a { padding: 2px; color: red }\n");
    expect(rules[0].declarations.map((d) => d.property)).toEqual(["padding", "color"]);
  });

  it("invents no declaration when a block closes on nothing", () => {
    const rules = parseRules("@media print {\n  .a { color: red; }\n}\n.b {}\n");
    expect(rules.map((r) => r.declarations.length)).toEqual([0, 1, 0]);
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

  // `.sb::before { content: "red}"; }` is legal, shipping CSS: the `}` is
  // ordinary text inside a string. A scanner with no string awareness ends
  // the block right there, flushes whatever the buffer holds as a phantom
  // declaration (`content` = `"red`, missing its closing quote), and
  // reports a colour that was never painted. `url("a}b.png")` truncates
  // the same way. Both need the whole value intact, brace included.
  it("keeps a brace inside a double-quoted string out of the block count", () => {
    const rules = parseRules('.sb::before { content: "red}"; }\n');
    expect(rules).toHaveLength(1);
    expect(rules[0].declarations).toEqual([
      { property: "content", value: '"red}"', line: 1 },
    ]);
  });

  it("keeps a brace inside a url()'s quoted string out of the block count", () => {
    const rules = parseRules('.a { background: url("a}b.png"); }\n');
    expect(rules[0].declarations).toEqual([
      { property: "background", value: 'url("a}b.png")', line: 1 },
    ]);
  });

  // The mutation in the other direction: string-awareness must not turn
  // into "never sees a closing brace again". A real `}` outside any string
  // still ends its block, and the next rule still starts its own.
  it("still closes a block on a real } once its string has ended", () => {
    const rules = parseRules('.a { content: "ok"; }\n.b { color: red; }\n');
    expect(rules.map((r) => r.selector)).toEqual([".a", ".b"]);
    expect(rules[0].declarations).toEqual([{ property: "content", value: '"ok"', line: 1 }]);
    expect(rules[1].declarations).toEqual([{ property: "color", value: "red", line: 2 }]);
  });

  // A backslash escapes the next character even inside a string, so an
  // escaped quote does not end it early. Without this, `"a\"b"` would close
  // after two characters and leave `b";` dangling as the start of the next
  // "declaration".
  it("does not end a string on an escaped quote", () => {
    const rules = parseRules('.a { content: "a\\"b"; }\n');
    expect(rules[0].declarations).toEqual([
      { property: "content", value: '"a\\"b"', line: 1 },
    ]);
  });

  // Browsers close an open block at EOF; a stylesheet is not required to
  // end its last rule with `}`. `flushDeclaration()` after the scanning
  // loop is what makes that declaration visible instead of silently
  // dropped along with the buffer holding it.
  it("still flushes the last declaration of a block left open at EOF", () => {
    const rules = parseRules(".eof { color: #ff00aa");
    expect(rules).toHaveLength(1);
    expect(rules[0].declarations).toEqual([
      { property: "color", value: "#ff00aa", line: 1 },
    ]);
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

  // The third shape: a comma-joined list of two individually legal theme
  // selectors. The violation check used to run per comma-part and the
  // three classifiers below matched the whole collapsed selector, so this
  // belonged to neither set — not a violation, not a theme block, not a
  // theme-layer rule — and could override the user's motion setting for
  // two themes at once. ADR-0022 forbids the sharing outright: every theme
  // carries its own complete palette.
  it("catches two themes comma-joined into one shared block", () => {
    const rules = parseRules(
      ':root[data-theme="x"], :root[data-theme="y"] {\n' +
        "  --knx-transition-duration: 900ms;\n" +
        "}\n",
    );
    expect(themeBlocks(rules)).toEqual([]);
    expect(themeVariationBlocks(rules)).toEqual([]);
    expect(rules.some(isThemeLayerRule)).toBe(false);
    expect(themeSelectorViolations(rules)).toHaveLength(1);
  });

  // Attribute *names* in a selector are ASCII case-insensitive, so this
  // selects exactly what the lowercase spelling selects. A guard that
  // matches one spelling recognises a spelling, not a shape.
  it("is not fooled by an uppercase attribute name", () => {
    const rules = parseRules(':root[DATA-THEME="x"] { --knx-transition-duration: 900ms; }\n');
    expect(themeBlocks(rules).map((block) => block.id)).toEqual(["x"]);
    expect(themeSelectorViolations(rules)).toEqual([]);
  });

  it("holds an uppercase accent variation to the same shape", () => {
    const rules = parseRules(':root[DATA-THEME="x"][Data-Accent="mint"] { --knx-accent: green; }\n');
    expect(themeVariationBlocks(rules)).toEqual([
      expect.objectContaining({ id: "x", accent: "mint" }),
    ]);
    expect(themeSelectorViolations(rules)).toEqual([]);
  });

  // The attribute *value* is not case-insensitive: "Porcelain" is a theme
  // id the registry has never heard of, and must stay a violation.
  it("still rejects an uppercase theme id, which really is a different value", () => {
    const rules = parseRules(':root[data-theme="Porcelain"] { --knx-accent: green; }\n');
    expect(themeBlocks(rules)).toEqual([]);
    expect(themeSelectorViolations(rules)).toHaveLength(1);
  });

  // `mentionsTheme`'s `/i` flag is what lets these three still be seen at
  // all: it is called on the raw selector text directly, not on the
  // normalised one `isLegalThemeSelector` checks. Swap it for a
  // case-sensitive `selector.includes("data-theme")` and every fixture
  // below stops mentioning theme as far as this file is concerned — not
  // "wrongly classified", simply invisible, which is a worse failure than
  // any of the illegal shapes above. Each fixture below carries no colour
  // and no duration literal on purpose, so a regression here fails for
  // this reason and not because some other guard also happens to catch the
  // shape.
  it("still catches an uppercase theme selector styling an element", () => {
    const rules = parseRules(':root[DATA-THEME="porcelain"] body::before { content: ""; }\n');
    expect(themeSelectorViolations(rules)).toHaveLength(1);
  });

  it("still catches the uppercase accent-by-negation trap", () => {
    const rules = parseRules(':root:not([DATA-THEME="porcelain"]) { display: block; }\n');
    expect(themeSelectorViolations(rules)).toHaveLength(1);
  });

  it("still catches an uppercase theme block nested inside a media query", () => {
    const rules = parseRules(
      "@media print {\n" +
        '  :root[DATA-THEME="porcelain"] { display: none; }\n' +
        "}\n",
    );
    expect(themeSelectorViolations(rules)).toHaveLength(1);
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

  // The shape the semicolon-blind parser hid: ADR-0022's own opening
  // failure, `color: #ffffff` on a `#f7931a` button at 2.3:1, walking back
  // into the stylesheet through a missing character.
  it("catches a hex literal in a rule written without its final semicolon", () => {
    const rules = parseRules(".evil { color: #ff00aa }\n");
    expect(componentColourLiterals(rules).map((f) => f.literal)).toEqual(["#ff00aa"]);
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

  // Before `parseRules` learned string awareness, the data-URI fixture
  // above never reached `withoutUrls()` at all — its `;base64` split the
  // declaration in two before the value was even assembled — so the whole
  // url() exemption could be deleted with the suite still green. Now that
  // the semicolon inside the quoted string survives intact, that fixture
  // does reach it, but this is the fixture that actually needs it: a
  // filename that happens to contain a colour word, which is exactly the
  // false positive the exemption exists to prevent.
  it("does not read a colour out of a filename", () => {
    const rules = parseRules('.logo { background-image: url("images/red-logo.png"); }\n');
    expect(componentColourLiterals(rules)).toEqual([]);
  });

  // `content: "red}"` is a string, not a colour — the same reasoning as
  // the filename above, one property over. This is also the fixture that
  // pins the phantom-declaration fix: before `parseRules` learned string
  // awareness, the `}` inside the quotes ended the block early and left
  // `content` holding the unterminated value `"red`, which this same guard
  // reported as a literal colour for the wrong reason entirely.
  it("does not read a colour out of a quoted string containing a brace", () => {
    const rules = parseRules('.sb::before { content: "red}"; }\n');
    expect(componentColourLiterals(rules)).toEqual([]);
  });

  it("does not read a colour out of a url()'s quoted string containing a brace", () => {
    const rules = parseRules('.a { background: url("a}b.png"); }\n');
    expect(componentColourLiterals(rules)).toEqual([]);
  });

  // Two ways to be told a math function paints tan. `tan()` is a CSS
  // function whose name really is a named colour, and skipping function
  // names is what saves it. `tanh()` used not to be saved by that skip at
  // all: `/[a-z][a-z0-9]*(?!\()/` looks like it refuses a name followed by
  // `(`, and instead gives back one character when the lookahead fails,
  // handing over `tan`. Nothing dangerous — a false positive either way —
  // just a baffling failure for whoever writes the first one.
  it("does not read the colour tan out of tan() or tanh()", () => {
    const rules = parseRules(
      ".a { width: calc(tan(1rad) * 1px); }\n.b { width: calc(tanh(1) * 1px); }\n",
    );
    expect(componentColourLiterals(rules)).toEqual([]);
  });

  // A custom property *name* is not a paint decision, and the word scan
  // cannot tell one from a value. A token named for a colour would
  // otherwise fail the guard for its own name.
  it("does not read a colour out of a token's name", () => {
    const rules = parseRules(".a { color: var(--knx-teal-surface); }\n");
    expect(componentColourLiterals(rules)).toEqual([]);
  });

  it("still reads a var() fallback, which really is a literal", () => {
    const rules = parseRules(".a { color: var(--knx-teal-surface, red); }\n");
    expect(componentColourLiterals(rules).map((f) => f.literal)).toEqual(["red"]);
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

describe("theme contrast evaluation", () => {
  it("rejects a duplicate theme/accent key even when the later CSS block is unreadable", () => {
    const duplicate = parseRules(
      ':root[data-theme="x"][data-accent="mint"] { --knx-accent: #000; --knx-on-accent: #fff; }\n' +
      ':root[data-theme="x"][data-accent="mint"] { --knx-accent: #fff; --knx-on-accent: #fff; }',
    );
    expect(() => themeVariationBlocks(duplicate)).toThrow("x/mint");
  });

  it("uses the current WCAG sRGB linear breakpoint", () => {
    expect(contrastRatio({ r: 10.2, g: 10.2, b: 10.2 }, "#000"))
      .toBeCloseTo(1.061919504643963, 12);
  });

  it.each([Number.NaN, Number.POSITIVE_INFINITY, -1, 256])("rejects invalid numeric color channels: %s", (r) => {
    expect(() => contrastRatio({ r, g: 0, b: 0 }, "#fff")).toThrow("unsupported");
  });

  it.each([
    ["foreground green", { r: 0, g: Number.NaN, b: 0 }, "#fff"],
    ["foreground blue", { r: 0, g: 0, b: 256 }, "#fff"],
    ["background green", "#000", { r: 0, g: -1, b: 0 }],
    ["background blue", "#000", { r: 0, g: 0, b: Number.POSITIVE_INFINITY }],
  ] as const)("rejects an invalid %s channel", (_label, foreground, background) => {
    expect(() => contrastRatio(foreground, background)).toThrow("unsupported");
  });

  it.each(["#000f", "#000000ff", "rgb(0,0,0)", "rgba(0,0,0,1)"])("accepts opaque forms without changing the ratio: %s", (black) => {
    expect(contrastRatio(black, "#fff")).toBe(21);
  });

  it.each(["#0008", "#00000080"])("rejects non-opaque hex: %s", (color) => {
    expect(() => contrastRatio(color, "#fff")).toThrow("alpha");
  });

  it.each(["rgb(999,999,999)", `rgb(${"9".repeat(400)},0,0)`])("rejects out-of-range RGB instead of accepting false contrast: %s", (value) => {
    const block = themeBlocks(parseRules(
      `:root[data-theme="overflow"] { --knx-foreground: ${value}; --knx-bg: #fff; --knx-surface: #fff; --knx-on-accent: #000; --knx-accent: #fff; }`,
    ))[0];
    expect(evaluateThemeContrast(block)).toEqual(expect.arrayContaining([
      expect.objectContaining({ theme: "overflow", pair: "foreground on bg", reason: "unsupported", value: expect.stringContaining(value) }),
    ]));
  });

  it("names the terminal unsupported color behind a token reference", () => {
    const block = themeBlocks(parseRules(
      ':root[data-theme="alias"] { --knx-foreground: var(--knx-ink); --knx-ink: oklch(60% 0.2 30); --knx-bg: #fff; --knx-surface: #fff; --knx-on-accent: #000; --knx-accent: #fff; }',
    ))[0];
    expect(evaluateThemeContrast(block)).toEqual(expect.arrayContaining([
      expect.objectContaining({ theme: "alias", pair: "foreground on bg", reason: "unsupported", value: expect.stringContaining("oklch(60% 0.2 30)") }),
    ]));
  });

  it("computes exact WCAG ratios for black, white, equal colors, and the AA boundary", () => {
    expect(contrastRatio("#000", "#fff")).toBe(21);
    expect(contrastRatio("#ffffff", "#ffffff")).toBe(1);
    expect(contrastRatio("#767676", "#fff")).toBeGreaterThanOrEqual(4.5);
    expect(contrastRatio("#777777", "#fff")).toBeLessThan(4.5);
  });

  it("resolves recursive token references", () => {
    const block = themeBlocks(parseRules(
      ':root[data-theme="x"] { --knx-foreground: var(--knx-ink); --knx-ink: #000; --knx-bg: #fff; --knx-surface: #fff; --knx-on-accent: #000; --knx-accent: #fff; }',
    ))[0];
    expect(resolveThemeColor("var(--knx-foreground)", block.rule)).toEqual({ r: 0, g: 0, b: 0 });
  });

  it.each([
    ["alpha", "rgba(0, 0, 0, 0.5)", "alpha"],
    ["unknown notation", "oklch(60% 0.2 30)", "unsupported"],
    ["missing token", "var(--knx-missing)", "unresolved"],
    ["reference cycle", "var(--knx-cycle-a)", "cycle"],
  ])("reports %s with theme, pair, and offending value", (_label, value, reason) => {
    const block = themeBlocks(parseRules(
      `:root[data-theme="x"] { --knx-foreground: ${value}; --knx-cycle-a: var(--knx-cycle-b); --knx-cycle-b: var(--knx-cycle-a); --knx-bg: #fff; --knx-surface: #fff; --knx-on-accent: #000; --knx-accent: #fff; }`,
    ))[0];
    const violations = evaluateThemeContrast(block);
    expect(violations.some((violation) => violation.reason === reason && violation.theme === "x" && violation.pair === "foreground on bg" && violation.value.includes(value))).toBe(true);
  });

  it("does not skip a deliberately illegible or unsupported palette", () => {
    const block = themeBlocks(parseRules(
      ':root[data-theme="broken"] { --knx-foreground: #777; --knx-bg: #777; --knx-surface: oklch(60% 0.2 30); --knx-on-accent: #000; --knx-accent: #fff; }',
    ))[0];
    const violations = evaluateThemeContrast(block);
    expect(violations.map((violation) => violation.pair)).toEqual(
      expect.arrayContaining(["foreground on bg", "foreground on surface"]),
    );
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
  it.each(paletteThemeIds)("theme %s passes the strict role contrast gate", (id) => {
    expect(evaluateThemeContrast(requireBlock(id)), id).toEqual([]);
  });
});

describe("accent variations", () => {
  const variations = themeVariationBlocks(rules);

  it.each(variations.map((variation) => [`${variation.id}/${variation.accent}`, variation] as const))(
    "variation %s passes the strict role contrast gate",
    (name, variation) => {
      expect(evaluateThemeContrast(requireBlock(variation.id), variation), name).toEqual([]);
    },
  );

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
