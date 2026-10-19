/** Parameter captions and a unit suffix in the Parameters tab, on intercepted fictional data. */
import { expect, test, type Page } from "@playwright/test";
import { join } from "node:path";
import type { ParameterField, ParameterPanel } from "../src/api";
import type { DeviceNode } from "../src/bindings/DeviceNode";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import { messages as de } from "../src/messages/de";
import { messages as en } from "../src/messages/en";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";

const device: DeviceNode = { id: 1, name: "Fictional keypad", address: "1.1.7", description: "Fictional device", com_object_count: 0 };
const tree: ProjectTree = { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
  group_address_style: "ThreeLevel", server_incarnation: "matrix-browser", project_incarnation: 1, snapshot_revision: 1,
  installations: [{ id: 1, name: "House", topology: [{ id: 1, name: "Area", address: 1,
    lines: [{ id: 2, name: "Line", address: 1, devices: [device] }] }], unassigned: [], buildings: [], group_ranges: [], group_addresses: [] }] };

const base = (etsId: string, group: string, label: string): ParameterField => ({
  etsId, writeEtsId: etsId, name: etsId, text: etsId, nameLanguage: null, textLanguage: null, kind: "Number", value: "1",
  valueSource: "Stored", editable: true, min: "0", max: "100", enumOptions: [], displayOrder: null, access: "ReadWrite",
  group: { key: group, label }, controlsComObjects: false, cell: null,
});
async function intercept(page: Page, language: "en" | "de") {
  const state = { unexpected: [] as string[], posts: [] as { etsId: string; raw: string }[], projectOpen: false, aLeft: "0" };
  const panel = (): ParameterPanel => ({ programId: "A-FICTIONAL", sourceLanguage: "en", tree: null, stale: [], diagnostics: [],
    sections: [{ scope: null, fields: [
      { ...base("Defaults", "defaults", "Functions: Ref default"), group: { key: "defaults", label: "Functions: Ref default",
        labelFill: { template: "Functions: {{0}}", source: "ProductDefault" } } },
      { ...base("Stored", "stored", "Functions: Kitchen"), suffixText: "s", suffixTextLanguage: null, group: { key: "stored", label: "Functions: Kitchen",
        labelFill: { template: "Functions: {{0}}", source: "StoredValue" } } },
      { ...base("Unresolved", "missing", "{{0}}"), group: { key: "missing", label: "{{0}}",
        labelFill: { template: "{{0}}", source: "UnknownParameter" } } },
      base("Marker", "marker", "#"), base("Empty", "empty", ""),
    ], separators: [{ position: 2, group: { key: "stored", label: "Functions: Kitchen",
      labelFill: { template: "Functions: {{0}}", source: "StoredValue" } }, elementId: "PS", text: "Note: Kitchen",
      textFill: { template: "Note: {{0}}", source: "StoredValue" }, uiHint: "Headline", access: null, blockAccess: null }] }] });
  await page.route("**/api/**", (route) => {
    const url = new URL(route.request().url()); const path = url.pathname; const method = route.request().method();
    let body: unknown; let status = 200;
    if (path === "/api/auth/status" && method === "GET") body = { required: false, authenticated: true };
    else if (path === "/api/settings" && method === "GET") body = { schemaVersion: 1, status: "ok", settings: { uiLanguage: language, theme: "porcelain" } };
    else if (path === "/api/project" && method === "GET") { if (state.projectOpen) body = tree; else { status = 400; body = { error: "no project open" }; } }
    else if (path === "/api/project/new" && method === "POST") { state.projectOpen = true; body = tree; }
    else if (path === "/api/project/load-progress" && method === "GET") body = null;
    else if (path === "/api/project/status-metrics" && method === "GET") body = statusMetricsFixtureBody(tree);
    else if (path === "/api/log" && method === "GET") body = [];
    else if (path === "/api/legacy-password" && method === "GET") body = { available: true, remembered: false, problem: null };
    else if (path === "/api/bus/discover" && method === "POST") body = { interfaces: [] };
    else if (path === "/api/product-languages" && method === "GET") body = [];
    else if (["/api/device-download/status", "/api/device-address/status"].includes(path) && method === "GET") { status = 404; body = { error: "no operation in this fixture" }; }
    else if (path === "/api/devices" && method === "GET") body = { schemaVersion: 1, serverIncarnation: "matrix-browser", snapshotRevision: 1,
      devices: [{ id: 1, device, productRef: "Fictional-P-1", resolution: "Resolved", manufacturerId: "Fictional-M-1",
        manufacturerName: "Fictional Devices", productText: "Fictional keypad", orderNumber: "FK-4", productTextLanguage: null, productSourceLanguage: null,
        system: { state: "family", mask: 0x07b0, family: "systemB" } }] };
    else if (path === "/api/device/1" && method === "GET") body = { ...device, com_objects: [], product: { resolution: "NoReference", product_ref: null, program_ref: null, catalog: null } };
    else if (path === "/api/device/1/parameters" && method === "GET") body = panel();
    else if (path === "/api/bus/monitor/telegrams" && method === "GET") body = { sessionId: 1, serverIncarnation: "matrix-browser", contextStatus: "current",
      projectOpen: true, status: "active", nextSince: 0, droppedBefore: 0, telegrams: [] };
    else if (path === "/api/device-readiness" && method === "GET") body = { devices: [], counts: {} };
    else if (path === "/api/device-address/availability" && method === "GET") body = { startAvailable: false, reason: "Not in this fixture" };
    else if (path === "/api/bus/activity" && method === "GET") body = { serverIncarnation: "fixture", coverage: "partial", historyState: "disabled", sessions: [], oneShot: [], oneShotDropped: 0, busyLocks: [], untracked: [] };
    else if (achievementsFixtureAnswer(method, path) !== undefined) body = achievementsFixtureAnswer(method, path);
    else { state.unexpected.push(`${method} ${path}`); status = 404; body = { error: "strict matrix fixture only" }; }
    return route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
  });
  return state;
}

