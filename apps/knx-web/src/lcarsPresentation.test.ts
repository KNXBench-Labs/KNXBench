/** Built-in LCARS geometry is not an imported theme-pack capability (ADR-0092). */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { runInNewContext } from "node:vm";
import { describe, expect, it } from "vitest";
import { parseRules } from "./themeTokens";

const here = dirname(fileURLToPath(import.meta.url));
const css = readFileSync(join(here, "styles.css"), "utf8");
const presentation = parseRules(css).filter((rule) => rule.selector.includes('[data-presentation="lcars"]'));
const html = readFileSync(join(here, "..", "index.html"), "utf8");

function bootstrap(theme: string) {
  const root = { dataset: {} as Record<string, string>, setAttribute(name: string, value: string) { this.dataset[name] = value; } };
  runInNewContext(html.match(/<script>([\s\S]*?)<\/script>/)![1], {
    localStorage: { getItem: (key: string) => key === "knx-desktop:settings-cache" ? JSON.stringify({ settings: { theme } }) : null },
    matchMedia: () => ({ matches: false }), document: { documentElement: root },
  });
  return root.dataset;
}

describe("LCARS built-in presentation", () => {
  it("restores the built-in palette and presentation before first paint", () => {
    expect(bootstrap("lcars")).toMatchObject({ theme: "lcars", presentation: "lcars" });
    expect(bootstrap("graphite").presentation).toBeUndefined();
    expect(bootstrap("user-lcars").presentation).toBeUndefined();
  });

  it("frames the existing toolbar and navigation without another interactive DOM layer", () => {
    expect(presentation.some((rule) => rule.selector.endsWith(".workbench-toolbar::before"))).toBe(true);
    expect(presentation.some((rule) => rule.selector.endsWith(".workbench-navigation button"))).toBe(true);
    const decoration = presentation.filter((rule) => rule.selector.includes("::"));
    expect(decoration.length).toBeGreaterThan(0);
    for (const rule of decoration.filter((rule) => rule.declarations.some((d) => d.property === "content"))) {
      expect(rule.declarations).toContainEqual(expect.objectContaining({ property: "pointer-events", value: "none" }));
    }
  });

  it("leaves density and motion durations with their settings owner", () => {
    expect(presentation.length).toBeGreaterThan(0);
    expect(presentation.flatMap((rule) => rule.declarations).filter((d) =>
      ["--knx-control-height", "--knx-cell-padding", "--knx-transition-duration", "--knx-motion-easing"].includes(d.property))).toEqual([]);
    expect(presentation.some((rule) => rule.declarations.some((d) => d.value.includes("var(--knx-control-height)")))).toBe(true);
  });

  it("disables effects by default and enables only finite setting-driven feedback under OS permission", () => {
    const selector = '.workbench-navigation button[aria-current="page"]::after';
    const rules = presentation.filter((rule) => rule.selector.endsWith(selector));
    expect(rules.some((rule) => rule.ancestors.length === 0 && rule.declarations.some((d) => d.property === "animation-name" && d.value === "none"))).toBe(true);
    const enabled = rules.filter((rule) => rule.declarations.some((d) => d.property === "animation"));
    expect(enabled.length).toBeGreaterThan(0);
    for (const rule of enabled) {
      expect(rule.ancestors).toContain("@media (prefers-reduced-motion: no-preference)");
      expect(rule.selector).toContain('[data-motion-level="standard"]');
      expect(rule.declarations.find((d) => d.property === "animation")!.value).toContain("var(--knx-transition-duration)");
      expect(rule.declarations.find((d) => d.property === "animation")!.value).not.toContain("infinite");
    }
  });
});
