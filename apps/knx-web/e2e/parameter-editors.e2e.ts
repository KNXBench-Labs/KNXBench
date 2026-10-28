/** Built device editor: checked parameter editors, source evidence, writes and refusals. */
import { openExplorerBranches, openParameterSections } from "./disclosure-fixture";
import { expect, test, type Page } from "@playwright/test";
import { join } from "node:path";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
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
  identity: boolean;
}

function projectTree(f: Fixture): ProjectTree {
  return {
    schema_version: 13, errors: 0, warnings: 0, can_undo: f.writes.length > 0, can_redo: false, is_modified: f.writes.length > 0,
    ...(f.identity ? { server_incarnation: "parameters-fixture", project_incarnation: 3, snapshot_revision: f.revision } : {}),
    group_address_style: "ThreeLevel",
    installations: [{ id: 0, name: "Fictional flat",
      topology: [{ id: 21, name: "Main area", address: 1, lines: [{ id: 31, name: "Ground floor", address: 1, devices: [DEVICE] }] }],
      buildings: [], unassigned: [], group_ranges: [], group_addresses: [] }],
  };
}

function field(etsId: string, text: string, kind: string, f: Fixture, min: string | null, max: string | null) {
  return { etsId, name: text, nameLanguage: null, text, textLanguage: null, kind, value: f.values[etsId], valueSource: "Stored",
    editable: true, min, max, enumOptions: [], displayOrder: null, access: "ReadWrite", writeEtsId: etsId };
}
function panelOf(f: Fixture, withTree: boolean) {
  return { programId: PROGRAM, sourceLanguage: null, stale: [], diagnostics: [], tree: withTree ? projectTree(f) : null,
    sections: [{ scope: null, fields: [
      field(`${PROGRAM}_P-1_R-1`, "Switch-off delay", "Time", f, "0", "3600"),
      field(`${PROGRAM}_P-2_R-1`, "Basic brightness", "Number", f, "0", "100"),
      field(`${PROGRAM}_P-3_R-1`, "Status LED colour", "Color", f, null, null),
    ] }] };
}
const SOURCES: Record<string, { child: string; type: string; attributes: Record<string, string> }> = {
  [`${PROGRAM}_P-1_R-1`]: { child: "TypeTime", type: "PT-Delay", attributes: { SizeInBit: "16", minInclusive: "0", maxInclusive: "3600", Unit: "Seconds" } },
  [`${PROGRAM}_P-2_R-1`]: { child: "TypeNumber", type: "PT-Brightness", attributes: { SizeInBit: "8", Type: "unsignedInt", minInclusive: "0", maxInclusive: "100", UIHint: "Slider" } },
  [`${PROGRAM}_P-3_R-1`]: { child: "TypeColor", type: "PT-Colour", attributes: {} },
};

async function fixture(page: Page, language: Language, theme: string, width: number, identity = true): Promise<Fixture> {
  const f: Fixture = { unexpected: [], revision: 1, writes: [], bindings: [], refuseWrites: false, identity,
    values: { [`${PROGRAM}_P-1_R-1`]: "0010", [`${PROGRAM}_P-2_R-1`]: "40", [`${PROGRAM}_P-3_R-1`]: "#00FF00" } };
  await page.setViewportSize({ width, height: 900 });
  await page.route("https://api.github.com/**", (route) => { f.unexpected.push(route.request().url()); return route.abort(); });
  await page.route("**/api/**", (route) => {
    const url = new URL(route.request().url());
    const path = url.pathname;
    const method = route.request().method();
    const ok = (body: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
    const refuse = (status: number, kind: string, error: string) => route.fulfill({ status, contentType: "application/json", body: JSON.stringify({ error, kind }) });
    const achievements = achievementsFixtureAnswer(method, path);
    if (achievements !== undefined) return ok(achievements);
    if (path === "/api/auth/status") return ok({ required: false, authenticated: true });
    if (path === "/api/settings" && method === "GET") return ok({ conditionalPatchVersion: 1, schemaVersion: 1, status: "ok", settings: {
      uiLanguage: language, theme, motionLevel: "off", releaseChecksEnabled: false,
      onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.9" } } });
    if (path === "/api/settings" && method === "PUT") return ok({ conditionalPatchVersion: 1, schemaVersion: 1, status: "ok", settings: route.request().postDataJSON().settings });
    if (path === "/api/version") return ok({ version: "0.1.0-alpha.9+parameters-fixture" });
    if (path === "/api/project" && method === "GET") return ok(projectTree(f));
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
      // Checked like the server (revision-bound-writes): any moved context or value is a 409.
      if (binding.serverIncarnation !== "parameters-fixture" || binding.projectIncarnation !== 3
        || binding.snapshotRevision !== f.revision || binding.expectedValue !== f.values[etsId]) {
        return refuse(409, "parameterWriteConflict", "parameter value or project changed since it was shown");
      }
      if (f.refuseWrites) return refuse(400, "invalidParameterValue", "Value 3601 is outside the parameter's range 0..3600");
      f.writes.push({ etsId, raw }); f.values[etsId] = raw; f.revision += 1;
      return ok(panelOf(f, true));
    }
    if (path === "/api/device/41/parameter-editor-source" && method === "GET") {
      const etsId = url.searchParams.get("etsId") ?? "";
      const source = SOURCES[etsId];
      const head = { device: 41, parameterRefId: etsId, serverIncarnation: "parameters-fixture", projectIncarnation: 3, snapshotRevision: f.revision };
      if (!source) return ok({ ...head, evidence: { status: "unavailable", reason: "missingReference" } });
      const typeId = `${PROGRAM}_${source.type}`;
      return ok({ ...head, evidence: { status: "declared", source: { sourceSha256: SHA, programId: PROGRAM, parameterTypeId: typeId, schema: 21,
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
