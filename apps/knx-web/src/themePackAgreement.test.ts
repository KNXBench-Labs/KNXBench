/** Prevents the frozen v1 token/registry contract from drifting behind the live stylesheet. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { ACCENTS } from "./appearance";
import { getThemeDefinitions, readThemeSelection, THEMES } from "./theme";
import { THEME_PACK_ACCENTS, THEME_TOKEN_CLASSES, readThemePackStore } from "./themePack";
import { requiredThemeTokens } from "./themeTokens";
import { themePackFixture } from "./themePackFixtures";

describe("theme runtime/build agreement", () => {
  it("matches the exact component-derived token boundary", () => {
    const css = readFileSync(new URL("./styles.css", import.meta.url), "utf8");
    expect(Object.keys(THEME_TOKEN_CLASSES).sort()).toEqual(requiredThemeTokens(css));
    expect(Object.keys(themePackFixture().tokens).sort()).toEqual(requiredThemeTokens(css));
    expect(THEME_PACK_ACCENTS).toEqual(ACCENTS);
  });
  it("extends rather than modifies the fixed built-in registry", () => {
    const pack = themePackFixture(); const variations = { ...pack, accents: { rose: { "--knx-accent": "#000000", "--knx-on-accent": "#ffffff" } } };
    for (const candidate of [pack, variations]) {
      const definitions = getThemeDefinitions(readThemePackStore({ [candidate.id]: candidate }));
      expect(definitions.slice(0, THEMES.length)).toEqual(THEMES);
      expect(definitions.at(-1)).toEqual({ id: candidate.id, name: candidate.name, hasAccentVariations: "accents" in candidate });
    }
  });
  it("discloses unsupported selection and stored entries without rewriting them", () => {
    const pack = { ...themePackFixture(), tokenVersion: 2 }; const raw = { [pack.id]: pack }; const before = JSON.stringify(raw);
    expect(readThemeSelection({ getItem: () => pack.id }, raw)).toMatchObject({ id: "system", diagnostics: [
      { id: pack.id, diagnostic: { kind: "unsupportedVersion" } }, { id: null, diagnostic: { kind: "missingSelection" } },
    ] });
    expect(JSON.stringify(raw)).toBe(before);
  });
});
