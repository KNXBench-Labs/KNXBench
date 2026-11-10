/** Built device editor: checked parameter editors, source evidence, writes and refusals. */
import { openExplorerBranches, openParameterSections } from "./disclosure-fixture";
import { expect, test, type Page } from "@playwright/test";
import { join } from "node:path";
import type { ParameterField, ParameterPanel } from "../src/api";
import { parameterEditorCopy } from "../src/features/parameter-editors/copy";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { busActivityFixtureAnswer } from "./bus-activity-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";

type Language = "en" | "de";
type Catalogue = { readonly [K in keyof typeof en]: string };
const CAPTURE = process.env.KNX_PARAMETERS_CAPTURE_DIR;
const DEVICE = { id: 41, name: "Dimmer living room", address: "1.1.7", description: null, com_object_count: 0 };
const PROGRAM = "M-00FA_A-0001-01-0000";
const SHA = "e".repeat(64);

interface Fixture {
  unexpected: string[];
  revision: number;
  values: Record<string, string>;
  writes: { etsId: string; raw: string }[];
  /** Every write body's Goal A2 binding, in order, conflicts included. */
  bindings: { serverIncarnation?: string; projectIncarnation?: number; snapshotRevision?: number; expectedValue?: string | null }[];
  refuseWrites: boolean;
  refusalReason: string;
  identity: boolean;
  history: Record<string, string>[];
  future: Record<string, string>[];
  sourceFault: "none" | "sha" | "ref" | "program" | "snapshot" | "missing";
  /** Hold complete HTTP replies, independently of the fixture's committed model. */
  beforeParameterReply?: (etsId: string, status: number) => Promise<void>;
}

function projectTree(f: Fixture): ProjectTree {
  return {
    schema_version: 13, errors: 0, warnings: 0, can_undo: f.history.length > 0, can_redo: f.future.length > 0, is_modified: f.writes.length > 0,
    ...(f.identity ? { server_incarnation: "parameters-fixture", project_incarnation: 3, snapshot_revision: f.revision } : {}),
    group_address_style: "ThreeLevel",
    installations: [{ id: 0, name: "Fictional flat",
      topology: [{ id: 21, name: "Main area", address: 1, lines: [{ id: 31, name: "Ground floor", address: 1, devices: [DEVICE] }] }],
      buildings: [], unassigned: [], group_ranges: [], group_addresses: [] }],
  };
}

function field(etsId: string, text: string, kind: string, f: Fixture, min: string | null, max: string | null): ParameterField {
  return { etsId, name: text, nameLanguage: null, text, textLanguage: null, kind, value: f.values[etsId], valueSource: "Stored",
    editable: true, min, max, enumOptions: [], displayOrder: null, access: "ReadWrite", writeEtsId: etsId };
}
function panelOf(f: Fixture, withTree: boolean): ParameterPanel {
  return { programId: PROGRAM, sourceLanguage: null, stale: [], diagnostics: [], tree: withTree ? projectTree(f) : null,
    sections: [{ scope: null, fields: [
      field(`${PROGRAM}_P-1_R-1`, "Switch-off delay", "Time", f, "0", "3600"),
      field(`${PROGRAM}_P-2_R-1`, "Basic brightness", "Number", f, "0", "100"),
      field(`${PROGRAM}_P-3_R-1`, "Status LED colour", "Color", f, null, null),
      field(`${PROGRAM}_P-4_R-1`, "Endpoint", "IPAddress", f, null, null),
      field(`${PROGRAM}_P-5_R-1`, "Reference date", "Date", f, null, null),
    ] }] };
}
const SOURCES: Record<string, { child: string; type: string; attributes: Record<string, string> }> = {
  [`${PROGRAM}_P-1_R-1`]: { child: "TypeTime", type: "PT-Delay", attributes: { SizeInBit: "16", minInclusive: "0", maxInclusive: "3600", Unit: "Seconds" } },
  [`${PROGRAM}_P-2_R-1`]: { child: "TypeNumber", type: "PT-Brightness", attributes: { SizeInBit: "8", Type: "unsignedInt", minInclusive: "0", maxInclusive: "100", UIHint: "Slider" } },
  [`${PROGRAM}_P-3_R-1`]: { child: "TypeColor", type: "PT-Colour", attributes: {} },
  [`${PROGRAM}_P-4_R-1`]: { child: "TypeIPAddress", type: "PT-Address", attributes: { AddressType: "HostAddress" } },
  [`${PROGRAM}_P-5_R-1`]: { child: "TypeDate", type: "PT-Date", attributes: { Encoding: "DPT 11" } },
};

