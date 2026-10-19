/** Browser proofs for local column resizing, sortable devices and virtualised history diffs. */
import { writeFile } from "node:fs/promises";
import { intercept, openList, openProject } from "./communication-fixture";
import { fakeServer, node, openFlow, telegram } from "./telegram-flow-server";
import { expect, test, type Locator, type Page } from "@playwright/test";
import { intrinsicFloors, assertFloorOverflow } from "./table-intrinsic-oracle";
import { wordsIntact, scrollsLocally } from "./table-word-assertions";

async function evidenceScreenshot(page: Page, name: string) {
  await page.screenshot({ path: test.info().outputPath(`${name}.png`), fullPage: true });
  const context = await page.evaluate(() => ({ uiScale: getComputedStyle(document.documentElement).getPropertyValue("--app-ui-scale"), path: location.pathname, query: location.search,
    viewport: { width: innerWidth, height: innerHeight }, theme: document.documentElement.dataset.theme ?? "unset",
    density: document.documentElement.dataset.density ?? "compact (CSS default)", fonts: document.fonts.status }));
  await writeFile(test.info().outputPath(`${name}.json`), JSON.stringify(context, null, 2));
}

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
  const scale = await table.evaluate(() => parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--app-ui-scale")) || 1);
  await expect.poll(async () => Math.abs(Number(await table.getByRole("separator").nth(column).getAttribute("aria-valuenow"))
    - (await table.locator("thead th").nth(column).boundingBox())!.width / scale)).toBeLessThan(1);
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

for (const language of ["en", "de"]) for (const flat of [false, true]) {
  test(`${language} ${flat ? "flat" : "grouped"} objects keep every word intact at 360 and status at 640`, async ({ page }) => {
    await page.setViewportSize({ width: 360, height: 900 });
    await page.route("**/api/**", route => route.fulfill({ status: 404, body: "{}" }));
    await page.goto(`/e2e/table-columns-fixture.html?lang=${language}&words=1`);
    const table = page.locator('[data-table-id="communication-objects"]');
    if (flat) await page.getByRole("button", { name: language === "de" ? "Alle Objekte" : "All objects", exact: true }).click();
    else await page.locator(".com-object-channel-summary").click();
    await expect(table.locator(".com-object-effective-dpt > span")).toHaveText(language === "de" ? "Beleuchtungsstärke (9.004)" : "Illuminance (9.004)");
    await wordsIntact(table.locator(".com-object-summary td"));
    await wordsIntact(table.locator("thead th"));
    await scrollsLocally(page, table);
    await evidenceScreenshot(page, `${language}-360-porcelain-${flat ? "flat" : "grouped"}-words`);
    await page.setViewportSize({ width: 640, height: 900 });
    await expect(table.locator(".com-object-status")).toHaveText(language === "de" ? "Nicht bestimmbar" : "Undetermined");
    await wordsIntact(table.locator(".com-object-summary td:nth-child(6)"));
  });
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

test("comparison owns exactly one field reset while single-instance devices retain their reset", async ({ page }) => {
  await page.goto("/e2e/diff-virtual-fixture.html");
  await page.getByRole("button", { name: "Group addresses (3300)" }).click();
  await page.getByRole("button", { name: "changed (300)", exact: true }).click();
  const tables = page.locator('[data-table-id="project-diff-fields"]');
  expect(await tables.count()).toBeGreaterThan(1);
  const reset = page.getByRole("button", { name: "Reset column widths: Field changes", exact: true });
  await expect(reset).toHaveCount(1);
  await expect(tables.locator(".table-column-reset")).toHaveCount(0);
  await tables.first().getByRole("separator").first().press("ArrowRight");
  await preference(page, "project-diff-fields", "field", 188);
  await tables.nth(1).getByRole("separator").first().dblclick();
  await preference(page, "project-diff-fields", "field", 180);
  await tables.nth(1).getByRole("separator").nth(1).press("ArrowRight");
  await reset.click();
  await preference(page, "project-diff-fields", "before", 240);
  for (const table of await tables.all()) await announcedWidth(table, 0);
  await page.goto("/e2e/table-columns-fixture.html");
  await expect(page.locator('[data-table-id="devices"]').getByRole("button", { name: /^Reset column widths:/ })).toHaveCount(1);
});

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
  await page.getByRole("button", { name: "Reset column widths: Field changes", exact: true }).click();
  await preference(page, "project-diff-fields", "field", 180); await announcedWidth(tables.nth(1), 0);
});

