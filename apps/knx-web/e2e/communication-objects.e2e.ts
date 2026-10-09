/** Built-app communication table UX, mutation boundaries and synthetic load checks. */
import { expect, test } from "@playwright/test";
import { join } from "node:path";
import { writeFileSync } from "node:fs";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { intercept, openList, openProject } from "./communication-fixture";
for (const language of ["en", "de"] as const) for (const width of [1440, 400]) for (const theme of ["porcelain", "graphite", "lcars", "cupertino"]) {
  test(`${language} CO table headings/filter/sort/keyboard/fit ${theme} ${width}px`, async ({ page }) => {
    const m = language === "en" ? en : de;
    await page.setViewportSize({ width, height: 900 });
    const state = await intercept(page, language, 8, theme); await openProject(page, language); await openList(page, language);
    await page.locator(".devices-table tbody td:nth-child(3) button").first().click();
    const panel = page.locator(".com-table-panel");
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    await expect(panel.locator("thead th")).toHaveCount(6);
    await expect(panel.locator(".com-object-channel-summary").first()).toHaveAttribute("aria-expanded", "false");
    const before = JSON.stringify(state.tree), requestStart = state.requests.length;
    await panel.getByRole("searchbox", { name: m["comTable.search"] }).fill("boiler");
    await expect(panel.locator(".com-description-match")).toContainText("Hidden boiler marker");
    await expect(panel.locator("tr[data-object-id]:visible")).toHaveCount(1);
    await panel.getByRole("button", { name: m["comTable.resetFilters"], exact: true }).click();
    await expect(panel.locator(".com-object-channel-summary").first()).toHaveAttribute("aria-expanded", "false");
    await panel.getByRole("button", { name: m["comTable.flat"], exact: true }).click();
    await expect(panel.locator("thead th")).toHaveCount(7);
    await expect(panel.locator("tr[data-object-id]:visible")).toHaveCount(8);
    const selectedView = panel.locator('.com-table-views [aria-pressed="true"]');
    const colors = await selectedView.evaluate(node => { const css = getComputedStyle(node); return { color: css.color, background: css.backgroundColor, image: css.backgroundImage }; });
    expect(colors.image).toBe("none"); expect(colors.color).not.toBe(colors.background);
    if (process.env.KNX_CO_CAPTURE_DIR && theme === (width === 1440 ? "graphite" : "porcelain"))
      await page.screenshot({ path: join(process.env.KNX_CO_CAPTURE_DIR, `co-table-${language}-${width}.png`) });
    const sort = panel.locator('th[data-column="number"] button');
    await sort.focus(); await page.keyboard.press("Enter");
    await expect(panel.locator('th[data-column="number"]')).toHaveAttribute("aria-sort", "ascending");
    await page.keyboard.press("Enter");
    await expect(panel.locator('th[data-column="number"]')).toHaveAttribute("aria-sort", "descending");
    await expect(panel.locator("tr[data-object-id]:visible").first()).toHaveAttribute("data-object-id", "8");
    await panel.getByRole("combobox", { name: m["comTable.statusFilter"] }).selectOption("Inactive");
    await panel.getByRole("combobox", { name: m["comTable.linkFilter"] }).selectOption("unlinked");
    await panel.getByRole("combobox", { name: m["comTable.dptFilter"] }).selectOption("family:9");
    await expect(panel.locator("tr[data-object-id]:visible")).toHaveCount(1);
    await expect(panel.locator("tr[data-object-id]:visible")).toHaveAttribute("data-object-id", "2");
    await panel.getByRole("button", { name: m["comTable.resetFilters"], exact: true }).click();
    const scroll = panel.locator(".com-table-scroll");
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    await expect(scroll).toBeVisible();
    const alignment = await panel.locator("table").evaluate(table => {
      const head = [...table.querySelectorAll("thead th")].map(n => n.getBoundingClientRect());
      const row = [...table.querySelector("tr[data-object-id]:not([hidden])")!.children].map(n => n.getBoundingClientRect());
      return head.map((h, i) => Math.abs(h.left - row[i].left));
    });
    expect(alignment.every(delta => delta < 1)).toBe(true);
    const editor = panel.locator('[data-object-id="1"] summary');
    await editor.scrollIntoViewIfNeeded(); await editor.focus(); await page.keyboard.press("Enter");
    await expect(panel.locator('[data-editor-id="1"] input[placeholder="DPST-9-1"]')).toBeVisible();
    const badgeHeight=await panel.locator('[data-editor-id="1"] .provenance-badge').first().evaluate(n=>n.getBoundingClientRect().height);
    expect(badgeHeight, "provenance badges must not stretch to the flag editor height").toBeLessThan(40);
    if (process.env.KNX_CO_CAPTURE_DIR && theme === (width === 1440 ? "graphite" : "porcelain"))
      await page.screenshot({ path: join(process.env.KNX_CO_CAPTURE_DIR, `co-${language}-${width}.png`) });
    expect(JSON.stringify(state.tree)).toBe(before);
    expect(state.requests.slice(requestStart).filter(r => r.startsWith("POST ") && r !== "POST /api/achievements/record")).toEqual([]);
    expect(state.unexpected).toEqual([]);
  });
}
test("blur saves exactly once and a refused editor survives filtering and view changes", async ({ page }) => {
  const state = await intercept(page, "en"); await openProject(page, "en"); await openList(page, "en");
  await page.locator(".devices-table tbody td:nth-child(3) button").first().click();
  const panel = page.locator(".com-table-panel");
  await panel.getByRole("button", { name: en["comTable.flat"], exact: true }).click();
  await panel.locator('[data-object-id="1"] summary').click();
  const editor = panel.locator('[data-editor-id="1"] input[placeholder="DPST-9-1"]');
  const before = JSON.stringify(state.tree); state.refuseDpt = true;
  await editor.fill("invalid");
  // Focusing the search field triggers the existing input blur-save gesture.
  await panel.getByRole("searchbox").fill("Object 2");
  await expect(panel.locator('[data-editor-id="1"] .field-error')).toHaveText("Fictional refusal");
  await expect(panel.locator('[data-object-id="1"]')).toContainText(en["comTable.editException"]);
  await panel.getByRole("button", { name: en["comTable.grouped"], exact: true }).click();
  await expect(panel.locator('[data-editor-id="1"] .field-error')).toBeVisible();
  expect(state.requests.filter(r => r === "POST /api/com-object-dpt")).toHaveLength(1);
  expect(JSON.stringify(state.tree)).toBe(before); expect(state.unexpected).toEqual([]);
});
test("committed DPT re-evaluates the filter without losing the editing exception", async ({ page }) => {
  const state = await intercept(page, "en"); await openProject(page, "en"); await openList(page, "en");
  await page.locator(".devices-table tbody td:nth-child(3) button").first().click();
  const panel = page.locator(".com-table-panel");
  await panel.getByRole("combobox", { name: en["comTable.dptFilter"] }).selectOption("family:1");
  await panel.locator('[data-object-id="1"] summary').click();
  const editor = panel.locator('[data-editor-id="1"] input[placeholder="DPST-9-1"]');
  await editor.fill("DPST-5-1"); await editor.press("Tab");
  await expect(panel.locator('[data-object-id="1"]')).toContainText(en["comTable.editException"]);
  await expect(editor).toHaveValue("DPST-5-1");
  expect(state.requests.filter(r => r === "POST /api/com-object-dpt")).toHaveLength(1);
  await panel.locator('[data-object-id="1"] summary').click();
  await expect(panel.locator('[data-object-id="1"]')).toBeHidden(); expect(state.unexpected).toEqual([]);
});
test("production synthetic 1000-object device records visible-operation timings", async ({ page }) => {
  test.setTimeout(90_000);
  const state = await intercept(page, "en", 1000); await openProject(page, "en"); await openList(page, "en");
  let start = performance.now(); await page.locator(".devices-table tbody td:nth-child(3) button").first().click();
  await expect(page.locator(".com-table-count")).toHaveText("1000 / 1000 objects");
  const renderMs = performance.now() - start;
  const panel = page.locator(".com-table-panel");
  start = performance.now(); await panel.getByRole("button", { name: en["comTable.flat"], exact: true }).click();
  await expect(panel.locator("tr[data-object-id]:visible")).toHaveCount(1000); const flatMs = performance.now() - start;
  start = performance.now(); await panel.getByRole("searchbox").fill("Object 1000");
  await expect(panel.locator("tr[data-object-id]:visible")).toHaveCount(1); const filterMs = performance.now() - start;
  await panel.getByRole("button", { name: en["comTable.resetFilters"], exact: true }).click();
  start = performance.now(); await panel.locator('th[data-column="number"] button').click();
  await expect(panel.locator('th[data-column="number"]')).toHaveAttribute("aria-sort", "ascending"); const sortMs = performance.now() - start;
  if (process.env.KNX_CO_CAPTURE_DIR) writeFileSync(join(process.env.KNX_CO_CAPTURE_DIR, "performance.json"), JSON.stringify({
    scope: "Synthetic intercepted production app; one sample; browser-driver action-to-visible assertion wall time, not isolated JS duration",
    objects: 1000, renderMs, flatMs, filterMs, sortMs }, null, 2));
  expect(state.unexpected).toEqual([]);
});
