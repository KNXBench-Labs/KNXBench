/** Revalidates settings/cache palettes without discarding unsupported source data. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { describe, expect, it } from "vitest";
import { readThemePackStore } from "./themePack";
import { themePackFixture } from "./themePackFixtures";

describe("stored theme admission", () => {
  it("returns validated installed packs without changing the source map", () => {
    const pack = themePackFixture(); const raw = { [pack.id]: pack }; const before = structuredClone(raw);
    expect(readThemePackStore(raw)).toEqual({ packs: [pack], diagnostics: [], valid: true });
    expect(raw).toEqual(before);
  });
  it.each([null, [], false, "corrupt"])("discloses malformed settings map %j", (raw) => {
    expect(readThemePackStore(raw)).toMatchObject({ packs: [], valid: false, diagnostics: [{ id: null, diagnostic: { kind: "invalidContract" } }] });
  });
  it("treats an absent map as empty without overwriting settings", () => {
    expect(readThemePackStore(undefined)).toEqual({ packs: [], diagnostics: [], valid: true });
  });
  it("retains valid entries while diagnosing unsupported and unsafe entries", () => {
    const pack = themePackFixture();
    const future = { ...pack, id: "user-future", tokenVersion: 2 };
    const unsafe = { ...pack, id: "user-unsafe", tokens: { ...pack.tokens, "--knx-bg": "url(x)" } };
    const raw = { [pack.id]: pack, [future.id]: future, [unsafe.id]: unsafe }; const before = structuredClone(raw);
    const result = readThemePackStore(raw);
    expect(result.packs).toEqual([pack]); expect(result.valid).toBe(false);
    expect(result.diagnostics).toEqual([
      { id: future.id, diagnostic: { kind: "unsupportedVersion", path: "$.tokenVersion" } },
      { id: unsafe.id, diagnostic: { kind: "invalidValue", path: "$.tokens.--knx-bg" } },
    ]);
    expect(raw).toEqual(before);
  });
  it("does not admit duplicate identities under a different storage key", () => {
    const pack = themePackFixture(); const raw = { [pack.id]: pack, "user-alias": pack };
    expect(readThemePackStore(raw)).toMatchObject({ packs: [pack], valid: false,
      diagnostics: [{ id: "user-alias", diagnostic: { kind: "identityMismatch" } }] });
    expect(Object.keys(raw)).toHaveLength(2);
  });
  it("bounds installed entries and retains an over-limit map", () => {
    const raw = Object.fromEntries(Array.from({ length: 17 }, (_, index) => {
      const pack = { ...themePackFixture(), id: `user-palette-${index}` }; return [pack.id, pack];
    }));
    const before = JSON.stringify(raw);
    expect(readThemePackStore(raw)).toMatchObject({ packs: [], valid: false, diagnostics: [{ diagnostic: { kind: "storeLimit" } }] });
    expect(JSON.stringify(raw)).toBe(before);
    delete raw["user-palette-16"];
    expect(readThemePackStore(raw).packs).toHaveLength(16);
  });
  it("bounds the serialized map even when its entries are uninterpreted", () => {
    const raw = { "user-large": { future: "x".repeat(524_288) } }; const before = JSON.stringify(raw);
    expect(readThemePackStore(raw)).toMatchObject({ packs: [], valid: false, diagnostics: [{ diagnostic: { kind: "storeLimit" } }] });
    expect(JSON.stringify(raw)).toBe(before);
  });
  it.each([new Map(), new Date(0)].map((raw) => ({ raw })))("rejects non-data map records: $raw", ({ raw }) => {
    expect(readThemePackStore(raw)).toMatchObject({ packs: [], valid: false, diagnostics: [{ diagnostic: { kind: "invalidContract" } }] });
  });
  it("does not execute accessor-backed map entries", () => {
    const raw = {}; let calls = 0;
    Object.defineProperty(raw, "user-blueprint", { enumerable: true, get: () => { calls++; return themePackFixture(); } });
    expect(readThemePackStore(raw)).toMatchObject({ packs: [], valid: false, diagnostics: [{ diagnostic: { kind: "invalidContract" } }] });
    expect(calls).toBe(0);
  });
});