for (const language of ["en", "de"] as const) for (const width of [1440, 1920]) {
  test(`${language} ${width}: unresized devices and objects fill their viewport; widening scrolls locally`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await page.route("**/api/**", route => route.fulfill({ status: 404, body: "{}" }));
    await page.goto(`/e2e/table-columns-fixture.html?lang=${language}`);
    for (const id of ["devices", "communication-objects"]) {
      const table = page.locator(`[data-table-id="${id}"]`);
      const viewport = table.locator(".resizable-table-scroll");
      expect(await viewport.evaluate(e => e.scrollWidth <= e.clientWidth)).toBe(true);
      expect(await table.locator("table").evaluate(e => Math.abs(e.getBoundingClientRect().width - e.parentElement!.clientWidth))).toBeLessThanOrEqual(1);
      for (const handle of await table.getByRole("separator").all()) await handle.press("End");
      await localOverflow(page, table);
    }
  });
  test(`${language} ${width}: monitor defaults fit; widened columns scroll locally`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await page.route("**/api/**", route => {
      const path = new URL(route.request().url()).pathname;
      const body = path.endsWith("/telegrams") ? { sessionId: 17, serverIncarnation: "fixture", contextStatus: "current", projectOpen: false, status: "active", nextSince: 1, droppedBefore: 0,
        telegrams: [{ seq: 1, timestamp: "2026-09-30T12:00:00Z", source: "1.1.5", destination: "1/2/3", destinationName: null, service: "GroupValueWrite", rawPayload: null, decoded: null, control: null }] } : { interfaces: [] };
      return route.fulfill({ contentType: "application/json", body: JSON.stringify(body) });
    });
    await page.goto(`/e2e/monitor-control-fixture.html?lang=${language}`);
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

// The oracle uses independent browser intrinsic layout at the real pane size.
async function defaultFit(page: Page, table: Locator, name: string, expectScroll = false, retained: number[] = []) {
  const viewport = table.locator(".resizable-table-scroll");
  await announcedWidth(table, 1);
  const floors = await intrinsicFloors(table, retained);
  const size = await viewport.evaluate(e => ({ scroll: e.scrollWidth, client: e.clientWidth }));
  const minimum = floors.reduce((a, b) => a + b, 0);
  const measurement = { name, ...size, minimum, floors };
  await test.info().attach(`${name}-fit-budget`, { body: JSON.stringify(measurement), contentType: "application/json" });
  console.log("fit-budget", JSON.stringify(measurement));
  expect(minimum > size.client, JSON.stringify(measurement)).toBe(expectScroll);
  if (expectScroll) {
    // Device text columns alone exceed the pane, independently of the compact
    // selection-column exception in the reviewed rendered budget.
    if (name.endsWith("devices")) expect(minimum - floors[0]).toBeGreaterThan(size.client);
    const reviewedTotal = name.endsWith("devices") ? (name.startsWith("de-") ? 955 : 919) : 800;
    await assertFloorOverflow(table, floors, reviewedTotal);
    expect(Math.abs(size.scroll - minimum), JSON.stringify(measurement)).toBeLessThanOrEqual(2 * floors.length);
    await scrollsLocally(page, table);
  } else {
    expect(size.scroll, JSON.stringify(measurement)).toBeLessThanOrEqual(size.client);
  }
  return floors;
}

for (const language of ["en", "de"] as const) for (const width of [1440, 1920]) {
  test(`${language} ${width}: built-app defaults fit or scroll at measured word floors with navigation present`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    const server = await intercept(page, language); await openProject(page, language); await openList(page, language);
    const devices = page.locator('[data-table-id="devices"]');
    await defaultFit(page, devices, `${language}-${width}-devices`, width === 1440);
    await wordsIntact(devices.locator("thead .table-column-label"));
    await wordsIntact(devices.locator("tbody td"));
    await evidenceScreenshot(page, `${language}-${width}-porcelain-devices`);
    await page.locator(".devices-table tbody td:nth-child(3) button").first().click();
    const objects = page.locator('[data-table-id="communication-objects"]');
    await page.locator(".com-object-channel-summary").first().click();
    const groupedFloors = await defaultFit(page, objects, `${language}-${width}-grouped`);
    await wordsIntact(objects.locator("thead .table-column-label"));
    await wordsIntact(objects.locator(".com-object-summary:not([hidden]) td"));
    await page.getByRole("button", { name: language === "de" ? "Alle Objekte" : "All objects", exact: true }).click();
    await defaultFit(page, objects, `${language}-${width}-flat`, language === "de" && width === 1440, groupedFloors);
    await wordsIntact(objects.locator("thead .table-column-label"));
    await wordsIntact(objects.locator(".com-object-summary:not([hidden]) td"));
    expect(server.unexpected).toEqual([]);
  });
  if (language === "en") test(`${width}: flow inspector fits with specialized padding and preserves words`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    const server = await fakeServer(page); server.rows.push(telegram(1, "1", "On")); await openFlow(page);
    await node(page, "Dimmer").focus(); await page.keyboard.press("Enter");
    for (const table of await page.locator(".flow-inspector .resizable-table").all()) {
      const id = await table.getAttribute("data-table-id");
      await defaultFit(page, table, `${width}-${id}`);
      await wordsIntact(table.locator("thead th"));
      await wordsIntact(table.locator("tbody th, tbody td"));
    }
  });
}

