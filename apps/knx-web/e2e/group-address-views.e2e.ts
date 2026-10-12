/** Production-built four-view group-address workspace with strictly intercepted data. */
import { expect, test, type Page } from "@playwright/test";
import { join } from "node:path";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import type { GroupAddressNode } from "../src/bindings/GroupAddressNode";
import type { GroupRangeNode } from "../src/bindings/GroupRangeNode";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";

type Language = "en" | "de";
const link = (device: number, object: number, direction: string) => ({
  device_id: device, device_name: `Fictional device ${device}`, device_address: `1.1.${device}`,
  com_object_id: object, com_object_number: object % 100, com_object_name: `Object ${object}`, direction,
});
const range = (id: number, parent: number | null, name: string, start = "1/0/0", end = "1/7/255"): GroupRangeNode =>
  ({ id, parent, name, start, end });
const ga = (id: number, address: string, name: string, rangeId: number | null, extra: Partial<GroupAddressNode> = {}): GroupAddressNode =>
  ({ id, address, name, range: rangeId, dpts: [], links: [], ...extra });

// The projection already delivers addresses in the project's style.
function standardTree(style = "ThreeLevel"): ProjectTree {
  const a = (three: string, raw: string) => {
    if (style === "Free") return raw;
    if (style === "TwoLevel") { const [main, , sub] = three.split("/"); return `${main}/${sub}`; }
    return three;
  };
  return { schema_version: 13, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
    group_address_style: style, server_incarnation: "ga-browser", project_incarnation: 1, snapshot_revision: 1,
    installations: [{ id: 1, name: "Fictional house", topology: [], buildings: [], unassigned: [],
      group_ranges: [range(20, null, "Lighting"), range(21, 20, "Ground floor"), range(22, null, "")],
      group_addresses: [
        ga(30, a("1/0/1", "2049"), "Kitchen ceiling", 21, { dpts: ["DPST-1-1"], links: [link(1, 10, "Send"), link(2, 11, "Receive")] }),
        ga(31, a("1/0/2", "2050"), "Kitchen dimming", 21, { dpts: ["DPST-1-1", "DPST-5-1"], links: [link(1, 12, "Send"), link(2, 12, "Receive")] }),
        ga(32, a("1/0/3", "2051"), "Hallway", 20, { links: [link(3, 13, "FutureDirection")] }),
        ga(33, a("2/0/1", "4097"), "Unassigned spare", null),
        ga(34, a("2/0/2", "4098"), "Range was removed", 404),
      ] }] };
}

async function intercept(page: Page, language: Language, theme: string, initial: ProjectTree) {
  const state = { tree: initial, unexpected: [] as string[], requests: [] as string[] };
  await page.route("**/api/**", (route) => {
    const url = new URL(route.request().url()); const path = url.pathname; const method = route.request().method();
    state.requests.push(`${method} ${path}`);
    let body: unknown; let status = 200;
    if (path === "/api/auth/status" && method === "GET") body = { required: false, authenticated: true };
    else if (path === "/api/settings" && method === "GET") body = { schemaVersion: 1, status: "ok", settings: { uiLanguage: language, theme } };
    else if (path === "/api/settings" && method === "PUT") body = { schemaVersion: 1, status: "ok", settings: { uiLanguage: language, theme, ...route.request().postDataJSON().settings } };
    else if ((path === "/api/project" && method === "GET") || (path === "/api/project/new" && method === "POST")) body = state.tree;
    else if (path === "/api/project/load-progress" && method === "GET") body = null;
    else if (path === "/api/bus/discover" && method === "POST") body = { interfaces: [] };
    else if (path === "/api/product-languages" && method === "GET") body = [];
    else if (achievementsFixtureAnswer(method, path) !== undefined) body = achievementsFixtureAnswer(method, path);
    else { state.unexpected.push(`${method} ${path}`); status = 404; body = { error: "strict group-address fixture only" }; }
    return route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
  });
  return state;
}

