/** Reads the theme layer out of a stylesheet so tests can enforce ADR-0022's token boundary. */

/**
 * ADR-0022 splits the `--knx-*` custom properties in two:
 *
 * - the **theme layer** — one `:root[data-theme="<id>"]` block per theme,
 *   holding every token the component layer reads that is a paint,
 *   typography or shape decision;
 * - the **component layer** — `:root` and the `[data-density]` /
 *   `[data-motion-*]` setting blocks, holding the tokens a *user setting*
 *   owns.
 *
 * A theme must define the whole theme layer and touch none of the
 * component layer. This module does the reading; `themeTokens.test.ts`
 * does the judging. Nothing here touches the filesystem, so the parser can
 * be tested against fixtures rather than against the real stylesheet.
 */

/** One `property: value` pair found directly inside a block. */
export interface CssDeclaration {
  property: string;
  value: string;
  /** 1-based line in the source text where the declaration starts. */
  line: number;
}

/** One brace-delimited block, with the declarations written directly in it. */
export interface CssRule {
  /** The prelude, whitespace collapsed: a selector list, or an at-rule. */
  selector: string;
  /** 1-based line in the source text where the prelude starts. */
  line: number;
  /** Nesting depth; 0 for a top-level rule. */
  depth: number;
  declarations: CssDeclaration[];
}

/**
 * Tokens the component layer owns. A theme block that sets one of these is
 * rebuilding a user setting inside a palette, which is the failure mode
 * ADR-0022 exists to prevent: the motion pair belongs to T27's motion
 * control, the geometry pair to the density control.
 */
export const COMPONENT_LAYER_TOKENS: readonly string[] = [
  "--knx-cell-padding",
  "--knx-control-height",
  "--knx-feedback-duration",
  "--knx-motion-easing",
  "--knx-transition-duration",
];

/**
 * The only non-custom property a theme block may declare. It is a paint
 * decision (form controls, scrollbars, the canvas behind everything) that
 * CSS offers no custom property for, so it is named rather than banned.
 */
export const THEME_BLOCK_PLAIN_PROPERTIES: readonly string[] = ["color-scheme"];

const THEME_BASE_SELECTOR = /^:root\[data-theme="([a-z0-9-]+)"\]$/;
const THEME_VARIATION_SELECTOR =
  /^:root\[data-theme="([a-z0-9-]+)"\]\[data-accent="([a-z0-9-]+)"\]$/;