test("1440: devices shrink by keyboard and pointer and redistribute space with local scrolling", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  const server = await intercept(page, "en"); await openProject(page, "en"); await openList(page, "en");
  const table = page.locator('[data-table-id="devices"]'); const viewport = table.locator(".resizable-table-scroll");
  const initialScroll = await viewport.evaluate(e => e.scrollWidth);
  await localOverflow(page, table);
  const handle = table.getByRole("separator").nth(2); const heading = table.locator("thead th").nth(2);
  const before = (await heading.boundingBox())!.width; const announced = Number(await handle.getAttribute("aria-valuenow"));
  await handle.press("ArrowLeft");
  await expect.poll(async () => (await heading.boundingBox())!.width).toBeLessThan(before - 1);
  expect(Number(await handle.getAttribute("aria-valuenow"))).toBeLessThan(announced);
  const keyed = (await heading.boundingBox())!.width;
  await drag(page, handle, -16);
  await expect.poll(async () => (await heading.boundingBox())!.width).toBeLessThan(keyed - 1);
  await table.getByRole("separator").nth(5).press("ArrowRight");
  expect(await viewport.evaluate(e => e.scrollWidth)).toBeLessThan(initialScroll);
  await localOverflow(page, table);
  expect(server.unexpected).toEqual([]);
});
test("overlay paints above device headers", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  const server = await intercept(page, "en"); await openProject(page, "en"); await openList(page, "en");
  const header = page.locator('[data-table-id="devices"] thead th').nth(1);
  const box = (await header.boundingBox())!;
  await page.keyboard.press("Control+k"); await expect(page.locator(".search-overlay")).toBeVisible();
  const overlayAboveHeader = await page.evaluate(({ x, y }) => {
    // Inert excludes the background from hit testing even if it paints above
    // the modal. Remove it only during this synchronous stacking probe.
    const isolated = [...document.querySelectorAll<HTMLElement>("[inert]")];
    try {
      isolated.forEach(element => { element.inert = false; });
      return !!document.elementFromPoint(x, y)?.closest(".search-overlay");
    } finally { isolated.forEach(element => { element.inert = true; }); }
  }, { x: box.x + box.width / 2, y: box.y + box.height / 2 });
  expect(overlayAboveHeader, "overlay must paint above the non-inert device header").toBe(true);
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

test("converted table inline padding follows compact, comfortable and custom theme tokens", async ({ page }) => {
  await page.setViewportSize({ width: 400, height: 900 });
  await page.route("**/api/**", route => route.fulfill({ status: 404, body: "{}" }));
  await page.goto("/e2e/table-columns-fixture.html?words=1");
  const table = page.locator('[data-table-id="communication-objects"]');
  await page.locator(".com-object-channel-summary").click();
  const padding = () => table.evaluate(element => [
    element.querySelector("thead th:nth-child(2)")!, element.querySelector(".com-object-summary td:nth-child(2)")!,
  ].map(cell => [getComputedStyle(cell).paddingLeft, getComputedStyle(cell).paddingRight]));
  await expect.poll(padding).toEqual([["12px", "12px"], ["12px", "12px"]]);
  await page.evaluate(() => { document.documentElement.dataset.density = "comfortable"; });
  await expect.poll(padding).toEqual([["14px", "14px"], ["14px", "14px"]]);
  await wordsIntact(table.locator(".com-object-summary td"));
  await page.evaluate(() => { document.documentElement.style.setProperty("--knx-cell-padding", "13px 19px"); });
  await expect.poll(padding).toEqual([["19px", "19px"], ["19px", "19px"]]);
  await wordsIntact(table.locator(".com-object-summary td"));
  await wordsIntact(table.locator("thead th"));
  await scrollsLocally(page, table);
});

test("sorting reserves the indicator footprint in every sortable column", async ({ page }) => {
  await page.setViewportSize({ width: 400, height: 900 });
  await page.route("**/api/**", route => route.fulfill({ status: 404, body: "{}" }));
  await page.goto("/e2e/table-columns-fixture.html?words=1");
  for (const id of ["devices", "communication-objects"]) {
    const table = page.locator(`[data-table-id="${id}"]`);
    const widths = () => table.locator("thead th").evaluateAll(cells => cells.map(cell => cell.getBoundingClientRect().width));
    const before = await widths();
    for (const button of await table.locator("thead th button").all()) {
      await button.click();
      await expect.poll(widths).toEqual(before);
    }
  }
});


test("colspan mutation remaps the measured floor into its new column", async ({ page }) => {
  await page.goto("/e2e/table-columns-fixture.html?invalidation=1");
  const table = page.locator('[data-table-id="invalidation"]');
  const width = (index: number) => table.locator("thead th").nth(index).evaluate(e => e.getBoundingClientRect().width);
  const long = await width(2);
  expect(long).toBeGreaterThan(80);
  expect(await width(1)).toBe(48);
  await table.locator("tbody td").first().evaluate(e => { (e as HTMLTableCellElement).colSpan = 1; });
  await expect.poll(() => width(1)).toBeGreaterThanOrEqual(long - 1);
});

for (const target of ["table-style", "table-class", "viewport-style", "pane-style", "pane-class"] as const) {
  test(`${target} inherited typography invalidates mounted cell floors`, async ({ page }) => {
    await page.goto("/e2e/table-columns-fixture.html?invalidation=1");
    await page.addStyleTag({ content: ".large-words { font-size: 24px !important; }" });
    const table = page.locator('[data-table-id="invalidation"]');
    const cell = table.locator("thead th").nth(2);
    const before = (await cell.boundingBox())!.width;
    const element = target.startsWith("table") ? table.locator("table") : target.startsWith("viewport")
      ? table.locator(".resizable-table-scroll") : page.locator("[data-invalidation-pane]");
    await element.evaluate((e, target) => {
      if (target.endsWith("class")) e.classList.add("large-words");
      else (e as HTMLElement).style.fontSize = "24px";
    }, target);
    await expect.poll(async () => (await cell.boundingBox())!.width).toBeGreaterThan(before + 50);
    await wordsIntact(table.locator("tbody td").last());
  });
}

test("specialized project diff preserves block padding across density and theme overrides", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/e2e/diff-virtual-fixture.html");
  await page.evaluate(() => { document.documentElement.dataset.theme = "porcelain"; document.documentElement.dataset.density = "compact"; });
  await page.getByRole("button", { name: "Group addresses (3300)" }).click();
  await page.getByRole("button", { name: "changed (300)", exact: true }).click();
  const table = page.locator('[data-table-id="project-diff-fields"]').first();
  const padding = () => table.locator("th, td").evaluateAll(cells => cells.map(e => {
    const s = getComputedStyle(e); return [s.paddingTop, s.paddingBottom];
  }));
  const expected = await page.evaluate(() => `${parseFloat(getComputedStyle(document.documentElement).fontSize) * 0.1}px`);
  await evidenceScreenshot(page, "en-1440-porcelain-project-diff-padding");
  for (const density of ["compact", "comfortable"]) {
    await page.evaluate(density => { document.documentElement.dataset.density = density; }, density);
    await expect.poll(padding).toEqual(Array(6).fill([expected, expected]));
  }
  await page.evaluate(() => { document.documentElement.style.setProperty("--knx-cell-padding", "13px 19px"); });
  await expect.poll(padding).toEqual(Array(6).fill([expected, expected]));
});