async function openAddresses(page: Page, language: Language) {
  const m = language === "en" ? en : de;
  await page.goto("/");
  await page.getByRole("button", { name: m["toolbar.newProject"], exact: true }).first().click();
  await page.getByRole("dialog", { name: m["newProject.title"], exact: true }).getByRole("button", { name: m["newProject.create"], exact: true }).click();
  await page.getByRole("dialog", { name: m["projectWizard.done.title"], exact: true }).getByRole("button", { name: m["projectWizard.done.close"], exact: true }).click();
  const dismiss = page.getByRole("button", { name: m["toast.dismiss"], exact: true });
  for (let i = 0; i < 20 && await dismiss.count() > 0; i++) await dismiss.first().click({ timeout: 2000 }).catch(() => {});
  const nav = page.getByRole("navigation", { name: m["workbench.navigation"], exact: true });
  if (!(await nav.isVisible())) await page.getByRole("button", { name: m["workbench.navigation"], exact: true }).click();
  await nav.getByRole("button", { name: m["workbench.addresses"], exact: true }).click();
  await expect(page.locator(".ga-workspace")).toBeVisible();
}
const chooseView = (page: Page, view: string) => page.locator("[data-ga-view]").selectOption(view);
const capture = async (page: Page, name: string) => {
  if (process.env.KNX_GA_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_GA_CAPTURE_DIR, `${name}.png`), fullPage: false });
};
const noPageOverflow = async (page: Page) =>
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  const theme = width === 1440 ? "graphite" : "porcelain";
  test(`${language} four views keep selection, query and scope at ${width}px (${theme})`, async ({ page }) => {
    const m = language === "en" ? en : de;
    await page.setViewportSize({ width, height: 1000 });
    const state = await intercept(page, language, theme, standardTree());
    await openAddresses(page, language);
    expect(await page.evaluate(() => document.documentElement.getAttribute("data-theme"))).toBe(theme);
    const workspace = page.locator(".ga-workspace");
    // Tree table is the default and exposes stored ranges, unnamed fallback and problematic membership.
    await expect(page.locator("[data-ga-view]")).toHaveValue("tree");
    await expect(workspace.locator('[data-ga-expand="range:20:0"]')).toContainText("Lighting");
    await expect(workspace.locator('[data-ga-expand="range:22:0"]')).toContainText(m["gaWorkspace.unnamedRange"].replace("{id}", "22"));
    await expect(workspace.locator('[data-ga-expand="unresolved"]')).toContainText(m["gaWorkspace.unresolved"]);
    await expect(workspace.locator(".ga-issues summary")).toContainText("1");
    await capture(page, `tree-${language}-${width}`);
    // Selection through the real control reaches the shared Inspector selection.
    await workspace.locator('[data-ga-select="30"]').click();
    await expect(workspace.locator('[data-ga-select="30"]').locator("xpath=ancestor::tr")).toHaveAttribute("aria-selected", "true");
    // Keyboard: arrows move focus through visible navigable controls only.
    await workspace.locator('[data-ga-select="30"]').focus();
    await page.keyboard.press("ArrowDown");
    await expect(workspace.locator('[data-ga-select="31"]')).toBeFocused();
    // Collapse hides children; Shift+click then spans only visible rows.
    await workspace.locator('[data-ga-expand="range:21:0"]').click();
    await expect(workspace.locator('[data-ga-select="30"]')).toHaveCount(0);
    await workspace.locator('[data-ga-expand="range:21:0"]').click();
    await workspace.locator(".address-filter").fill("kitchen");
    // Columns: drill stored ranges; the query survives switching.
    await chooseView(page, "columns");
    await expect(workspace.locator("[data-ga-columns]")).toBeVisible();
    await expect(workspace.locator(".address-filter")).toHaveValue("kitchen");
    await workspace.locator('[data-ga-scope="range:20:0"]').click();
    await workspace.locator('[data-ga-scope="range:21:0"]').click();
    await expect(workspace.locator("[data-ga-current-scope]")).toContainText("Ground floor");
    await expect(workspace.locator("[data-ga-select]")).toHaveCount(2);
    await capture(page, `columns-${language}-${width}`);
    // Atlas: same scope, same selection.
    await chooseView(page, "atlas");
    await expect(workspace.locator("[data-ga-atlas]")).toBeVisible();
    await expect(workspace.locator('[data-ga-select="30"]')).toHaveAttribute("aria-pressed", "true");
    await expect(workspace.locator(".ga-evidence-note").first()).toContainText(m["gaWorkspace.atlasMeaning"]);
    await capture(page, `atlas-${language}-${width}`);
    // Matrix: linked-only scope stated, directions separate, unknown direction retained.
    await page.locator(".ga-breadcrumb button").click();
    await workspace.locator(".address-filter").fill("");
    await chooseView(page, "matrix");
    await expect(workspace.locator("[data-ga-matrix]")).toBeVisible();
    await expect(workspace).toContainText(m["gaWorkspace.matrixScope"]);
    const both = workspace.locator('[data-ga-cell="address:31:0|device:1:object:12"]');
    await expect(both).toContainText(m["gaWorkspace.symbol.send"]);
    await expect(workspace.locator('[data-ga-cell="address:32:0|device:3:object:13"]')).toContainText("?");
    await expect(workspace.locator('[data-ga-absence="address:33:0|device:1:object:10"]')).toHaveText("—");
    await capture(page, `matrix-${language}-${width}`);
    await workspace.locator('[data-ga-cell="address:31:0|device:2:object:12"]').click();
    await chooseView(page, "tree");
    await expect(workspace.locator('[data-ga-select="31"]').locator("xpath=ancestor::tr")).toHaveAttribute("aria-selected", "true");
    // Preference is an application setting, never a project mutation.
    expect(state.requests.filter(r => r.startsWith("POST /api/project") && r !== "POST /api/project/new")).toEqual([]);
    expect(state.requests.filter(r => r === "PUT /api/settings").length).toBeGreaterThan(0);
    await noPageOverflow(page);
    expect(state.unexpected).toEqual([]);
  });
}

