/** Browser proofs for local column resizing, sortable devices and virtualised history diffs. */
import { intercept, openList, openProject } from "./communication-fixture";
import { fakeServer, node, openFlow, telegram } from "./telegram-flow-server";
import { expect, test, type Locator, type Page } from "@playwright/test";

async function drag(page: Page, handle: Locator, delta: number) {
  await handle.scrollIntoViewIfNeeded(); const box = (await handle.boundingBox())!;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down(); await page.mouse.move(box.x + box.width / 2 + delta, box.y + box.height / 2, { steps: 5 }); await page.mouse.up();
}
async function preference(page: Page, table: string, column: string, value: number) {
  await expect.poll(() => page.evaluate(({ table, column }) =>
    JSON.parse(localStorage.getItem(`knxbench:table-column-widths:v1:${table}`) ?? "{}")[column], { table, column })).toBe(value);
}
async function announcedWidth(table: Locator, column: number) {
  await expect.poll(async () => Math.abs(Number(await table.getByRole("separator").nth(column).getAttribute("aria-valuenow"))
    - (await table.locator("thead th").nth(column).boundingBox())!.width)).toBeLessThan(1);
}
async function aligned(table: Locator, column: number) {
  const header = await table.locator("thead th").nth(column).boundingBox();
  const body = await table.locator("tbody tr").first().locator("th, td").nth(column).boundingBox();
  expect(Math.abs(header!.x - body!.x)).toBeLessThan(1);
  expect(Math.abs(header!.width - body!.width)).toBeLessThan(1);
}
async function localOverflow(page: Page, table: Locator) {
  const region = table.locator(".resizable-table-scroll");
  expect(await region.evaluate(element => element.scrollWidth > element.clientWidth)).toBe(true);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1)).toBe(true);
  await region.evaluate(element => { element.scrollLeft = 100; });
  expect(await region.evaluate(element => element.scrollLeft)).toBeGreaterThan(0);
}

for (const [language, theme, width] of [
  ["en", "porcelain", 1440], ["de", "graphite", 400], ["en", "lcars", 1920],
  ["de", "user-modern-retro-green-crt", 400], ["en", "cupertino", 1440],
] as const) {
  test(`${language} ${theme} ${width}: drag, keyboard, sorting isolation, reload and local overflow`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 }); await page.emulateMedia({ reducedMotion: "reduce" });
    const writes: string[] = [];
    await page.route("**/api/**", route => {
      if (route.request().method() !== "GET") writes.push(route.request().url());
      return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify({ schemaVersion: 1, serverIncarnation: "table-columns", snapshotRevision: 1, devices: [] }) });
    });
    await page.goto(`/e2e/table-columns-fixture.html?lang=${language}&theme=${theme}`);
    const devices = page.locator('[data-table-id="devices"]');
    const handle = devices.getByRole("separator").nth(1);
    const sorting = devices.locator("thead th").nth(1);
    await expect(sorting).toHaveAttribute("aria-sort", "ascending");
    const initial = Number(await handle.getAttribute("aria-valuenow"));
    await announcedWidth(devices, 1);
    await drag(page, handle, 48); await preference(page, "devices", "address", 148); await announcedWidth(devices, 1);
    await expect(sorting).toHaveAttribute("aria-sort", "ascending"); await expect(page.locator("[data-row-clicks]")).toHaveText("0");
    await handle.focus(); await page.keyboard.press("Shift+ArrowRight"); await preference(page, "devices", "address", 188); await announcedWidth(devices, 1);
    await aligned(devices, 1);
    // Header sticks inside the resized table's actual vertical scroller.
    const scroller = devices.locator(".resizable-table-scroll");
    await scroller.evaluate(element => { element.scrollTop = 200; });
    const top = (await scroller.boundingBox())!.y;
    expect(Math.abs((await sorting.boundingBox())!.y - top)).toBeLessThan(3);
    const resizedDeviceWidth = await handle.getAttribute("aria-valuenow");
    await page.reload(); await preference(page, "devices", "address", 188); await announcedWidth(devices, 1);
    await expect(handle).toHaveAttribute("aria-valuenow", resizedDeviceWidth!);
    await handle.press("Home"); await preference(page, "devices", "address", 48); await announcedWidth(devices, 1);
    await handle.press("ArrowLeft"); await preference(page, "devices", "address", 48); await announcedWidth(devices, 1);
    await handle.press("Enter"); await preference(page, "devices", "address", 100); await expect(handle).toHaveAttribute("aria-valuenow", String(initial));
    await sorting.getByRole("button").click(); await expect(sorting).toHaveAttribute("aria-sort", "descending");
    await handle.press("ArrowRight"); await devices.locator(".table-column-reset").click(); await expect(handle).toHaveAttribute("aria-valuenow", String(initial));
    const comparison = page.locator('[data-table-id="device-comparison"]');
    const other = comparison.getByRole("separator").first();
    await drag(page, other, 32); await preference(page, "device-comparison", "field", 272); await announcedWidth(comparison, 0);
    await other.press("ArrowLeft"); await preference(page, "device-comparison", "field", 264); await announcedWidth(comparison, 0);
    const resizedComparisonWidth = await other.getAttribute("aria-valuenow");
    await page.reload(); await preference(page, "device-comparison", "field", 264); await announcedWidth(comparison, 0);
    await expect(other).toHaveAttribute("aria-valuenow", resizedComparisonWidth!);
    await aligned(comparison, 0);
    await page.screenshot({ path: test.info().outputPath(`${theme}-${width}.png`), fullPage: true });
    await page.setViewportSize({ width: 400, height: 900 });
    // Explicitly widen before asserting local scrolling after a reset.
    for (const separator of await devices.getByRole("separator").all()) await separator.press("End");
    await localOverflow(page, devices); await localOverflow(page, comparison);
    await other.dblclick(); await preference(page, "device-comparison", "field", 240); await announcedWidth(comparison, 0);
    const paint = await other.evaluate(element => {
      const line = getComputedStyle(element, "::before"); return { color: line.backgroundColor, width: line.width };
    });
    expect(paint.color).not.toBe("rgba(0, 0, 0, 0)"); expect(parseFloat(paint.width)).toBeGreaterThanOrEqual(1);
    await page.screenshot({ path: test.info().outputPath(`${theme}-narrow.png`), fullPage: true });
    expect(writes).toEqual([]);
  });
}

