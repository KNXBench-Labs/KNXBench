/** Issue #20: a range heading's + creates a group address in that range, next free or typed. */
import { openNavigationSections } from "./disclosure-fixture";
import { expect, test, type Page } from "@playwright/test";
import { join } from "node:path";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import type { InstallationNode } from "../src/bindings/InstallationNode";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";

type Language = "en" | "de";
const installation = (): InstallationNode => ({
  id: 1, name: "Fictional house", unassigned: [], topology: [], buildings: [],
  group_ranges: [
    { id: 10, name: "Fictional lights", start: "1/0/0", end: "1/7/255", parent: null },
    { id: 11, name: "Fictional ground floor with a rather long name", start: "1/0/0", end: "1/0/255", parent: 10 },
  ],
  group_addresses: [
    { id: 101, name: "Hall", address: "1/0/1", range: 11, dpts: [], links: [] },
    { id: 102, name: "Kitchen", address: "1/0/2", range: 11, dpts: [], links: [] },
    { id: 104, name: "Porch", address: "1/0/4", range: 11, dpts: [], links: [] },
  ],
});
const project = (): ProjectTree => ({ schema_version: 15, errors: 0, warnings: 0, can_undo: false, can_redo: false,
  is_modified: false, group_address_style: "ThreeLevel", server_incarnation: "range-create", project_incarnation: 1,
  snapshot_revision: 1, installations: [installation()] });

async function intercept(page: Page, language: Language, theme: string) {
  const state = { tree: project(), unexpected: [] as string[], created: [] as unknown[] };
  let projectOpen = false; // Like the real server: no project until the test creates one (ADR-0127).
  await page.route("**/api/**", (route) => {
    const url = new URL(route.request().url()); const path = url.pathname; const method = route.request().method();
    let body: unknown; let status = 200;
    if (path === "/api/auth/status" && method === "GET") body = { required: false, authenticated: true };
    else if (path === "/api/settings" && method === "GET") body = { schemaVersion: 1, status: "ok", settings: { uiLanguage: language, theme, motionLevel: "off" } };
    else if (path === "/api/settings" && method === "PUT") body = { schemaVersion: 1, status: "ok", settings: { uiLanguage: language, theme, ...route.request().postDataJSON().settings } };
    else if (path === "/api/project/new" && method === "POST") { projectOpen = true; body = state.tree; }
    else if (path === "/api/project" && method === "GET") { if (projectOpen) body = state.tree; else { status = 400; body = { error: "no project open" }; } }
    else if (path === "/api/project/load-progress" && method === "GET") body = null;
    else if (path === "/api/project/status-metrics" && method === "GET") body = statusMetricsFixtureBody(state.tree);
    else if (path === "/api/bus/discover" && method === "POST") body = { interfaces: [] };
    else if (path === "/api/product-languages" && method === "GET") body = [];
    else if (path === "/api/group-addresses" && method === "POST") {
      const request = route.request().postDataJSON();
      state.created.push(request);
      state.tree = { ...state.tree, snapshot_revision: 2, can_undo: true, is_modified: true };
      state.tree.installations[0].group_addresses.push({ id: 103, name: request.name, address: request.address, range: request.rangeId ?? null, dpts: [], links: [] });
      body = state.tree;
    }
    else if (achievementsFixtureAnswer(method, path) !== undefined) body = achievementsFixtureAnswer(method, path);
    else { state.unexpected.push(`${method} ${path}`); status = 404; body = { error: "strict range-create fixture only" }; }
    return route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
  });
  return state;
}

const capture = async (page: Page, name: string) => {
  if (process.env.KNX_RANGE_CREATE_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_RANGE_CREATE_CAPTURE_DIR, `${name}.png`) });
};
const noPageOverflow = async (page: Page) =>
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  const theme = (language === "en") === (width === 1440) ? "graphite" : "porcelain";
  test(`${language} creates the next free address from a range heading at ${width}px (${theme})`, async ({ page }) => {
    const m = language === "en" ? en : de;
    await page.setViewportSize({ width, height: 900 });
    const state = await intercept(page, language, theme);
    await page.goto("/");
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    await page.getByRole("button", { name: m["toolbar.newProject"], exact: true }).first().click();
    await page.getByRole("dialog", { name: m["newProject.title"], exact: true }).getByRole("button", { name: m["newProject.create"], exact: true }).click();
    await page.getByRole("dialog", { name: m["projectWizard.done.title"], exact: true }).getByRole("button", { name: m["projectWizard.done.close"], exact: true }).click();
    await openNavigationSections(page);
    const dismiss = page.getByRole("button", { name: m["toast.dismiss"], exact: true });
    for (let i = 0; i < 20 && await dismiss.count() > 0; i++) await dismiss.first().click({ timeout: 2000 }).catch(() => {});
    const nav = page.getByRole("navigation", { name: m["workbench.navigation"], exact: true });
    if (!(await nav.isVisible())) await page.getByRole("button", { name: m["workbench.navigation"], exact: true }).click();
    await nav.getByRole("button", { name: m["workbench.addresses"], exact: true }).click();

    await page.locator('[data-ga-expand="range:10:0"]').click();
    const add = page.locator('[data-ga-range-add="11"]');
    await expect(add).toBeVisible();
    await expect(add).toHaveAccessibleName(m["gaWorkspace.newInRange.add"].replace("{range}", "Fictional ground floor with a rather long name"));
    const heading = (await add.locator("xpath=ancestor::tr").boundingBox())!;
    const plus = (await add.boundingBox())!;
    expect(plus.y).toBeGreaterThanOrEqual(heading.y - 1);
    expect(plus.y + plus.height).toBeLessThanOrEqual(heading.y + heading.height + 1);
    await capture(page, `range-create-tree-${language}-${width}-${theme}`);
    await add.click();

    const dialog = page.getByRole("dialog", { name: m["gaWorkspace.newInRange.title"], exact: true });
    await expect(dialog).toBeVisible();
    await expect(dialog.locator("[data-ga-next-free]")).toHaveText("1/0/3");
    await expect(dialog.getByRole("radio", { name: new RegExp(`^${m["gaWorkspace.newInRange.next"]}`) })).toBeChecked();
    const box = (await dialog.boundingBox())!;
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(width);
    await dialog.getByLabel(m["workbench.name"], { exact: true }).fill("Dining");
    await capture(page, `range-create-dialog-${language}-${width}-${theme}`);
    await noPageOverflow(page);
    await dialog.getByLabel(m["workbench.name"], { exact: true }).press("Enter");

    await expect.poll(() => state.created.length).toBe(1);
    expect(state.created).toEqual([{ name: "Dining", address: "1/0/3", rangeId: 11 }]);
    await expect(dialog).toHaveCount(0);
    await expect(page.locator('[data-ga-select="103"]'), "the new address is visible in its range").toBeVisible();
    await capture(page, `range-create-done-${language}-${width}-${theme}`);
    await noPageOverflow(page);
    expect(state.unexpected).toEqual([]);
  });
}