/** Blanks comment bodies, keeping newlines so line numbers stay true. */
function stripComments(css: string): string {
  return css.replace(/\/\*[\s\S]*?\*\//g, (m) => m.replace(/[^\n]/g, " "));
}

/**
 * Walks a stylesheet by brace counting — no CSS parser, same approach as
 * the motion guard — and returns every block with the declarations written
 * directly inside it. Declarations inside a nested block belong to that
 * nested block, not to its parent.
 */
export function parseRules(css: string): CssRule[] {
  const source = stripComments(css);
  const rules: CssRule[] = [];
  const open: CssRule[] = [];
  let line = 1;
  let buffer = "";
  let bufferLine = 1;
  let bufferStarted = false;

  const startBuffer = (ch: string) => {
    if (!bufferStarted && ch.trim() !== "") {
      bufferLine = line;
      bufferStarted = true;
    }
  };
  const resetBuffer = () => {
    buffer = "";
    bufferStarted = false;
  };

  for (const ch of source) {
    if (ch === "{") {
      const rule: CssRule = {
        selector: buffer.trim().replace(/\s+/g, " "),
        line: bufferLine,
        depth: open.length,
        declarations: [],
      };
      rules.push(rule);
      open.push(rule);
      resetBuffer();
      continue;
    }
    if (ch === "}") {
      open.pop();
      resetBuffer();
      continue;
    }
    if (ch === ";") {
      const statement = buffer.trim();
      const colon = statement.indexOf(":");
      const owner = open[open.length - 1];
      if (owner && colon > 0) {
        owner.declarations.push({
          property: statement.slice(0, colon).trim(),
          value: statement.slice(colon + 1).trim().replace(/\s+/g, " "),
          line: bufferLine,
        });
      }
      resetBuffer();
      continue;
    }
    startBuffer(ch);
    buffer += ch;
    if (ch === "\n") line++;
  }

  return rules;
}

/** A `:root[data-theme="<id>"]` block: one theme's whole palette. */
export interface ThemeBlock {
  id: string;
  rule: CssRule;
}

/** A `:root[data-theme="<id>"][data-accent="<name>"]` block: an opt-in variation. */
export interface ThemeVariationBlock {
  id: string;
  accent: string;
  rule: CssRule;
}

/** Splits a selector list into its comma-separated parts. */
function selectorParts(selector: string): string[] {
  return selector.split(",").map((part) => part.trim()).filter((part) => part !== "");
}

export function themeBlocks(rules: readonly CssRule[]): ThemeBlock[] {
  const blocks: ThemeBlock[] = [];
  for (const rule of rules) {
    const match = THEME_BASE_SELECTOR.exec(rule.selector);
    if (match) blocks.push({ id: match[1], rule });
  }
  return blocks;
}

export function themeVariationBlocks(rules: readonly CssRule[]): ThemeVariationBlock[] {
  const blocks: ThemeVariationBlock[] = [];
  for (const rule of rules) {
    const match = THEME_VARIATION_SELECTOR.exec(rule.selector);
    if (match) blocks.push({ id: match[1], accent: match[2], rule });
  }
  return blocks;
}

/**
 * Every selector that mentions `data-theme` in a shape the boundary does
 * not allow. Two shapes are allowed and no others: the base block and the
 * accent variation. Everything else — a theme styling an element
 * (`[data-theme="x"] body::before`), or the accent-by-negation trap
 * (`:root:not([data-theme="x"])`, which silently swept every future theme
 * into one palette) — is reported here.
 */
export function themeSelectorViolations(rules: readonly CssRule[]): CssRule[] {
  return rules.filter((rule) =>
    selectorParts(rule.selector).some(
      (part) =>
        part.includes("data-theme") &&
        !THEME_BASE_SELECTOR.test(part) &&
        !THEME_VARIATION_SELECTOR.test(part),
    ),
  );
}

/** Every `--knx-*` token the stylesheet reads through `var()`, sorted. */
export function referencedTokens(css: string): string[] {
  const found = new Set<string>();
  for (const match of stripComments(css).matchAll(/var\(\s*(--knx-[a-z0-9-]+)/g)) {
    found.add(match[1]);
  }
  return [...found].sort();
}

/** Every `--knx-*` token the stylesheet declares anywhere, sorted. */
export function declaredTokens(rules: readonly CssRule[]): string[] {
  const found = new Set<string>();
  for (const rule of rules) {
    for (const declaration of rule.declarations) {
      if (declaration.property.startsWith("--knx-")) found.add(declaration.property);
    }
  }
  return [...found].sort();
}

/**
 * The boundary itself, derived rather than hand-maintained: a token is a
 * theme's to set exactly when the component layer reads it and no user
 * setting owns it. Deriving it is the point — add `var(--knx-whatever)` to
 * a component rule and every theme must answer for it on the next test run,
 * which no hand-written list would have caught.
 */
export function requiredThemeTokens(css: string): string[] {
  const owned = new Set(COMPONENT_LAYER_TOKENS);
  return referencedTokens(css).filter((token) => !owned.has(token));
}

/** The `--knx-*` tokens one block declares, sorted. */
export function blockTokens(rule: CssRule): string[] {
  return rule.declarations
    .map((declaration) => declaration.property)
    .filter((property) => property.startsWith("--knx-"))
    .sort();
}

/** The non-custom properties one block declares, sorted. */
export function blockPlainProperties(rule: CssRule): string[] {
  return rule.declarations
    .map((declaration) => declaration.property)
    .filter((property) => !property.startsWith("--"))
    .sort();
}
