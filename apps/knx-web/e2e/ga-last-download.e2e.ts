/** Browser checks for last-download link marks in the group-address views (WP-37). */
import { openExplorerBranches, openNavigationSections } from "./disclosure-fixture";
import { expect, test, type Page } from "@playwright/test";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import type { GroupAddressNode } from "../src/bindings/GroupAddressNode";
import type { GroupRangeNode } from "../src/bindings/GroupRangeNode";
import type { DeviceNode } from "../src/bindings/DeviceNode";
import type { DeviceDetail } from "../src/bindings/DeviceDetail";
import type { ObjectLastDownload } from "../src/bindings/ObjectLastDownload";
import type { DownloadObjectDifference } from "../src/bindings/DownloadObjectDifference";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { formatLogTime } from "../src/logRows";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { busActivityFixtureAnswer } from "./bus-activity-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";

// The expected time is formatted in this Node process by the app's own helper;
// the browser runs in the same zone, so both sides format the same instant alike.
const zone = Intl.DateTimeFormat().resolvedOptions().timeZone;
test.use({ timezoneId: zone });

const range = (id: number, parent: number | null, name: string): GroupRangeNode =>
  ({ id, parent, name, start: "1/0/0", end: "1/7/255" });
const ga = (id: number, address: string, name: string, rangeId: number | null): GroupAddressNode =>
  ({ id, address, name, range: rangeId, dpts: [], links: [] });
const device = (id: number): DeviceNode =>
  ({ id, name: `Fictional device ${id}`, address: `1.1.${id}`, description: null, com_object_count: 1 });

// 1/0/1 = 0x0801, 1/0/2 = 0x0802, 2/0/1 = 0x1001.
const at = "2026-11-12T09:30:00.000Z";
const added: DownloadObjectDifference = { number: 10, instance_id: 10, flags: null,
  links: [{ group_address: 0x0801, downloaded: null, current: true }] };
const removed: DownloadObjectDifference = { number: 11, instance_id: 11, flags: null,
  links: [{ group_address: 0x0802, downloaded: true, current: null }] };
const redirected: DownloadObjectDifference = { number: 12, instance_id: 12, flags: null,
  links: [{ group_address: 0x1001, downloaded: false, current: true }] };
const flagsOnly: DownloadObjectDifference = { number: 13, instance_id: 13, links: [], flags: {
  downloaded: { read: true, write: true, transmit: false, update: false, communication: true, read_on_init: null },
  current: { read: false, write: true, transmit: false, update: false, communication: true, read_on_init: null } } };
const usable: ObjectLastDownload = { links: { state: "usable", at }, flags: { state: "usable", at },
  differing: [added, removed, redirected, flagsOnly], instances: { 10: added, 11: removed, 12: redirected, 13: flagsOnly },
  differing_objects: 4, no_selected_row: 0 };

function tree(): ProjectTree {
  return { schema_version: 13, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
    group_address_style: "ThreeLevel", server_incarnation: "ga-dl-browser", project_incarnation: 1, snapshot_revision: 1,
    installations: [{ id: 1, name: "Fictional house", buildings: [], unassigned: [],
      topology: [{ id: 5, name: "Fictional area", address: 1, lines: [{ id: 6, name: "Fictional line", address: 1,
        devices: [device(1)] }] }],
      group_ranges: [range(20, null, "Lighting"), range(21, 20, "Ground floor")],
      group_addresses: [
        ga(30, "1/0/1", "Kitchen ceiling", 21),
        ga(31, "1/0/2", "Kitchen dimming", 21),
        ga(32, "1/0/3", "Hallway", 20),
        ga(33, "2/0/1", "Unassigned spare", null),
      ] }] };
}

const detail: DeviceDetail = { id: 1, name: "Fictional device 1", description: null, address: "1.1.1",
  com_objects: [], product: { product_ref: null, program_ref: null, catalog: null, resolution: "NoReference" },
  last_download: usable };

