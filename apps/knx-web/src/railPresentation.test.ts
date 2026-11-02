/** Built-in Rail geometry and motion stay a trusted presentation, never a pack (ADR-0157). */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { runInNewContext } from "node:vm";
import { describe, expect, it } from "vitest";
import { builtInPresentation } from "./theme";
import { parseRules, themeBlocks } from "./themeTokens";

const here = dirname(fileURLToPath(import.meta.url));
const css = readFileSync(join(here, "styles.css"), "utf8");
const rules = parseRules(css);
const presentation = rules.filter((rule) => rule.selector.includes('[data-presentation="rail"]'));
const html = readFileSync(join(here, "..", "index.html"), "utf8");
const NO_PREFERENCE = "@media (prefers-reduced-motion: no-preference)";

function bootstrap(theme: string) {
  const root = { dataset: {} as Record<string, string>, setAttribute(name: string, value: string) { this.dataset[name] = value; } };
  runInNewContext(html.match(/<script>([\s\S]*?)<\/script>/)![1], {
    localStorage: { getItem: (key: string) => key === "knx-desktop:settings-cache" ? JSON.stringify({ settings: { theme } }) : null },
    matchMedia: () => ({ matches: false }), document: { documentElement: root },
  });
  return root.dataset;
}

function declarations(selectorEnd: string, ancestors: number) {
  return presentation.filter((rule) => rule.ancestors.length === ancestors && rule.selector.endsWith(selectorEnd))
    .flatMap((rule) => rule.declarations);
}

describe("Rail built-in presentation", () => {
  it("maps exactly the two Rail palettes to one presentation, before first paint and at runtime", () => {
    expect(bootstrap("rail")).toMatchObject({ theme: "rail", presentation: "rail" });
    expect(bootstrap("rail-dark")).toMatchObject({ theme: "rail-dark", presentation: "rail" });
    expect(bootstrap("lcars").presentation).toBe("lcars");
    for (const id of ["graphite", "user-rail", "rail-light"]) expect(bootstrap(id).presentation).toBeUndefined();
    expect(builtInPresentation("rail")).toBe("rail");
    expect(builtInPresentation("rail-dark")).toBe("rail");
    expect(builtInPresentation("lcars")).toBe("lcars");
    for (const id of ["system", "porcelain", "user-rail", "toString", "__proto__"]) expect(builtInPresentation(id)).toBeUndefined();
  });

  it("ships square buttons, fields and cards in both palettes", () => {
    for (const id of ["rail", "rail-dark"]) {
      const block = themeBlocks(rules).find((theme) => theme.id === id);
      expect(block, id).toBeDefined();
      const radius = (name: string) => block!.rule.declarations.find((d) => d.property === name)?.value;
      expect([radius("--knx-radius-button"), radius("--knx-radius-input"), radius("--knx-radius-card")]).toEqual(["0", "0", "0"]);
      expect(block!.rule.declarations.find((d) => d.property === "--knx-font-body")?.value).toBe('"Geist", sans-serif');
    }
  });

  it("keeps decoration pointer-inert and leaves density and motion tokens with their settings", () => {
    for (const rule of presentation.filter((rule) => rule.declarations.some((d) => d.property === "content"))) {
      expect(rule.declarations, rule.selector).toContainEqual(expect.objectContaining({ property: "pointer-events", value: "none" }));
    }
    expect(presentation.flatMap((rule) => rule.declarations).filter((d) =>
      ["--knx-control-height", "--knx-cell-padding", "--knx-transition-duration", "--knx-motion-easing", "--knx-feedback-duration"].includes(d.property))).toEqual([]);
  });

  it("animates only behind OS permission and the theme category, finitely, on the motion token", () => {
    const animated = presentation.filter((rule) => rule.declarations.some((d) => d.property === "animation" || d.property === "transition"));
    expect(animated.length).toBeGreaterThanOrEqual(8);
    for (const rule of animated) {
      expect(rule.ancestors, rule.selector).toContain(NO_PREFERENCE);
      expect(rule.selector).toMatch(/\[data-motion-theme="(standard|subtle)"\]/);
      expect(rule.selector).not.toContain("data-motion-level");
      for (const declaration of rule.declarations.filter((d) => d.property === "animation" || d.property === "transition")) {
        expect(declaration.value).not.toMatch(/infinite|forwards|\bboth\b/);
        if (declaration.property === "animation") expect(declaration.value).toMatch(/var\(--knx-transition-duration\)/);
      }
    }
  });

  it("starts every animated surface from a motion-free baseline", () => {
    expect(declarations(".workbench-toolbar::after", 0)).toContainEqual(expect.objectContaining({ property: "animation-name", value: "none" }));
    expect(declarations(".search-panel", 0)).toContainEqual(expect.objectContaining({ property: "animation-name", value: "none" }));
    expect(declarations(":is(.file-menu-content, .rename-context-menu)", 0)).toContainEqual(expect.objectContaining({ property: "animation-name", value: "none" }));
    expect(declarations(".workbench-navigation button::before", 0)).toContainEqual(expect.objectContaining({ property: "transition-property", value: "none" }));
    expect(presentation.flatMap((rule) => rule.declarations).filter((d) => d.property === "animation-play-state")).toEqual([]);
  });

  it("keyframes touch only paint and transform, never layout", () => {
    const frames = rules.filter((rule) => rule.ancestors.some((a) => /^@keyframes rail-/.test(a)));
    expect(frames.length).toBeGreaterThan(0);
    for (const property of new Set(frames.flatMap((rule) => rule.declarations.map((d) => d.property)))) {
      expect(["opacity", "transform", "clip-path", "background-size"]).toContain(property);
    }
  });
});