async function openParameters(page: Page, language: "en" | "de") {
  const m = language === "en" ? en : de;
  await page.goto("/");
  await page.getByRole("button", { name: m["toolbar.newProject"], exact: true }).first().click();
  await page.getByRole("dialog", { name: m["newProject.title"], exact: true }).getByRole("button", { name: m["newProject.create"], exact: true }).click();
  await page.getByRole("dialog", { name: m["projectWizard.done.title"], exact: true }).getByRole("button", { name: m["projectWizard.done.close"], exact: true }).click();
  await page.keyboard.press("Control+Shift+P");
  await page.getByRole("combobox").fill(m["workbench.devices"]);
  await page.getByRole("combobox").press("Enter");
  await page.locator(".devices-table tbody td:nth-child(3) button").filter({ hasText: /^Fictional keypad$/ }).click();
  await page.getByRole("tab", { name: m["workbench.parameters"], exact: true }).click();
}

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} parameter caption resolution and fallbacks at ${width}px`, async ({ page }) => {
    const m = language === "de" ? de : en;
    await page.setViewportSize({ width, height: 1000 });
    const state = await intercept(page, language);
    await openParameters(page, language);
    const workspace = page.locator(".parameter-workspace");
    const buttons = workspace.locator(".parameter-workspace-nav button");
    await expect(buttons).toHaveCount(5);
    await expect(buttons.nth(0)).toContainText("Functions: Ref default");
    await expect(buttons.nth(0)).toHaveAttribute("title", /Functions: \{\{0\}\}/);
    await expect(buttons.nth(1)).toContainText("Functions: Kitchen");
    const fallback = (number: number) => m["parameters.groupNumber"].replace("{number}", String(number));
    for (let index = 2; index < 5; index++) {
      await expect(buttons.nth(index).locator("span").first()).toHaveText(fallback(index + 1));
      await expect(buttons.nth(index)).toHaveAttribute("title", /.+/);
    }
    expect(await workspace.locator(".parameter-workspace-nav").innerText()).not.toContain("{{");
    expect(await workspace.locator(".parameter-workspace-nav").innerText()).not.toContain("#");
    await buttons.nth(2).click();
    await expect(workspace.locator(".parameter-workspace-heading h4")).toHaveText(fallback(3));
    await expect(workspace.locator(".parameter-workspace-heading h4")).toHaveAttribute("title", /\{\{0\}\}/);
    await buttons.nth(1).click();
    await expect(workspace.locator(".parameter-workspace-separator h5")).toHaveText("Note: Kitchen");
    // WP-02: the unit suffix sits right after the value, on its line, and is not part of it.
    const stored = workspace.locator('[data-ets-id="Stored"]');
    await expect(stored.locator(".parameter-field-suffix")).toHaveText("s");
    await expect(stored.locator("input")).toHaveValue("1");
    const input = (await stored.locator("input").boundingBox())!;
    const suffix = (await stored.locator(".parameter-field-suffix").boundingBox())!;
    expect(suffix.x).toBeGreaterThanOrEqual(input.x + input.width - 1);
    expect(Math.abs(suffix.y + suffix.height / 2 - (input.y + input.height / 2))).toBeLessThanOrEqual(4);
    expect(suffix.x + suffix.width).toBeLessThanOrEqual(width);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    expect(state.posts).toEqual([]);
    expect(state.unexpected).toEqual([]);
    if (process.env.KNX_PARAMETER_CAPTIONS_CAPTURE_DIR) {
      await workspace.scrollIntoViewIfNeeded();
      await workspace.screenshot({ path: join(process.env.KNX_PARAMETER_CAPTIONS_CAPTURE_DIR, `captions-${language}-${width}.png`) });
    }
  });
}