test("intrinsic oracle bounds known words, padding, controls and ignores rendered column inflation", async ({ page }) => {
  await page.goto("/e2e/table-columns-fixture.html?invalidation=1");
  const table = page.locator('[data-table-id="invalidation"]');
  await table.locator("tbody td").first().evaluate(e => { (e as HTMLTableCellElement).colSpan = 1; e.innerHTML = '<button style="min-width:170px;padding:0 9px">x</button>'; });
  const floors = await intrinsicFloors(table);
  expect(floors[0]).toBeGreaterThanOrEqual(194); // 170 px control + 24 px cell padding.
  expect(floors[0]).toBeLessThanOrEqual(196);
  expect(floors[1]).toBeGreaterThan(80);
  expect(floors[1]).toBeLessThan(150);
  expect(floors[2]).toBe(48);
  await table.locator("tbody td").last().evaluate(e => {
    e.textContent = "MMMMMMMMMM"; e.style.font = "10px monospace"; e.style.letterSpacing = "0px";
  });
  const ten = (await intrinsicFloors(table))[1];
  expect(ten).toBeGreaterThanOrEqual(84); expect(ten).toBeLessThanOrEqual(86);
  await table.locator("tbody td").last().evaluate(e => { e.style.paddingInline = "20px"; });
  expect((await intrinsicFloors(table))[1] - ten).toBe(16);
  await table.locator("tbody td").last().evaluate(e => { e.style.fontSize = "20px"; });
  const twenty = (await intrinsicFloors(table))[1];
  expect(twenty).toBeGreaterThanOrEqual(160); expect(twenty).toBeLessThanOrEqual(162);
  await table.locator("tbody td").last().evaluate(e => { e.textContent = "x".repeat(500); });
  expect((await intrinsicFloors(table))[1]).toBe(320);
});