test("virtualised history field tables share widths, align through remounts and persist", async ({ page }) => {
  await page.setViewportSize({ width: 400, height: 900 });
  await page.route("**/api/**", route => route.fulfill({ status: 404, body: "{}" }));
  await page.goto("/e2e/diff-virtual-fixture.html");
  await page.getByRole("button", { name: "Group addresses (3300)" }).click();
  await page.getByRole("button", { name: "changed (300)", exact: true }).click();
  const tables = page.locator('[data-table-id="project-diff-fields"]');
  const handle = tables.first().getByRole("separator").first();
  await drag(page, handle, 48); await preference(page, "project-diff-fields", "field", 228); await announcedWidth(tables.first(), 0);
  await handle.press("ArrowRight"); await preference(page, "project-diff-fields", "field", 236); await announcedWidth(tables.first(), 0);
  await announcedWidth(tables.nth(1), 0);
  expect(await tables.nth(1).getByRole("separator").first().getAttribute("aria-valuenow")).toBe(await handle.getAttribute("aria-valuenow"));
  const resizedFieldWidth = await handle.getAttribute("aria-valuenow");
  await aligned(tables.first(), 0); await localOverflow(page, tables.first());
  const viewport = page.locator(".project-diff-viewport"); await viewport.focus(); await page.keyboard.press("End");
  await expect(page.locator("li[aria-posinset]").last()).toHaveAttribute("aria-posinset", "300");
  await preference(page, "project-diff-fields", "field", 236); await announcedWidth(tables.last(), 0);
  await expect(tables.last().getByRole("separator").first()).toHaveAttribute("aria-valuenow", resizedFieldWidth!);
  await aligned(tables.last(), 0);
  expect(await page.locator("li[aria-posinset]").count()).toBeLessThan(120);
  await page.reload(); await page.getByRole("button", { name: "Group addresses (3300)" }).click();
  await page.getByRole("button", { name: "changed (300)", exact: true }).click();
  await preference(page, "project-diff-fields", "field", 236); await announcedWidth(tables.first(), 0);
  await expect(handle).toHaveAttribute("aria-valuenow", resizedFieldWidth!);
  await tables.first().locator(".table-column-reset").click();
  await preference(page, "project-diff-fields", "field", 180); await announcedWidth(tables.nth(1), 0);
});

