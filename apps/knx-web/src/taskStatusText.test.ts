/** Pins the slow text sweep, quiet pause and structural motion/accessibility gates. */
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { parseRules, themeBlocks, resolveThemeColor, contrastRatio } from "./themeTokens";

const rules = parseRules(readFileSync(new URL("./styles.css", import.meta.url), "utf8"));
const shimmer = () => rules.find(rule => rule.selector === ':root:not([data-motion-level="off"]) .task-status-text');
const value = (selector: string, property: string) => rules.find(rule => rule.selector === selector)
  ?.declarations.find(declaration => declaration.property === property)?.value;

describe("task status text shimmer", () => {
  it("only animates with motion allowed, normal colours and text clipping supported", () => {
    expect(shimmer()).toBeDefined();
    expect(shimmer()!.ancestors).toContain("@media (prefers-reduced-motion: no-preference)");
    expect(shimmer()!.ancestors).toContain("@media (forced-colors: none)");
    expect(shimmer()!.ancestors).toContain("@supports (background-clip: text) or (-webkit-background-clip: text)");
    expect(shimmer()!.ancestors).toContain("@supports (color: color-mix(in srgb, currentColor 70%, transparent)) and (-webkit-text-fill-color: transparent)");
    expect(shimmer()!.declarations.find(declaration => declaration.property === "animation")?.value)
      .toBe("knx-task-text-shimmer calc(var(--knx-transition-duration) * var(--task-status-cycle-factor, 17.5)) linear infinite");
  });

  it("sweeps left to right for 2.5 seconds then holds outside the text for one second", () => {
    const frames = rules.filter(rule => rule.ancestors.includes("@keyframes knx-task-text-shimmer"));
    expect(frames.map(rule => rule.selector)).toEqual(["0%", "71.428571%, 100%"]);
    expect(frames.map(rule => rule.declarations.find(declaration => declaration.property === "background-position")?.value))
      .toEqual(["110% 0", "-10% 0"]);
  });

  it("uses opaque theme text colours, not blinking opacity or layout transforms", () => {
    expect(shimmer()).toBeDefined();
    const declarations = shimmer()!.declarations;
    expect(declarations.find(declaration => declaration.property === "background-image")?.value).toContain("currentColor");
    expect(declarations.find(declaration => declaration.property === "background-image")?.value).toContain("var(--knx-muted)");
    expect(declarations.some(declaration => ["opacity", "transform", "width", "display"].includes(declaration.property))).toBe(false);
  });

  it("keeps the dimmest shimmer text above 4.5:1 on built-in backgrounds", () => {
    const themes = themeBlocks(rules);
    expect(themes.length).toBeGreaterThanOrEqual(2);
    for (const theme of themes) {
      const foreground = resolveThemeColor("var(--knx-foreground)", theme.rule);
      const muted = resolveThemeColor("var(--knx-muted)", theme.rule);
      const baseline = { r: foreground.r * 0.7 + muted.r * 0.3,
        g: foreground.g * 0.7 + muted.g * 0.3, b: foreground.b * 0.7 + muted.b * 0.3 };
      for (const token of ["--knx-bg", "--knx-surface"]) {
        const background = resolveThemeColor(`var(${token})`, theme.rule);
        expect(contrastRatio(baseline, background), `${theme.id} shimmer on ${token}`).toBeGreaterThanOrEqual(4.5);
        expect(contrastRatio(foreground, background), `${theme.id} glint on ${token}`).toBeGreaterThanOrEqual(4.5);
      }
    }
  });

  it("keeps warning text's semantic colour", () => {
    expect(value(':root:not([data-motion-level="off"]) .form-warning .task-status-text', "color")).toBe("inherit");
  });
});