test("intrinsic oracle waits for a font requested after the table appears", async ({ page }) => {
  await page.goto("/e2e/table-columns-fixture.html?invalidation=1");
  await page.evaluate(() => document.fonts.ready.then(() => undefined));
  const table = page.locator('[data-table-id="invalidation"]');
  let release!: () => void;
  const released = new Promise<void>(resolve => { release = resolve; });
  let requested!: () => void;
  const request = new Promise<void>(resolve => { requested = resolve; });
  await page.route("**/oracle-deferred.woff2", async route => {
    requested(); await released;
    await route.fulfill({ path: "node_modules/@fontsource/jetbrains-mono/files/jetbrains-mono-latin-400-normal.woff2",
      contentType: "font/woff2" });
  });
  await table.evaluate(element => {
    const face = new FontFace("OracleDeferred", 'url("/oracle-deferred.woff2")');
    document.fonts.add(face);
    element.querySelectorAll<HTMLElement>("th, td, th *, td *").forEach(cell => {
      cell.style.fontFamily = '"OracleDeferred", monospace';
    });
    element.getBoundingClientRect(); // Request the font before starting the measurement.
  });
  await request;
  await request;
  await table.evaluate(() => {
    const fonts = document.fonts;
    Object.defineProperty(document, "fonts", { configurable: true, get: () => {
      document.documentElement.dataset.oracleFontRead = "true";
      return fonts;
    } });
    // Observe the oracle's isolated measurement sandbox, not production table
    // remeasurement (which may legitimately happen while this font is pending).
    new MutationObserver(records => {
      for (const record of records) for (const node of record.addedNodes) {
        if (node instanceof HTMLElement && node.style.contain === "layout style"
          && node.style.visibility === "hidden") {
          document.documentElement.dataset.oracleMeasured = "true";
        }
      }
    }).observe(document.body, { childList: true });
  });
  const measurement = intrinsicFloors(table);
  try {
    // Observe the helper entering either its font barrier or its measurement;
    // no sleep or assumption about Node/browser message scheduling is needed.
    await page.waitForFunction(() => document.documentElement.dataset.oracleFontRead
      || document.documentElement.dataset.oracleMeasured);
    expect(await page.evaluate(() => document.documentElement.dataset.oracleMeasured),
      "the oracle must not measure fallback-font widths").toBeUndefined();
  } finally { release(); }
  const floors = await measurement;
  expect(await page.evaluate(() => document.fonts.check('12px "OracleDeferred"'))).toBe(true);
  expect(await intrinsicFloors(table)).toEqual(floors);
});

