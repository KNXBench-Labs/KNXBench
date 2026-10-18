/** P4: real App gestures with strictly intercepted, fictional device comparisons only. */
import { expect, test, type Page } from "@playwright/test";
import { join } from "node:path";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import type { DeviceNode } from "../src/bindings/DeviceNode";
import type { ComparisonBinding, BoundDeviceComparison } from "../src/deviceComparisons";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { fulfillNoProjectOpen } from "./server-project-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";

const gateway = "192.0.2.10:3671";
function tree(): ProjectTree {
  const devices: DeviceNode[] = [1, 2, 3, 4, 5, 6].map((id) => ({ id, name: `Fictional device ${id}`,
    address: id === 6 ? null : `1.1.${id}`, description: null, com_object_count: 0 }));
  return { schema_version: 11, errors: 0, warnings: 0, can_undo: true, can_redo: false, is_modified: false,
    group_address_style: "ThreeLevel", server_incarnation: "comparison-browser", project_incarnation: 3,
    snapshot_revision: 1, comparison_revisions: { 1: 7, 2: 7, 3: 7, 4: 7, 5: 7, 6: 7, 7: 7 },
    installations: [{ id: 1, name: "House", topology: [{ id: 1, name: "Area", address: 1, lines: [
      { id: 1, name: "Ground floor", address: 1, devices },
      { id: 2, name: "Upper floor", address: 2, devices: [{ id: 7, name: "Outside filter", address: "1.2.7", description: null, com_object_count: 0 }] },
    ] }], unassigned: [], buildings: [], group_ranges: [], group_addresses: [] }] };
}
function result(binding: ComparisonBinding, address: string): BoundDeviceComparison {
  const different = binding.deviceId === 2;
  return { ...binding, address, deviceName: `Fictional device ${binding.deviceId}`, programId: "Fictional application",
    written: false, partial: false, mask: 0x0701, manufacturer: 0x83, loadStates: [{ machine: "ApplicationProgram", state: "Loaded" }],
    octets: 3, differingOctets: different ? 1 : 0, same: !different, outdated: false,
    changes: different ? [{ address: 0x4003, segment: "Addresses", project: [3], device: [4] }] : [],
    decoded: [
      { kind: "groupAddress", label: "1", address: 0x4003, project: "2/2/3", device: different ? "2/2/4" : "2/2/3", reason: null },
      { kind: "association", label: "0", address: 0x4101, project: "TSAP 1 → 2/2/3 · object 7", device: `TSAP 1 → ${different ? "2/2/4" : "2/2/3"} · object 7`, reason: null },
      { kind: "parameter", label: "Delay · R-Delay · bit 5 / 9", address: 0x4400, project: "235", device: different ? "345" : "235", reason: null },
      { kind: "parameter", label: "Room text · R-Text · bit 0 / 48", address: 0x4410, project: "Grüß", device: different ? "5 € ok" : "Grüß", reason: null },
      { kind: "parameter", label: "Unknown encoding · R-Unknown · bit 0 / 16", address: 0x4420, project: "12", device: null, reason: "parameterNotDecoded" },
      { kind: "raw", label: "", address: 0x4003, project: "12 03 A5", device: different ? "12 04 A5" : "12 03 A5", reason: null },
    ] };
}
async function intercept(page: Page, language: "en" | "de", mode: "normal" | "busy" | "held" = "normal") {
  const state = { tree: tree(), unexpected: [] as string[], requests: [] as { address: string; gateway: string; binding: ComparisonBinding }[],
    release: () => {} };
  let opened = false;
  await page.route("**/api/**", async (route) => {
    const path = new URL(route.request().url()).pathname; const method = route.request().method();
    let body: unknown; let status = 200;
    if (path === "/api/auth/status" && method === "GET") body = { required: false, authenticated: true };
    else if (path === "/api/settings" && method === "GET") body = { schemaVersion: 1, status: "ok", settings: { uiLanguage: language, theme: language === "en" ? "graphite" : "porcelain" } };
    else if (path === "/api/project" && method === "GET") { if (!opened) return fulfillNoProjectOpen(route); body = state.tree; }
    else if (path === "/api/project/new" && method === "POST") { opened = true; body = state.tree; }
    else if (path === "/api/project/load-progress" && method === "GET") body = null;
    else if (path === "/api/project/status-metrics" && method === "GET") body = statusMetricsFixtureBody(state.tree);
    else if (path === "/api/bus/discover" && method === "POST") body = { interfaces: [] };
    else if (path === "/api/product-languages" && method === "GET") body = [];
    else if (["/api/device-download/status", "/api/device-address/status"].includes(path) && method === "GET") { status = 404; body = { error: "no active operation" }; }
    else if (path === "/api/devices" && method === "GET") body = { schemaVersion: 1, serverIncarnation: "comparison-browser",
      snapshotRevision: state.tree.snapshot_revision, devices: [] };
    else if (path === "/api/undo" && method === "POST") {
      state.tree = { ...state.tree, snapshot_revision: 2, comparison_revisions: { ...state.tree.comparison_revisions, 1: 8 } };
      body = state.tree;
    } else if (path === "/api/device-compare" && method === "POST") {
      const request = route.request().postDataJSON() as (typeof state.requests)[number]; state.requests.push(request);
      if (mode === "held") await new Promise<void>((resolve) => { state.release = resolve; });
      if (mode === "busy") { status = 409; body = { error: "Stop the bus monitor first", kind: "gatewayBusy",
        holder: { kind: "busMonitor", sessionId: 1, gateway, state: "active" } }; }
      else if (request.binding.deviceId >= 3) { status = 502; body = { error: "Fictional device diagnostic", kind:
        request.binding.deviceId === 3 ? "deviceCompareWrongApplication" : request.binding.deviceId === 4 ? "deviceCompareNoAnswer" : "deviceCompareNotComparable" }; }
      else body = result(request.binding, request.address);
    } else if (achievementsFixtureAnswer(method, path) !== undefined) body = achievementsFixtureAnswer(method, path);
    else { state.unexpected.push(`${method} ${path}`); status = 404; body = { error: "strict comparison fixture" }; }
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
async function capture(page: Page, name: string) {
  if (process.env.KNX_COMPARE_CAPTURE_DIR) {
    const mode = test.info().config.configFile?.endsWith("playwright.devices.config.ts") ? "production" : "development";
    await page.screenshot({ path: join(process.env.KNX_COMPARE_CAPTURE_DIR, mode, `${name}.png`) });
  }
}
for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  const m = language === "en" ? en : de;
  test(`${language} comparison of filtered devices at ${width}px is confirmed, decoded and session-bound`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    const state = await intercept(page, language); await open(page, language);
    expect(state.requests).toEqual([]); await expect(page.locator(".comparison-tag")).toHaveCount(0);
    await page.getByRole("searchbox", { name: m["devices.filter"], exact: true }).fill("Ground floor");
    await expect(page.locator(".devices-table tbody tr")).toHaveCount(6);
    await page.getByRole("button", { name: m["compare.all"], exact: true }).click();
    const dialog = page.getByRole("dialog", { name: m["compare.title"], exact: true });
    await expect(dialog.locator(".comparison-targets li")).toHaveCount(6); expect(state.requests).toEqual([]);
    await capture(page, `review-${language}-${width}`);
    await dialog.locator(".comparison-gateway").fill(gateway);
    await dialog.getByRole("button", { name: m["compare.confirm"], exact: true }).click();
    await expect(dialog.getByRole("button", { name: m["compare.close"], exact: true })).toBeVisible();
    expect(state.requests.map((r) => r.binding.deviceId)).toEqual([1, 2, 3, 4, 5]);
    expect(state.requests.every((r) => r.gateway === gateway && r.binding.comparisonRevision === 7 && !Object.hasOwn(r, "confirmation"))).toBe(true);
    for (const [id, tag] of [[1, "synced"], [2, "different"], [3, "wrongApplication"], [4, "noAnswer"], [5, "notComparable"], [6, "notComparable"]] as const) {
      await expect(dialog.locator(".comparison-results li").filter({ hasText: `Fictional device ${id}` }).locator(".comparison-tag")).toHaveText(m[`compare.tag.${tag}`]);
    }
    await dialog.locator(".comparison-results li").filter({ hasText: "Fictional device 2" }).locator("button").first().click();
    await expect(dialog.locator(".comparison-decoded")).toContainText("2/2/4");
    await expect(dialog.locator(".comparison-decoded")).toContainText("12 04 A5");
    const parameter = dialog.locator(".comparison-decoded tbody tr").filter({ hasText: "R-Delay" });
    await expect(parameter.locator("td").nth(0)).toHaveText("235");
    await expect(parameter.locator("td").nth(1)).toHaveText("345");
    await expect(dialog.locator(".comparison-decoded")).toContainText("5 € ok");
    await expect(dialog.locator(".comparison-decoded .comparison-reason")).toHaveText(m["compare.parameterNotDecoded"]);
    await expect(dialog.locator(".comparison-tag-synced")).toHaveCSS("background-color", await dialog.evaluate((el) => getComputedStyle(el).backgroundColor));
    const box = await dialog.boundingBox(); expect(box!.x).toBeGreaterThanOrEqual(0); expect(box!.x + box!.width).toBeLessThanOrEqual(width + 1);
    expect(box!.y + box!.height).toBeLessThanOrEqual(884);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth)).toBeLessThanOrEqual(1);
    await dialog.locator(".comparison-decoded").scrollIntoViewIfNeeded();
    await capture(page, `results-${language}-${width}`);
    if (width === 400) {
      const scroll = dialog.locator(".comparison-table-scroll");
      await scroll.evaluate((el) => { el.scrollLeft = el.scrollWidth - el.clientWidth; });
      const deviceCell = dialog.locator(".comparison-decoded tbody tr").first().locator("td").nth(1);
      const cell = await deviceCell.boundingBox(); const region = await scroll.boundingBox();
      expect(cell!.x).toBeGreaterThanOrEqual(region!.x - 1);
      expect(cell!.x + cell!.width).toBeLessThanOrEqual(region!.x + region!.width + 1);
      await capture(page, `device-values-${language}-${width}`);
    }
    await dialog.getByRole("button", { name: m["compare.close"], exact: true }).click();
    await page.keyboard.press("Control+z");
    const first = page.locator(".devices-table tbody tr").filter({ hasText: "Fictional device 1" });
    await expect(first.locator(".comparison-tag")).toHaveText(m["compare.tag.outdated"]);
    await first.locator(".comparison-tag").scrollIntoViewIfNeeded();
    await capture(page, `outdated-${language}-${width}`);
    expect(state.unexpected).toEqual([]);
  });
}
test("cancellation drains one in-flight comparison, leaves the rest untagged and permits another review", async ({ page }) => {
  const state = await intercept(page, "en", "held"); await open(page, "en");
  await page.getByRole("button", { name: en["compare.all"], exact: true }).click();
  const dialog = page.getByRole("dialog", { name: en["compare.title"], exact: true });
  await dialog.locator(".comparison-gateway").fill(gateway);
  await dialog.getByRole("button", { name: en["compare.confirm"], exact: true }).click();
  await expect.poll(() => state.requests.length).toBe(1);
  await dialog.getByRole("button", { name: en["deviceChecks.cancel"], exact: true }).click();
  await expect(dialog).toContainText(en["compare.stopping"]); state.release();
  await expect(dialog.getByRole("button", { name: en["compare.close"], exact: true })).toBeVisible();
  expect(state.requests).toHaveLength(1); await expect(dialog.locator(".comparison-tag")).toHaveCount(2);
  await expect(dialog.locator(".comparison-results li").filter({ hasText: "Fictional device 2" }).locator(".comparison-tag")).toHaveCount(0);
  await dialog.getByRole("button", { name: en["compare.close"], exact: true }).click();
  await page.locator(".devices-table tbody tr").first().getByRole("button", { name: en["compare.one"], exact: true }).click();
  await expect(dialog.locator(".comparison-targets li")).toHaveCount(1);
  expect(state.requests).toHaveLength(1); expect(state.unexpected).toEqual([]);
});
test("a busy bus monitor refuses the run clearly without marking any device", async ({ page }) => {
  const state = await intercept(page, "en", "busy"); await open(page, "en");
  await page.getByRole("button", { name: en["compare.all"], exact: true }).click();
  const dialog = page.getByRole("dialog", { name: en["compare.title"], exact: true });
  await dialog.locator(".comparison-gateway").fill(gateway); await dialog.getByRole("button", { name: en["compare.confirm"], exact: true }).click();
  await expect(dialog.locator("[role=alert]")).toContainText(/monitor/i);
  // The sorted list starts with its unaddressed device: that offline refusal
  // is retained, but no live device acquires a comparison tag on gateway refusal.
  await expect(dialog.locator(".comparison-tag")).toHaveCount(1);
  await expect(dialog.locator(".comparison-results li").filter({ hasText: "Fictional device 1" }).locator(".comparison-tag")).toHaveCount(0);
  expect(state.requests).toHaveLength(1); expect(state.unexpected).toEqual([]);
});
