/** Shipped CRT palette admission, exact design values and export/DOM roundtrip. */
// SPDX-License-Identifier: AGPL-3.0-or-later
// @vitest-environment happy-dom
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { parseThemePackText, THEME_TOKEN_CLASSES } from "./themePack";
import { exportThemePack } from "./themePackFiles";
import { applyThemePack } from "./themePackDom";
import { contrastRatio } from "./themeTokens";

const file = join(dirname(fileURLToPath(import.meta.url)), "../themes/modern-retro-green-crt.knx-theme.json");
const text = () => existsSync(file) ? readFileSync(file, "utf8") : "{}";
function admitted() {
  const result = parseThemePackText(text());
  expect(result.ok, JSON.stringify(result)).toBe(true);
  if (!result.ok) throw new Error("CRT palette was not admitted");
  return result.pack;
}

describe("Modern Retro Green CRT", () => {
  it("ships a complete importable v1 palette", () => {
    const pack = admitted();
    expect(pack).toMatchObject({ id: "user-modern-retro-green-crt", name: "Modern Retro Green CRT",
      format: "knxbench-theme", formatVersion: 1, tokenVersion: 1, version: "1.1.0", colorScheme: "dark" });
    expect(Object.keys(pack.tokens).sort()).toEqual(Object.keys(THEME_TOKEN_CLASSES).sort());
  });
  it("uses the reference's phosphor ink and quiet green rules while keeping interactive neon", () => {
    const { tokens } = admitted();
    expect(tokens["--knx-bg"]).toBe("#050505");
    expect(tokens["--knx-surface"]).toBe("#050b06");
    expect(tokens["--knx-foreground"]).toBe("#bedbbb");
    expect(tokens["--knx-muted"]).toBe("#91b48b");
    expect(tokens["--knx-border"]).toBe("#31573b");
    for (const key of ["--knx-border-hover", "--knx-accent"] as const) {
      expect(tokens[key]).toBe("#39ff14");
    }
  });
  it("uses installed JetBrains Mono throughout, crisp corners and restrained green glow", () => {
    const { tokens } = admitted();
    for (const key of ["--knx-font-heading", "--knx-font-body", "--knx-font-mono"] as const) {
      expect(tokens[key]).toBe('"JetBrains Mono", monospace');
    }
    for (const key of ["--knx-radius-card", "--knx-radius-button", "--knx-radius-input"] as const) {
      expect(tokens[key]).toBe("2px");
    }
    expect(tokens["--knx-shadow-raised"]).toBe("0px 0px 6px 0px #39ff141a");
    expect(tokens["--knx-shadow-hover"]).toBe("0px 0px 16px 0px #39ff1466");
  });
  it("retains readable selection/action ink rather than using purple as the shared accent", () => {
    const { tokens } = admitted();
    expect(contrastRatio(tokens["--knx-accent"], tokens["--knx-bg"])).toBeGreaterThanOrEqual(4.5);
    expect(contrastRatio(tokens["--knx-muted"], tokens["--knx-surface"])).toBeGreaterThanOrEqual(4.5);
    expect(contrastRatio(tokens["--knx-on-accent"], tokens["--knx-accent"])).toBeGreaterThanOrEqual(4.5);
  });
  it("does not impersonate motion, density or external assets", () => {
    const pack = admitted();
    expect(pack.accents).toBeUndefined();
    expect(Object.keys(pack.tokens)).not.toContain("--knx-transition-duration");
    expect(Object.values(pack.tokens).join(" ")).not.toMatch(/url\(|@import|animation|transition/);
    expect(pack.tokens["--knx-backdrop-image"]).toBe("none");
  });
  it("roundtrips admitted values and metadata through the production exporter", () => {
    const pack = admitted();
    const output = exportThemePack(pack);
    expect(output.ok).toBe(true);
    if (!output.ok) throw new Error("Export failed");
    expect(output.fileName).toBe("user-modern-retro-green-crt.knx-theme.json");
    expect(parseThemePackText(output.text)).toEqual({ ok: true, pack });
  });
  it("applies and releases the palette without changing existing user controls", () => {
    const root = document.createElement("div");
    root.style.setProperty("--knx-bg", "#123456", "important");
    root.style.setProperty("--knx-transition-duration", "0ms");
    const previous = root.style.cssText;
    const application = applyThemePack(root, admitted(), "violet");
    expect(application.ok).toBe(true);
    if (!application.ok) throw new Error("DOM admission failed");
    expect(root.style.getPropertyValue("--knx-bg")).toBe("#050505");
    expect(root.style.getPropertyValue("--knx-accent")).toBe("#39ff14");
    expect(root.style.getPropertyValue("--knx-transition-duration")).toBe("0ms");
    application.release();
    expect(root.style.cssText).toBe(previous);
  });
});

describe("CRT reference design study", () => {
  const preview = () => readFileSync(join(dirname(file), "../../../design/modern-retro-green-crt.preview.html"), "utf8");
  it("uses native table cells and independent checkbox/address-button controls", () => {
    const document = new DOMParser().parseFromString(preview(), "text/html");
    expect(document.querySelectorAll("table")).toHaveLength(1);
    const rows = [...document.querySelectorAll("tbody tr")];
    expect(rows).toHaveLength(12);
    for (const row of rows) {
      expect(row.querySelectorAll("td")).toHaveLength(6);
      expect(row.querySelectorAll('input[type="checkbox"]')).toHaveLength(1);
      expect(row.querySelectorAll("button.table-select")).toHaveLength(1);
    }
    expect(new Set(rows.map((row) => row.getAttribute("data-address"))).size).toBe(rows.length);
    expect(document.querySelector("#row-light")?.getAttribute("aria-hidden")).toBe("true");
    expect(document.querySelector("#row-light .beam-core")?.tagName).toBe("SPAN");
  });
  it("keeps the core preview palette aligned with the admitted theme", () => {
    const source = preview(), { tokens } = admitted();
    for (const [variable, token] of [["bg", "--knx-bg"], ["surface", "--knx-surface"],
      ["ink", "--knx-foreground"], ["muted", "--knx-muted"], ["border", "--knx-border"],
      ["neon", "--knx-accent"]] as const) {
      expect(source).toContain(`--${variable}: ${tokens[token]};`);
    }
  });
  it("does not embed the private reference photograph or fetch external data", () => {
    const source = preview();
    expect(source).not.toMatch(/watermarked_img|<img\b|\bfetch\s*\(|XMLHttpRequest|WebSocket/);
    expect(source).toContain("synthetische Daten");
  });
});