test("independent fit assertion rejects an inflated rendered floor and accepts restoration", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  const server = await intercept(page, "en"); await openProject(page, "en"); await openList(page, "en");
  const table = page.locator('[data-table-id="devices"]');
  const floors = await intrinsicFloors(table);
  await assertFloorOverflow(table, floors, 919);
  const original = await table.locator("col").nth(2).getAttribute("style");
  await table.locator("col").nth(2).evaluate(e => { e.style.width = "240px"; });
  expect(await intrinsicFloors(table)).toEqual(floors);
  await expect(assertFloorOverflow(table, floors, 919)).rejects.toThrow();
  await table.locator("col").nth(2).evaluate((e, style) => { e.setAttribute("style", style!); }, original);
  await assertFloorOverflow(table, floors, 919);
  expect(server.unexpected).toEqual([]);
});


for (const target of ["table", "viewport", "pane"] as const) {
  test(`${target} inherited padding token changes invalidate cell insets`, async ({ page }) => {
    await page.goto("/e2e/table-columns-fixture.html?invalidation=1");
    const table = page.locator('[data-table-id="invalidation"]');
    const cell = table.locator("thead th").nth(2);
    const before = (await cell.boundingBox())!.width;
    const element = target === "table" ? table.locator("table") : target === "viewport"
      ? table.locator(".resizable-table-scroll") : page.locator("[data-invalidation-pane]");
    await element.evaluate(e => { (e as HTMLElement).style.setProperty("--knx-cell-padding", "7px 30px"); });
    await expect.poll(async () => (await cell.boundingBox())!.width).toBe(before + 36);
  });
}

test("scrolling virtualised history does not remeasure retained field tables", { tag: "@exclusive" }, async ({ page }) => {
  await page.goto("/e2e/diff-virtual-fixture.html");
  await page.getByRole("button", { name: "Group addresses (3300)" }).click();
  await page.getByRole("button", { name: "changed (300)", exact: true }).click();
  await page.evaluate(async () => {
    await document.fonts.ready;
    for (let i = 0; i < 4; i++) await new Promise(requestAnimationFrame);
  });
  const list = page.locator("ul.project-diff-entries");
  const padding = await list.getAttribute("style");
  await page.evaluate(() => {
    const state = window as unknown as { diffTableMeasurements: WeakMap<HTMLTableElement, number>;
      diffTableSnapshot: { table: HTMLTableElement; passes: number }[] };
    state.diffTableSnapshot = [...document.querySelectorAll<HTMLTableElement>('[data-table-id="project-diff-fields"] table')]
      .map(table => ({ table, passes: state.diffTableMeasurements.get(table)! }));
    document.querySelector(".project-diff-viewport")!.scrollTop = 600;
  });
  await expect(list).not.toHaveAttribute("style", padding!);
  const counts = await page.evaluate(async () => {
    for (let i = 0; i < 4; i++) await new Promise(requestAnimationFrame);
    const state = window as unknown as { diffTableMeasurements: WeakMap<HTMLTableElement, number>;
      diffTableSnapshot: { table: HTMLTableElement; passes: number }[] };
    return state.diffTableSnapshot.filter(({ table }) => table.isConnected)
      .map(({ table, passes }) => ({ before: passes, after: state.diffTableMeasurements.get(table)! }));
  });
  expect(counts.length).toBeGreaterThan(1);
  expect(counts.every(({ before }) => before > 0)).toBe(true);
  expect(counts.map(({ before, after }) => after - before)).toEqual(counts.map(() => 0));
});

test("intrinsic oracle preserves asymmetric borders, padding and nested control margins", async ({ page }) => {
  await page.goto("/e2e/table-columns-fixture.html?invalidation=1");
  const table = page.locator('[data-table-id="invalidation"]');
  const cell = table.locator("tbody td").first();
  await cell.evaluate(e => { (e as HTMLTableCellElement).colSpan = 1; e.innerHTML = '<button style="min-width:170px;padding:0;border:0;margin:0">x</button>'; });
  const baseline = (await intrinsicFloors(table))[0];
  await cell.evaluate(e => {
    e.style.borderLeft = "7px solid red"; e.style.borderRight = "11px dashed blue";
    e.style.paddingLeft = "17px"; e.style.paddingRight = "23px";
    const button = e.querySelector("button")!;
    button.style.marginLeft = "5px"; button.style.marginRight = "9px";
  });
  // 18 px borders + 16 px extra padding + 14 px nested margins.
  expect((await intrinsicFloors(table))[0] - baseline).toBe(48);
});

