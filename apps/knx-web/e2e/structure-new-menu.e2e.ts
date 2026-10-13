/** Issues #12/#15: one `+ New` menu per structure page, grouped per installation. */
import { expect, test, type Page } from "@playwright/test";
import { join } from "node:path";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import type { InstallationNode } from "../src/bindings/InstallationNode";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";

type Language = "en" | "de";
const installation = (id: number, name: string): InstallationNode => ({
  id, name, unassigned: [], group_addresses: [],
  topology: [{ id: id * 10, name: `Fictional area ${id}`, address: id, lines: [{ id: id * 10 + 1, name: `Fictional line ${id}`, address: 1, devices: [] }] }],
  buildings: [{ id: id * 100, name: `Fictional building ${id}`, kind: "Building", devices: [], children: [] }],
  group_ranges: [{ id: id * 1000, name: `Fictional main ${id}`, start: `${id}/0/0`, end: `${id}/7/255`, parent: null }],
});
const project = (): ProjectTree => ({ schema_version: 15, errors: 0, warnings: 0, can_undo: false, can_redo: false,
  is_modified: false, group_address_style: "ThreeLevel", server_incarnation: "new-menu", project_incarnation: 1, snapshot_revision: 1,
  installations: [installation(1, "Fictional house"), installation(2, "Fictional annex with a long name")] });

async function intercept(page: Page, language: Language, theme: string) {
  const state = { tree: project(), unexpected: [] as string[], created: [] as unknown[] };
  await page.route("**/api/**", (route) => {
    const url = new URL(route.request().url()); const path = url.pathname; const method = route.request().method();
    let body: unknown; let status = 200;
    if (path === "/api/auth/status" && method === "GET") body = { required: false, authenticated: true };
    else if (path === "/api/settings" && method === "GET") body = { schemaVersion: 1, status: "ok", settings: { uiLanguage: language, theme, motionLevel: "off" } };
    else if (path === "/api/settings" && method === "PUT") body = { schemaVersion: 1, status: "ok", settings: { uiLanguage: language, theme, ...route.request().postDataJSON().settings } };
    else if ((path === "/api/project" && method === "GET") || (path === "/api/project/new" && method === "POST")) body = state.tree;
    else if (path === "/api/project/load-progress" && method === "GET") body = null;
    else if (path === "/api/bus/discover" && method === "POST") body = { interfaces: [] };
    else if (path === "/api/product-languages" && method === "GET") body = [];
    else if (path === "/api/group-addresses" && method === "POST") {
      const request = route.request().postDataJSON();
      state.created.push(request);
      const target = state.tree.installations.find((item) => item.id === request.installationId)!;
      target.group_addresses.push({ id: 900, name: request.name, address: request.address, range: request.rangeId ?? null, dpts: [], links: [] });
      body = state.tree;
    }
    else if (achievementsFixtureAnswer(method, path) !== undefined) body = achievementsFixtureAnswer(method, path);
    else { state.unexpected.push(`${method} ${path}`); status = 404; body = { error: "strict new-menu fixture only" }; }
    return route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
  });
  return state;
}

async function openPage(page: Page, m: typeof en | typeof de, target: "workbench.buildings" | "workbench.topology" | "workbench.addresses") {
  const nav = page.getByRole("navigation", { name: m["workbench.navigation"], exact: true });
  if (!(await nav.isVisible())) await page.getByRole("button", { name: m["workbench.navigation"], exact: true }).click();
  await nav.getByRole("button", { name: m[target], exact: true }).click();
  await expect(page.locator(".structure-workspace")).toBeVisible();
}
const capture = async (page: Page, name: string) => {
  if (process.env.KNX_NEW_MENU_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_NEW_MENU_CAPTURE_DIR, `${name}.png`) });
};
const noPageOverflow = async (page: Page) =>
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  // Each theme in each language and at each width.
  const theme = (language === "en") === (width === 1440) ? "graphite" : "porcelain";
  test(`${language} one + New menu per structure page at ${width}px (${theme})`, async ({ page }) => {
    const m = language === "en" ? en : de;
    await page.setViewportSize({ width, height: 900 });
    const state = await intercept(page, language, theme);
    await page.goto("/");
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    await page.getByRole("button", { name: m["toolbar.newProject"], exact: true }).first().click();
    await page.getByRole("dialog", { name: m["newProject.title"], exact: true }).getByRole("button", { name: m["newProject.create"], exact: true }).click();
    await page.getByRole("dialog", { name: m["projectWizard.done.title"], exact: true }).getByRole("button", { name: m["projectWizard.done.close"], exact: true }).click();
    const dismiss = page.getByRole("button", { name: m["toast.dismiss"], exact: true });
    for (let i = 0; i < 20 && await dismiss.count() > 0; i++) await dismiss.first().click({ timeout: 2000 }).catch(() => {});

    // Buildings: the overview shows only the structure; everything to create is in one menu.
    await openPage(page, m, "workbench.buildings");
    await expect(page.locator(".installation-diagram [data-structure-create], details.structure-create")).toHaveCount(0);
    const menu = page.locator(".structure-new-menu");
    await menu.locator("summary").click();
    const content = menu.locator(".structure-new-menu-content");
    await expect(content.locator(".structure-new-group-heading")).toHaveText(["Fictional house", "Fictional annex with a long name"]);
    await expect(content.locator('[data-create-entry="device"]')).toHaveText(m["structure.addDevice"]);
    await expect(content.locator('[data-create-entry="building-root"]')).toHaveCount(2);
    const box = (await content.boundingBox())!;
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(width);
    await capture(page, `new-menu-buildings-${language}-${width}-${theme}`);
    await noPageOverflow(page);
    await content.locator('[data-create-entry="site-root"][data-installation-id="2"]').click();
    const site = page.locator('[data-structure-create="site-root"]');
    await expect(site.locator("h2")).toHaveText(m["structure.addSite"]);
    await expect(site.locator(".structure-create-target")).toHaveText("Fictional annex with a long name");
    await site.locator(".structure-create-close").click();
    await expect(site).toHaveCount(0);

    // Group addresses (#15): an address can be created right here, in the installation you pick.
    await openPage(page, m, "workbench.addresses");
    await menu.locator("summary").click();
    await expect(content.locator('[data-create-entry="group-address"]')).toHaveCount(2);
    await content.locator('[data-create-entry="group-address"][data-installation-id="2"]').click();
    const form = page.locator('[data-structure-create="group-address"]');
    await expect(form.locator("h2")).toHaveText(m["structure.addGroupAddress"]);
    await form.getByLabel(m["workbench.address"], { exact: true }).fill("2/0/7");
    await form.getByLabel(m["workbench.name"], { exact: true }).fill("Annex blind");
    await capture(page, `new-menu-address-${language}-${width}-${theme}`);
    await noPageOverflow(page);
    await form.locator("ul button").click();
    await expect.poll(() => state.created.length).toBe(1);
    expect(state.created).toEqual([{ name: "Annex blind", address: "2/0/7", installationId: 2 }]);
    await expect(form.getByLabel(m["workbench.name"], { exact: true }), "ready for the next address").toHaveValue("");

    // Topology: the same menu; Escape closes the form.
    await openPage(page, m, "workbench.topology");
    await expect(form).toHaveCount(0);
    await menu.locator("summary").click();
    await content.locator('[data-create-entry="area"][data-installation-id="1"]').click();
    const area = page.locator('[data-structure-create="area"]');
    await area.locator("input").first().focus();
    await page.keyboard.press("Escape");
    await expect(area).toHaveCount(0);
    expect(state.unexpected).toEqual([]);
  });
}
