/** Theme files retain admitted semantics without changing settings. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { expect, it, vi } from "vitest";
import { themePackFixture } from "./themePackFixtures";
import { MAX_THEME_PACK_BYTES } from "./themePack";
import { exportThemePack, exportThemeRecovery, readThemePackFile } from "./themePackFiles";

it("decodes an actual UTF-8 buffer through the existing pack admission", async () => {
  const pack = themePackFixture();
  pack.name = " Blueprint 🛠 ";
  const bytes = new TextEncoder().encode(JSON.stringify(pack));
  const arrayBuffer = vi.fn(async () => bytes.buffer);
  const result = await readThemePackFile({ size: bytes.byteLength, arrayBuffer });
  expect(result).toEqual({ ok: true, pack });
  expect(arrayBuffer).toHaveBeenCalledTimes(1);
});

it.each([-1, 0.5, NaN, Infinity, MAX_THEME_PACK_BYTES + 1])(
  "rejects invalid/oversized declared bytes (%s) before reading", async (size) => {
    const arrayBuffer = vi.fn(async () => new ArrayBuffer(0));
    expect(await readThemePackFile({ size, arrayBuffer })).toEqual({
      ok: false, diagnostic: { kind: "sizeLimit", path: "$" },
    });
    expect(arrayBuffer).not.toHaveBeenCalled();
  },
);

it("checks actual buffer bytes before BOM stripping can hide overflow", async () => {
  const text = JSON.stringify(themePackFixture()).padEnd(MAX_THEME_PACK_BYTES, " ");
  const bytes = new TextEncoder().encode("\uFEFF" + text);
  expect(await readThemePackFile({ size: 0, arrayBuffer: async () => bytes.buffer })).toEqual({
    ok: false, diagnostic: { kind: "sizeLimit", path: "$" },
  });
});

it.each([[0x80], [0xc0, 0xaf], [0xed, 0xa0, 0x80], [0xf0, 0x9f]])(
  "never silently replaces malformed UTF-8 bytes inside valid metadata: %j", async (...invalid) => {
    const text = JSON.stringify(themePackFixture());
    const [prefix, suffix] = text.split("Blueprint");
    const bytes = new Uint8Array([...new TextEncoder().encode(prefix), ...invalid,
      ...new TextEncoder().encode(suffix)]);
    expect(await readThemePackFile({ size: bytes.byteLength, arrayBuffer: async () => bytes.buffer })).toEqual({
      ok: false, diagnostic: { kind: "invalidEncoding", path: "$" },
    });
  },
);

it("reports read refusal without leaking arbitrary error text or changing input", async () => {
  const arrayBuffer = vi.fn(async () => { throw new Error("private detail must not escape"); });
  expect(await readThemePackFile({ size: 10, arrayBuffer })).toEqual({
    ok: false, diagnostic: { kind: "fileRead", path: "$" },
  });
  expect(arrayBuffer).toHaveBeenCalledTimes(1);
});

it.each(["", "\uFEFF"])("accepts optional initial BOM within the byte limit: %j", async (bom) => {
  const pack = themePackFixture();
  const bytes = new TextEncoder().encode(bom + JSON.stringify(pack));
  const before = bytes.slice();
  expect(await readThemePackFile({ size: bytes.byteLength, arrayBuffer: async () => bytes.buffer })).toEqual({ ok: true, pack });
  expect(bytes).toEqual(before);
});

it.each([
  ["{}", "invalidContract"], ["[]", "invalidContract"],
  [JSON.stringify({ ...themePackFixture(), formatVersion: 2 }), "unsupportedVersion"],
  [JSON.stringify(themePackFixture()).replace('"tokens":', '"id":"user-other","tokens":'), "duplicateKey"],
  [JSON.stringify(themePackFixture()) + "{}", "invalidJson"],
  ["\uFEFF\uFEFF" + JSON.stringify(themePackFixture()), "invalidJson"],
])("keeps existing strict admission on file input: %s", async (text, kind) => {
  const bytes = new TextEncoder().encode(text);
  const result = await readThemePackFile({ size: bytes.byteLength, arrayBuffer: async () => bytes.buffer });
  expect(result).toMatchObject({ ok: false, diagnostic: { kind } });
});


it("exports deterministic recursive key order with unchanged metadata and authored values", () => {
  const pack = { ...themePackFixture(), name: " Blueprint 🛠 ", version: " 1.0 ",
    accents: { mint: { "--knx-on-accent": "#ffffffFF", "--knx-accent": "#137551" },
      blue: { "--knx-on-accent": "#ffffff", "--knx-accent": "#7041dc" } } };
  pack.tokens["--knx-bg"] = "#F5F6FAff";
  pack.tokens["--knx-radius-card"] = "8.000px";
  const before = structuredClone(pack);
  const result = exportThemePack(pack);
  expect(result.ok).toBe(true);
  if (!result.ok) throw new Error("export refused");
  expect(result.fileName).toBe("user-blueprint.knx-theme.json");
  expect(JSON.parse(result.text)).toEqual(pack);
  expect(result.text.endsWith("\n")).toBe(true);
  expect(result.text.startsWith('{\n  "accents": {\n    "blue": {\n      "--knx-accent":')).toBe(true);
  const parsed = JSON.parse(result.text) as typeof pack;
  expect(Object.keys(parsed)).toEqual(Object.keys(pack).sort());
  expect(Object.keys(parsed.tokens)).toEqual(Object.keys(pack.tokens).sort());
  expect(Object.keys(parsed.accents)).toEqual(["blue", "mint"]);
  const reordered = { ...pack, tokens: Object.fromEntries(Object.entries(pack.tokens).reverse()),
    accents: { blue: pack.accents.blue, mint: pack.accents.mint } };
  expect(exportThemePack(reordered)).toEqual(result);
  expect(pack).toEqual(before);
});

it.each([
  { ...themePackFixture(), id: "graphite" },
  { ...themePackFixture(), tokenVersion: 99 },
  { ...themePackFixture(), tokens: {} },
])("revalidates rejected data before offering a supported theme export", (raw) => {
  const before = structuredClone(raw);
  expect(exportThemePack(raw).ok).toBe(false);
  expect(raw).toEqual(before);
});

it("exports bounded retained raw theme scope for recovery, not unrelated preferences or an admitted pack", () => {
  const raw = { "user-future": { tokenVersion: 99, unsupported: ["unchanged", null] },
    "user-damaged": "original malformed shape" };
  const original = structuredClone(raw);
  const exported = exportThemeRecovery(raw, "user-future");
  expect(exported.ok).toBe(true);
  if (!exported.ok) throw new Error("recovery unavailable");
  expect(JSON.parse(exported.text)).toEqual({ format: "knxbench-theme-recovery", formatVersion: 1,
    selectedTheme: "user-future", uiThemePacks: raw });
  expect(exported.fileName).toBe("knxbench-theme-recovery.json");
  expect(raw).toEqual(original);
});

it("reports oversized raw recovery without deleting or truncating its original", () => {
  const raw = { "user-future": { unsupported: "x".repeat(1_048_576) } };
  const result = exportThemeRecovery(raw, "user-future");
  expect(result.ok).toBe(false);
  if (result.ok) throw new Error("oversized recovery admitted");
  expect(result.diagnostic).toEqual({ kind: "sizeLimit", path: "$" });
  expect(raw["user-future"].unsupported.length).toBe(1_048_576);
});

it("reports nonserializable recovery without throwing or fabricating a successful export", () => {
  const raw: Record<string, unknown> = {}; raw.self = raw;
  expect(exportThemeRecovery(raw, "system")).toEqual({ ok: false, diagnostic: { kind: "invalidJson", path: "$" } });
  expect(raw.self).toBe(raw);
});
