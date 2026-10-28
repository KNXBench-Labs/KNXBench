/** WP-20: the real Devices list System column over a strict, fictional catalogue. */
import { expect, test, type Page } from "@playwright/test";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import type { DeviceNode } from "../src/bindings/DeviceNode";
import type { DeviceCatalog } from "../src/deviceList";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { fulfillNoProjectOpen } from "./server-project-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";

const SYSTEMS: Record<number, NonNullable<DeviceCatalog["devices"][number]["system"]>> = {
  1: { state: "family", mask: 0x0012, family: "system1" },
  2: { state: "family", mask: 0x0025, family: "system2" },
  3: { state: "family", mask: 0x0701, family: "system7" },
  4: { state: "family", mask: 0x07b0, family: "systemB" },
  5: { state: "family", mask: 0x0300, family: "system300" },
  6: { state: "unknownMask", mask: 0x0afd },
  7: { state: "noApplicationProgram" },
  8: { state: "noProduct" },
  9: { state: "applicationNotOfProduct" },
  10: { state: "applicationNotInDatabase" },
};
function tree(): ProjectTree {
  const devices: DeviceNode[] = Object.keys(SYSTEMS).map(Number).map((id) => ({ id, name: `Fictional device ${id}`,
    address: `1.1.${id}`, description: null, com_object_count: 0 }));
  return { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
    group_address_style: "ThreeLevel", server_incarnation: "system-browser", snapshot_revision: 1,
    installations: [{ id: 1, name: "House", topology: [{ id: 1, name: "Area", address: 1, lines: [
      { id: 1, name: "Ground floor", address: 1, devices }] }], unassigned: [], buildings: [], group_ranges: [], group_addresses: [] }] };
}
function catalogue(project: ProjectTree): DeviceCatalog {
  const devices = project.installations[0].topology[0].lines[0].devices;
  return { schemaVersion: 1, serverIncarnation: "system-browser", snapshotRevision: 1, devices: devices.map((device) => {
    const system = SYSTEMS[device.id];
    const resolved = system.state !== "noProduct";
    return { id: device.id, device, productRef: resolved ? "Fictional-P-1" : null, resolution: resolved ? "Resolved" : "NoReference",
      manufacturerId: resolved ? "M-0000" : null, manufacturerName: resolved ? "Fictional Devices" : null,
      productText: resolved ? "Fictional product" : null, orderNumber: null, productTextLanguage: null, productSourceLanguage: null, system };
  }) };
}
async function intercept(page: Page, language: "en" | "de") {
  const state = { tree: tree(), unexpected: [] as string[] };
  let opened = false;
  await page.route("**/api/**", async (route) => {
    const path = new URL(route.request().url()).pathname; const method = route.request().method();
    let body: unknown; let status = 200;
    if (path === "/api/auth/status" && method === "GET") body = { required: false, authenticated: true };
    else if (path === "/api/settings" && method === "GET") body = { schemaVersion: 1, status: "ok", settings: { uiLanguage: language, theme: "porcelain" } };
    else if (path === "/api/project" && method === "GET") { if (!opened) return fulfillNoProjectOpen(route); body = state.tree; }
    else if (path === "/api/project/new" && method === "POST") { opened = true; body = state.tree; }
    else if (path === "/api/project/load-progress" && method === "GET") body = null;
    else if (path === "/api/project/status-metrics" && method === "GET") body = statusMetricsFixtureBody(state.tree);
    else if (path === "/api/bus/discover" && method === "POST") body = { interfaces: [] };
    else if (path === "/api/product-languages" && method === "GET") body = [];
    else if (["/api/device-download/status", "/api/device-address/status"].includes(path) && method === "GET") { status = 404; body = { error: "no active operation" }; }
    else if (path === "/api/device-system2/status" && method === "GET") body = null;
    else if (path === "/api/devices" && method === "GET") body = catalogue(state.tree);
    else if (achievementsFixtureAnswer(method, path) !== undefined) body = achievementsFixtureAnswer(method, path);
    else { state.unexpected.push(`${method} ${path}`); status = 404; body = { error: "strict system fixture" }; }
    return route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
  });
  return state;
}
async function open(page: Page, language: "en" | "de") {
  const m = language === "en" ? en : de;
  await page.goto("/");
  await page.getByRole("button", { name: m["toolbar.newProject"], exact: true }).first().click();
  await page.getByRole("dialog", { name: m["newProject.title"], exact: true }).getByRole("button", { name: m["newProject.create"], exact: true }).click();
  await page.getByRole("dialog", { name: m["projectWizard.done.title"], exact: true }).getByRole("button", { name: m["projectWizard.done.close"], exact: true }).click();
  await page.keyboard.press("Control+Shift+P"); await page.getByRole("combobox").fill(m["workbench.devices"]);
  await page.getByRole("combobox").press("Enter"); await expect(page.locator(".devices-workspace")).toBeVisible();
}

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  const m = language === "en" ? en : de;
  test(`${language} ${width}px: System column names declared families and explicit states, sorts and filters`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    const state = await intercept(page, language); await open(page, language);
    const table = page.locator('[data-table-id="devices"]');
    const header = table.locator("thead th").filter({ has: page.getByRole("button", { name: m["devices.system"], exact: true }) });
    await expect(header).toHaveCount(1);
    await expect(header).toHaveAttribute("aria-sort", "none");
    const column = await header.evaluate((cell) => [...cell.parentElement!.children].indexOf(cell));
    const cell = (id: number) => table.locator(`tbody tr[data-rename-id="${id}"] td`).nth(column);
    const expected: Record<number, string> = {
      1: m["devices.system.system1"], 2: m["devices.system.system2"], 3: m["devices.system.system7"], 4: m["devices.system.systemB"],
      5: m["devices.system.system300"], 6: m["devices.system.unknownMask"].replace("{mask}", "0AFDh"),
      7: m["devices.system.noApplicationProgram"], 8: m["devices.noReference"],
      9: m["devices.system.applicationNotOfProduct"], 10: m["devices.system.applicationNotInDatabase"],
    };
    for (const [id, text] of Object.entries(expected)) await expect(cell(Number(id))).toHaveText(text);
    await expect(cell(3)).toHaveAttribute("title", m["devices.systemDeclared"].replace("{mask}", "0701h"));
    await header.getByRole("button").click();
    await expect(header).toHaveAttribute("aria-sort", "ascending");
    const order = () => table.locator("tbody tr").evaluateAll((rows) => rows.map((row) => row.getAttribute("data-rename-id")));
    const sorted = Object.entries(expected).sort(([a, x], [b, y]) =>
      new Intl.Collator(language, { numeric: true, sensitivity: "base" }).compare(x, y) || Number(a) - Number(b)).map(([id]) => id);
    expect(await order()).toEqual(sorted);
    const filter = page.getByRole("searchbox", { name: m["devices.filter"], exact: true });
    await filter.fill("System B"); await expect(table.locator("tbody tr")).toHaveCount(1);
    await expect(table.locator('tbody tr[data-rename-id="4"]')).toBeVisible();
    await filter.fill("0afd"); await expect(table.locator("tbody tr")).toHaveCount(1);
    await expect(table.locator('tbody tr[data-rename-id="6"]')).toBeVisible();
    await filter.fill("");
    const box = (await header.boundingBox())!;
    expect(box.width).toBeGreaterThanOrEqual(24);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth)).toBeLessThanOrEqual(1);
    expect(state.unexpected).toEqual([]);
  });
}
