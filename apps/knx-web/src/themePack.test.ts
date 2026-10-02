/** Executable v1 theme admission and lossless value-preservation contract. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { describe, expect, it } from "vitest";
import { THEME_TOKEN_CLASSES, parseThemePackText, validateThemePack } from "./themePack";
import { themePackFixture } from "./themePackFixtures";
import { contrastRatio } from "./themeTokens";

describe("theme pack admission", () => {
  it("admits a complete v1 pack without normalizing author values", () => {
    const pack = themePackFixture();
    expect(parseThemePackText(JSON.stringify(pack))).toEqual({ ok: true, pack });
  });
  it.each(["", "{", "{} trailing", '{"x":1,}', "/* no comments */ {}"])(
    "reports malformed JSON without throwing: %s", (text) => {
      expect(parseThemePackText(text)).toEqual({ ok: false, diagnostic: { kind: "invalidJson", path: "$" } });
    },
  );
  it.each([null, [], 1, "pack", { ...themePackFixture(), extra: true }])(
    "rejects a non-contract root: %j", (raw) => {
      expect(parseThemePackText(JSON.stringify(raw))).toMatchObject({ ok: false, diagnostic: { kind: "invalidContract" } });
    },
  );
  it.each(Object.keys(themePackFixture()))("requires the root field %s", (key) => {
    const raw: Record<string, unknown> = themePackFixture();
    delete raw[key];
    expect(parseThemePackText(JSON.stringify(raw))).toMatchObject({ ok: false, diagnostic: { kind: "invalidContract" } });
  });
  it.each(["formatVersion", "tokenVersion"])("rejects unsupported or untyped %s", (field) => {
    for (const value of [0, 2, 1.1, "1", null, true]) {
      const raw = { ...themePackFixture(), [field]: value };
      expect(parseThemePackText(JSON.stringify(raw))).toMatchObject({ ok: false, diagnostic: { kind: "unsupportedVersion" } });
    }
  });
  it("rejects a foreign format marker", () => {
    expect(parseThemePackText(JSON.stringify({ ...themePackFixture(), format: "css" })))
      .toMatchObject({ ok: false, diagnostic: { kind: "unsupportedFormat" } });
  });
  it.each(["system", "porcelain", "light", "user-", "user--x", "user-X", "user-a/x", `user-a${"x".repeat(43)}`, 1])(
    "rejects an unsafe or reserved identity: %s", (id) => {
      expect(parseThemePackText(JSON.stringify({ ...themePackFixture(), id })))
        .toMatchObject({ ok: false, diagnostic: { kind: "invalidMetadata", path: "$.id" } });
    },
  );
  it.each(["name", "version"])("bounds safe display metadata: %s", (field) => {
    const max = field === "name" ? 80 : 32;
    for (const value of [null, 2, "", "  ", "a".repeat(max + 1), "\u0001", "\u0085", "\u061c", "\u200e", "\u202e", "\u2068", "\ud800", "\udfff"]) {
      expect(parseThemePackText(JSON.stringify({ ...themePackFixture(), [field]: value })))
        .toMatchObject({ ok: false, diagnostic: { kind: "invalidMetadata", path: `$.${field}` } });
    }
    const pack = { ...themePackFixture(), [field]: "😀".repeat(max) };
    expect(parseThemePackText(JSON.stringify(pack))).toEqual({ ok: true, pack });
  });
  it("retains harmless metadata text and the minimum/maximum ASCII identities", () => {
    for (const id of ["user-a", `user-a${"x".repeat(42)}`]) {
      const pack = { ...themePackFixture(), id, name: ' <img src=x onerror="alert(1)"> ' };
      expect(parseThemePackText(JSON.stringify(pack))).toEqual({ ok: true, pack });
    }
  });
  it.each(["system", "LIGHT", "", 0, null])("rejects non-enum colorScheme %s", (colorScheme) => {
    expect(parseThemePackText(JSON.stringify({ ...themePackFixture(), colorScheme })))
      .toMatchObject({ ok: false, diagnostic: { kind: "invalidMetadata", path: "$.colorScheme" } });
  });
  it.each(Object.keys(themePackFixture().tokens))("requires palette token %s", (key) => {
    const pack = themePackFixture(); delete pack.tokens[key];
    expect(parseThemePackText(JSON.stringify(pack)))
      .toMatchObject({ ok: false, diagnostic: { kind: "invalidTokens", path: "$.tokens" } });
  });
  it.each(["--app-ui-scale", "--knx-cell-padding", "--knx-control-height", "--knx-transition-duration", "--unknown", "__proto__", "constructor"])(
    "rejects extra/foreign/component-owned token %s", (key) => {
      const pack = themePackFixture(); const tokens = { ...pack.tokens, [key]: "1px" };
      expect(Object.hasOwn(tokens, key)).toBe(true);
      expect(parseThemePackText(JSON.stringify({ ...pack, tokens })))
        .toMatchObject({ ok: false, diagnostic: { kind: "invalidTokens", path: "$.tokens" } });
    },
  );
  it.each([null, [], "css", false])("rejects non-object token map %j", (tokens) => {
    expect(parseThemePackText(JSON.stringify({ ...themePackFixture(), tokens })))
      .toMatchObject({ ok: false, diagnostic: { kind: "invalidTokens", path: "$.tokens" } });
  });
  it.each(Object.keys(themePackFixture().tokens))("requires bounded safe string value for %s", (key) => {
    for (const value of [null, 2, [], {}, "x".repeat(513), "\u0085", "\ud800"]) {
      const pack = themePackFixture();
      const tokens: Record<string, unknown> = { ...pack.tokens, [key]: value };
      expect(parseThemePackText(JSON.stringify({ ...pack, tokens })))
        .toMatchObject({ ok: false, diagnostic: { kind: "invalidValue", path: `$.tokens.${key}` } });
    }
  });
  it.each(Object.entries(THEME_TOKEN_CLASSES).filter(([, kind]) => kind === "opaque" || kind === "colour"))(
    "admits only full hex colour syntax for %s", (key) => {
      for (const value of ["", "red", "#fff", "#12345g", "#123456789", "#123456 ", " #123456", "var(--knx-bg)", "url(x)", "#123456; color:red", "#123456!important", "#123456/*x*/", "\\23ffffff"]) {
        const pack = themePackFixture(); pack.tokens[key] = value;
        expect(parseThemePackText(JSON.stringify(pack)))
          .toMatchObject({ ok: false, diagnostic: { kind: "invalidValue", path: `$.tokens.${key}` } });
      }
    },
  );
  it.each(["--knx-bg", "--knx-surface", "--knx-foreground", "--knx-accent", "--knx-on-accent"])(
    "requires role colour %s to be opaque", (key) => {
      const pack = themePackFixture(); pack.tokens[key] += "fe";
      expect(parseThemePackText(JSON.stringify(pack)))
        .toMatchObject({ ok: false, diagnostic: { kind: "invalidValue", path: `$.tokens.${key}` } });
    },
  );
  it("preserves uppercase opaque alpha and non-role translucency", () => {
    const pack = themePackFixture();
    for (const key of ["--knx-bg", "--knx-surface", "--knx-foreground", "--knx-accent", "--knx-on-accent"]) {
      pack.tokens[key] = `${pack.tokens[key].toUpperCase()}FF`;
    }
    pack.tokens["--knx-muted"] = "#1234567a";
    expect(parseThemePackText(JSON.stringify(pack))).toEqual({ ok: true, pack });
  });
  it.each(["--knx-gradient-primary", "--knx-gradient-display", "--knx-backdrop-image", "--knx-backdrop-mask"])(
    "bounds complete gradient grammar for %s", (key) => {
      const invalid = ["none; color:red", "url(data:text/html,x)", "var(--knx-gradient-primary)",
        "radial-gradient(#000000,#ffffff)", "linear-gradient(0deg,#000000)",
        "linear-gradient(0deg,#000000,#111111,#222222,#333333,#444444)",
        "linear-gradient(0deg,  #000000,#ffffff)", "linear-gradient(0deg,#000000 ,#ffffff)",
        "linear-gradient(0deg,#000000 10%,#ffffff)", "linear-gradient(0deg,red,#ffffff)",
        "linear-gradient(0deg,#000000,#ffffff)/*x*/", "LINEAR-GRADIENT(0deg,#000000,#ffffff)",
        ...["361", "-1", "-0", "+1", ".5", "01", "1e2", "Infinity", "NaN"].map((angle) => `linear-gradient(${angle}deg,#000000,#ffffff)`)];
      for (const value of invalid) {
        const pack = themePackFixture(); pack.tokens[key] = value;
        expect(parseThemePackText(JSON.stringify(pack)))
          .toMatchObject({ ok: false, diagnostic: { kind: "invalidValue", path: `$.tokens.${key}` } });
      }
      for (const value of ["none", "linear-gradient(0deg,#00112233,#ffffff)", "linear-gradient(360deg, #000000, #ffffff)",
        "linear-gradient(12.25deg,#000000,#111111,#222222,#333333)"]) {
        const pack = themePackFixture(); pack.tokens[key] = value;
        expect(parseThemePackText(JSON.stringify(pack))).toEqual({ ok: true, pack });
      }
    },
  );
  it("does not round a decimal overflow into the gradient angle bound", () => {
    const pack = themePackFixture();
    pack.tokens["--knx-gradient-primary"] = "linear-gradient(360.000000000000000000001deg,#000000,#ffffff)";
    expect(parseThemePackText(JSON.stringify(pack)))
      .toMatchObject({ ok: false, diagnostic: { kind: "invalidValue" } });
  });
  it.each(["--knx-radius-card", "--knx-radius-button", "--knx-radius-input"])("bounds complete radius %s", (key) => {
    for (const value of ["-1px", "-0px", "1000px", "999.000000000000000000001px", "1em", "+1px", ".5px", "01px", "1e2px", "1px ", "1px; color:red", "var(--knx-radius-card)", `${"9".repeat(300)}px`]) {
      const pack = themePackFixture(); pack.tokens[key] = value;
      expect(parseThemePackText(JSON.stringify(pack))).toMatchObject({ ok: false, diagnostic: { kind: "invalidValue" } });
    }
    for (const value of ["0px", "999px", "999.000px", "12.25px"]) {
      const pack = themePackFixture(); pack.tokens[key] = value;
      expect(parseThemePackText(JSON.stringify(pack))).toEqual({ ok: true, pack });
    }
  });
  it.each(["--knx-shadow-raised", "--knx-shadow-hover"])("bounds full shadow %s", (key) => {
    for (const value of ["0px 1px 2px #000000", "65px 0px 0px 0px #000000", "0px -65px 0px 0px #000000",
      "0px 0px 129px 0px #000000", "0px 0px -0px 0px #000000", "0px 0px 0px 65px #000000",
      "-64.000000000000000000001px 0px 0px 0px #000000", "0px 0px 0px 0px red", "inset 0px 0px 0px 0px #000000",
      "0px  0px 0px 0px #000000", "0px 0px 0px 0px #000000,0px 0px 0px 0px #ffffff", "url(x)"]) {
      const pack = themePackFixture(); pack.tokens[key] = value;
      expect(parseThemePackText(JSON.stringify(pack))).toMatchObject({ ok: false, diagnostic: { kind: "invalidValue" } });
    }
    for (const value of ["none", "-64px 64px 128px -64px #AABBCcdd", "0px 0px 0px 0px #ffffff"]) {
      const pack = themePackFixture(); pack.tokens[key] = value;
      expect(parseThemePackText(JSON.stringify(pack))).toEqual({ ok: true, pack });
    }
  });
  it("bounds both backdrop dimensions", () => {
    for (const value of ["auto auto", "1px", "0px 1px", "1px 513px", "1px 512.000000000000000000001px", "1px  1px", "1em 1px", "1px 1px 1px", "var(--knx-bg)", "url(x)"]) {
      const pack = themePackFixture(); pack.tokens["--knx-backdrop-size"] = value;
      expect(parseThemePackText(JSON.stringify(pack))).toMatchObject({ ok: false, diagnostic: { kind: "invalidValue" } });
    }
    for (const value of ["auto", "1px 512px", "1.25px 20.5px"]) {
      const pack = themePackFixture(); pack.tokens["--knx-backdrop-size"] = value;
      expect(parseThemePackText(JSON.stringify(pack))).toEqual({ ok: true, pack });
    }
  });
  it.each(["--knx-font-heading", "--knx-font-body", "--knx-font-mono"])("admits only installed/system font stacks for %s", (key) => {
    for (const value of ["Inter", '"Unknown", sans-serif', "url(https://example.invalid/font)", "@font-face{}",
      "var(--knx-font-body)", '"Inter", sans-serif; color:red', '"Inter", sans-serif ', "none"]) {
      const pack = themePackFixture(); pack.tokens[key] = value;
      expect(parseThemePackText(JSON.stringify(pack))).toMatchObject({ ok: false, diagnostic: { kind: "invalidValue" } });
    }
    for (const value of ['"Inter", sans-serif', '"Space Grotesk", sans-serif', '"JetBrains Mono", monospace',
      "system-ui, sans-serif", "ui-monospace, monospace"]) {
      const pack = themePackFixture(); pack.tokens[key] = value;
      expect(parseThemePackText(JSON.stringify(pack))).toEqual({ ok: true, pack });
    }
  });
  it.each([null, [], true, "violet", { unknown: {} }, { constructor: {} },
    { violet: null }, { violet: [] }, { violet: { "--knx-accent": "#7041dc" } },
    { violet: { "--knx-accent": "#7041dc", "--knx-on-accent": "#ffffff", "--knx-bg": "#ffffff" } },
    { violet: { "--knx-accent": "#7041dcfe", "--knx-on-accent": "#ffffff" } },
    { violet: { "--knx-accent": "#7041dc", "--knx-on-accent": "url(x)" } }].map((accents) => ({ accents })))(
    "rejects an invalid accent section: $accents", ({ accents }) => {
      expect(parseThemePackText(JSON.stringify({ ...themePackFixture(), accents })))
        .toMatchObject({ ok: false, diagnostic: { kind: "invalidAccents" } });
    },
  );
  it("preserves an empty or complete supported accent map", () => {
    const variations = Object.fromEntries(["violet", "mint", "blue", "amber", "rose"].map((id) =>
      [id, { "--knx-accent": "#7041dcFF", "--knx-on-accent": "#FFFFFF" }]));
    for (const accents of [{}, variations]) {
      const pack = { ...themePackFixture(), accents };
      expect(parseThemePackText(JSON.stringify(pack))).toEqual({ ok: true, pack });
    }
  });
  it.each([
    ["--knx-bg", "#202438", "foreground on bg"],
    ["--knx-surface", "#202438", "foreground on surface"],
    ["--knx-accent", "#ffffff", "on-accent on accent"],
  ])("checks runtime role pair for %s", (key, value, pair) => {
    const pack = themePackFixture(); pack.tokens[key] = value;
    expect(parseThemePackText(JSON.stringify(pack))).toMatchObject({ ok: false, diagnostic: {
      kind: "contrast", path: "$.tokens", violations: expect.arrayContaining([expect.objectContaining({ pair, reason: "contrast" })]),
    } });
  });
  it.each(["violet", "mint", "blue", "amber", "rose"])("checks runtime contrast for declared accent %s", (accent) => {
    const pack = { ...themePackFixture(), accents: { [accent]: { "--knx-accent": "#ffffff", "--knx-on-accent": "#ffffff" } } };
    expect(parseThemePackText(JSON.stringify(pack))).toMatchObject({ ok: false, diagnostic: {
      kind: "contrast", path: `$.accents.${accent}`, violations: expect.arrayContaining([expect.objectContaining({ accent, pair: "on-accent on accent" })]),
    } });
  });
  it("does not stop validation after the first valid accent", () => {
    const pack = { ...themePackFixture(), accents: {
      violet: { "--knx-accent": "#7041dc", "--knx-on-accent": "#ffffff" },
      rose: { "--knx-accent": "#ffffff", "--knx-on-accent": "#ffffff" },
    } };
    expect(parseThemePackText(JSON.stringify(pack))).toMatchObject({ ok: false, diagnostic: { kind: "contrast", path: "$.accents.rose" } });
  });
  it("never rounds failing role contrast into acceptance", () => {
    const pack = themePackFixture(); pack.tokens["--knx-foreground"] = "#777777";
    pack.tokens["--knx-bg"] = "#ffffff"; pack.tokens["--knx-surface"] = "#ffffff";
    const ratio = contrastRatio("#777777", "#ffffff");
    expect(ratio).toBeLessThan(4.5); expect(ratio.toFixed(1)).toBe("4.5");
    expect(parseThemePackText(JSON.stringify(pack))).toMatchObject({ ok: false, diagnostic: {
      kind: "contrast", violations: expect.arrayContaining([expect.objectContaining({ ratio })]),
    } });
  });
  it("rejects an equivalent escaped duplicate root key before admission", () => {
    const text = JSON.stringify(themePackFixture()).replace('"id":"user-blueprint"', '"id":"user-blueprint","\\u0069d":"user-blueprint"');
    expect(parseThemePackText(text)).toEqual({ ok: false, diagnostic: { kind: "duplicateKey", path: "$" } });
  });
  it("rejects duplicate token keys, even if the two values agree", () => {
    const text = JSON.stringify(themePackFixture()).replace('"--knx-bg":"#f5f6fa"', '"--knx-bg":"#f5f6fa","--knx-bg":"#f5f6fa"');
    expect(parseThemePackText(text)).toEqual({ ok: false, diagnostic: { kind: "duplicateKey", path: "$" } });
  });
  it("rejects duplicate accent identifiers and nested pair keys", () => {
    const pair = { "--knx-accent": "#7041dc", "--knx-on-accent": "#ffffff" };
    const text = JSON.stringify({ ...themePackFixture(), accents: { violet: pair } });
    const duplicateId = text.replace(`"violet":${JSON.stringify(pair)}`, `"violet":${JSON.stringify(pair)},"violet":${JSON.stringify(pair)}`);
    const duplicatePair = text.replace('"--knx-accent":"#7041dc"', '"--knx-accent":"#7041dc","--knx-accent":"#7041dc"');
    for (const duplicate of [duplicateId, duplicatePair]) {
      expect(parseThemePackText(duplicate)).toEqual({ ok: false, diagnostic: { kind: "duplicateKey", path: "$" } });
    }
  });
  it("rejects one byte over the file limit without trusting character count", () => {
    const text = JSON.stringify(themePackFixture());
    expect(parseThemePackText(text.padEnd(65_537, " "))).toEqual({ ok: false, diagnostic: { kind: "sizeLimit", path: "$" } });
    const unicode = JSON.stringify({ ...themePackFixture(), name: "😀" }).padEnd(65_536, " ");
    expect(new TextEncoder().encode(unicode).length).toBeGreaterThan(65_536);
    expect(parseThemePackText(unicode)).toEqual({ ok: false, diagnostic: { kind: "sizeLimit", path: "$" } });
    expect(parseThemePackText(text.padEnd(65_536, " "))).toEqual({ ok: true, pack: themePackFixture() });
  });
  it("checks container nesting before typed conversion", () => {
    expect(parseThemePackText(`${"[".repeat(9)}0${"]".repeat(9)}`)).toEqual({ ok: false, diagnostic: { kind: "depthLimit", path: "$" } });
    expect(parseThemePackText(`${"[".repeat(8)}0${"]".repeat(8)}`)).toMatchObject({ ok: false, diagnostic: { kind: "invalidContract" } });
  });
  it("bounds JSON value nodes independently of bytes", () => {
    expect(parseThemePackText(JSON.stringify({ x: Array(1023).fill(0) }))).toEqual({ ok: false, diagnostic: { kind: "nodeLimit", path: "$" } });
    expect(parseThemePackText(JSON.stringify({ x: Array(1022).fill(0) }))).toMatchObject({ ok: false, diagnostic: { kind: "invalidContract" } });
  });
  it("handles escaped quotes, backslashes and key-like metadata as data", () => {
    const pack = { ...themePackFixture(), name: 'Café \\ "id":"id" 😀' };
    const text = JSON.stringify(pack).replace('"id":', '"\\u0069d":');
    expect(parseThemePackText(text)).toEqual({ ok: true, pack });
  });
  it("does not round a positive decimal underflow into the dimension minimum", () => {
    const pack = themePackFixture(); pack.tokens["--knx-backdrop-size"] = "0.999999999999999999999px 1px";
    expect(parseThemePackText(JSON.stringify(pack))).toMatchObject({ ok: false, diagnostic: { kind: "invalidValue" } });
    pack.tokens["--knx-backdrop-size"] = "1.000000000000000000001px 1px";
    expect(parseThemePackText(JSON.stringify(pack))).toEqual({ ok: true, pack });
  });
  it("refuses executable accessors at the unknown-object admission boundary", () => {
    const pack = themePackFixture(); let calls = 0;
    Object.defineProperty(pack, "format", { enumerable: true, get: () => { calls++; return "knxbench-theme"; } });
    expect(validateThemePack(pack)).toMatchObject({ ok: false, diagnostic: { kind: "invalidContract" } });
    expect(calls).toBe(0);
  });
  it("accepts inert null-prototype data records without changing their values", () => {
    const pack = Object.assign(Object.create(null) as Record<string, unknown>, themePackFixture());
    expect(validateThemePack(pack)).toEqual({ ok: true, pack });
  });
});
