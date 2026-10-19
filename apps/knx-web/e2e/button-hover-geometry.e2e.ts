/** Issue #14: hovering a button never scales it; Save As rows keep the filename input still. */
import { expect, test, type Page } from "@playwright/test";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { fulfillNoProjectOpen } from "./server-project-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";

type Catalogue = { readonly [K in keyof typeof en]: string };
const FILES = Array.from({ length: 14 }, (_, i) => ({
  name: `fictional-${String(i + 1).padStart(2, "0")}-single-family-home-with-a-rather-long-name.knxdb`, is_dir: false,
}));

async function serve(page: Page, language: "en" | "de", motionLevel: "off" | "full") {
  const tree: ProjectTree = {
    schema_version: 15, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
    group_address_style: "ThreeLevel", server_incarnation: "hover-fixture", project_incarnation: 1, snapshot_revision: 1,
    installations: [{ id: 0, name: "Fictional flat", topology: [], buildings: [], unassigned: [], group_addresses: [], group_ranges: [] }],
  };
  const unexpected: string[] = [];
  // Like the real server: no project until the test creates one (ADR-0127).
  let projectOpen = false;
  await page.route("https://api.github.com/**", (route) => { unexpected.push(route.request().url()); return route.abort(); });
  await page.route("**/api/**", (route) => {
    const path = new URL(route.request().url()).pathname;
    const method = route.request().method();
    const ok = (body: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
    const achievements = achievementsFixtureAnswer(method, path);
    if (achievements !== undefined) return ok(achievements);
    if (path === "/api/auth/status") return ok({ required: false, authenticated: true });
    if (path === "/api/settings" && method === "GET") return ok({ schemaVersion: 1, status: "ok", settings: {
      uiLanguage: language, theme: "porcelain", motionLevel, releaseChecksEnabled: false, achievementsEnabled: "false",
      onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.11" } } });
    if (path === "/api/settings" && method === "PUT") return ok({ schemaVersion: 1, status: "ok", settings: route.request().postDataJSON().settings });
    if (path === "/api/project/new" && method === "POST") { projectOpen = true; return ok(tree); }
    if (path === "/api/project" && method === "GET") return projectOpen ? ok(tree) : fulfillNoProjectOpen(route);
    if (path === "/api/project/status-metrics" && method === "GET") return ok(statusMetricsFixtureBody(tree));
    if (path === "/api/fs/list") return ok(FILES);
    if (path === "/api/bus/discover" && method === "POST") return ok({ interfaces: [] });
    if (method === "GET" && ["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items"].includes(path)) return ok([]);
    unexpected.push(`${method} ${path}`);
    return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"offline fixture only"}' });
  });
  return unexpected;
}

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) for (const motionLevel of ["off", "full"] as const) {
  test(`hovering Save As rows moves nothing (${language}, ${width}px, motion ${motionLevel})`, { tag: "@exclusive" }, async ({ page }) => {
    const m: Catalogue = language === "de" ? de : en;
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    const unexpected = await serve(page, language, motionLevel);
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/");
    await page.getByRole("button", { name: m["toolbar.newProject"], exact: true }).click();
    await page.getByRole("dialog").getByRole("button", { name: m["newProject.create"], exact: true }).click();
    await page.getByRole("dialog").getByRole("button", { name: m["projectWizard.done.close"], exact: true }).click();
    await page.locator(".file-menu summary").click();
    await page.locator(".file-menu-content").getByRole("menuitem", { name: m["toolbar.saveAs"], exact: true }).click();
    const dialog = page.getByRole("dialog", { name: m["fsPicker.saveAs"], exact: true });
    const list = dialog.locator(".fs-picker-list");
    const filename = dialog.locator(`input[placeholder="${m["fsPicker.filenamePlaceholder"]}"]`);
    await expect(list.locator("button")).toHaveCount(FILES.length);
    const before = await filename.boundingBox();
    expect(before).not.toBeNull();
    const rows = list.locator("button");
    for (let index = 0; index < FILES.length; index += 1) {
      const row = rows.nth(index);
      await row.scrollIntoViewIfNeeded();
      const settledBefore = await filename.boundingBox();
      await row.hover();
      // The hover transition, if any, would have finished well inside this.
      await page.waitForTimeout(motionLevel === "full" ? 250 : 20);
      expect(await row.evaluate((element) => getComputedStyle(element).transform), FILES[index].name).toBe("none");
      const geometry = await list.evaluate((element) => ({ overflow: element.scrollWidth - element.clientWidth }));
      expect(geometry.overflow, FILES[index].name).toBeLessThanOrEqual(0);
      expect(await filename.boundingBox(), FILES[index].name).toEqual(settledBefore);
    }
    if (process.env.KNX_HOVER_CAPTURE_DIR && motionLevel === "full") {
      await rows.nth(1).hover();
      await page.screenshot({ path: `${process.env.KNX_HOVER_CAPTURE_DIR}/save-as-hover-${language}-${width}.png` });
    }
    // The fix is app-wide: an ordinary dialog button does not grow either.
    const cancel = dialog.getByRole("button", { name: m["fsPicker.cancel"], exact: true });
    await cancel.hover();
    await page.waitForTimeout(motionLevel === "full" ? 250 : 20);
    expect(await cancel.evaluate((element) => getComputedStyle(element).transform)).toBe("none");
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    expect(unexpected).toEqual([]);
    expect(errors).toEqual([]);
  });
}
