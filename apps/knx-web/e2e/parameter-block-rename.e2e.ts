/** A renamed parameter area keeps one identity while its caption follows the selector value. */
import { parameterNavigationCapturesAvailable, runParameterNavigationCapture } from "./parameter-navigation-capture";
import { openExplorerBranches } from "./disclosure-fixture";
import { expect, test, type Page } from "@playwright/test";
import type { ParameterField, ParameterPanel } from "../src/api";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { busActivityFixtureAnswer } from "./bus-activity-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";

type Language = "en" | "de";
const DEVICE = { id: 41, name: "Fictional dimmer", address: "1.1.7", description: null, com_object_count: 0 };
const PROGRAM = "A-FICTIONAL-RENAME";
// What a server answers for the same area key under each selector value, per UI language.
const CAPTIONS: Record<Language, readonly string[]> = {
  en: ["fixture_base_px", "fixture_switch_py", "fixture_switch_pz"],
  de: ["probe_de_px", "probe_de_py", "probe_de_pz"],
};
const LOCKED: Record<Language, string> = { en: "Renamed locked", de: "Umbenannt gesperrt" };

interface Fixture {
  unexpected: string[];
  revision: number;
  selector: string;
  history: string[];
  future: string[];
  writes: { etsId: string; raw: string }[];
}

function projectTree(f: Fixture): ProjectTree {
  return {
    schema_version: 13, errors: 0, warnings: 0, can_undo: f.history.length > 0, can_redo: f.future.length > 0, is_modified: f.writes.length > 0,
    server_incarnation: "rename-fixture", project_incarnation: 3, snapshot_revision: f.revision,
    group_address_style: "ThreeLevel",
    installations: [{ id: 0, name: "Fictional flat",
      topology: [{ id: 21, name: "Main area", address: 1, lines: [{ id: 31, name: "Ground floor", address: 1, devices: [DEVICE] }] }],
      buildings: [], unassigned: [], group_ranges: [], group_addresses: [] }],
  };
}

const base = (etsId: string, group: { key: string; label: string }, extra: Partial<ParameterField>): ParameterField => ({
  etsId, name: etsId, nameLanguage: null, text: etsId, textLanguage: null, kind: "Number", value: "1", valueSource: "Stored",
  editable: true, min: "0", max: "100", enumOptions: [], displayOrder: null, access: "ReadWrite", writeEtsId: etsId, group, ...extra,
});
function panelOf(f: Fixture, language: Language, withTree: boolean): ParameterPanel {
  const area = { key: "PB-1", label: CAPTIONS[language][Number(f.selector)] };
  const locked = { key: "PB-2", label: LOCKED[language] };
  return { programId: PROGRAM, sourceLanguage: null, stale: [], diagnostics: [], tree: withTree ? projectTree(f) : null,
    sections: [{ scope: null, fields: [
      base("Selector", area, { text: "Function", name: "Function", kind: "Restriction", value: f.selector,
        enumOptions: ["0", "1", "2"].map(value => ({ value, text: `Option ${value}`, language: null })) }),
      base("Delay", area, { text: "Switch-off delay" }),
      base("Locked", locked, { text: "Locked value", editable: false, access: "Read", blockAccess: "Read" }),
    ] }] };
}

