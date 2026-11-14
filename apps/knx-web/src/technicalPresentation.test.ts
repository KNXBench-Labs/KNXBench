/** Pins technical palette identity, trusted bootstrap framing and independent density geometry. */
import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";
import { describe, expect, it } from "vitest";
import { builtInPresentation, getThemeAccentOptions, THEMES } from "./theme";
import { parseRules, themeBlocks } from "./themeTokens";

const css = readFileSync(new URL("./styles.css", import.meta.url), "utf8");
const html = readFileSync(new URL("../index.html", import.meta.url), "utf8");
const rules = parseRules(css);
const palettes = [
  { id: "technical-light", name: "Technical Light", scheme: "light", colors: [
    "#edf1f5", "#ffffff", "#202d3b", "#536376", "#a8b6c5", "#145b8f", "#ffffff", "#a52336", "#805000", "#176344",
  ] },
  { id: "technical-dark", name: "Technical Dark", scheme: "dark", colors: [
    "#161e27", "#202a35", "#e6edf5", "#b4c1cf", "#607286", "#8fcaff", "#102c42", "#ff9d9d", "#ffd180", "#8dd9b4",
  ] },
];
const colorTokens = ["bg", "surface", "foreground", "muted", "border", "accent", "on-accent", "error-color", "warning-color", "success-color"];

function bootstrap(theme: string, density: string, dark: boolean) {
  const root = { dataset: {} as Record<string, string>, setAttribute(name: string, value: string) { this.dataset[name] = value; } };
  runInNewContext(html.match(/<script>([\s\S]*?)<\/script>/)![1], {
    localStorage: { getItem: (key: string) => key === "knx-desktop:settings-cache"
      ? JSON.stringify({ settings: { theme, density, motionLevel: "off" } }) : null },
    matchMedia: () => ({ matches: dark }), document: { documentElement: root },
  });
  return root.dataset;
}

describe("Technical built-in presentation", () => {
  for (const palette of palettes) {
    it(`${palette.id} keeps its exact palette and identity without accent variants`, () => {
      expect(THEMES.find(theme => theme.id === palette.id)).toEqual({ id: palette.id, name: palette.name, hasAccentVariations: false });
      expect(getThemeAccentOptions(palette.id)).toEqual([]);
      const block = themeBlocks(rules).find(theme => theme.id === palette.id)!;
      const declarations = Object.fromEntries(block.rule.declarations.map(d => [d.property, d.value]));
      expect(declarations["color-scheme"]).toBe(palette.scheme);
      expect(colorTokens.map(token => declarations[`--knx-${token}`])).toEqual(palette.colors);
      expect(declarations["--knx-font-body"]).toBe('"Inter", sans-serif');
      expect(declarations["--knx-font-mono"]).toBe('"JetBrains Mono", monospace');
      for (const shape of ["card", "button", "input"]) expect(declarations[`--knx-radius-${shape}`]).toBe("2px");
      for (const shadow of ["raised", "hover"]) expect(declarations[`--knx-shadow-${shadow}`]).toBe("none");
    });
    for (const density of ["compact", "comfortable"]) for (const dark of [false, true]) {
      it(`${palette.id} restores ${density} before first paint under OS dark=${dark}`, () => {
        expect(bootstrap(palette.id, density, dark)).toMatchObject({ theme: palette.id, presentation: "technical", density });
        expect(builtInPresentation(palette.id)).toBe("technical");
      });
    }
  }

  it("does not grant technical framing to packs, arbitrary IDs or inherited keys", () => {
    for (const id of ["user-technical", "technical", "technical-blue", "__proto__", "toString"]) {
      expect(builtInPresentation(id)).toBeUndefined();
      expect(bootstrap(id, "compact", false).presentation).toBeUndefined();
    }
    expect(builtInPresentation("rail-dark")).toBe("rail");
    expect(bootstrap("system", "comfortable", true).theme).toBe("graphite");
  });

  it("adds no animation or density overrides to the trusted presentation", () => {
    const presentation = rules.filter(rule => rule.selector.includes('[data-presentation="technical"]'));
    expect(presentation.length).toBeGreaterThan(0);
    expect(presentation.flatMap(rule => rule.declarations).filter(d =>
      /animation|transition|--workspace-|--knx-control-height|--knx-cell-padding/.test(d.property))).toEqual([]);
    const root = Object.fromEntries(rules.filter(rule => rule.selector === ":root").flatMap(rule => rule.declarations).map(d => [d.property, d.value]));
    expect(root).toMatchObject({ "--workspace-scalar-width": "10rem", "--workspace-text-width": "24rem",
      "--workspace-panel-padding": "12px", "--workspace-row-padding": "4px", "--knx-control-height": "32px" });
    const comfortable = Object.fromEntries(rules.filter(rule => rule.selector === ':root[data-density="comfortable"]')
      .flatMap(rule => rule.declarations).map(d => [d.property, d.value]));
    expect(comfortable).toMatchObject({ "--workspace-panel-padding": "16px", "--workspace-row-padding": "8px", "--knx-control-height": "40px" });
    expect(comfortable).not.toHaveProperty("--workspace-scalar-width");
    expect(comfortable).not.toHaveProperty("--workspace-text-width");
  });
});