async function intercept(page: Page, language: "en" | "de", lastDownload?: ObjectLastDownload, marksEnabled = true) {
  const state = { tree: tree(), unexpected: [] as string[], projectOpen: false,
    settings: { uiLanguage: language, theme: "porcelain", gaLastDownloadMarks: marksEnabled ? "1" : "0" } as Record<string, unknown> };
  await page.route("**/api/**", (route) => {
    const url = new URL(route.request().url()); const path = url.pathname; const method = route.request().method();
    let body: unknown; let status = 200;
    if (path === "/api/auth/status" && method === "GET") body = { required: false, authenticated: true };
    else if (path === "/api/settings" && method === "GET") body = { schemaVersion: 1, status: "ok", settings: { ...state.settings } };
    else if (path === "/api/settings" && method === "PUT") {
      state.settings = { ...state.settings, ...route.request().postDataJSON().settings };
      body = { schemaVersion: 1, status: "ok", settings: { ...state.settings } };
    }
    else if (path === "/api/project/new" && method === "POST") { state.projectOpen = true; body = state.tree; }
    else if (path === "/api/project" && method === "GET") { if (state.projectOpen) body = state.tree; else { status = 400; body = { error: "no project open" }; } }
    else if (path === "/api/project/load-progress" && method === "GET") body = null;
    else if (path === "/api/project/status-metrics" && method === "GET") body = statusMetricsFixtureBody(state.tree);
    else if (path === "/api/bus/discover" && method === "POST") body = { interfaces: [] };
    else if (path === "/api/product-languages" && method === "GET") body = [];
    else if (/^\/api\/device\/\d+$/.test(path) && method === "GET") {
      const id = Number(path.split("/").pop());
      body = id === 1 ? { ...detail, last_download: lastDownload } : { ...detail, id, last_download: lastDownload };
    }
    else if (achievementsFixtureAnswer(method, path) !== undefined) body = achievementsFixtureAnswer(method, path);
    else if (busActivityFixtureAnswer(method, path) !== undefined) body = busActivityFixtureAnswer(method, path);
    else { state.unexpected.push(`${method} ${path}`); status = 404; body = { error: "strict ga last-download fixture only" }; }
    return route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
  });
  return state;
}

async function openAddresses(page: Page, language: "en" | "de") {
  const m = language === "en" ? en : de;
  await page.goto("/");
  await page.getByRole("button", { name: m["toolbar.newProject"], exact: true }).first().click();
  await page.getByRole("dialog", { name: m["newProject.title"], exact: true }).getByRole("button", { name: m["newProject.create"], exact: true }).click();
  await page.getByRole("dialog", { name: m["projectWizard.done.title"], exact: true }).getByRole("button", { name: m["projectWizard.done.close"], exact: true }).click();
  await openNavigationSections(page);
  const dismiss = page.getByRole("button", { name: m["toast.dismiss"], exact: true });
  for (let i = 0; i < 20 && await dismiss.count() > 0; i++) await dismiss.first().click({ timeout: 2000 }).catch(() => {});
  const nav = page.getByRole("navigation", { name: m["workbench.navigation"], exact: true });
  if (!(await nav.isVisible())) await page.getByRole("button", { name: m["workbench.navigation"], exact: true }).click();
  await nav.getByRole("button", { name: m["workbench.addresses"], exact: true }).click();
  await expect(page.locator(".ga-workspace")).toBeVisible();
  for (const button of await page.locator("[data-ga-expand-all]").all()) await button.click();
}

