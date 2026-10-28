/** P3 browser contract: pending-programming chips, filter, explorer dot and marking by hand. */
import { openExplorerBranches, openNavigationSections } from "./disclosure-fixture";
import { expect, test, type Page } from "@playwright/test";
import { join } from "node:path";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import type { DeviceNode } from "../src/bindings/DeviceNode";
import type { PendingProgramming } from "../src/bindings/PendingProgramming";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { fulfillNoProjectOpen } from "./server-project-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";

const pending = (address: boolean, application: boolean, parameters: boolean, groupAddresses: boolean): PendingProgramming =>
  ({ address, application, parameters, group_addresses: groupAddresses });
const CLEAN = pending(false, false, false, false);

function nodes(first: PendingProgramming): DeviceNode[] {
  return [
    { id: 1, name: "Dimmer kitchen", address: "1.1.2", description: "Fictional dimmer", com_object_count: 0, pending_programming: first },
    { id: 2, name: "Blind living room", address: "1.1.3", description: "Fictional blind", com_object_count: 0, pending_programming: CLEAN },
    { id: 3, name: "Sensor hall", address: "1.1.4", description: "Fictional sensor", com_object_count: 0,
      pending_programming: pending(true, true, true, true) },
  ];
}
function fixtureTree(first: PendingProgramming, revision: number): ProjectTree {
  return { schema_version: 11, errors: 0, warnings: 0, can_undo: revision > 1, can_redo: false, is_modified: revision > 1,
    group_address_style: "ThreeLevel", server_incarnation: "pending-browser", project_incarnation: 3, snapshot_revision: revision,
    installations: [{ id: 1, name: "House", topology: [{ id: 1, name: "Area", address: 1,
      lines: [{ id: 2, name: "Ground floor", address: 1, devices: nodes(first) }] }], unassigned: [],
      buildings: [], group_ranges: [], group_addresses: [] }] };
}
async function intercept(page: Page, language: "en" | "de", theme = "porcelain") {
  const state = { tree: fixtureTree(pending(true, false, true, false), 1), unexpected: [] as string[], marks: [] as unknown[], system2: null as unknown, system2Reads: 0, failProjectReads: 0 };
  let projectOpen = false;
  await page.route("**/api/**", (route) => {
    const url = new URL(route.request().url()); const path = url.pathname; const method = route.request().method();
    let body: unknown; let status = 200;
    if (path === "/api/auth/status" && method === "GET") body = { required: false, authenticated: true };
    else if (path === "/api/settings" && method === "GET") body = { schemaVersion: 1, status: "ok", settings: { uiLanguage: language, theme } };
    else if (path === "/api/project" && method === "GET") {
      if (!projectOpen) return fulfillNoProjectOpen(route);
      if (state.failProjectReads > 0) { state.failProjectReads -= 1; status = 503; body = { error: "synthetic project reload failure" }; }
      else body = state.tree;
    }
    else if (path === "/api/project/new" && method === "POST") { projectOpen = true; body = state.tree; }
    else if (path === "/api/project/load-progress" && method === "GET") body = null;
    else if (path === "/api/project/status-metrics" && method === "GET") body = statusMetricsFixtureBody(state.tree);
    else if (path === "/api/bus/discover" && method === "POST") body = { interfaces: [] };
    else if (path === "/api/product-languages" && method === "GET") body = [];
    else if (["/api/device-download/status", "/api/device-address/status"].includes(path) && method === "GET") { status = 404; body = { error: "no operation in this fixture" }; }
    else if (path === "/api/device-system2/status" && method === "GET") { state.system2Reads += 1; body = state.system2; }
    else if (path === "/api/devices" && method === "GET") {
      const devices = state.tree.installations[0].topology[0].lines[0].devices;
      body = { schemaVersion: 1, serverIncarnation: "pending-browser", snapshotRevision: state.tree.snapshot_revision,
        devices: devices.map((device) => ({ id: device.id, device, productRef: null, resolution: "NoReference",
          manufacturerId: null, manufacturerName: null, productText: null, orderNumber: null,
          productTextLanguage: null, productSourceLanguage: null, system: { state: "noProduct" } })) };
    } else if (/^\/api\/device\/\d+$/.test(path) && method === "GET") {
      const device = state.tree.installations[0].topology[0].lines[0].devices.find((node) => node.id === Number(path.split("/").at(-1)));
      if (!device) { status = 400; body = { error: "device not found" }; }
      else body = { id: device.id, name: device.name, description: device.description, address: device.address, com_objects: [],
        product: { resolution: "NoReference", product_ref: null, program_ref: null, catalog: null } };
    } else if (/^\/api\/device\/\d+\/parameters$/.test(path) && method === "GET") body = { programId: null, sections: [], stale: [], diagnostics: [] };
    else if (path === "/api/device/1/programming" && method === "POST") {
      const request = route.request().postDataJSON() as { programmed: boolean; snapshotRevision: number };
      state.marks.push(request);
      if (request.snapshotRevision !== state.tree.snapshot_revision) { status = 409; body = { error: "stale", kind: "programmingStale" }; }
      else { state.tree = fixtureTree(request.programmed ? CLEAN : pending(true, true, true, true), state.tree.snapshot_revision + 1); body = state.tree; }
    } else if (achievementsFixtureAnswer(method, path) !== undefined) body = achievementsFixtureAnswer(method, path);
    else { state.unexpected.push(`${method} ${path}`); status = 404; body = { error: "strict pending-programming fixture only" }; }
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
  await openNavigationSections(page);
  await openExplorerBranches(page);
}
async function openList(page: Page, language: "en" | "de") {
  const m = language === "en" ? en : de;
  await page.keyboard.press("Control+Shift+P");
  await page.getByRole("combobox").fill(m["workbench.devices"]);
  await page.getByRole("combobox").press("Enter");
  await expect(page.locator(".devices-workspace")).toBeVisible();
}
async function capture(page: Page, name: string) {
  if (process.env.KNX_PENDING_CAPTURE_DIR) {
    const mode = test.info().config.configFile?.endsWith("playwright.devices.config.ts") ? "production" : "development";
    await page.screenshot({ path: join(process.env.KNX_PENDING_CAPTURE_DIR, mode, `${name}.png`) });
  }
}
const parts = (m: Record<string, string>, keys: (keyof typeof en)[], language: string) =>
  new Intl.ListFormat(language, { type: "conjunction" }).format(keys.map((key) => m[key]));

for (const language of ["en", "de"] as const) {
  const m = language === "en" ? en : de;
  for (const width of [1440, 400]) {
    test(`${language} device list shows pending parts and narrows to them at ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 850 });
      // One case in the dark theme, so the chips are seen on both grounds.
      const theme = width === 1440 && language === "en" ? "graphite" : "porcelain";
      const state = await intercept(page, language, theme); await openProject(page, language); await openList(page, language);
      await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
      // Visible headings may be compact ("Prog."); the column is identified by its accessible name.
      const headers = await page.locator(".devices-table thead th").evaluateAll((cells) =>
        cells.map((cell) => cell.querySelector("[aria-label]")?.getAttribute("aria-label") ?? cell.textContent ?? ""));
      expect(headers.indexOf(m["devices.programming"])).toBe(headers.findIndex((text) => text.startsWith(m["workbench.name"])) + 1);
      const row = (name: string) => page.locator(".devices-table tbody tr").filter({ hasText: name });
      await expect(row("Dimmer kitchen").locator(".programming-chip")).toHaveText([
        m["programming.part.address.short"], m["programming.part.parameters.short"]]);
      await expect(row("Blind living room").locator(".programming-chip")).toHaveCount(0);
      await expect(row("Sensor hall").locator(".programming-chip")).toHaveCount(4);
      await expect(row("Dimmer kitchen").locator(".programming-chips")).toHaveAttribute("title",
        m["devices.programmingPending"].replace("{parts}", parts(m, ["programming.part.address", "programming.part.parameters"], language)));
      await capture(page, `devices-${language}-${width}`);
      await page.getByRole("checkbox", { name: m["devices.pendingOnly"], exact: true }).check();
      await expect(page.locator(".devices-table tbody tr")).toHaveCount(2);
      await expect(row("Blind living room")).toHaveCount(0);
      const box = await page.locator(".devices-toolbar").boundingBox();
      expect(box!.x).toBeGreaterThanOrEqual(0);
      expect(box!.x + box!.width).toBeLessThanOrEqual(width + 1);
      expect(await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth)).toBeLessThanOrEqual(1);
      await capture(page, `devices-pending-only-${language}-${width}`);
      expect(state.unexpected).toEqual([]);
      expect(state.marks).toEqual([]);
    });
  }
  test(`${language} explorer dot and marking by hand at 1440px`, async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 850 });
    const state = await intercept(page, language); await openProject(page, language);
    const explorerRow = (label: string) => page.locator(".tree-row").filter({ has: page.locator(".tree-label", { hasText: label }) });
    const dot = explorerRow("1.1.2 Dimmer kitchen").locator(".tree-pending-dot");
    await expect(dot).toHaveAttribute("aria-label",
      m["explorer.pendingProgramming"].replace("{parts}", parts(m, ["programming.part.address", "programming.part.parameters"], language)));
    await expect(explorerRow("1.1.3 Blind living room").locator(".tree-pending-dot")).toHaveCount(0);
    await page.locator(".tree-label", { hasText: "1.1.2 Dimmer kitchen" }).click();
    const section = page.locator(".device-programming");
    await expect(section.locator('[data-part="parameters"] dd')).toHaveText(m["programming.pending"]);
    await expect(section.locator('[data-part="application"] dd')).toHaveText(m["programming.programmed"]);
    await capture(page, `inspector-${language}-1440`);
    await section.getByRole("button", { name: m["programming.markProgrammed"], exact: true }).click();
    await expect(dot).toHaveCount(0);
    await expect(section.locator('[data-part="parameters"] dd')).toHaveText(m["programming.programmed"]);
    await expect(section.getByRole("button", { name: m["programming.markProgrammed"], exact: true })).toBeDisabled();
    expect(state.marks).toEqual([{ programmed: true, serverIncarnation: "pending-browser", projectIncarnation: 3, snapshotRevision: 1 }]);
    await capture(page, `inspector-marked-${language}-1440`);
    expect(state.unexpected).toEqual([]);
  });
}

// Issue #31: a retained System-2 run explains, beside the untouched marks, what is provable about the device.
const failedRun = {
  schemaVersion: 4, failureDiagnostics: [], finalStepFailures: [], downloadResult: "transferCompleteVerificationUnfinished", operationId: "a".repeat(32),
  binding: { serverIncarnation: "pending-browser", projectIncarnation: "3", projectRevision: "1", deviceId: 1 }, address: "1.1.2", deviceName: "Dimmer kitchen",
  state: "failed", completedSteps: 26, totalSteps: 27, backupAvailable: true, recoveryPhrase: null, downloadPhrase: null, releasePhrase: null,
  message: null, refusal: null, mutationMayHaveStarted: true, prestateDiffers: false, pointerObservations: [], programVersionObservation: null,
  prestateUnreadableAllocations: [],
  restoreAttempted: false, downloadCompletedSteps: 26, downloadTotalSteps: 27, projectSettlement: "persisted",
};
for (const language of ["en", "de"] as const) {
  const m = language === "en" ? en : de;
  for (const width of [1440, 400]) {
    test(`${language} device list explains an unverified System-2 run without clearing pending marks at ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 850 });
      const state = await intercept(page, language); state.system2 = failedRun;
      await openProject(page, language); await openList(page, language);
      const row = (name: string) => page.locator(".devices-table tbody tr").filter({ hasText: name });
      const marker = row("Dimmer kitchen").locator("[data-system2-result]");
      await expect(marker).toHaveCount(1);
      const sentence = m["devices.system2.title"].replace("{result}", m["system2.result.transferCompleteVerificationUnfinished"]);
      await expect(marker).toHaveAttribute("aria-label", sentence);
      await expect(marker).toHaveAttribute("title", sentence);
      await expect(marker).toHaveAttribute("data-system2-result", "transferCompleteVerificationUnfinished");
      // The project's own marks stay; the run is never shown as a clean or loaded device.
      await expect(row("Dimmer kitchen").locator(".programming-chip")).toHaveText([
        m["programming.part.address.short"], m["programming.part.parameters.short"]]);
      await expect(row("Dimmer kitchen").locator('.programming-chips[data-state="pending"]')).toHaveCount(1);
      await expect(page.locator("[data-system2-result]")).toHaveCount(1);
      // Sorting and the pending filter keep their meaning.
      await page.getByRole("checkbox", { name: m["devices.pendingOnly"], exact: true }).check();
      await expect(page.locator(".devices-table tbody tr")).toHaveCount(2);
      await expect(marker).toHaveCount(1);
      expect(await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth)).toBeLessThanOrEqual(1);
      await capture(page, `system2-result-${language}-${width}`);
      expect(state.system2Reads).toBe(1);
      expect(state.unexpected).toEqual([]);
    });
  }
  test(`${language} device list ignores a System-2 run bound to another project`, async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 850 });
    const state = await intercept(page, language);
    state.system2 = { ...failedRun, binding: { ...failedRun.binding, projectIncarnation: "9" } };
    await openProject(page, language); await openList(page, language);
    await expect(page.locator(".devices-table tbody tr")).toHaveCount(3);
    await expect(page.locator("[data-system2-result]")).toHaveCount(0);
    expect(state.unexpected).toEqual([]);
  });
}

