/** WP-49.5 Q17: rename dialog focus returns to the tree row (menu path + F2 control). */
import { openExplorerBranches } from "./disclosure-fixture";
import { expect, test, type Page } from "@playwright/test";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import { messages as en } from "../src/messages/en";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { busActivityFixtureAnswer } from "./bus-activity-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";

const GA = { id: 7, name: "Fictional switch light", address: "1/2/3", range: null, dpts: [], links: [] };

const baseTree: ProjectTree = {
  schema_version: 13, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
  group_address_style: "ThreeLevel", server_incarnation: "rename-focus-fixture", project_incarnation: 1, snapshot_revision: 1,
  installations: [{ id: 1, name: "Fictional flat", topology: [], buildings: [], unassigned: [], group_ranges: [], group_addresses: [GA] }],
};

function makeTree(): ProjectTree { return structuredClone(baseTree); }

async function fixture(page: Page) {
  const unexpected: string[] = [];
  let tree: ProjectTree = makeTree();
  await page.setViewportSize({ width: 1440, height: 900 });
  const appOrigin = new URL(String(test.info().project.use.baseURL)).origin;
  page.on("request", request => { if (new URL(request.url()).origin !== appOrigin) unexpected.push(request.url()); });
  await page.route("**/api/**", route => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    const method = request.method();
    const ok = (body: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
    const achievements = achievementsFixtureAnswer(method, path);
    if (achievements !== undefined) return ok(achievements);
    const busActivity = busActivityFixtureAnswer(method, path);
    if (busActivity !== undefined) return ok(busActivity);
    if (path === "/api/auth/status") return ok({ required: false, authenticated: true });
    if (path === "/api/settings" && method === "GET") return ok({ conditionalPatchVersion: 1, schemaVersion: 1, status: "ok", settings: {
      uiLanguage: "en", theme: "porcelain", motionLevel: "off", releaseChecksEnabled: false,
      onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.9" } } });
    if (path === "/api/settings" && method === "PUT") return ok({ conditionalPatchVersion: 1, schemaVersion: 1, status: "ok", settings: request.postDataJSON().settings });
    if (path === "/api/version") return ok({ version: "0.1.0-alpha.9+rename-focus-fixture" });
    if (path === "/api/project" && method === "GET") return ok(tree);
    if (path === "/api/project/load-progress") return ok(null);
    if (path === "/api/project/status-metrics" && method === "GET") return ok(statusMetricsFixtureBody(tree));
    if (path === "/api/log") return ok([]);
    if (path === "/api/bus/discover" && method === "POST") return ok({ interfaces: [] });
    if (["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items"].includes(path)) return ok([]);
    if (["/api/device-download/status", "/api/device-download/batch/status", "/api/device-address/status"].includes(path)) {
      return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"no fixture operation"}' });
    }
    if (path === `/api/group-addresses/${GA.id}/name` && method === "PATCH") {
      const sent = request.postDataJSON() as { name: string };
      tree = { ...tree, is_modified: true, snapshot_revision: (tree.snapshot_revision ?? 1) + 1,
        installations: tree.installations.map(inst => ({ ...inst, group_addresses: inst.group_addresses.map(ga => ga.id === GA.id ? { ...ga, name: sent.name } : ga) })) };
      return ok(tree);
    }
    unexpected.push(`${method} ${path}`);
    return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"strict rename-focus fixture"}' });
  });
  await page.goto("/?view=editor");
  await openExplorerBranches(page);
  return { unexpected };
}

const row = (page: Page) => page.locator('[data-rename-kind="group_address"][data-rename-id="7"]').first();

test("WP-49.5 Q17: closing the rename dialog opened from the context menu returns focus to the tree row", async ({ page }) => {
  const f = await fixture(page);
  await expect(row(page)).toBeVisible();
  await row(page).click({ button: "right" });
  const item = page.getByRole("menuitem", { name: en["rename.title"], exact: false });
  await expect(item).toBeVisible();
  await item.click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();
  // Cancel path: Escape closes the dialog, focus lands on the row, not on <body>.
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
  await expect(row(page)).toBeFocused();
  expect(f.unexpected).toEqual([]);
});

test("WP-49.5 Q17: F2 control — closing the rename dialog returns focus to the row; a successful rename keeps that too", async ({ page }) => {
  const f = await fixture(page);
  await expect(row(page)).toBeVisible();
  await row(page).click();
  await expect(row(page)).toBeFocused();
  await page.keyboard.press("F2");
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
  await expect(row(page)).toBeFocused();
  // Successful rename through the menu path: focus still returns to the (renamed) row.
  await row(page).click({ button: "right" });
  await page.getByRole("menuitem", { name: en["rename.title"], exact: false }).click();
  await expect(dialog).toBeVisible();
  const input = page.locator('[role="dialog"] input[data-rename-name]');
  await input.fill("Renamed fictional light");
  await input.press("Enter");
  await expect(dialog).toHaveCount(0);
  const renamed = page.locator('[data-rename-kind="group_address"][data-rename-id="7"]').first();
  await expect(renamed).toContainText("Renamed fictional light");
  await expect(renamed).toBeFocused();
  expect(f.unexpected).toEqual([]);
});