const noPageOverflow = async (page: Page) =>
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} group-address tree marks differing links at ${width}px`, async ({ page }) => {
    const m = language === "en" ? en : de;
    await page.setViewportSize({ width, height: 1000 });
    const state = await intercept(page, language, usable);
    await openAddresses(page, language);
    const workspace = page.locator(".ga-workspace");
    const row = (id: string) => workspace.locator(`[data-ga-select="${id}"]`).locator("xpath=ancestor::tr");
    // 1/0/1 added, 1/0/2 removed, 2/0/1 redirected: dot, caption, description; name unchanged.
    for (const id of ["30", "31", "33"]) {
      await expect(row(id)).toHaveAttribute("data-differs-from-download", "true");
      const button = workspace.locator(`[data-ga-select="${id}"]`);
      await expect(button.locator(".ga-download-mark")).toHaveCount(1);
      await expect(button).toHaveAccessibleDescription(m["gaWorkspace.lastDownload.row"]);
    }
    await expect(row("32")).not.toHaveAttribute("data-differs-from-download", "true");
    await expect(row("32").locator(".ga-download-mark")).toHaveCount(0);
    // Group headings holding differing rows, collapsed included.
    for (const key of ["range:20:0", "range:21:0", "unassigned"]) {
      const heading = workspace.locator(`[data-ga-expand="${key}"]`);
      await expect(heading).toHaveAttribute("data-differs-from-download", "true");
      await expect(heading).toHaveAccessibleDescription(m["gaWorkspace.lastDownload.group"]);
    }
    // Summary above the views: count and the links compare sentence with the time.
    const summary = workspace.locator(".ga-last-download");
    await expect(summary).toHaveAttribute("data-links", "usable");
    await expect(summary).toHaveAttribute("data-differs", "true");
    const time = formatLogTime(Date.parse(at), language, new Date())!;
    await expect(summary).toHaveText(`${m["gaWorkspace.lastDownload.differs.other"].replace("{count}", "3")} ${
      m["comTable.lastDownload.links.usable"].replace("{time}", time)}`);
    // A query hiding every differing row adds the hidden count; rows and marks stay consistent.
    await workspace.locator(".address-filter").fill("hallway");
    await expect(summary).toContainText(m["gaWorkspace.lastDownload.hidden.other"].replace("{count}", "3"));
    await expect(workspace.locator(".ga-download-mark")).toHaveCount(1); // only the group heading of the matched branch
    await noPageOverflow(page);
    expect(state.unexpected).toEqual([]);
  });
}

test("en columns and atlas views carry the same marks", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  const state = await intercept(page, "en", usable);
  await openAddresses(page, "en");
  const workspace = page.locator(".ga-workspace");
  await page.locator("[data-ga-view]").selectOption("columns");
  await expect(workspace.locator("[data-ga-columns]")).toBeVisible();
  const lighting = workspace.locator('.ga-column [data-ga-scope="range:20:0"]');
  await expect(lighting).toHaveAttribute("data-differs-from-download", "true");
  await expect(lighting).toHaveAccessibleDescription(en["gaWorkspace.lastDownload.group"]);
  await lighting.click();
  const floor = workspace.locator('.ga-column [data-ga-scope="range:21:0"]');
  await expect(floor).toHaveAttribute("data-differs-from-download", "true");
  await expect(workspace.locator('[data-ga-select="30"]').locator("xpath=ancestor::tr")).toHaveAttribute("data-differs-from-download", "true");
  await page.locator("[data-ga-view]").selectOption("atlas");
  await expect(workspace.locator("[data-ga-atlas]")).toBeVisible();
  // The atlas keeps its own scope state; leave the columns drill-down first.
  await workspace.locator('.ga-breadcrumb button', { hasText: en["gaWorkspace.all"] }).click();
  const lightingCard = workspace.locator('.ga-atlas-ranges [data-ga-scope="range:20:0"]');
  await expect(lightingCard).toHaveAttribute("data-differs-from-download", "true");
  await lightingCard.click();
  const floorCard = workspace.locator('.ga-atlas-ranges [data-ga-scope="range:21:0"]');
  await expect(floorCard).toHaveAttribute("data-differs-from-download", "true");
  await floorCard.click();
  const card = workspace.locator('.ga-atlas-addresses [data-ga-select="30"]');
  await expect(card).toHaveAccessibleDescription(en["gaWorkspace.lastDownload.row"]);
  await expect(card.locator(".ga-download-mark")).toHaveCount(1);
  await noPageOverflow(page);
  expect(state.unexpected).toEqual([]);
});

test("no baseline (absent) shows the state sentence and no marks", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  const absent: ObjectLastDownload = { ...usable, links: { state: "absent" }, differing: [], instances: {}, differing_objects: 0 };
  const state = await intercept(page, "en", absent);
  await openAddresses(page, "en");
  const summary = page.locator(".ga-workspace .ga-last-download");
  await expect(summary).toHaveAttribute("data-links", "absent");
  await expect(summary).toHaveText(en["comTable.lastDownload.links.absent"]);
  await expect(page.locator(".ga-download-mark")).toHaveCount(0);
  expect(state.unexpected).toEqual([]);
});

test("opt-out preference: no device detail is read and nothing is marked", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  const requests: string[] = [];
  const state = await intercept(page, "en", usable, false);
  await page.route("**/api/device/**", (route) => { requests.push(route.request().url()); return route.continue(); });
  await openAddresses(page, "en");
  await page.locator(".ga-workspace [data-ga-expand-all]").first().click();
  await expect(page.locator(".ga-workspace .ga-last-download")).toHaveCount(0);
  await expect(page.locator(".ga-download-mark")).toHaveCount(0);
  expect(requests).toEqual([]);
  expect(state.unexpected).toEqual([]);
});