async function fixture(page: Page, language: Language, width: number): Promise<Fixture> {
  const f: Fixture = { unexpected: [], revision: 1, selector: "0", history: [], future: [], writes: [] };
  await page.setViewportSize({ width, height: 900 });
  const appOrigin = new URL(String(test.info().project.use.baseURL)).origin;
  page.on("request", request => { if (new URL(request.url()).origin !== appOrigin) f.unexpected.push(request.url()); });
  await page.route("https://api.github.com/**", route => { f.unexpected.push(route.request().url()); return route.abort(); });
  await page.route("**/api/**", route => {
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
      uiLanguage: language, theme: "porcelain", motionLevel: "off", releaseChecksEnabled: false,
      onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.9" } } });
    if (path === "/api/settings" && method === "PUT") return ok({ conditionalPatchVersion: 1, schemaVersion: 1, status: "ok", settings: route.request().postDataJSON().settings });
    if (path === "/api/version") return ok({ version: "0.1.0-alpha.9+rename-fixture" });
    if (path === "/api/project" && method === "GET") return ok(projectTree(f));
    if ((path === "/api/undo" || path === "/api/redo") && method === "POST") {
      const from = path === "/api/undo" ? f.history : f.future;
      const to = path === "/api/undo" ? f.future : f.history;
      const previous = from.pop();
      if (previous === undefined) return refuse(400, "emptyHistory", "No fixture history");
      to.push(f.selector); f.selector = previous; f.revision += 1;
      return ok(projectTree(f));
    }
    if (path === "/api/project/load-progress") return ok(null);
    if (path === "/api/project/status-metrics" && method === "GET") return ok(statusMetricsFixtureBody(projectTree(f)));
    if (path === "/api/log") return ok([]);
    if (path === "/api/bus/discover" && method === "POST") return ok({ interfaces: [] });
    if (["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items"].includes(path)) return ok([]);
    if (["/api/device-download/status", "/api/device-download/batch/status", "/api/device-address/status"].includes(path)) return refuse(404, "none", "no fixture operation");
    if (path === "/api/device-system2/status" && method === "GET") return ok(null);
    if (path === "/api/devices" && method === "GET") return ok({ schemaVersion: 1, serverIncarnation: "rename-fixture", snapshotRevision: f.revision, devices: [] });
    if (path === "/api/device/41" && method === "GET") return ok({ ...DEVICE, com_objects: [], product: { resolution: "NoReference", product_ref: null, program_ref: null, catalog: null } });
    if (path === "/api/device/41/parameters" && method === "GET") return ok(panelOf(f, language, false));
    if (path === "/api/device/41/parameters" && method === "POST") {
      const { etsId, raw } = route.request().postDataJSON() as { etsId: string; raw: string };
      if (etsId !== "Selector") return refuse(400, "invalidParameterValue", "only the selector is writable in this fixture");
      f.history.push(f.selector); f.future = [];
      f.writes.push({ etsId, raw }); f.selector = raw; f.revision += 1;
      return ok(panelOf(f, language, true));
    }
    f.unexpected.push(`${method} ${path}`);
    return refuse(404, "strict", "strict rename fixture");
  });
  await page.goto("/?view=editor");
  return f;
}

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} ${width}px: a renamed area follows the selector under one identity`, async ({ page }) => {
    const m = language === "de" ? de : en;
    const captions = CAPTIONS[language];
    const errors: string[] = [];
    page.on("pageerror", error => errors.push(error.message));
    const f = await fixture(page, language, width);
    await openExplorerBranches(page);
    await page.locator(".project-explorer").getByText(DEVICE.name, { exact: false }).first().click();
    await page.getByRole("tab", { name: m["workbench.parameters"], exact: true }).click();
    const workspace = page.locator(".parameter-workspace:visible");
    const navigation = workspace.locator(".parameter-workspace-nav button");
    const heading = workspace.locator(".parameter-workspace-heading h4");
    const context = workspace.locator(".parameter-workspace-context");
    await expect(navigation).toHaveCount(1);
    await navigation.nth(0).click();
    await expect(heading).toHaveText(captions[0]);
    // The row element is the identity: it must survive every caption change.
    const selectorRow = workspace.locator('[data-parameter-row="Selector"]');
    await selectorRow.evaluate(row => row.setAttribute("data-probe", "same-row"));
    await selectorRow.click();
    const choose = async (value: string, expected: string) => {
      await selectorRow.locator("select").selectOption(value);
      await expect(navigation.nth(0)).toContainText(expected);
      await expect(heading).toHaveText(expected);
      await expect(context).toContainText(`/ ${expected}`);
      await expect(navigation).toHaveCount(1);
      await expect(workspace.locator('[data-parameter-row="Selector"]')).toHaveAttribute("data-probe", "same-row");
      await expect(navigation.nth(0)).toHaveAttribute("aria-current", "page");
    };
    await choose("1", captions[1]);
    await choose("2", captions[2]);
    // Search sees the shown caption only.
    const search = workspace.locator('input[type="search"]');
    await search.fill(captions[2].toLowerCase());
    await expect(workspace.locator('[data-parameter-row="Delay"]')).toBeVisible();
    await search.fill(captions[1].toLowerCase());
    await expect(workspace.locator(".parameter-workspace-empty")).toBeVisible();
    await search.fill("");
    // Authoritative undo and redo move the caption with the value.
    await page.getByRole("button", { name: m["toolbar.undo"], exact: true }).click();
    await expect(heading).toHaveText(captions[1]);
    await expect(navigation.nth(0)).toContainText(captions[1]);
    await page.getByRole("button", { name: m["toolbar.redo"], exact: true }).click();
    await expect(heading).toHaveText(captions[2]);
    await choose("0", captions[0]);
    // The manufacturer-restricted view (section accordions) is renamed too; its row stays read-only.
    await page.getByRole("tab", { name: m["parameters.restrictedTab"], exact: true }).click();
    const restricted = page.locator('[role="tabpanel"]:not([hidden])');
    const section = restricted.locator("details.parameter-section");
    if (!await section.evaluate(node => node.hasAttribute("open"))) await section.locator("summary").click();
    const toggle = restricted.locator(".parameter-group-toggle");
    await expect(toggle).toHaveCount(1);
    await expect(toggle).toContainText(LOCKED[language]);
    await toggle.click();
    await expect(restricted.locator('[data-ets-id="Locked"]')).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    expect(f.writes.map(write => write.raw)).toEqual(["1", "2", "0"]);
    expect(f.unexpected).toEqual([]);
    expect(errors).toEqual([]);
  });
}

for (const language of ["en", "de"] as const) for (const width of parameterNavigationCapturesAvailable ? [1440, 400] : []) {
  test(`${language} ${width}px: producer-backed parameter navigation retains drafts and write authority`, async ({ page }) => {
    await runParameterNavigationCapture(page, language, width);
  });
}