for (const width of [1440, 1920]) {
  test(`${width}: unresized devices and objects fill their viewport; widening scrolls locally`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await page.route("**/api/**", route => route.fulfill({ status: 404, body: "{}" }));
    await page.goto("/e2e/table-columns-fixture.html");
    for (const id of ["devices", "communication-objects"]) {
      const table = page.locator(`[data-table-id="${id}"]`);
      const viewport = table.locator(".resizable-table-scroll");
      expect(await viewport.evaluate(e => e.scrollWidth <= e.clientWidth)).toBe(true);
      expect(await table.locator("table").evaluate(e => Math.abs(e.getBoundingClientRect().width - e.parentElement!.clientWidth))).toBeLessThanOrEqual(1);
      for (const handle of await table.getByRole("separator").all()) await handle.press("End");
      await localOverflow(page, table);
    }
  });
  test(`${width}: monitor defaults fit; widened columns scroll locally`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await page.route("**/api/**", route => {
      const path = new URL(route.request().url()).pathname;
      const body = path.endsWith("/telegrams") ? { sessionId: 17, serverIncarnation: "fixture", contextStatus: "current", projectOpen: false, status: "active", nextSince: 1, droppedBefore: 0,
        telegrams: [{ seq: 1, timestamp: "2026-09-30T12:00:00Z", source: "1.1.5", destination: "1/2/3", destinationName: null, service: "GroupValueWrite", rawPayload: null, decoded: null, control: null }] } : { interfaces: [] };
      return route.fulfill({ contentType: "application/json", body: JSON.stringify(body) });
    });
    await page.goto("/e2e/monitor-control-fixture.html");
    const table = page.locator('[data-table-id="bus-monitor"]'); await expect(table).toBeVisible();
    expect(await table.locator(".resizable-table-scroll").evaluate(e => e.scrollWidth <= e.clientWidth)).toBe(true);
    for (const handle of await table.getByRole("separator").all()) await handle.press("End");
    await localOverflow(page, table);
  });
}
test("resize target spans 24 pixels while a header button still sorts", async ({ page }) => {
  await page.route("**/api/**", route => route.fulfill({ status: 404, body: "{}" }));
  await page.goto("/e2e/table-columns-fixture.html");
  const table = page.locator('[data-table-id="devices"]'); const handle = table.getByRole("separator").nth(1);
  const box = (await handle.boundingBox())!; expect(box.width).toBeGreaterThanOrEqual(24);
  const heading = table.locator("thead th").nth(1); const headingBox = (await heading.boundingBox())!;
  expect(box.x).toBeLessThan(headingBox.x + headingBox.width); expect(box.x + box.width).toBeGreaterThan(headingBox.x + headingBox.width);
  await heading.getByRole("button").click(); await expect(heading).toHaveAttribute("aria-sort", "descending");
  await drag(page, handle, 48); await expect(heading).toHaveAttribute("aria-sort", "descending");
});

for (const width of [1440, 1920]) {
  test(`${width}: built-app device and object defaults fit with navigation present`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await intercept(page, "en"); await openProject(page, "en"); await openList(page, "en");
    const devices = page.locator('[data-table-id="devices"] .resizable-table-scroll');
    const size = await devices.evaluate(e => ({ scroll: e.scrollWidth, client: e.clientWidth }));
    expect(size.scroll, JSON.stringify(size)).toBeLessThanOrEqual(size.client);
    await page.locator(".devices-table tbody td:nth-child(3) button").first().click();
    const objects = page.locator('[data-table-id="communication-objects"] .resizable-table-scroll');
    const objectSize = await objects.evaluate(e => ({ scroll: e.scrollWidth, client: e.clientWidth }));
    expect(objectSize.scroll, JSON.stringify(objectSize)).toBeLessThanOrEqual(objectSize.client);
  });
  test(`${width}: flow inspector defaults fit its own pane`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    const server = await fakeServer(page); server.rows.push(telegram(1, "1", "On")); await openFlow(page);
    await node(page, "Dimmer").focus(); await page.keyboard.press("Enter");
    for (const viewport of await page.locator(".flow-inspector .resizable-table-scroll").all()) {
      const size = await viewport.evaluate(e => ({ table: e.parentElement!.getAttribute("data-table-id"), scroll: e.scrollWidth, client: e.clientWidth }));
      expect(size.scroll, JSON.stringify(size)).toBeLessThanOrEqual(size.client);
    }
  });
}

