/** Declared parameter grids in the Parameters tab, with strictly intercepted fictional data. */
import { expect, test, type Page } from "@playwright/test";
import { join } from "node:path";
import type { ParameterField, ParameterGroupLayout, ParameterPanel } from "../src/api";
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
const choice = (etsId: string, value: string, row: number, column: number): ParameterField => ({
  ...base(etsId, "pairs", "Fictional pairs"), kind: "Restriction", value, cell: { row, column },
  text: `Fictional ${etsId}`, enumOptions: [{ value: "0", text: "Off", language: null }, { value: "1", text: "Toggle", language: null }],
});
const pairs: ParameterGroupLayout = { groupKey: "pairs", version: 1, declaredLayout: "Table", status: "grid",
  rows: [{ id: "R-1", text: "Pair A", textParameterRefId: null, collapseIfEmpty: null },
    { id: "R-2", text: "Pair B", textParameterRefId: null, collapseIfEmpty: null }],
  columns: [{ id: "C-1", text: null, textParameterRefId: null, width: "30%" },
    { id: "C-2", text: "Left side", textParameterRefId: null, width: "35%" },
    { id: "C-3", text: "Right side", textParameterRefId: null, width: "35%" }],
  labels: [{ row: 1, column: 1, text: "Operation" }, { row: 2, column: 1, text: "Operation" }],
  issues: [{ kind: "unshownContent", element: "Button", reference: "B-1", cell: "1,1" }] };
const legacy: ParameterGroupLayout = { ...pairs, groupKey: "legacy", status: "unknownLayout", declaredLayout: "Carousel", labels: [], issues: [] };

async function intercept(page: Page, language: "en" | "de") {
  const state = { unexpected: [] as string[], posts: [] as { etsId: string; raw: string }[], projectOpen: false, aLeft: "0" };
  const panel = (): ParameterPanel => ({ programId: "A-FICTIONAL", sourceLanguage: "en", tree: null, stale: [], diagnostics: [],
    sections: [{ scope: null, groupLayouts: [pairs, legacy], fields: [
      choice("A-left", state.aLeft, 1, 2), choice("A-right", "0", 1, 3),
      { ...base("B-left", "pairs", "Fictional pairs"), cell: { row: 2, column: 2 } },
      { ...base("B-right", "pairs", "Fictional pairs"), cell: { row: 2, column: 3 } },
      { ...base("Loose", "pairs", "Fictional pairs"), kind: "Text", value: "Fictional note" },
      base("Delay", "timing", "Fictional timing"),
      base("Variant", "legacy", "Fictional carousel"),
    ] }] });
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
    else if (path === "/api/device-system2/status" && method === "GET") body = null;
    else if (path === "/api/devices" && method === "GET") body = { schemaVersion: 1, serverIncarnation: "matrix-browser", snapshotRevision: 1,
      devices: [{ id: 1, device, productRef: "Fictional-P-1", resolution: "Resolved", manufacturerId: "Fictional-M-1",
        manufacturerName: "Fictional Devices", productText: "Fictional keypad", orderNumber: "FK-4", productTextLanguage: null, productSourceLanguage: null,
        system: { state: "family", mask: 0x07b0, family: "systemB" } }] };
    else if (path === "/api/device/1" && method === "GET") body = { ...device, com_objects: [], product: { resolution: "NoReference", product_ref: null, program_ref: null, catalog: null } };
    else if (path === "/api/device/1/parameters" && method === "GET") body = panel();
    else if (path === "/api/device/1/parameters" && method === "POST") {
      const write = route.request().postDataJSON() as { etsId: string; raw: string };
      state.posts.push({ etsId: write.etsId, raw: write.raw });
      if (write.etsId === "A-left") state.aLeft = write.raw;
      body = { ...panel(), tree: { ...tree, snapshot_revision: 2, can_undo: true, is_modified: true } };
    } else if (path === "/api/bus/monitor/telegrams" && method === "GET") body = { sessionId: 1, serverIncarnation: "matrix-browser", contextStatus: "current",
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
  test(`${language} declared parameter grid at ${width}px`, async ({ page }) => {
    const m = language === "de" ? de : en;
    await page.setViewportSize({ width, height: 1000 });
    const state = await intercept(page, language);
    await openParameters(page, language);
    const workspace = page.locator(".parameter-workspace");
    const area = (label: string) => workspace.locator(".parameter-workspace-nav button").filter({ hasText: label });
    const matrix = workspace.locator('.parameter-workspace-rows[data-layout="matrix"]');
    const note = workspace.locator(".parameter-workspace-layout-note");
    await expect(workspace.locator(".parameter-workspace-nav button")).toHaveCount(3);
    await expect(workspace.locator('.parameter-workspace-nav [aria-current="page"]')).toHaveCount(0);
    await expect(matrix).toHaveCount(0);

    await area("Fictional pairs").click();
    await expect(matrix).toBeVisible();
    await expect(note.first()).toContainText("Table");
    for (const caption of ["Left side", "Right side", "Pair A", "Pair B", "Operation"]) {
      await expect(matrix.locator(".parameter-matrix-caption").filter({ hasText: caption }).first()).toBeVisible();
    }
    await expect(matrix.locator(".parameter-matrix-unplaced")).toHaveText(m["parameters.matrix.unplaced"]);
    await expect(workspace.locator(".parameter-matrix-issues")).not.toHaveAttribute("open", "");
    const box = async (etsId: string) => (await matrix.locator(`[data-parameter-row="${etsId}"]`).boundingBox())!;
    const [aLeft, aRight, bLeft, loose] = [await box("A-left"), await box("A-right"), await box("B-left"), await box("Loose")];
    expect(Math.abs(aLeft.y - aRight.y)).toBeLessThanOrEqual(1);
    expect(aRight.x).toBeGreaterThan(aLeft.x);
    expect(bLeft.y).toBeGreaterThan(aLeft.y);
    expect(Math.abs(bLeft.x - aLeft.x)).toBeLessThanOrEqual(1);
    expect(loose.y).toBeGreaterThan(bLeft.y);

    // The same editor elements survive navigation; the one write path is the existing row.
    const select = matrix.locator('[data-parameter-row="A-left"] select');
    await select.evaluate(node => node.setAttribute("data-retained-editor", "yes"));
    await area("Fictional timing").click();
    await expect(matrix).toHaveCount(0);
    await expect(note).toHaveText(m["parameters.matrix.noGrid"]);
    await expect(workspace.locator('[data-parameter-row="Delay"]')).toBeVisible();
    await area("Fictional carousel").click();
    await expect(matrix).toHaveCount(0);
    await expect(note).toContainText("Carousel");
    await area("Fictional pairs").click();
    await expect(select).toHaveAttribute("data-retained-editor", "yes");
    expect(state.posts).toEqual([]);
    await select.selectOption("1");
    await expect.poll(() => state.posts).toEqual([{ etsId: "A-left", raw: "1" }]);
    await expect(select).toHaveValue("1");
    await expect(select).toHaveAttribute("data-retained-editor", "yes");

    const geometry = await matrix.evaluate(node => { const rect = node.getBoundingClientRect();
      return { left: rect.left, right: rect.right, inner: innerWidth }; });
    expect(geometry.left).toBeGreaterThanOrEqual(0);
    expect(geometry.right).toBeLessThanOrEqual(geometry.inner + 1);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    if (process.env.KNX_MATRIX_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_MATRIX_CAPTURE_DIR, `parameter-matrix-${language}-${width}.png`), fullPage: true });
    expect(state.unexpected).toEqual([]);
  });
}
