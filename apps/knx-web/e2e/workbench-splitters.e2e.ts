/** The left column's splitters resize their blocks with a project loaded (user bug report). */
import { openNavigationSections } from "./disclosure-fixture";
import { expect, test, type Locator, type Page } from "@playwright/test";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";

// Forty installations: an explorer much taller than the column, as in a real project.
const tree = {
  schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
  group_address_style: "ThreeLevel", server_incarnation: "splitter-fixture", snapshot_revision: 1,
  installations: Array.from({ length: 40 }, (_, i) => ({
    id: i + 1, name: `Installation ${i + 1}`, topology: [], buildings: [], unassigned: [], group_addresses: [], group_ranges: [],
  })),
};

// `stored` (e.g. a `uiScale`) drives zoom the real way: a server setting App
// applies itself. Only a caller that passes it accepts setting writes.
async function openProject(page: Page, stored?: Record<string, unknown>) {
  const unexpected: string[] = [];
  // Like the real server: no project until the test creates one (ADR-0127).
  let projectOpen = false;
  await page.route("**/api/**", (route) => {
    const path = new URL(route.request().url()).pathname;
    const method = route.request().method();
    let body: unknown;
    let status = 200;
    if (path === "/api/auth/status" && method === "GET") body = { required: false, authenticated: true };
    else if (path === "/api/settings" && method === "GET") body = { schemaVersion: 1, status: "ok", settings: stored ?? {} };
    else if (path === "/api/settings" && method === "PUT" && stored) {
      Object.assign(stored, route.request().postDataJSON().settings); body = { schemaVersion: 1, status: "ok", settings: stored };
    }
    else if (path === "/api/project" && method === "GET") { if (projectOpen) body = tree; else { status = 400; body = { error: "no project open" }; } }
    else if (path === "/api/project/new" && method === "POST") { projectOpen = true; body = tree; }
    else if (path === "/api/project/status-metrics" && method === "GET") body = statusMetricsFixtureBody(tree);
    // The sidebar's automatic search is intercepted, never a real UDP scan.
    else if (path === "/api/bus/discover" && method === "POST") body = { interfaces: [] };
    else if (method === "GET" && ["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items"].includes(path)) body = [];
    else if (achievementsFixtureAnswer(method, path) !== undefined) body = achievementsFixtureAnswer(method, path);
    else {
      unexpected.push(`${method} ${path}`);
      return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"local fixture only"}' });
    }
    return route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
  });
  await page.setViewportSize({ width: 1400, height: 900 });
  await page.goto("/");
  await page.getByRole("button", { name: "New project…" }).click();
  await page.getByRole("dialog", { name: "New project" }).getByRole("button", { name: "Create project" }).click();
  await page.getByRole("dialog", { name: "Project created" }).getByRole("button", { name: "Done" }).click();
  await openNavigationSections(page);
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

async function dragX(page: Page, separator: Locator, dx: number) {
  const box = (await separator.boundingBox())!;
  const x = box.x + box.width / 2;
  const y = box.y + box.height / 2;
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x + dx, y, { steps: 8 });
  await page.mouse.up();
}

// UISET follow-up: pane edges follow the pointer at every application scale;
// widths are layout pixels, so the stored width changes by 60 / scale.
for (const scale of [0.8, 1, 1.5]) {
  test(`scale ${scale}: navigation, inspector and block edges follow a real pointer drag`, async ({ page }) => {
    const stored: Record<string, unknown> = { uiScale: scale };
    const unexpected = await openProject(page, stored);
    await expect.poll(() => page.evaluate(() => document.documentElement.style.getPropertyValue("--app-ui-scale"))).toBe(String(scale));
    const navigation = page.locator(".workbench-pane-left");
    const navigationSeparator = page.getByRole("separator", { name: "Navigation", exact: true });
    const right = async () => { const box = (await navigation.boundingBox())!; return box.x + box.width; };
    const beforeRight = await right();
    await dragX(page, navigationSeparator, 60);
    expect(Math.abs(await right() - beforeRight - 60), "navigation edge follows the pointer").toBeLessThan(1.5);
    const navigationWidth = Math.round(250 + 60 / scale);
    await expect.poll(() => stored.navigationPaneWidth).toBe(navigationWidth);
    expect(Number(await navigationSeparator.getAttribute("aria-valuenow"))).toBe(navigationWidth);
    expect(Math.abs((await navigation.boundingBox())!.width / scale - navigationWidth)).toBeLessThan(1);

    const inspector = page.locator(".workbench-pane-right");
    const inspectorSeparator = page.getByRole("separator", { name: "Properties", exact: true });
    const left = async () => (await inspector.boundingBox())!.x;
    const beforeLeft = await left();
    await dragX(page, inspectorSeparator, -60);
    expect(Math.abs(await left() - beforeLeft + 60), "inspector edge follows the pointer").toBeLessThan(1.5);
    const inspectorWidth = Math.round(360 + 60 / scale);
    await expect.poll(() => stored.inspectorPaneWidth).toBe(inspectorWidth);
    expect(Number(await inspectorSeparator.getAttribute("aria-valuenow"))).toBe(inspectorWidth);

    const separator = page.getByRole("separator", { name: "Height of the navigation block" });
    const block = page.getByRole("navigation", { name: "Navigation" });
    const before = await height(block);
    await drag(page, separator, 60);
    expect(Math.abs(await height(block) - before - 60), "block edge follows the pointer").toBeLessThan(1.5);
    expect(Math.abs(Number(await separator.getAttribute("aria-valuenow")) - await height(block) / scale)).toBeLessThan(1);
    expect(unexpected).toEqual([]);
  });
}

// WP-11 (28 October 2026): in WebKitGTK a separator drag selected document
// text and later drags then lost their moves. Chromium does not reproduce that
// loss (this case also passes without the fix); the unit guards in
// separatorSelection.test.tsx discriminate. Here real drags must still commit
// and leave no selection with the cancelled pointerdown default.
test("repeated separator drags start no text selection and every drag commits", async ({ page }) => {
  const stored: Record<string, unknown> = {};
  const unexpected = await openProject(page, stored);
  const selection = () => page.evaluate(() => window.getSelection()?.toString() ?? "");
  const navigationSeparator = page.getByRole("separator", { name: "Navigation", exact: true });
  for (const [step, dx] of [[1, 100], [2, -40], [3, 60]] as const) {
    const before = Number(await navigationSeparator.getAttribute("aria-valuenow"));
    await dragX(page, navigationSeparator, dx);
    expect(await selection(), `drag ${step} selected text`).toBe("");
    await expect.poll(() => stored.navigationPaneWidth, `drag ${step} commits`).toBe(before + dx);
  }
  const block = page.getByRole("navigation", { name: "Navigation" });
  const splitter = page.getByRole("separator", { name: "Height of the navigation block" });
  for (const dy of [40, -20, 30]) {
    const before = await height(block);
    await drag(page, splitter, dy);
    expect(await selection(), "block drag selected text").toBe("");
    expect(Math.abs(await height(block) - before - dy)).toBeLessThan(1.5);
  }
  expect(unexpected).toEqual([]);
});