for (const scale of [0.8, 1.5]) {
  test(`scale ${scale}: layout-pixel fit, intact words, visual pointer tracking, keyboard, reset and restore`, async ({ page }) => {
    await page.setViewportSize({ width: Math.round(1440 * scale), height: 1000 });
    const server = await intercept(page, "en"); await openProject(page, "en");
    // Set root zoom before showing the table, so 100% session floors cannot
    // mask underestimated words at 80%. This is the property App itself sets.
    await page.evaluate(scale => { document.documentElement.style.setProperty("--app-ui-scale", String(scale)); }, scale);
    await openList(page, "en");
    const table = page.locator('[data-table-id="devices"]');
    await defaultFit(page, table, `en-zoom-${scale}-devices`, true);
    await wordsIntact(table.locator("thead .table-column-label"));
    await wordsIntact(table.locator("tbody td"));
    const handle = table.getByRole("separator").nth(1);
    const heading = table.locator("thead th").nth(1);
    const edge = async () => { const box = (await heading.boundingBox())!; return box.x + box.width; };
    const layoutWidth = async () => (await heading.boundingBox())!.width / scale;
    const initial = await layoutWidth();
    await handle.scrollIntoViewIfNeeded();
    const before = await edge();
    await drag(page, handle, 60);
    await preference(page, "devices", "address", 100 + 60 / scale);
    await expect.poll(async () => Math.abs(await edge() - before - 60), "60 visual pixels follow the pointer").toBeLessThan(1);
    await announcedWidth(table, 1);
    const dragged = await layoutWidth();
    await handle.press("ArrowRight");
    await preference(page, "devices", "address", 108 + 60 / scale);
    await expect.poll(async () => Math.abs(await layoutWidth() - dragged - 8)).toBeLessThan(1);
    await announcedWidth(table, 1);
    await handle.press("Enter");
    await preference(page, "devices", "address", 100);
    await expect.poll(async () => Math.abs(await layoutWidth() - initial)).toBeLessThan(1);
    await drag(page, handle, 60);
    await table.locator(".table-column-reset").click();
    await expect.poll(async () => Math.abs(await layoutWidth() - initial)).toBeLessThan(1);
    await drag(page, handle, 60);
    const savedLayoutWidth = await layoutWidth();
    const saved = 100 + 60 / scale;
    await preference(page, "devices", "address", saved);
    await page.setViewportSize({ width: 1440, height: 1000 });
    await page.reload();
    // A real button waits for React to mount after reload; a shortcut sent
    // immediately after navigation can precede the global key listener.
    await page.getByRole("button", { name: "Commands… (Ctrl+Shift+P)", exact: true }).click();
    await page.getByRole("combobox").fill("Devices");
    await page.getByRole("combobox").press("Enter");
    await expect(table).toBeVisible();
    await preference(page, "devices", "address", saved);
    await expect.poll(async () => Math.abs((await heading.boundingBox())!.width - savedLayoutWidth), "same saved layout width at 100% after reload").toBeLessThan(1);
    await announcedWidth(table, 1);
    expect(server.unexpected).toEqual([]);
  });
}

// UISET follow-up: the cases above write the root property directly. This one
// drives the scale only through the real `uiScale` setting (stored on the
// server, then the zoom shortcuts), so App's own effect, its narrow-window
// inspector stacking (`viewportWidth / uiScale`) and a startup with a stored
// scale run together with the table's measurement.
async function fitsOrScrollsLocally(page: Page, table: Locator, name: string) {
  const viewport = table.locator(".resizable-table-scroll");
  await announcedWidth(table, 1);
  const floors = await intrinsicFloors(table);
  const size = await viewport.evaluate(e => ({ scroll: e.scrollWidth, client: e.clientWidth }));
  const minimum = floors.reduce((a, b) => a + b, 0);
  const measurement = { name, ...size, minimum, floors };
  await test.info().attach(`${name}-fit-budget`, { body: JSON.stringify(measurement), contentType: "application/json" });
  if (minimum > size.client) {
    expect(Math.abs(size.scroll - minimum), JSON.stringify(measurement)).toBeLessThanOrEqual(2 * floors.length);
    await scrollsLocally(page, table);
  } else expect(size.scroll, JSON.stringify(measurement)).toBeLessThanOrEqual(size.client);
  await wordsIntact(table.locator("thead .table-column-label"));
  await wordsIntact(table.locator("tbody td"));
}

