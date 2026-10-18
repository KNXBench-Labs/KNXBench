/** Browser proofs for local column resizing, sortable devices and virtualised history diffs. */
import { expect, test, type Locator, type Page } from "@playwright/test";

async function drag(page: Page, handle: Locator, delta: number) {
  await handle.scrollIntoViewIfNeeded(); const box = (await handle.boundingBox())!;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down(); await page.mouse.move(box.x + box.width / 2 + delta, box.y + box.height / 2, { steps: 5 }); await page.mouse.up();
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
    await drag(page, handle, 48); await expect(handle).toHaveAttribute("aria-valuenow", String(initial + 48));
    await expect(sorting).toHaveAttribute("aria-sort", "ascending"); await expect(page.locator("[data-row-clicks]")).toHaveText("0");
    await handle.focus(); await page.keyboard.press("Shift+ArrowRight"); await expect(handle).toHaveAttribute("aria-valuenow", String(initial + 88));
    await aligned(devices, 1);
    // Header sticks inside the resized table's actual vertical scroller.
    const scroller = devices.locator(".resizable-table-scroll");
    await scroller.evaluate(element => { element.scrollTop = 200; });
    const top = (await scroller.boundingBox())!.y;
    expect(Math.abs((await sorting.boundingBox())!.y - top)).toBeLessThan(3);
    await page.reload(); await expect(handle).toHaveAttribute("aria-valuenow", String(initial + 88));
    await handle.press("Home"); await expect(handle).toHaveAttribute("aria-valuenow", "100");
    await handle.press("ArrowLeft"); await expect(handle).toHaveAttribute("aria-valuenow", "100");
    await handle.press("Enter"); await expect(handle).toHaveAttribute("aria-valuenow", String(initial));
    await sorting.getByRole("button").click(); await expect(sorting).toHaveAttribute("aria-sort", "descending");
    await handle.press("ArrowRight"); await devices.locator(".table-column-reset").click(); await expect(handle).toHaveAttribute("aria-valuenow", String(initial));
    const comparison = page.locator('[data-table-id="device-comparison"]');
    const other = comparison.getByRole("separator").first();
    await drag(page, other, 32); await expect(other).toHaveAttribute("aria-valuenow", "272");
    await other.press("ArrowLeft"); await expect(other).toHaveAttribute("aria-valuenow", "264");
    await page.reload(); await expect(other).toHaveAttribute("aria-valuenow", "264");
    await aligned(comparison, 0);
    await page.screenshot({ path: test.info().outputPath(`${theme}-${width}.png`), fullPage: true });
    await page.setViewportSize({ width: 400, height: 900 });
    await localOverflow(page, devices); await localOverflow(page, comparison);
    await other.dblclick(); await expect(other).toHaveAttribute("aria-valuenow", "240");
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
  await drag(page, handle, 48); await expect(handle).toHaveAttribute("aria-valuenow", "228");
  await handle.press("ArrowRight"); await expect(handle).toHaveAttribute("aria-valuenow", "236");
  await expect(tables.nth(1).getByRole("separator").first()).toHaveAttribute("aria-valuenow", "236");
  await aligned(tables.first(), 0); await localOverflow(page, tables.first());
  const viewport = page.locator(".project-diff-viewport"); await viewport.focus(); await page.keyboard.press("End");
  await expect(page.locator("li[aria-posinset]").last()).toHaveAttribute("aria-posinset", "300");
  await expect(tables.last().getByRole("separator").first()).toHaveAttribute("aria-valuenow", "236");
  await aligned(tables.last(), 0);
  expect(await page.locator("li[aria-posinset]").count()).toBeLessThan(120);
  await page.reload(); await page.getByRole("button", { name: "Group addresses (3300)" }).click();
  await page.getByRole("button", { name: "changed (300)", exact: true }).click();
  await expect(handle).toHaveAttribute("aria-valuenow", "236");
  await tables.first().locator(".table-column-reset").click();
  await expect(tables.nth(1).getByRole("separator").first()).toHaveAttribute("aria-valuenow", "180");
});