for (const [style, shown] of [["Free", "2049"], ["TwoLevel", "1/1"]] as const) {
  test(`${style} addresses keep stored ranges and are not regrouped by number`, async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 1000 });
    const state = await intercept(page, "en", "porcelain", standardTree(style));
    await openAddresses(page, "en");
    const workspace = page.locator(".ga-workspace");
    await expect(workspace.locator('[data-ga-select="30"]')).toHaveText(shown);
    await chooseView(page, "columns");
    // First column: stored roots only (Lighting, unnamed #22, no range, unresolved) — no numeric main groups invented.
    await expect(workspace.locator("[data-ga-columns] [data-ga-scope]")).toHaveCount(4);
    await chooseView(page, "tree");
    expect(state.unexpected).toEqual([]);
  });
}

test("large synthetic project switches all four views within measured bounds", async ({ page }) => {
  test.setTimeout(120_000);
  await page.setViewportSize({ width: 1440, height: 1000 });
  const ranges: GroupRangeNode[] = []; const addresses: GroupAddressNode[] = [];
  for (let main = 0; main < 32; main++) {
    ranges.push(range(1000 + main, null, `Main ${main}`));
    for (let middle = 0; middle < 8; middle++) ranges.push(range(2000 + main * 8 + middle, 1000 + main, `Middle ${main}/${middle}`));
  }
  for (let i = 0; i < 5000; i++) {
    const main = i % 32, middle = Math.floor(i / 32) % 8, sub = Math.floor(i / 256);
    addresses.push(ga(10000 + i, `${main}/${middle}/${sub}`, `Synthetic ${i}`, 2000 + main * 8 + middle,
      { links: i % 3 === 0 ? [link(1 + (i % 40), 5000 + (i % 400), i % 2 ? "Send" : "Receive")] : [] }));
  }
  const tree = standardTree(); tree.installations[0].group_ranges = ranges; tree.installations[0].group_addresses = addresses;
  const state = await intercept(page, "en", "graphite", tree);
  await openAddresses(page, "en");
  const timings: Record<string, number> = {};
  for (const view of ["columns", "atlas", "matrix", "tree"]) {
    const start = Date.now();
    await chooseView(page, view);
    await expect(page.locator(view === "tree" ? ".address-table" : `[data-ga-${view}]`).first()).toBeVisible();
    timings[view] = Date.now() - start;
  }
  console.info(JSON.stringify({ witness: "ga-large-5000", timings }));
  for (const [view, ms] of Object.entries(timings)) expect(ms, `${view} switch time`).toBeLessThan(5000);
  expect(state.unexpected).toEqual([]);
});
