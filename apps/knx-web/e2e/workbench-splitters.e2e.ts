/** The left column's splitters resize their blocks with a project loaded (user bug report). */
import { expect, test, type Locator, type Page } from "@playwright/test";
import { achievementsFixtureAnswer } from "./achievements-fixture";

// Forty installations: an explorer much taller than the column, as in a real project.
const tree = {
  schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
  group_address_style: "ThreeLevel", server_incarnation: "splitter-fixture", snapshot_revision: 1,
  installations: Array.from({ length: 40 }, (_, i) => ({
    id: i + 1, name: `Installation ${i + 1}`, topology: [], buildings: [], unassigned: [], group_addresses: [], group_ranges: [],
  })),
};

async function openProject(page: Page) {
  const unexpected: string[] = [];
  await page.route("**/api/**", (route) => {
    const path = new URL(route.request().url()).pathname;
    const method = route.request().method();
    let body: unknown;
    if (path === "/api/auth/status" && method === "GET") body = { required: false, authenticated: true };
    else if (path === "/api/settings" && method === "GET") body = { schemaVersion: 1, status: "ok", settings: {} };
    else if (path === "/api/project" && method === "GET") body = tree;
    else if (path === "/api/project/new" && method === "POST") body = tree;
    // The sidebar's automatic search is intercepted, never a real UDP scan.
    else if (path === "/api/bus/discover" && method === "POST") body = { interfaces: [] };
    else if (method === "GET" && ["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items"].includes(path)) body = [];
    else if (achievementsFixtureAnswer(method, path) !== undefined) body = achievementsFixtureAnswer(method, path);
    else {
      unexpected.push(`${method} ${path}`);
      return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"local fixture only"}' });
    }
    return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
  });
  await page.setViewportSize({ width: 1400, height: 900 });
  await page.goto("/");
  await page.getByRole("button", { name: "New project…" }).click();
  await page.getByRole("dialog", { name: "New project" }).getByRole("button", { name: "Create project" }).click();
  await expect(page.getByRole("separator", { name: "Height of the navigation block" })).toBeVisible();
  return unexpected;
}

const height = async (block: Locator) => (await block.boundingBox())!.height;

async function drag(page: Page, separator: Locator, dy: number) {
  const box = (await separator.boundingBox())!;
  const x = box.x + box.width / 2;
  const y = box.y + box.height / 2;
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x, y + dy, { steps: 8 });
  await page.mouse.up();
}

test("dragging the navigation splitter down grows the navigation block by the dragged distance", async ({ page }) => {
  const unexpected = await openProject(page);
  const separator = page.getByRole("separator", { name: "Height of the navigation block" });
  const block = page.getByRole("navigation", { name: "Navigation" });
  const before = await height(block);
  await drag(page, separator, 80);
  expect(await height(block)).toBeCloseTo(before + 80, 0);
  // What the separator reports is what is drawn.
  expect(Number(await separator.getAttribute("aria-valuenow"))).toBeCloseTo(await height(block), 0);
  // The explorer stays in the column and scrolls instead of pushing the blocks.
  await expect(page.locator(".workbench-pane-left .project-explorer")).toBeVisible();
  expect(unexpected).toEqual([]);
});

test("dragging the diagnostics splitter up grows the diagnostics block, and the keyboard moves it too", async ({ page }) => {
  await openProject(page);
  const separator = page.getByRole("separator", { name: "Height of the diagnostics block" });
  const block = page.getByRole("navigation", { name: "Bus monitor" });
  const before = await height(block);
  await drag(page, separator, -60);
  const dragged = await height(block);
  expect(dragged).toBeCloseTo(before + 60, 0);
  await separator.focus();
  await page.keyboard.press("ArrowUp");
  expect(await height(block)).toBeCloseTo(dragged + 16, 0);
});

test("with both blocks dragged to their maximum, the explorer keeps a visible minimum and scrolls", async ({ page }) => {
  await openProject(page);
  await drag(page, page.getByRole("separator", { name: "Height of the navigation block" }), 500);
  await drag(page, page.getByRole("separator", { name: "Height of the diagnostics block" }), -500);
  const explorer = page.locator(".workbench-pane-left .project-explorer");
  expect(await height(explorer)).toBeGreaterThanOrEqual(72);
  // Nothing spills out of the column: the last diagnostics button stays inside it.
  const column = (await page.locator(".workbench-pane-left .workbench-pane-content").boundingBox())!;
  const settings = (await page.getByRole("navigation", { name: "Bus monitor" }).getByRole("button", { name: "Settings" }).boundingBox())!;
  expect(settings.y + settings.height).toBeLessThanOrEqual(column.y + column.height + 1);
});