for (const language of ["en", "de"] as const) {
  const m = language === "en" ? en : de;
  test(`${language} stale programming marks stay visible and retryable outside the device list`, async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 850 });
    const state = await intercept(page, language);
    state.system2 = { ...failedRun, state: "running", downloadResult: "inProgress", projectSettlement: "notRecorded" };
    await openProject(page, language); await openList(page, language);
    await page.locator(".tree-label", { hasText: "1.1.2 Dimmer kitchen" }).click();
    await expect(page.locator(".devices-workspace")).toBeHidden();
    const hiddenReads = state.system2Reads;
    await expect.poll(() => state.system2Reads, { timeout: 15_000 }).toBeGreaterThan(hiddenReads + 1);
    state.tree = fixtureTree(pending(true, true, true, true), 2);
    state.failProjectReads = 1;
    state.system2 = failedRun;
    const notice = page.locator(".workbench-center > [data-system2-belief='stale']");
    await expect(notice).toBeVisible();
    await expect(notice).toContainText(m["system2.belief.stale"]);
    await capture(page, `system2-root-stale-${language}`);
    await notice.getByRole("button", { name: m["system2.belief.retry"], exact: true }).click();
    await expect(page.locator("[data-system2-belief]")).toHaveCount(0);
    await openList(page, language);
    await expect(page.locator(".devices-table tbody tr").filter({ hasText: "Dimmer kitchen" }).locator(".programming-chip")).toHaveCount(4);
    expect(state.unexpected).toEqual([]);
  });
  test(`${language} a System-2 run that ends while the table is hidden is settled, reloaded and shown on return`, async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 850 });
    const state = await intercept(page, language);
    state.system2 = { ...failedRun, state: "running", downloadResult: "inProgress", projectSettlement: "notRecorded" };
    await openProject(page, language); await openList(page, language);
    const kitchen = () => page.locator(".devices-table tbody tr").filter({ hasText: "Dimmer kitchen" });
    await expect(kitchen().locator(".programming-chip")).toHaveCount(2);
    await expect(page.locator("[data-system2-result]")).toHaveCount(0);
    // Leave the table (open the editor); the application-level owner keeps following the run.
    await page.locator(".tree-label", { hasText: "1.1.2 Dimmer kitchen" }).click();
    await expect(page.locator(".devices-workspace")).toBeHidden();
    // With the table hidden, only the application-level owner can still be reading the status.
    const hiddenReads = state.system2Reads;
    await expect.poll(() => state.system2Reads, { timeout: 15_000 }).toBeGreaterThan(hiddenReads + 1);
    // The run ends; the server has settled the marks (all memory parts pending now).
    state.tree = fixtureTree(pending(true, true, true, true), 2);
    state.system2 = { ...failedRun, projectSettlement: "persisted" };
    const endedAt = state.system2Reads;
    await expect.poll(() => state.system2Reads, { timeout: 15_000 }).toBeGreaterThan(endedAt);
    await openList(page, language);
    await expect(page.locator("[data-system2-belief]")).toHaveCount(0);
    await expect(kitchen().locator("[data-system2-result]")).toHaveAttribute("data-system2-result", "transferCompleteVerificationUnfinished");
    await expect(kitchen().locator(".programming-chip")).toHaveCount(4);
    expect(state.unexpected).toEqual([]);
    void m;
  });
}