async function pointerTracks(page: Page, table: Locator, scale: number) {
  const handle = table.getByRole("separator").nth(1);
  const heading = table.locator("thead th").nth(1);
  const edge = async () => { const box = (await heading.boundingBox())!; return box.x + box.width; };
  await handle.scrollIntoViewIfNeeded();
  const before = await edge();
  await drag(page, handle, 60);
  await preference(page, "devices", "address", 100 + 60 / scale);
  await expect.poll(async () => Math.abs(await edge() - before - 60), "60 visual pixels follow the pointer").toBeLessThan(1);
  await announcedWidth(table, 1);
  await handle.press("Enter");
  await preference(page, "devices", "address", 100);
}

test("stored uiScale: App's own scale drives inspector stacking, table fit and drags; a reload remeasures at the stored scale", async ({ page }) => {
  await page.setViewportSize({ width: 1200, height: 1000 });
  const server = await intercept(page, "en");
  const settings: Record<string, unknown> = { uiLanguage: "en", theme: "porcelain", uiScale: 1.5 };
  const scales: unknown[] = [];
  // Registered after `intercept`, so it answers settings first.
  await page.route("**/api/settings", route => {
    if (route.request().method() === "PUT") {
      const patch = route.request().postDataJSON().settings as Record<string, unknown>;
      if ("uiScale" in patch) scales.push(patch.uiScale);
      Object.assign(settings, patch);
    } else if (route.request().method() !== "GET") server.unexpected.push(`${route.request().method()} /api/settings`);
    return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify({ schemaVersion: 1, status: "ok", settings }) });
  });
  const rootScale = () => page.evaluate(() => document.documentElement.style.getPropertyValue("--app-ui-scale"));
  const workbench = page.locator("main.workbench");
  const table = page.locator('[data-table-id="devices"]');
  await openProject(page, "en");
  await expect.poll(rootScale, "App applies the stored scale").toBe("1.5");
  await openList(page, "en");
  // 1200 / 1.5 = 800 layout px, below 250 + 360 + 200: stacked (not at 100%).
  await expect(workbench).toHaveClass(/workbench--stacked-inspector/);
  expect(Math.abs((await page.locator(".workbench-pane-right").boundingBox())!.width - 1200)).toBeLessThan(2);
  await fitsOrScrollsLocally(page, table, "en-setting-1.5-devices");
  await pointerTracks(page, table, 1.5);
  // The real shortcut path: 150% → 100% → 90% → 80%, each written as a setting.
  await table.getByRole("separator").nth(1).focus();
  for (const key of ["Control+0", "Control+Minus", "Control+Minus"]) await page.keyboard.press(key);
  await expect.poll(rootScale).toBe("0.8");
  await expect.poll(() => scales).toEqual([1, 0.9, 0.8]);
  // 1200 / 0.8 = 1500 layout px: side by side again, the mounted table remeasured.
  await expect(workbench).not.toHaveClass(/workbench--stacked-inspector/);
  await fitsOrScrollsLocally(page, table, "en-setting-0.8-mounted-devices");
  // Startup: the stored 80% is applied before the table mounts again.
  await page.reload();
  await expect.poll(rootScale, "startup applies the stored scale").toBe("0.8");
  await page.getByRole("button", { name: "Commands… (Ctrl+Shift+P)", exact: true }).click();
  await page.getByRole("combobox").fill("Devices");
  await page.getByRole("combobox").press("Enter");
  await expect(table).toBeVisible();
  await expect(workbench).not.toHaveClass(/workbench--stacked-inspector/);
  await fitsOrScrollsLocally(page, table, "en-setting-0.8-startup-devices");
  await pointerTracks(page, table, 0.8);
  expect(server.unexpected).toEqual([]);
});