test("1440: devices shrink by keyboard and pointer and can redistribute space without scrolling", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  const server = await intercept(page, "en"); await openProject(page, "en"); await openList(page, "en");
  const table = page.locator('[data-table-id="devices"]'); const viewport = table.locator(".resizable-table-scroll");
  expect(await viewport.evaluate(e => e.scrollWidth <= e.clientWidth)).toBe(true);
  const handle = table.getByRole("separator").nth(2); const heading = table.locator("thead th").nth(2);
  const before = (await heading.boundingBox())!.width; const announced = Number(await handle.getAttribute("aria-valuenow"));
  await handle.press("ArrowLeft");
  await expect.poll(async () => (await heading.boundingBox())!.width).toBeLessThan(before - 1);
  expect(Number(await handle.getAttribute("aria-valuenow"))).toBeLessThan(announced);
  const keyed = (await heading.boundingBox())!.width;
  await drag(page, handle, -16);
  await expect.poll(async () => (await heading.boundingBox())!.width).toBeLessThan(keyed - 1);
  await table.getByRole("separator").nth(5).press("ArrowRight");
  expect(await viewport.evaluate(e => e.scrollWidth <= e.clientWidth)).toBe(true);
  expect(server.unexpected).toEqual([]);
});
test("overlay paints above device headers", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  const server = await intercept(page, "en"); await openProject(page, "en"); await openList(page, "en");
  const header = page.locator('[data-table-id="devices"] thead th').nth(1);
  const box = (await header.boundingBox())!;
  expect(await header.evaluate(e => getComputedStyle(e.closest(".resizable-table-scroll")!).isolation)).toBe("isolate");
  await page.keyboard.press("Control+k"); await expect(page.locator(".search-overlay")).toBeVisible();
  expect(await page.evaluate(({ x, y }) => !!document.elementFromPoint(x, y)?.closest(".search-overlay"), { x: box.x + box.width / 2, y: box.y + box.height / 2 })).toBe(true);
  expect(server.unexpected).toEqual([]);
});
test("narrow German sorted heading retains its visible sort indicator", async ({ page }) => {
  await page.setViewportSize({ width: 780, height: 900 });
  await page.route("**/api/**", route => route.fulfill({ status: 404, body: "{}" }));
  await page.goto("/e2e/table-columns-fixture.html?lang=de");
  const header = page.locator('[data-table-id="devices"] thead th').nth(3);
  await header.getByRole("button").click(); await header.getByRole("separator").press("Home");
  const indicator = header.locator('button [aria-hidden="true"]');
  await expect(indicator).toBeVisible();
  expect(await indicator.evaluate(element => {
    const arrow = element.getBoundingClientRect(); const button = element.closest("button")!; const box = button.getBoundingClientRect();
    return arrow.bottom <= box.bottom + 1 && arrow.right <= box.right + 1 && getComputedStyle(button).webkitLineClamp !== "3";
  })).toBe(true);
});
test("announced pixels track measured fitted and stretched device headings", async ({ page }) => {
  await page.setViewportSize({ width: 780, height: 900 });
  await page.route("**/api/**", route => route.fulfill({ status: 404, body: "{}" }));
  await page.goto("/e2e/table-columns-fixture.html");
  const table = page.locator('[data-table-id="devices"]');
  const handle = table.getByRole("separator").nth(2); const heading = table.locator("thead th").nth(2);
  for (const key of [null, "Home", "Enter", "End"] as const) {
    if (key) await handle.press(key);
    await expect.poll(async () => Math.abs(Number(await handle.getAttribute("aria-valuenow")) - (await heading.boundingBox())!.width)).toBeLessThan(1);
  }
});
