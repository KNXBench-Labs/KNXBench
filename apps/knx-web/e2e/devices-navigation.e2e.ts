/** Devices navigation browser contracts with strictly intercepted, fictional API data. */
import { expect, test, type Page } from "@playwright/test";
import { join } from "node:path";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import type { DeviceNode } from "../src/bindings/DeviceNode";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";

const devices = (count: number): DeviceNode[] => Array.from({ length: count }, (_, index) => ({
  id: index + 1, name: `Actuator ${index + 1}`, address: index < 2 ? `1.1.${index === 0 ? 2 : 10}` : null,
  description: `Fictional device ${index + 1}`, com_object_count: 0,
}));
function fixtureTree(nodes: DeviceNode[]): ProjectTree {
  return { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
    group_address_style: "ThreeLevel", server_incarnation: "devices-browser", snapshot_revision: 1,
    installations: [{ id: 1, name: "House", topology: [{ id: 1, name: "Area", address: 1,
      lines: [{ id: 2, name: "Ground floor", address: 1, devices: nodes }] }], unassigned: [],
      buildings: [{ id: 3, name: "Kitchen", kind: "Room", children: [], devices: nodes.slice(0, 1) }], group_ranges: [],
      group_addresses: [{ id: 500, name: "Kitchen light", address: "1/1/1", range: null, dpts: [],
        links: [{ device_id: 1, device_name: "Actuator 1", device_address: "1.1.2", com_object_id: 20,
          com_object_number: 1, com_object_name: "Switch", direction: "Receive" }] }],
    }] };
}
async function intercept(page: Page, language: "en" | "de", count = 3, theme = "porcelain") {
  const nodes = devices(count); const tree = fixtureTree(nodes);
  const state = { tree, nodes, unexpected: [] as string[], requests: [] as string[], polls: 0, failCatalogue: false };
  await page.route("**/api/**", (route) => {
    const url = new URL(route.request().url()); const path = url.pathname; const method = route.request().method();
    state.requests.push(`${method} ${path}`);
    let body: unknown; let status = 200;
    if (path === "/api/auth/status" && method === "GET") body = { required: false, authenticated: true };
    else if (path === "/api/settings" && method === "GET") body = { schemaVersion: 1, status: "ok", settings: { uiLanguage: language, theme } };
    else if (path === "/api/project" && method === "GET") body = state.tree;
    else if (path === "/api/project/new" && method === "POST") body = state.tree;
    else if (path === "/api/project/load-progress" && method === "GET") body = null;
    else if (path === "/api/bus/discover" && method === "POST") body = { interfaces: [] };
    else if (path === "/api/product-languages" && method === "GET") body = [];
    else if (["/api/device-download/status", "/api/device-address/status"].includes(path) && method === "GET") { status = 404; body = { error: "no operation in this fixture" }; }
    else if (path === "/api/devices" && method === "GET") {
      if (state.failCatalogue) { status = 500; body = { error: "fixture product database unavailable" }; }
      else body = { schemaVersion: 1, serverIncarnation: "devices-browser", snapshotRevision: state.tree.snapshot_revision,
        devices: nodes.map((device) => ({ id: device.id, device, productRef: "Fictional-P-1", resolution: "Resolved",
          manufacturerId: "Fictional-M-1", manufacturerName: "Fictional Devices", productText: "Switch actuator", orderNumber: "SA-4",
          productTextLanguage: null, productSourceLanguage: null })) };
    } else if (/^\/api\/device\/\d+$/.test(path) && method === "GET") {
      const device = nodes.find((node) => node.id === Number(path.split("/").at(-1)));
      if (!device) { status = 400; body = { error: "device not found" }; }
      else body = { ...device, com_objects: [], product: { resolution: "NoReference", product_ref: null, program_ref: null, catalog: null } };
    } else if (/^\/api\/device\/\d+\/parameters$/.test(path) && method === "GET") body = { programId: null, sections: [], stale: [], diagnostics: [] };
    else if (path === "/api/bus/monitor/telegrams" && method === "GET") {
      state.polls++;
      body = { sessionId: 1, serverIncarnation: "devices-browser", contextStatus: "current", projectOpen: true,
        status: "active", nextSince: 1, droppedBefore: 0, telegrams: Number(url.searchParams.get("since")) === 0 ? [{
          seq: 1, timestamp: "2026-10-08T12:00:00Z", source: "1.1.2", destination: "1/1/1", destinationName: "Kitchen light",
          service: "GroupValueWrite", rawPayload: "01", decoded: null, control: null,
        }] : [] };
    } else if (path === "/api/device-readiness" && method === "GET") body = { devices: nodes.map((node) => ({
      address: node.address, name: node.name, programRef: "fictional", readiness: "untested", category: null, detail: null, steps: 1, octets: 1,
    })), counts: { untested: nodes.length } };
    else if (path === "/api/device-address/availability" && method === "GET") body = { startAvailable: false, reason: "Device-specific recovery not implemented" };
    else if (path === "/api/bus/scan/results" && method === "GET") body = { sessionId: 4, status: "completed", error: null,
      nextSince: 1, completedCount: 1, totalCount: 1, omittedAddresses: [], excludedAddresses: [],
      results: [{ address: "1.1.2", outcome: { kind: "occupied", maskVersion: 1793 } }] };
    else if (path === "/api/bus/scan/comparison" && method === "GET") body = { unexpected: [], missing: [], excludedInProject: [] };
    else if (achievementsFixtureAnswer(method, path) !== undefined) body = achievementsFixtureAnswer(method, path);
    else {
      state.unexpected.push(`${method} ${path}`); status = 404; body = { error: "strict devices fixture only" };
    }
    return route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
  });
  return state;
}
async function openProject(page: Page, language: "en" | "de") {
  const m = language === "en" ? en : de;
  await page.goto("/");
  await page.getByRole("button", { name: m["toolbar.newProject"], exact: true }).first().click();
  await page.getByRole("dialog", { name: m["newProject.title"], exact: true }).getByRole("button", { name: m["newProject.create"], exact: true }).click();
  await page.getByRole("dialog", { name: m["projectWizard.done.title"], exact: true }).getByRole("button", { name: m["projectWizard.done.close"], exact: true }).click();
}
async function openList(page: Page, language: "en" | "de") {
  const m = language === "en" ? en : de;
  await page.keyboard.press("Control+Shift+P");
  await page.getByRole("combobox").fill(m["workbench.devices"]);
  await page.getByRole("combobox").press("Enter");
  await expect(page.locator(".devices-workspace")).toBeVisible();
}
for (const language of ["en", "de"] as const) {
  const m = language === "en" ? en : de;
  for (const width of [1440, 400]) {
    test(`${language} device list/editor retains filter, sort, scroll and multi-selection at ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 850 });
      const theme = width === 1440 && language === "en" ? "graphite" : "porcelain";
      const state = await intercept(page, language, 3, theme); await openProject(page, language); await openList(page, language);
      await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
      await expect(page.locator(".devices-table tbody tr")).toHaveCount(3);
      await expect(page.locator(".devices-table")).toContainText("Fictional Devices");
      if (process.env.KNX_DEVICES_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_DEVICES_CAPTURE_DIR, `devices-${language}-${width}.png`) });
      await page.getByRole("searchbox", { name: m["devices.filter"] }).fill("Actuator");
      await page.locator(".devices-table th button").filter({ hasText: new RegExp(`^${m["workbench.name"]}`) }).click();
      await page.getByRole("checkbox", { name: m["devices.select"].replace("{name}", "Actuator 1"), exact: true }).check();
      const scroller = page.locator(".devices-table-scroll");
      await scroller.evaluate((node) => { node.scrollLeft = 180; });
      const scrollLeft = await scroller.evaluate((node) => node.scrollLeft);
      const before = state.requests.length;
      await page.locator(".devices-table-scroll").evaluate((node) => {
        node.addEventListener("click", () => { (window as unknown as { devicesReturnScroll: number }).devicesReturnScroll = node.scrollLeft; }, { once: true });
      });
      await page.locator(".devices-table tbody td:nth-child(3) button").filter({ hasText: /^Actuator 1$/ }).click();
      await expect(page.locator(".device-workspace h2")).toHaveText("Actuator 1");
      await expect(page.locator(".workbench-center .structure-workspace")).toHaveCount(0);
      await expect(page.locator(".workbench-center .device-workspace")).toHaveCount(1);
      await page.locator(".device-editor-navigation button").first().click();
      await expect(page.getByRole("searchbox", { name: m["devices.filter"] })).toHaveValue("Actuator");
      await expect(page.locator('.devices-table th[aria-sort="ascending"]')).toContainText(m["workbench.name"]);
      await expect(page.getByRole("checkbox", { name: m["devices.select"].replace("{name}", "Actuator 1"), exact: true })).toBeChecked();
      // Clicking a off-screen name can scroll its cell into view; capture the
      // actual post-click list position before it is hidden in the editor.
      expect(await scroller.evaluate((node) => node.scrollLeft)).toBe(await page.evaluate(() => (window as unknown as { devicesReturnScroll: number }).devicesReturnScroll));
      expect(scrollLeft).toBeGreaterThan(0);
      expect(state.requests.slice(before).every((request) => request.startsWith("GET ") || request === "POST /api/achievements/record")).toBe(true);
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth - innerWidth);
      const boxes = await page.locator(".workbench-panel-controls, .bulk-action-toolbar, .bulk-action-toolbar select, .bulk-action-toolbar button, .devices-toolbar, .workbench-center, .workbench-status").evaluateAll((nodes) => nodes.map((node) => {
        const box = node.getBoundingClientRect(); return { tag: node.tagName, cls: node.className, x: box.x, width: box.width, right: box.right };
      }));
      if (process.env.KNX_DEVICES_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_DEVICES_CAPTURE_DIR, `devices-selected-${language}-${width}.png`) });
      expect(overflow, JSON.stringify(boxes)).toBeLessThanOrEqual(1);
      expect(state.unexpected).toEqual([]);
    });
  }
  test(`${language} GA table and inspector device links share the central editor`, async ({ page }) => {
    const state = await intercept(page, language); await openProject(page, language);
    await page.getByRole("navigation", { name: m["workbench.navigation"], exact: true }).getByRole("button", { name: m["workbench.addresses"], exact: true }).click();
    await page.locator(".address-table .table-select").first().click();
    await expect(page.locator(".ga-linked-devices .device-link")).toHaveCount(1);
    await page.locator(".address-links-panel .device-link").click();
    await expect(page.locator(".device-workspace h2")).toHaveText("Actuator 1");
    await page.locator(".device-editor-navigation button").first().click();
    await page.locator(".ga-linked-devices .device-link").click();
    await expect(page.locator(".device-workspace h2")).toHaveText("Actuator 1");
    expect(state.unexpected).toEqual([]);
  });
}
test("monitor source navigation keeps capture/polling alive with no reconnect, stop or write", async ({ page }) => {
  const state = await intercept(page, "en"); await openProject(page, "en");
  await page.locator(".diagnostic-navigation").getByRole("button", { name: en["toolbar.busMonitor"], exact: true }).click();
  await expect(page.locator(".bus-monitor-table .device-link")).toHaveCount(1);
  const polls = state.polls; const before = state.requests.length;
  await page.locator(".bus-monitor-table .device-link").click();
  await expect(page.locator(".device-workspace h2")).toHaveText("Actuator 1");
  await expect.poll(() => state.polls).toBeGreaterThan(polls);
  await page.locator(".device-editor-navigation button").first().click();
  await expect(page.locator(".bus-monitor-table tbody tr")).toHaveCount(1);
  await expect(page.locator('.bus-monitor-table tr[aria-selected="true"]')).toHaveCount(0);
  expect(state.requests.slice(before).every((request) => request.startsWith("GET ") || request === "POST /api/achievements/record")).toBe(true);
  expect(state.requests.some((request) => /monitor\/(start|stop)|bus\/write/.test(request))).toBe(false);
  expect(state.unexpected).toEqual([]);
});
for (const panel of ["download", "address", "checks", "scan"] as const) {
  test(`${panel} links open the device editor without invoking its hardware operation`, async ({ page }) => {
    const state = await intercept(page, "en"); await openProject(page, "en");
    await page.locator(".diagnostic-navigation").getByRole("button", { name: en["toolbar.busMonitor"], exact: true }).click();
    const labels = { download: en["deviceDownload.title"], address: en["addressProgramming.tab"], checks: en["deviceChecks.tab"], scan: en["lineScan.title"] };
    await page.locator(".bus-diagnostics-tabs").getByRole("button", { name: labels[panel], exact: true }).click();
    if (panel === "download") await page.locator(".device-download-config select").selectOption("1.1.2");
    if (panel === "address") await page.locator('.address-programming-config input[name="address"]').fill("1.1.2");
    const selector = { download: ".device-download-panel .device-link", address: ".address-programming-panel .device-link",
      checks: ".device-checks-readiness .device-link", scan: ".line-scan-results .device-link" }[panel];
    await page.locator(selector).first().click(); await expect(page.locator(".device-workspace h2")).toHaveText("Actuator 1");
    expect(state.requests.some((request) => /^POST .*\/(?:start|plan|write|estimate|device-compare)$/.test(request))).toBe(false);
    expect(state.unexpected).toEqual([]);
  });
}
test("catalogue failures retain every project device and explain unavailable product data", async ({ page }) => {
  const state = await intercept(page, "en"); state.failCatalogue = true;
  await openProject(page, "en"); await openList(page, "en");
  await expect(page.locator(".devices-workspace [role=alert]")).toContainText("product database unavailable");
  await expect(page.locator(".devices-table tbody tr")).toHaveCount(3);
  expect(state.unexpected).toEqual([]);
});
test("measures a 5000-device production table and keeps the last device searchable", async ({ page }, info) => {
  test.setTimeout(120_000);
  const state = await intercept(page, "en", 5000); await openProject(page, "en");
  const started = Date.now(); await openList(page, "en");
  await expect(page.locator(".devices-table tbody tr")).toHaveCount(5000);
  const renderMs = Date.now() - started;
  const filterStarted = Date.now(); await page.getByRole("searchbox", { name: en["devices.filter"] }).fill("Actuator 5000");
  await expect(page.locator(".devices-table tbody tr")).toHaveCount(1);
  await expect(page.locator(".devices-table")).toContainText("Actuator 5000");
  await info.attach("devices-performance", { body: JSON.stringify({ devices: 5000, renderMs, filterMs: Date.now() - filterStarted }), contentType: "application/json" });
  expect(state.requests.filter((request) => request === "GET /api/devices")).toHaveLength(1);
  expect(state.unexpected).toEqual([]);
});
