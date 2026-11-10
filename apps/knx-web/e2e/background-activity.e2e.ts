/** Exercises delayed activities in the real built App with strict, fully intercepted producers. */
import { expect, test, type Page } from "@playwright/test";
import { join } from "node:path";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { busActivityFixtureAnswer } from "./bus-activity-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";
import { openNavigationSections } from "./disclosure-fixture";
import { catalogueJobsFixture } from "./catalogue-jobs-fixture";
import type { PublicRecord } from "../src/catalogueJobsTypes";

const captureDirectory = process.env.KNX_ACTIVITY_CAPTURE_DIR;
async function capture(page: Page, name: string) {
  const mode = test.info().config.configFile?.endsWith("playwright.task-status.config.ts") ? "production" : "development";
  if (captureDirectory) await page.screenshot({ path: join(captureDirectory, `${mode}-${name}.png`) });
}
const tree: ProjectTree = {
  schema_version: 13, server_incarnation: "activity-browser", project_incarnation: 1, snapshot_revision: 1,
  errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false, group_address_style: "ThreeLevel",
  installations: [{ id: 1, name: "Fictional activity project", topology: [], buildings: [], unassigned: [], group_ranges: [], group_addresses: [] }],
};
const records = (kind: "preview" | "result"): PublicRecord[] => kind === "preview"
  ? [{ ordinal: "0", kind: "previewSummary", packageAlreadyInstalled: false, wrapper: false, packages: "1", programs: "0",
    productsAdded: "0", hardware2ProgramsAdded: "0", idConflicts: "0", growthCliffPossible: true, withheld: [] }]
  : [{ ordinal: "0", kind: "resultSummary", packages: "1", cataloguePageBytes: "3145728", nextJobInputLimitExceeded: false, withheld: [] }];

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} ${width}: real project open and retained catalogue work share the accessible footer`, async ({ page }) => {
    const m = language === "en" ? en : de;
    const unexpected: string[] = []; const pageErrors: string[] = [];
    let token: string | null = null; let loaded = false; let saveAttempts = 0;
    let activeTree = tree;
    let finishCompare: (() => Promise<void>) | undefined;
    let finishOpen: (() => Promise<void>) | undefined;
    let finishUpload: (() => Promise<void>) | undefined;
    const jobs = catalogueJobsFixture({ records, backup: "products.sqlite.backup-1760270400-ab12ab12ab12.sqlite" });
    page.on("pageerror", error => pageErrors.push(error.message));
    await page.setViewportSize({ width, height: 900 });
    await page.clock.install({ time: new Date("2026-11-10T12:00:00Z") });
    await page.clock.pauseAt(new Date("2026-11-10T12:00:01Z"));
    await page.emulateMedia({ reducedMotion: "no-preference", forcedColors: "none" });
    await page.route("https://api.github.com/**", route => { unexpected.push(route.request().url()); return route.abort(); });
    await page.route("**/api/**", route => {
      const request = route.request(); const path = new URL(request.url()).pathname; const method = request.method();
      const ok = (body: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
      if (method === "PUT" && /^\/api\/catalog\/update\/jobs\/[0-9a-f]+\/upload$/.test(path)) {
        finishUpload = () => jobs.answer(route)!;
        return;
      }
      const jobAnswer = jobs.answer(route); if (jobAnswer) return jobAnswer;
      const achievements = achievementsFixtureAnswer(method, path); if (achievements !== undefined) return ok(achievements);
      const bus = busActivityFixtureAnswer(method, path); if (bus !== undefined) return ok(bus);
      if (path === "/api/auth/status" && method === "GET") return ok({ required: false, authenticated: true });
      if (path === "/api/settings" && method === "GET") return ok({ conditionalPatchVersion: 1, schemaVersion: 1, status: "ok", settings: {
        uiLanguage: language, theme: width === 400 ? "porcelain" : "graphite", motionLevel: "standard", releaseChecksEnabled: false,
        onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.20" },
      } });
      if (path === "/api/settings" && method === "PUT") return ok({ conditionalPatchVersion: 1, schemaVersion: 1, status: "ok", settings: request.postDataJSON().settings });
      if (path === "/api/version" && method === "GET") return ok({ version: "0.0.0-activity-fixture" });
      if (path === "/api/bus/discover" && method === "POST") return ok({ interfaces: [] });
      if (path === "/api/project/save" && method === "POST") {
        saveAttempts++;
        return saveAttempts === 1
          ? route.fulfill({ status: 500, contentType: "application/json", body: '{"error":"Fictional save failure"}' })
          : ok({});
      }
      if (path === "/api/device-system2/status" && method === "GET") return ok(null);
      if (path === "/api/devices" && method === "GET") return ok({ schemaVersion: 1, serverIncarnation: activeTree.server_incarnation, snapshotRevision: activeTree.snapshot_revision, devices: [] });
      if (path === "/api/device-compare" && method === "POST") {
        finishCompare = () => route.fulfill({ status: 502, contentType: "application/json", body: '{"kind":"deviceCompareNoAnswer","error":"Fictional comparison stopped"}' });
        return;
      }
      if (path === "/api/project" && method === "GET") return loaded ? ok({ ...activeTree, has_store_path: true })
        : route.fulfill({ status: 400, contentType: "application/json", body: '{"error":"no project open"}' });
      if (path === "/api/project/status-metrics" && method === "GET") return ok(statusMetricsFixtureBody(tree));
      if (path === "/api/project/journal" && method === "GET") return ok({ state: "saved" });
      if (path === "/api/project/load-progress" && method === "GET") return ok(token === null ? null : {
        operationId: 1, clientToken: token, kind: "open", source: "fictional-activity.knxdb", phase: "loadStoredProject",
        completed: null, total: null, status: loaded ? "succeeded" : "running", error: null,
      });
      if (path === "/api/fs/list" && method === "GET") return ok([{ name: "fictional-activity.knxdb", is_dir: false }]);
      if (path === "/api/project/open" && method === "POST") {
        token = request.postDataJSON().clientToken; expect(token).toBeTruthy();
        finishOpen = async () => { loaded = true; await ok(activeTree); };
        return;
      }
      if (method === "GET" && ["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items", "/api/log", "/api/session-log"].includes(path)) return ok([]);
      if (method === "GET" && ["/api/device-download/status", "/api/device-address/status"].includes(path))
        return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"no fixture operation"}' });
      unexpected.push(`${method} ${path}`);
      return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"unexpected fixture request"}' });
    });
    await page.goto("/?view=editor");
    await expect(page.locator("html")).toHaveAttribute("lang", language);
    await page.keyboard.press("Control+Shift+P");
    await page.getByRole("combobox").fill(m["toolbar.openNativeProject"]);
    await page.getByRole("option").filter({ hasText: m["toolbar.openNativeProject"] }).click();
    await page.getByRole("button", { name: "fictional-activity.knxdb", exact: true }).click();
    await expect.poll(() => Boolean(finishOpen)).toBe(true);
    const footer = page.locator(".workbench-status"); const summary = footer.locator(".activity-footer-summary");
    await expect(page.locator(".load-progress")).toBeVisible();
    await expect(summary).toBeEmpty();
    await page.clock.runFor(1_999); await expect(summary.locator(".task-status-text")).toHaveCount(0);
    await page.clock.runFor(1); await expect(summary).toContainText(m["backgroundActivity.kind.open"]);
    await expect(summary.locator(".task-status-text")).toHaveCount(1);
    await openNavigationSections(page);
    await page.locator(".workbench-navigation").getByRole("button", { name: m["workbench.catalog"], exact: true }).click();
    await page.locator('.catalog-workspace input[type="file"]').setInputFiles({ name: "fictional-activity.knxprod", mimeType: "application/zip", buffer: Buffer.from("fixture") });
    await page.getByRole("button", { name: m["catalogJob.start"], exact: true }).click();
    await expect.poll(() => Boolean(finishUpload)).toBe(true);
    // Poll updates at the deadline may re-arm a zero-delay reveal for the next fake-clock tick.
    await page.clock.runFor(2_001);
    const toggle = footer.locator(".activity-footer-toggle");
    await toggle.focus(); await page.keyboard.press("Tab"); await page.keyboard.press("Shift+Tab");
    await expect(toggle).toBeFocused();
    expect(await toggle.evaluate(element => element.matches(":focus-visible"))).toBe(true);
    expect(await toggle.evaluate(element => getComputedStyle(element).outlineStyle)).not.toBe("none");
    await page.keyboard.press("Enter");
    await expect(toggle).toHaveAttribute("aria-expanded", "true");
    const panel = page.locator(".activity-footer-panel");
    await expect(panel.locator('li[data-state="running"]')).toHaveCount(2);
    await expect(panel.getByRole("button")).toHaveText([m["backgroundActivity.details"], m["backgroundActivity.close"]]);
    await capture(page, `activity-${language}-${width}-concurrent`);
    const box = await panel.boundingBox(); expect(box!.x).toBeGreaterThanOrEqual(0); expect(box!.x + box!.width).toBeLessThanOrEqual(width);
    await page.keyboard.press("Escape"); await expect(toggle).toBeFocused();
    await expect(toggle).toHaveAttribute("aria-expanded", "false");
    // Complete project loading while the initiating catalogue view is still on screen: it unmounts.
    await finishOpen!(); await expect(page.locator(".catalog-workspace")).toHaveCount(0);
    await expect(summary).toContainText(m["backgroundActivity.kind.catalogue"]);
    await expect(footer.locator(".bus-connection")).toBeVisible();
    expect(await footer.evaluate(element => element.lastElementChild?.previousElementSibling?.classList.contains("bus-connection"))).toBe(true);
    // Reuse the existing shimmer and its actual reduced/off/forced-colour static fallback.
    const shimmer = summary.locator(".task-status-text");
    await expect.poll(() => shimmer.evaluate(element => getComputedStyle(element).animationName)).toBe("knx-task-text-shimmer");
    for (const mode of ["reduced", "off", "forced"] as const) {
      await page.emulateMedia({ reducedMotion: mode === "reduced" ? "reduce" : "no-preference", forcedColors: mode === "forced" ? "active" : "none" });
      await page.locator("html").evaluate((element, off) => element.setAttribute("data-motion-level", off ? "off" : "standard"), mode === "off");
      await expect.poll(() => shimmer.evaluate(element => getComputedStyle(element).animationName)).toBe("none");
      expect(await shimmer.evaluate(element => getComputedStyle(element).webkitTextFillColor)).not.toBe("rgba(0, 0, 0, 0)");
    }
    await page.emulateMedia({ reducedMotion: "no-preference", forcedColors: "none" });
    await page.locator("html").evaluate(element => element.setAttribute("data-motion-level", "standard"));
    await finishUpload!(); await expect(summary).toContainText(m["backgroundActivity.state.waiting"]);
    await expect(summary.locator(".task-status-text")).toHaveCount(0);
    await toggle.click();
    await panel.getByRole("button", { name: m["backgroundActivity.details"], exact: true }).click();
    const dialog = page.locator('[aria-labelledby="catalogue-root-job-title"]');
    await expect(dialog).toBeVisible();
    await dialog.locator('.catalogue-job-check input[type="checkbox"]').check();
    await dialog.getByRole("button", { name: m["catalogUpdate.apply"], exact: true }).click();
    await expect(dialog).toContainText(m["catalogJob.state.succeeded"]);
    await dialog.getByRole("button", { name: m["catalogJob.hide"], exact: true }).click();
    await expect(summary.locator(".task-status-text")).toHaveCount(0);
    await toggle.click();
    await expect(panel).toBeVisible();
    await capture(page, `activity-${language}-${width}-completed`);
    await page.clock.runFor(3_000); await expect(summary).toBeEmpty();
    await expect(panel.locator('li[data-state="succeeded"]')).toHaveCount(2);
    expect(jobs.creates).toHaveLength(1); expect(jobs.uploads).toHaveLength(1); expect(jobs.applies).toHaveLength(1); expect(jobs.disposes).toHaveLength(0);
    const geometry = await page.evaluate(() => ({ width: innerWidth, document: document.documentElement.scrollWidth }));
    expect(geometry.document).toBeLessThanOrEqual(geometry.width);
    // An acknowledged old error leaves the bounded recent set. Activate with real keyboard input.
    await page.keyboard.press("Escape");
    await expect(toggle).toHaveAttribute("aria-expanded", "false");
    const save = page.getByRole("button", { name: m["toolbar.save"], exact: true });
    await save.click();
    await expect(summary).toContainText(m["backgroundActivity.state.failed"]);
    await toggle.click(); await expect(panel).toBeVisible();
    for (let index = 0; index < 20; index++) {
      // On narrow screens the growing inventory legitimately overlays the toolbar.
      await toggle.click(); await expect(panel).toHaveCount(0);
      await save.click();
      await expect.poll(() => saveAttempts).toBe(index + 2);
      await toggle.click(); await expect(panel).toBeVisible();
      // Wait for the full readback, not just POST dispatch, before the next save.
      await expect(panel.locator('li[data-state="running"]')).toHaveCount(0);
    }
    await toggle.focus(); await page.keyboard.press("Escape");
    await expect(toggle).toHaveAttribute("aria-expanded", "false");
    await toggle.focus(); await page.keyboard.press("Tab"); await page.keyboard.press("Shift+Tab");
    await expect(toggle).toBeFocused(); await page.keyboard.press("Enter");
    await expect(panel.locator("li")).toHaveCount(21);
    const failed = panel.locator('li[data-state="failed"]');
    await expect(failed).toContainText("Fictional save failure");
    const ack = failed.getByRole("button", { name: m["backgroundActivity.acknowledge"], exact: true });
    await page.keyboard.press("Tab"); await expect(ack).toBeFocused();
    expect(await ack.evaluate(element => element.matches(":focus-visible"))).toBe(true);
    expect(await ack.evaluate(element => getComputedStyle(element).outlineStyle)).not.toBe("none");
    expect(await ack.evaluate(element => Number.parseFloat(getComputedStyle(element).outlineWidth))).toBeGreaterThan(0);
    await page.keyboard.press("Enter");
    await expect(failed).toHaveCount(0); await expect(panel.locator("li")).toHaveCount(20);
    await expect(toggle).toBeFocused(); await expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(await toggle.evaluate(element => element.matches(":focus-visible"))).toBe(true);
    expect(await toggle.evaluate(element => getComputedStyle(element).outlineStyle)).not.toBe("none");
    expect(await toggle.evaluate(element => Number.parseFloat(getComputedStyle(element).outlineWidth))).toBeGreaterThan(0);
    expect(saveAttempts).toBe(21);
    await page.keyboard.press("Escape");
    // The paused fixture clock also owns toast exit fallbacks; dismiss by keyboard, then let each owner settle.
    for (let index = 0; index < 30 && await page.locator(".toast-dismiss").count(); index += 1) {
      await page.locator(".toast-dismiss").first().focus(); await page.keyboard.press("Enter");
      await page.clock.runFor(1_000);
    }
    await expect(page.locator(".toast-stack")).toHaveCount(0);
    // Exercise the real Compare-all producer with many reviewed targets, not injected footer DOM.
    const devices = Array.from({ length: 500 }, (_, index) => ({ id: index + 1, name: `Controller ${String(index + 1).padStart(3, "0")}`,
      address: `1.${Math.floor(index / 250)}.${index % 250 + 1}`, description: null, com_object_count: 0 }));
    activeTree = { ...tree, project_incarnation: 2, snapshot_revision: 2,
      comparison_revisions: Object.fromEntries(devices.map(device => [device.id, 1])),
      installations: [{ ...tree.installations[0], name: "Large fictional installation", unassigned: devices }] };
    finishOpen = undefined;
    await page.keyboard.press("Control+Shift+P");
    await page.getByRole("combobox").fill(m["toolbar.openNativeProject"]);
    await page.getByRole("option").filter({ hasText: m["toolbar.openNativeProject"] }).click();
    await page.getByRole("button", { name: "fictional-activity.knxdb", exact: true }).click();
    await expect.poll(() => Boolean(finishOpen)).toBe(true); await finishOpen!();
    await openNavigationSections(page);
    await page.locator(".workbench-navigation").getByRole("button", { name: m["workbench.devices"], exact: true }).click();
    const beforeComparisonFooter = await footer.boundingBox();
    await page.getByRole("button", { name: m["compare.all"], exact: true }).click();
    const comparison = page.locator(".device-comparison-dialog");
    await comparison.locator(".comparison-gateway").fill("192.0.2.10:3671");
    await comparison.getByRole("button", { name: m["compare.confirm"], exact: true }).click();
    await expect.poll(() => Boolean(finishCompare)).toBe(true); await page.clock.runFor(2_001);
    await expect(summary).toContainText("Controller 001");
    await expect(summary).toContainText(language === "en" ? "499 more devices" : "499 weitere Geräte");
    await expect(summary).not.toContainText("Controller 500");
    const summaryBox = await summary.boundingBox(); expect(summaryBox!.height).toBeLessThanOrEqual(37);
    const comparisonFooter = await footer.boundingBox();
    expect(comparisonFooter!.height).toBeLessThanOrEqual(beforeComparisonFooter!.height + 37);
    // The established narrow shell scrolls; only the wide, fixed-height shell promises a viewport-pinned footer.
    if (width >= 1_100) for (const target of [footer, footer.locator(".bus-connection"), footer.locator(":scope > span:last-child")]) {
      const bounds = await target.boundingBox();
      expect(bounds!.y).toBeGreaterThanOrEqual(0); expect(bounds!.y + bounds!.height).toBeLessThanOrEqual(901);
    }
    await comparison.getByRole("button", { name: m["deviceChecks.cancel"], exact: true }).click();
    await finishCompare!();
    await comparison.getByRole("button", { name: m["compare.close"], exact: true }).click();
    await toggle.click(); await expect(panel).toBeVisible();
    const context = panel.locator('li[data-state="cancelled"] strong');
    await expect(context).toContainText("Controller 001"); await expect(context).toContainText("Controller 500");
    const inventoryBounds = await panel.boundingBox();
    expect(inventoryBounds!.y).toBeGreaterThanOrEqual(0); expect(inventoryBounds!.x + inventoryBounds!.width).toBeLessThanOrEqual(width);
    await capture(page, `activity-${language}-${width}-many-targets`);
    expect(unexpected).toEqual([]); expect(pageErrors).toEqual([]);
  });
}