async function fixture(page: Page, language: Language, theme: string, width: number, identity = true): Promise<Fixture> {
  const f: Fixture = { unexpected: [], revision: 1, writes: [], bindings: [], refuseWrites: false,
    refusalReason: "Value 3601 is outside the parameter's range 0..3600", identity, history: [], future: [], sourceFault: "none",
    values: { [`${PROGRAM}_P-1_R-1`]: "0010", [`${PROGRAM}_P-2_R-1`]: "40", [`${PROGRAM}_P-3_R-1`]: "#00FF00", [`${PROGRAM}_P-4_R-1`]: "203.0.113.19", [`${PROGRAM}_P-5_R-1`]: "1899-12-31" } };
  await page.setViewportSize({ width, height: 900 });
  // The same file runs against the preview (parameters lane) and the dev server (functional lane).
  const appOrigin = new URL(String(test.info().project.use.baseURL)).origin;
  page.on("request", request => {
    if (new URL(request.url()).origin !== appOrigin) f.unexpected.push(request.url());
  });
  await page.route("https://api.github.com/**", (route) => { f.unexpected.push(route.request().url()); return route.abort(); });
  await page.route("**/api/**", (route) => {
    const url = new URL(route.request().url());
    const path = url.pathname;
    const method = route.request().method();
    const ok = (body: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
    const refuse = (status: number, kind: string, error: string) => route.fulfill({ status, contentType: "application/json", body: JSON.stringify({ error, kind }) });
    const achievements = achievementsFixtureAnswer(method, path);
    if (achievements !== undefined) return ok(achievements);
    const busActivity = busActivityFixtureAnswer(method, path);
    if (busActivity !== undefined) return ok(busActivity);
    if (path === "/api/auth/status") return ok({ required: false, authenticated: true });
    if (path === "/api/settings" && method === "GET") return ok({ conditionalPatchVersion: 1, schemaVersion: 1, status: "ok", settings: {
      uiLanguage: language, theme, motionLevel: "off", releaseChecksEnabled: false,
      onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.9" } } });
    if (path === "/api/settings" && method === "PUT") return ok({ conditionalPatchVersion: 1, schemaVersion: 1, status: "ok", settings: route.request().postDataJSON().settings });
    if (path === "/api/version") return ok({ version: "0.1.0-alpha.9+parameters-fixture" });
    if (path === "/api/project" && method === "GET") return ok(projectTree(f));
    if ((path === "/api/undo" || path === "/api/redo") && method === "POST") {
      const from = path === "/api/undo" ? f.history : f.future;
      const to = path === "/api/undo" ? f.future : f.history;
      const values = from.pop();
      if (!values) return refuse(400, "emptyHistory", "No fixture history");
      to.push({ ...f.values }); f.values = values; f.revision += 1;
      return ok(projectTree(f));
    }
    if (path === "/api/project/load-progress") return ok(null);
    if (path === "/api/project/status-metrics" && method === "GET") return ok(statusMetricsFixtureBody(projectTree(f)));
    if (path === "/api/log") return ok([]);
    if (path === "/api/bus/discover" && method === "POST") return ok({ interfaces: [] });
    if (["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items"].includes(path)) return ok([]);
    if (["/api/device-download/status", "/api/device-address/status"].includes(path)) return refuse(404, "none", "no fixture operation");
    if (path === "/api/device-system2/status" && method === "GET") return ok(null);
    if (path === "/api/devices" && method === "GET") return ok({ schemaVersion: 1, serverIncarnation: identity ? "parameters-fixture" : null,
      snapshotRevision: identity ? f.revision : null, devices: [] });
    if (path === "/api/device/41" && method === "GET") return ok({ ...DEVICE, com_objects: [], product: { resolution: "NoReference", product_ref: null, program_ref: null, catalog: null } });
    if (path === "/api/device/41/parameters" && method === "GET") return ok(panelOf(f, false));
    if (path === "/api/device/41/parameters" && method === "POST") {
      const { etsId, raw, ...binding } = route.request().postDataJSON() as { etsId: string; raw: string } & Fixture["bindings"][number];
      f.bindings.push(binding);
      const reply = async (body: unknown, status = 200) => {
        // Serialize now: a held old reply must not follow later mutable fixture values.
        const bytes = JSON.stringify(body);
        await f.beforeParameterReply?.(etsId, status);
        return route.fulfill({ status, contentType: "application/json", body: bytes });
      };
      // Checked like the server (revision-bound-writes): any moved context or value is a 409.
      if (binding.serverIncarnation !== "parameters-fixture" || binding.projectIncarnation !== 3
        || binding.snapshotRevision !== f.revision || binding.expectedValue !== f.values[etsId]) {
        return reply({ kind: "parameterWriteConflict", error: "parameter value or project changed since it was shown" }, 409);
      }
      if (f.refuseWrites) return reply({ kind: "invalidParameterValue", error: f.refusalReason }, 400);
      f.history.push({ ...f.values }); f.future = [];
      f.writes.push({ etsId, raw }); f.values[etsId] = raw; f.revision += 1;
      return reply(panelOf(f, true));
    }
    if (path === "/api/device/41/parameter-editor-source" && method === "GET") {
      const etsId = url.searchParams.get("etsId") ?? "";
      const source = SOURCES[etsId];
      const head = { device: 41, parameterRefId: f.sourceFault === "ref" ? "foreign-ref" : etsId, serverIncarnation: "parameters-fixture", projectIncarnation: 3, snapshotRevision: f.sourceFault === "snapshot" ? f.revision + 1 : f.revision };
      if (f.sourceFault === "missing") return ok({ ...head, evidence: { status: "unavailable", reason: "missingSource" } });
      if (!source) return ok({ ...head, evidence: { status: "unavailable", reason: "missingReference" } });
      const typeId = `${PROGRAM}_${source.type}`;
      return ok({ ...head, evidence: { status: "declared", source: { sourceSha256: f.sourceFault === "sha" ? "g".repeat(64) : SHA, programId: f.sourceFault === "program" ? "foreign-program" : PROGRAM, parameterTypeId: typeId, schema: 21,
        child: source.child, attributes: source.attributes, parentAttributes: { Id: typeId, Name: source.type } } } });
    }
    f.unexpected.push(`${method} ${path}`);
    return refuse(404, "strict", "strict parameters fixture");
  });
  await page.goto("/?view=editor");
  await expect(page.locator("html")).toHaveAttribute("lang", language);
  await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
  return f;
}

async function noHorizontalOverflow(page: Page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
}
async function openParameters(page: Page, m: Catalogue) {
  await openExplorerBranches(page);
  await page.locator(".project-explorer").getByText(DEVICE.name, { exact: false }).first().click();
  await page.getByRole("tab", { name: m["workbench.parameters"], exact: true }).click();
  const panel = page.locator('[id="device-panel-41-1"]');
  await openParameterSections(page);
  await expect(panel.locator(".parameter-field").first()).toBeVisible();
  return panel;
}
function opener(panel: ReturnType<Page["locator"]>, etsId: string) {
  return panel.locator(`[data-checked-for="${etsId}"]`);
}
async function capture(page: Page, name: string) {
  if (CAPTURE) await page.screenshot({ path: join(CAPTURE, `${name}.png`), fullPage: false });
}

for (const language of ["en", "de"] as const) for (const theme of ["porcelain", "graphite"]) for (const width of [1440, 400]) {
  test(`${language} ${theme} ${width}: time count applies, slider only drafts`, async ({ page }) => {
    const m: Catalogue = language === "de" ? de : en;
    const tag = `${language}-${theme}-${width}`;
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    const f = await fixture(page, language, theme, width);
    const panel = await openParameters(page, m);
    await expect(opener(panel, `${PROGRAM}_P-1_R-1`)).toHaveText(m["parameters.checked.open"]);
    await opener(panel, `${PROGRAM}_P-1_R-1`).click();
    const delay = panel.locator(`[data-ets-id="${PROGRAM}_P-1_R-1"]`);
    await expect(delay).toContainText(language === "de" ? "Dezimaler Zähler" : "Decimal count");
    await expect(delay).toContainText("Seconds");
    const input = delay.getByRole("textbox", { name: "Switch-off delay", exact: true });
    await expect(input).toHaveValue("0010");
    await delay.scrollIntoViewIfNeeded();
    await noHorizontalOverflow(page);
    await capture(page, `${tag}-1-time`);

    await input.fill("12");
    await delay.getByRole("button", { name: language === "de" ? "Wert übernehmen" : "Apply value", exact: true }).click();
    await expect(delay).toContainText(language === "de" ? "Übernommen. Das Feld zeigt den Wert aus dem Projekt." : "Applied. The field shows the project's value.");
    expect(f.writes).toEqual([{ etsId: `${PROGRAM}_P-1_R-1`, raw: "12" }]);
    expect(f.bindings).toEqual([{ serverIncarnation: "parameters-fixture", projectIncarnation: 3, snapshotRevision: 1, expectedValue: "0010" }]);
    await expect(input).toHaveValue("12");
    await expect(page.getByRole("button", { name: m["toolbar.undo"], exact: true })).toBeEnabled();
    await noHorizontalOverflow(page);
    await capture(page, `${tag}-2-applied`);

    await opener(panel, `${PROGRAM}_P-2_R-1`).click();
    const brightness = panel.locator(`[data-ets-id="${PROGRAM}_P-2_R-1"]`);
    const slider = brightness.getByRole("slider");
    await slider.focus();
    await page.keyboard.press("ArrowRight");
    await expect(brightness.getByRole("textbox", { name: "Basic brightness", exact: true })).toHaveValue("41");
    expect(f.writes).toHaveLength(1);
    await brightness.scrollIntoViewIfNeeded();
    await noHorizontalOverflow(page);
    await capture(page, `${tag}-3-slider`);
    expect(f.unexpected).toEqual([]);
    expect(errors).toEqual([]);
  });
}

test("de 400: a colour stays an unconverted original, never a picker", async ({ page }) => {
  const f = await fixture(page, "de", "porcelain", 400);
  const panel = await openParameters(page, de);
  await opener(panel, `${PROGRAM}_P-3_R-1`).click();
  const colour = panel.locator(`[data-ets-id="${PROGRAM}_P-3_R-1"]`);
  await expect(colour).toContainText("Dieses Wertformat ist nicht verifiziert");
  await expect(colour).toContainText("#00FF00");
  await expect(colour.locator('input[type="color"], img')).toHaveCount(0);
  await colour.getByRole("button", { name: de["parameters.checked.close"], exact: true }).click();
  await expect(opener(panel, `${PROGRAM}_P-3_R-1`)).toBeVisible();
  await capture(page, "colour-de-400");
  expect(f.writes).toEqual([]);
  expect(f.unexpected).toEqual([]);
});

test("en 1440: a refused write keeps the draft and shows the server's reason", async ({ page }) => {
  const f = await fixture(page, "en", "graphite", 1440);
  const panel = await openParameters(page, en);
  f.refuseWrites = true;
  await opener(panel, `${PROGRAM}_P-1_R-1`).click();
  const delay = panel.locator(`[data-ets-id="${PROGRAM}_P-1_R-1"]`);
  await delay.getByRole("textbox", { name: "Switch-off delay", exact: true }).fill("3599");
  await delay.getByRole("button", { name: "Apply value", exact: true }).click();
  await expect(delay.getByRole("alert")).toContainText("outside the parameter's range");
  await expect(delay.getByRole("textbox", { name: "Switch-off delay", exact: true })).toHaveValue("3599");
  expect(f.values[`${PROGRAM}_P-1_R-1`]).toBe("0010");
  await capture(page, "refusal-en-1440");
  expect(f.unexpected).toEqual([]);
});

test("en 1440: without a complete project identity no checked editor and no write is offered", async ({ page }) => {
  const f = await fixture(page, "en", "porcelain", 1440, false);
  const panel = await openParameters(page, en);
  await expect(panel.locator("[data-checked-for]")).toHaveCount(0);
  const row = panel.locator(`[data-ets-id="${PROGRAM}_P-1_R-1"]`);
  await expect(row.locator("input")).toHaveValue("0010");
  await expect(row.locator("input")).toBeDisabled();
  await expect(row).toContainText(en["parameters.noContextCaption"]);
  expect(f.bindings).toEqual([]);
  expect(f.unexpected).toEqual([]);
});

test("OKX06 original shell offers the checked IP literal editor", async ({ page }) => {
  const f = await fixture(page, "en", "graphite", 1440);
  const panel = await openParameters(page, en);
  await expect(opener(panel, `${PROGRAM}_P-4_R-1`)).toBeVisible();
  expect(f.writes).toEqual([]);
  expect(f.unexpected).toEqual([]);
});

for (const [language, theme, width] of [["en", "graphite", 1440], ["de", "porcelain", 400]] as const) {
  test(`OKX06 ${language} ${width}: keyboard literal Apply and authoritative Undo/Redo`, async ({ page }) => {
    const m: Catalogue = language === "de" ? de : en;
    const copy = parameterEditorCopy[language];
    const errors: string[] = []; page.on("pageerror", error => errors.push(error.message));
    const f = await fixture(page, language, theme, width);
    const panel = await openParameters(page, m);
    const id = `${PROGRAM}_P-4_R-1`;
    await opener(panel, id).focus(); await page.keyboard.press("Enter");
    const editor = panel.locator(`[data-ets-id="${id}"]`);
    const input = editor.getByRole("textbox", { name: "Endpoint", exact: true });
    await expect(input).toHaveAttribute("type", "text");
    await expect(input).toHaveAttribute("inputmode", "text");
    await expect(input).toHaveValue("203.0.113.19");
    await expect(editor).toContainText(copy.ipFormat);
    const apply = editor.getByRole("button", { name: copy.apply, exact: true });
    await expect(apply).toBeDisabled(); expect(f.bindings).toEqual([]);
    await input.focus(); await input.press("ControlOrMeta+A");
    const raw = "2001:0DB8:0012:0345:0067:0089:00Ab:CDEF";
    await page.keyboard.type(raw); await input.press("Enter"); await input.press("Tab");
    expect(f.bindings).toEqual([]);
    await expect(apply).toBeEnabled(); await apply.focus(); await apply.press("Enter");
    await expect(input).toHaveValue(raw); await expect(editor).toContainText(copy.settled);
    expect(f.writes).toEqual([{ etsId: id, raw }]);
    expect(f.bindings).toEqual([{ serverIncarnation: "parameters-fixture", projectIncarnation: 3, snapshotRevision: 1, expectedValue: "203.0.113.19" }]);
    await editor.scrollIntoViewIfNeeded(); await noHorizontalOverflow(page);
    if (CAPTURE) await editor.screenshot({ path: join(CAPTURE, `ip-${language}-${width}-applied.png`) });
    const reset = editor.getByRole("button", { name: copy.reset, exact: true });
    await reset.focus(); await page.keyboard.press("ControlOrMeta+z");
    await expect(input).toHaveValue("203.0.113.19");
    await reset.focus(); await page.keyboard.press("ControlOrMeta+Shift+z");
    await expect(input).toHaveValue(raw); expect(f.writes).toHaveLength(1);
    await input.focus(); await input.press("ControlOrMeta+A"); await page.keyboard.type("2001:db8::1"); await input.press("Tab");
    await expect(editor.getByRole("alert")).toHaveText(copy["ip-address"]); await expect(apply).toBeDisabled();
    expect(f.bindings).toHaveLength(1);
    if (CAPTURE) await editor.screenshot({ path: join(CAPTURE, `ip-${language}-${width}-invalid.png`) });
    await reset.focus(); await reset.press("Enter"); await expect(input).toHaveValue(raw);
    expect(f.bindings).toHaveLength(1); expect(f.unexpected).toEqual([]); expect(errors).toEqual([]);
  });
}

test("OKX06 conflict keeps IP draft and needs explicit reset and fresh Apply", async ({ page }) => {
  const f = await fixture(page, "en", "graphite", 1440);
  const panel = await openParameters(page, en); const id = `${PROGRAM}_P-4_R-1`;
  await opener(panel, id).click();
  const editor = panel.locator(`[data-ets-id="${id}"]`);
  const input = editor.getByRole("textbox", { name: "Endpoint", exact: true });
  await input.fill("198.51.100.254");
  f.values[id] = "0.0.0.0"; f.revision += 1;
  await editor.getByRole("button", { name: "Apply value", exact: true }).click();
  await expect(editor).toContainText(parameterEditorCopy.en.stale);
  await expect(input).toHaveValue("198.51.100.254");
  expect(f.writes).toEqual([]); expect(f.bindings).toHaveLength(1);
  await input.focus(); await input.press("Tab"); expect(f.bindings).toHaveLength(1);
  await editor.getByRole("button", { name: parameterEditorCopy.en.reset, exact: true }).click();
  await expect(input).toHaveValue("0.0.0.0");
  await input.fill("198.51.100.254"); await editor.getByRole("button", { name: "Apply value", exact: true }).click();
  await expect(editor).toContainText(parameterEditorCopy.en.settled);
  expect(f.writes).toEqual([{ etsId: id, raw: "198.51.100.254" }]);
  expect(f.bindings).toEqual([
    { serverIncarnation: "parameters-fixture", projectIncarnation: 3, snapshotRevision: 1, expectedValue: "203.0.113.19" },
    { serverIncarnation: "parameters-fixture", projectIncarnation: 3, snapshotRevision: 2, expectedValue: "0.0.0.0" },
  ]);
  expect(f.unexpected).toEqual([]);
});

test("OKX06 pasted final newline remains an invalid raw draft, never a normalized write", async ({ page }) => {
  const f = await fixture(page, "en", "graphite", 1440); const panel = await openParameters(page, en);
  await opener(panel, `${PROGRAM}_P-4_R-1`).click();
  const editor = panel.locator(`[data-ets-id="${PROGRAM}_P-4_R-1"]`);
  const input = editor.getByRole("textbox", { name: "Endpoint", exact: true });
  await input.focus(); await input.press("ControlOrMeta+A");
  await input.evaluate(node => {
    const clipboardData = new DataTransfer(); clipboardData.setData("text/plain", "198.51.100.254\n");
    node.dispatchEvent(new ClipboardEvent("paste", { bubbles: true, cancelable: true, clipboardData }));
  });
  await expect(editor.getByRole("alert")).toHaveText(parameterEditorCopy.en["ip-address"]);
  await expect(editor.getByLabel(parameterEditorCopy.en.draft)).toHaveText("198.51.100.254\n", { useInnerText: false });
  await expect(editor.getByRole("button", { name: "Apply value", exact: true })).toBeDisabled();
  expect(f.bindings).toEqual([]); expect(f.values[`${PROGRAM}_P-4_R-1`]).toBe("203.0.113.19"); expect(f.unexpected).toEqual([]);
  await input.focus(); await input.press("ControlOrMeta+A");
  await input.evaluate(node => {
    const clipboardData = new DataTransfer(); clipboardData.setData("text/plain", "203.0.113.20");
    node.dispatchEvent(new ClipboardEvent("paste", { bubbles: true, cancelable: true, clipboardData }));
  });
  await editor.getByRole("button", { name: "Apply value", exact: true }).click();
  await expect(editor).toContainText(parameterEditorCopy.en.settled);
  expect(f.writes).toEqual([{ etsId: `${PROGRAM}_P-4_R-1`, raw: "203.0.113.20" }]);
  expect(f.bindings).toEqual([{ serverIncarnation: "parameters-fixture", projectIncarnation: 3, snapshotRevision: 1, expectedValue: "203.0.113.19" }]);
  expect(f.unexpected).toEqual([]);
});

for (const mode of [
  { language: "en", width: 1440, theme: "graphite", newline: "\n" },
  { language: "de", width: 400, theme: "porcelain", newline: "\r\n" },
] as const) {
  test(`OKX06 R1 ${mode.language} ${mode.width}: hidden raw line breaks never splice a different address`, async ({ page }) => {
    const m: Catalogue = mode.language === "de" ? de : en;
    const copy = parameterEditorCopy[mode.language];
    const f = await fixture(page, mode.language, mode.theme, mode.width);
    const id = `${PROGRAM}_P-4_R-1`;
    const original = `203.0.113.${mode.newline}19`;
    f.values[id] = original;
    const panel = await openParameters(page, m); await opener(panel, id).click();
    const editor = panel.locator(`[data-ets-id="${id}"]`);
    const input = editor.getByRole("textbox", { name: "Endpoint", exact: true });
    await expect(input).toHaveValue("203.0.113.19");
    await expect(editor.getByLabel(copy.original)).toHaveJSProperty("textContent", original);
    await input.evaluate(node => {
      if (!(node instanceof HTMLInputElement)) throw new Error("Expected the actual address textbox");
      node.focus(); node.setSelectionRange(node.value.lastIndexOf(".") + 1, node.value.length);
      const clipboardData = new DataTransfer(); clipboardData.setData("text/plain", "20");
      node.dispatchEvent(new ClipboardEvent("paste", { bubbles: true, cancelable: true, clipboardData }));
    });
    await expect(editor.getByRole("alert")).toHaveText(copy.literalPasteComplete);
    await expect(input).toHaveValue("203.0.113.19");
    await expect(editor.getByRole("button", { name: copy.apply, exact: true })).toBeDisabled();
    expect(f.writes).toEqual([]); expect(f.bindings).toEqual([]); expect(f.values[id]).toBe(original);
    await noHorizontalOverflow(page); await capture(page, `ip-r1-${mode.language}-${mode.width}-partial`);
    await input.focus(); await input.press("ControlOrMeta+A");
    await input.evaluate(node => {
      const clipboardData = new DataTransfer(); clipboardData.setData("text/plain", "203.0.113.20");
      node.dispatchEvent(new ClipboardEvent("paste", { bubbles: true, cancelable: true, clipboardData }));
    });
    await expect(input).toHaveValue("203.0.113.20");
    await editor.getByRole("button", { name: copy.apply, exact: true }).click();
    await expect(editor).toContainText(copy.settled);
    expect(f.writes).toEqual([{ etsId: id, raw: "203.0.113.20" }]);
    expect(f.bindings).toEqual([{ serverIncarnation: "parameters-fixture", projectIncarnation: 3, snapshotRevision: 1, expectedValue: original }]);
    expect(f.unexpected).toEqual([]);
  });
}

for (const sourceFault of ["sha", "ref", "program", "snapshot", "missing"] as const) {
  test(`OKX06 ${sourceFault} source never admits a checked IP input`, async ({ page }) => {
    const f = await fixture(page, "en", "graphite", 1440); f.sourceFault = sourceFault;
    const panel = await openParameters(page, en); const id = `${PROGRAM}_P-4_R-1`;
    await opener(panel, id).click();
    const editor = panel.locator(`[data-ets-id="${id}"]`);
    await expect(editor.getByRole("button", { name: en["parameters.checked.close"], exact: true })).toBeVisible();
    await expect(editor).toContainText(sourceFault === "sha" ? parameterEditorCopy.en.fallback["source-conflict"] : parameterEditorCopy.en.fallback["missing-evidence"]);
    await expect(editor.getByRole("textbox")).toHaveCount(0);
    expect(f.bindings).toEqual([]);
    await editor.getByRole("button", { name: en["parameters.checked.close"], exact: true }).click();
    await expect(panel.locator(`[data-ets-id="${id}"] input`)).toHaveValue("203.0.113.19");
    expect(f.unexpected).toEqual([]);
  });
}

// Intercepted HTTP proves the Date production adapter and refresh, not native persistence.
for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`Date ${language} ${width}: text, raw paste and exact explicit Apply`, async ({ page }) => {
    const m: Catalogue = language === "de" ? de : en;
    const copy = parameterEditorCopy[language];
    const errors: string[] = []; page.on("pageerror", error => errors.push(error.message));
    const f = await fixture(page, language, language === "de" ? "porcelain" : "graphite", width);
    const panel = await openParameters(page, m); const id = `${PROGRAM}_P-5_R-1`;
    await expect(opener(panel, id)).toHaveText(m["parameters.checked.open"]);
    await opener(panel, id).focus(); await page.keyboard.press("Enter");
    const editor = panel.locator(`[data-ets-id="${id}"]`);
    const input = editor.getByRole("textbox", { name: "Reference date", exact: true });
    const apply = editor.getByRole("button", { name: copy.apply, exact: true });
    await expect(input).toHaveAttribute("type", "text"); await expect(input).toHaveAttribute("inputmode", "text");
    await expect(input).toHaveValue("1899-12-31"); await expect(editor).toContainText(copy.dateFormat);
    await expect(editor.locator('input[type="date"],input[type="number"],img,a')).toHaveCount(0);
    await expect(apply).toBeDisabled(); expect(f.bindings).toEqual([]);
    await input.focus(); await input.press("ControlOrMeta+A"); await page.keyboard.type("1900-02-29");
    await input.press("Enter"); await input.press("Tab");
    await expect(editor.getByRole("alert")).toHaveText(copy.date); await expect(apply).toBeDisabled();
    expect(f.bindings).toEqual([]);
    await input.focus(); await input.press("ControlOrMeta+A");
    const raw = "2000-02-29\r\n";
    await input.evaluate((node, value) => {
      const clipboardData = new DataTransfer(); clipboardData.setData("text/plain", value);
      node.dispatchEvent(new ClipboardEvent("paste", { bubbles: true, cancelable: true, clipboardData }));
    }, raw);
    await expect(input).toHaveValue("2000-02-29");
    await expect(editor.getByLabel(copy.draft)).toHaveJSProperty("textContent", raw);
    await expect(editor.getByRole("alert")).toHaveText(copy.date); await expect(apply).toBeDisabled();
    await input.evaluate(node => {
      if (!(node instanceof HTMLInputElement)) throw new Error("Expected the Date textbox");
      node.focus(); node.setSelectionRange(8, 10);
      const clipboardData = new DataTransfer(); clipboardData.setData("text/plain", "28");
      node.dispatchEvent(new ClipboardEvent("paste", { bubbles: true, cancelable: true, clipboardData }));
    });
    await expect(editor).toContainText(copy.literalPasteComplete);
    await expect(editor.getByLabel(copy.draft)).toHaveJSProperty("textContent", raw);
    expect(f.bindings).toEqual([]); expect(f.values[id]).toBe("1899-12-31");
    await editor.scrollIntoViewIfNeeded(); await noHorizontalOverflow(page);
    if (CAPTURE) await editor.screenshot({ path: join(CAPTURE, `date-${language}-${width}-invalid.png`) });
    await input.focus(); await input.press("ControlOrMeta+A");
    await input.evaluate(node => {
      const clipboardData = new DataTransfer(); clipboardData.setData("text/plain", "0000-02-29");
      node.dispatchEvent(new ClipboardEvent("paste", { bubbles: true, cancelable: true, clipboardData }));
    });
    await expect(input).toHaveValue("0000-02-29"); await expect(editor.getByRole("alert")).toHaveCount(0);
    await input.press("Enter"); await input.press("Tab"); expect(f.bindings).toEqual([]);
    await apply.focus(); await apply.press("Enter");
    await expect(editor).toContainText(copy.settled); await expect(input).toHaveValue("0000-02-29");
    expect(f.writes).toEqual([{ etsId: id, raw: "0000-02-29" }]);
    expect(f.bindings).toEqual([{ serverIncarnation: "parameters-fixture", projectIncarnation: 3, snapshotRevision: 1, expectedValue: "1899-12-31" }]);
    await expect(apply).toBeDisabled(); await input.focus(); await input.press("Tab");
    expect(f.bindings).toHaveLength(1);
    await editor.scrollIntoViewIfNeeded(); await noHorizontalOverflow(page);
    if (CAPTURE) await editor.screenshot({ path: join(CAPTURE, `date-${language}-${width}-applied.png`) });
    expect(f.unexpected).toEqual([]); expect(errors).toEqual([]);
  });
}

for (const [language, width] of [["en", 1440], ["de", 400]] as const) {
  test(`Date ${language} ${width}: historical hidden line breaks require complete replacement`, async ({ page }) => {
    const copy = parameterEditorCopy[language]; const m: Catalogue = language === "de" ? de : en;
    const f = await fixture(page, language, language === "de" ? "porcelain" : "graphite", width);
    const id = `${PROGRAM}_P-5_R-1`; const original = "2000-02-\n29"; f.values[id] = original;
    const panel = await openParameters(page, m); await opener(panel, id).click();
    const editor = panel.locator(`[data-ets-id="${id}"]`);
    const input = editor.getByRole("textbox", { name: "Reference date", exact: true });
    await expect(input).toHaveValue("2000-02-29"); await expect(editor.getByLabel(copy.original)).toHaveJSProperty("textContent", original);
    await input.evaluate(node => {
      if (!(node instanceof HTMLInputElement)) throw new Error("Expected the Date textbox");
      node.focus(); node.setSelectionRange(8, 10);
      const clipboardData = new DataTransfer(); clipboardData.setData("text/plain", "28");
      node.dispatchEvent(new ClipboardEvent("paste", { bubbles: true, cancelable: true, clipboardData }));
    });
    await expect(editor.getByRole("alert")).toHaveText(copy.literalPasteComplete);
    await expect(editor.getByLabel(copy.original)).toHaveJSProperty("textContent", original);
    await expect(editor.getByRole("button", { name: copy.apply, exact: true })).toBeDisabled(); expect(f.bindings).toEqual([]);
    await input.focus(); await input.press("ControlOrMeta+A");
    await input.evaluate(node => {
      const clipboardData = new DataTransfer(); clipboardData.setData("text/plain", "2090-01-01");
      node.dispatchEvent(new ClipboardEvent("paste", { bubbles: true, cancelable: true, clipboardData }));
    });
    await editor.getByRole("button", { name: copy.apply, exact: true }).click();
    await expect(editor).toContainText(copy.settled);
    expect(f.writes).toEqual([{ etsId: id, raw: "2090-01-01" }]);
    expect(f.bindings).toEqual([{ serverIncarnation: "parameters-fixture", projectIncarnation: 3, snapshotRevision: 1, expectedValue: original }]);
    expect(f.unexpected).toEqual([]);
  });
}

test("Date service refusal retains its draft and retries only on explicit Apply", async ({ page }) => {
  const copy = parameterEditorCopy.en; const f = await fixture(page, "en", "graphite", 1440);
  const panel = await openParameters(page, en); const id = `${PROGRAM}_P-5_R-1`; await opener(panel, id).click();
  const editor = panel.locator(`[data-ets-id="${id}"]`); const input = editor.getByRole("textbox", { name: "Reference date", exact: true });
  f.refuseWrites = true; f.refusalReason = "Authoritative Date edit refused";
  await input.fill("9999-12-31"); await editor.getByRole("button", { name: copy.apply, exact: true }).click();
  await expect(editor.getByRole("alert")).toHaveText(f.refusalReason); await expect(input).toHaveValue("9999-12-31");
  expect(f.writes).toEqual([]); expect(f.values[id]).toBe("1899-12-31"); expect(f.bindings).toHaveLength(1);
  await input.focus(); await input.press("Enter"); await input.press("Tab"); expect(f.bindings).toHaveLength(1);
  f.refuseWrites = false;
  await editor.getByRole("button", { name: copy.apply, exact: true }).click(); await expect(editor).toContainText(copy.settled);
  expect(f.writes).toEqual([{ etsId: id, raw: "9999-12-31" }]); expect(f.bindings).toHaveLength(2); expect(f.unexpected).toEqual([]);
});

test("Date conflict retains its draft until reset and sends a new authoritative binding", async ({ page }) => {
  const copy = parameterEditorCopy.en; const f = await fixture(page, "en", "graphite", 1440);
  const panel = await openParameters(page, en); const id = `${PROGRAM}_P-5_R-1`; await opener(panel, id).click();
  const editor = panel.locator(`[data-ets-id="${id}"]`); const input = editor.getByRole("textbox", { name: "Reference date", exact: true });
  await input.fill("2000-02-29"); f.values[id] = "0001-01-01"; f.revision += 1;
  await editor.getByRole("button", { name: copy.apply, exact: true }).click();
  await expect(editor).toContainText(copy.stale); await expect(input).toHaveValue("2000-02-29");
  await expect(editor.getByRole("button", { name: copy.apply, exact: true })).toBeDisabled();
  expect(f.writes).toEqual([]); expect(f.bindings).toHaveLength(1);
  await editor.getByRole("button", { name: copy.reset, exact: true }).click(); await expect(input).toHaveValue("0001-01-01");
  await input.fill("2000-02-29"); await editor.getByRole("button", { name: copy.apply, exact: true }).click();
  await expect(editor).toContainText(copy.settled); expect(f.writes).toEqual([{ etsId: id, raw: "2000-02-29" }]);
  expect(f.bindings).toEqual([
    { serverIncarnation: "parameters-fixture", projectIncarnation: 3, snapshotRevision: 1, expectedValue: "1899-12-31" },
    { serverIncarnation: "parameters-fixture", projectIncarnation: 3, snapshotRevision: 2, expectedValue: "0001-01-01" },
  ]);
  expect(f.unexpected).toEqual([]);
});

for (const sourceFault of ["sha", "ref", "program", "snapshot", "missing"] as const) {
  test(`Date ${sourceFault} source keeps fallback and the unchanged generic text field`, async ({ page }) => {
    const f = await fixture(page, "en", "graphite", 1440); f.sourceFault = sourceFault;
    const panel = await openParameters(page, en); const id = `${PROGRAM}_P-5_R-1`; await opener(panel, id).click();
    const editor = panel.locator(`[data-ets-id="${id}"]`);
    await expect(editor).toContainText(parameterEditorCopy.en.fallback[sourceFault === "sha" ? "source-conflict" : "missing-evidence"]);
    await expect(editor.getByRole("textbox")).toHaveCount(0); expect(f.bindings).toEqual([]);
    await editor.getByRole("button", { name: en["parameters.checked.close"], exact: true }).click();
    await expect(panel.locator(`[data-ets-id="${id}"] input[type="text"]`)).toHaveValue("1899-12-31");
    expect(f.unexpected).toEqual([]);
  });
}

// Goal B1: another client changed the value after this panel was shown.
for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} ${width}: a value changed elsewhere keeps the draft and resends only on request`, async ({ page }) => {
    const m: Catalogue = language === "de" ? de : en;
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    const f = await fixture(page, language, language === "de" ? "porcelain" : "graphite", width);
    const panel = await openParameters(page, m);
    const id = `${PROGRAM}_P-2_R-1`;
    const row = panel.locator(`label[data-ets-id="${id}"]`);
    const input = row.locator('input[type="number"]');
    await expect(input).toHaveValue("40");
    f.values[id] = "55"; f.revision += 1;
    await input.fill("60");
    await input.press("Enter");
    await expect(row.getByRole("alert")).toHaveText(m["parameters.conflict"]);
    const conflict = panel.locator(`[data-conflict-for="${id}"]`);
    await expect(conflict).toContainText(m["parameters.conflictCurrent"].replace("{value}", "55"));
    await expect(input).toHaveValue("60");
    expect(f.writes).toEqual([]);
    await input.focus(); await input.press("Tab");
    expect(f.bindings).toHaveLength(1);
    await conflict.scrollIntoViewIfNeeded();
    await noHorizontalOverflow(page);
    await capture(page, `conflict-${language}-${width}`);
    await conflict.getByRole("button", { name: m["parameters.conflictApply"], exact: true }).click();
    await expect(conflict).toHaveCount(0);
    expect(f.writes).toEqual([{ etsId: id, raw: "60" }]);
    expect(f.bindings).toEqual([
      { serverIncarnation: "parameters-fixture", projectIncarnation: 3, snapshotRevision: 1, expectedValue: "40" },
      { serverIncarnation: "parameters-fixture", projectIncarnation: 3, snapshotRevision: 2, expectedValue: "55" },
    ]);
    await expect(input).toHaveValue("60");
    expect(f.unexpected).toEqual([]);
    expect(errors).toEqual([]);
  });
}

test("#37: a committed reply after navigation still updates App history and same-device reopening", async ({ page }) => {
  const f = await fixture(page, "en", "graphite", 1440);
  const editor = await openParameters(page, en);
  const id = `${PROGRAM}_P-1_R-1`;
  const input = editor.locator(`label[data-ets-id="${id}"] input[type="number"]`);
  const undo = page.getByRole("button", { name: en["toolbar.undo"], exact: true });
  await expect(undo).toBeDisabled();
  let release!: () => void;
  const held = new Promise<void>(done => { release = done; });
  f.beforeParameterReply = () => held;
  try {
    await input.fill("12"); await input.press("Enter"); await expect(input).toBeDisabled();
    await page.getByRole("navigation", { name: en["workbench.navigation"], exact: true })
      .getByRole("button", { name: en["workbench.devices"], exact: true }).click();
    await expect(page.locator(".device-workspace")).toHaveCount(0);
    await expect(undo).toBeDisabled();
    release();
    // Retiring a row must not lose the only notification of a committed command.
    await expect(undo).toBeEnabled();
    await page.locator(".devices-workspace").getByRole("button", { name: DEVICE.name, exact: true }).click();
    await page.getByRole("tab", { name: en["workbench.parameters"], exact: true }).click();
    await openParameterSections(page);
    await expect(input).toBeEnabled(); await expect(input).toHaveValue("12");
    expect(f.writes).toEqual([{ etsId: id, raw: "12" }]);
    expect(f.unexpected).toEqual([]);
  } finally { release(); }
});

// #37: real App publication rejects an old tree, so the shared parameter
// panel must reject that response too instead of waiting for a healing refetch.
for (const language of ["en", "de"] as const) {
  test(`#37 ${language}: overlapping rows settle, conflict retries bind freshly, late success cannot roll back`, async ({ page }) => {
    const m: Catalogue = language === "de" ? de : en;
    const f = await fixture(page, language, "graphite", 1440);
    const editor = await openParameters(page, m);
    const aId = `${PROGRAM}_P-1_R-1`, bId = `${PROGRAM}_P-2_R-1`;
    const aRow = editor.locator(`label[data-ets-id="${aId}"]`), bRow = editor.locator(`label[data-ets-id="${bId}"]`);
    const a = aRow.locator('input[type="number"]'), b = bRow.locator('input[type="number"]');
    let releaseA!: () => void, releaseB!: () => void;
    const heldA = new Promise<void>(done => { releaseA = done; });
    const heldB = new Promise<void>(done => { releaseB = done; });
    f.beforeParameterReply = (id, status) => id === aId ? heldA : status === 409 ? heldB : Promise.resolve();
    try {
      await a.fill("12"); await a.press("Enter");
      await expect(a).toBeDisabled();
      await b.fill("60"); await b.press("Enter");
      await expect(b).toBeDisabled();
      await expect(aRow.locator(".parameter-field-saving")).toBeVisible();
      await expect(bRow.locator(".parameter-field-saving")).toBeVisible();
      await expect.poll(() => f.bindings.length).toBe(2);
      expect(f.writes).toEqual([{ etsId: aId, raw: "12" }]);
      releaseB();
      await expect(b).toBeEnabled(); await expect(a).toBeDisabled();
      await expect(bRow.getByRole("alert")).toHaveText(m["parameters.conflict"]);
      await expect(b).toHaveValue("60");
      const retry = editor.locator(`[data-conflict-for="${bId}"]`).getByRole("button", { name: m["parameters.conflictApply"], exact: true });
      await expect(retry).toBeEnabled(); await retry.click();
      await expect(b).toBeEnabled();
      expect(f.bindings).toEqual([
        { serverIncarnation: "parameters-fixture", projectIncarnation: 3, snapshotRevision: 1, expectedValue: "0010" },
        { serverIncarnation: "parameters-fixture", projectIncarnation: 3, snapshotRevision: 1, expectedValue: "40" },
        { serverIncarnation: "parameters-fixture", projectIncarnation: 3, snapshotRevision: 2, expectedValue: "40" },
      ]);
      releaseA();
      await expect(a).toBeEnabled(); await expect(b).toBeEnabled();
      await expect(a).toHaveValue("12"); await expect(b).toHaveValue("60");
      await expect(editor.locator(".parameter-field-saving")).toHaveCount(0);
      expect(f.writes).toEqual([{ etsId: aId, raw: "12" }, { etsId: bId, raw: "60" }]);
      // No lost command and no extra retry: normal history remains authoritative.
      await page.locator(".device-workspace h2").click();
      await page.getByRole("button", { name: m["toolbar.undo"], exact: true }).click();
      await expect(b).toHaveValue("40"); await expect(a).toHaveValue("12");
      await page.getByRole("button", { name: m["toolbar.redo"], exact: true }).click();
      await expect(b).toHaveValue("60"); await expect(a).toHaveValue("12");
      expect(f.unexpected).toEqual([]);
    } finally { releaseA(); releaseB(); }
  });
}
