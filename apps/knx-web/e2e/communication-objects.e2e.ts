/** Built-app communication table UX, mutation boundaries and synthetic load checks. */
import { expect, test } from "@playwright/test";
import { join } from "node:path";
import { writeFileSync } from "node:fs";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { intercept, openList, openProject } from "./communication-fixture";
// Issue #9 (devices view): a channel header reads "▾ name · count" from the left,
// and its objects start one level right of the header's marker.
for (const language of ["en", "de"] as const) for (const width of [1440, 400]) for (const theme of ["porcelain", "graphite"]) {
  test(`${language} CO channel header is left-aligned and its objects indented ${theme} ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    const state = await intercept(page, language, 8, theme); await openProject(page, language); await openList(page, language);
    await page.locator(".devices-table tbody td:nth-child(3) button").first().click();
    const panel = page.locator(".com-table-panel");
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    const header = panel.locator(".com-object-channel-summary").first();
    await header.click(); await expect(header).toHaveAttribute("aria-expanded", "true");
    const key = await header.locator("xpath=ancestor::tr").getAttribute("data-channel-key");
    if (process.env.KNX_CO_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_CO_CAPTURE_DIR, `co-indent-${language}-${width}-${theme}.png`) });
    const geometry = await panel.evaluate((root, channel) => {
      const button = root.querySelector<HTMLElement>(`tr[data-channel-key="${channel}"] .com-object-channel-summary`)!;
      const parts = [...button.children].map(n => n.getBoundingClientRect()).filter(r => r.width > 0);
      // Gaps between neighbours that share a line; a wrapped part starts a new line.
      const gaps = parts.slice(1).flatMap((r, i) => Math.abs(r.top - parts[i].top) < 4 ? [r.left - parts[i].right] : []);
      const first = root.querySelector<HTMLElement>(`tr.com-object-summary[data-channel-key="${channel}"]:not([hidden]) summary`)!;
      return { marker: parts[0].left, gaps, object: first.getBoundingClientRect().left };
    }, key);
    expect(geometry.gaps.length, "header parts measured").toBeGreaterThan(0);
    for (const gap of geometry.gaps) expect(gap, "header parts follow each other from the left").toBeLessThan(24);
    expect(geometry.object, "object right of the channel marker").toBeGreaterThan(geometry.marker + 8);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    expect(state.unexpected).toEqual([]);
  });
}

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
    // The shared button background transitions for 120ms; inspect the settled style.
    const colors = await selectedView.evaluate(async node => {
      await Promise.all(node.getAnimations().map(animation => animation.finished));
      const css = getComputedStyle(node);
      return { color: css.color, background: css.backgroundColor, image: css.backgroundImage };
    });
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
    await expect(panel.locator('[data-editor-id="1"] .dpt-field select')).toBeVisible();
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
  const editor = panel.locator('[data-editor-id="1"] .dpt-field select');
  const before = JSON.stringify(state.tree); state.refuseDpt = true;
  await editor.focus(); await editor.selectOption("DPST-9-2");
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
  const editor = panel.locator('[data-editor-id="1"] .dpt-field select');
  await editor.focus(); await editor.selectOption("DPST-5-1"); await editor.press("Tab");
  await expect(panel.locator('[data-object-id="1"]')).toContainText(en["comTable.editException"]);
  await expect(editor).toHaveValue("DPST-5-1");
  expect(state.requests.filter(r => r === "POST /api/com-object-dpt")).toHaveLength(1);
  await panel.locator('[data-object-id="1"] summary').click();
  await expect(panel.locator('[data-object-id="1"]')).toBeHidden(); expect(state.unexpected).toEqual([]);
});
test("production synthetic 1000-object device records visible-operation timings", { tag: "@exclusive" }, async ({ page }) => {
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

// Issue #16: collapsing a channel hides its open object rows; reopening shows them again.
for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} ${width}px: a collapsed channel hides its open editor and keeps it for reopening`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    const state = await intercept(page, language); await openProject(page, language); await openList(page, language);
    await page.locator(".devices-table tbody td:nth-child(3) button").first().click();
    const panel = page.locator(".com-table-panel");
    const key = await panel.locator('tr[data-object-id="1"]').getAttribute("data-channel-key");
    const header = panel.locator(`tr.com-object-channel[data-channel-key="${key}"] .com-object-channel-summary`);
    const siblings = panel.locator(`tr.com-object-summary[data-channel-key="${key}"]`);
    await header.click(); await expect(header).toHaveAttribute("aria-expanded", "true");
    const total = await siblings.count();
    expect(total, "the channel holds the opened object").toBeGreaterThan(0);
    await panel.locator('[data-object-id="1"] summary').click();
    const editor = panel.locator('[data-editor-id="1"] .dpt-field select');
    await expect(editor).toBeVisible();
    await editor.focus(); await editor.selectOption("DPST-9-2");
    const handle = await editor.elementHandle();
    // The header click blurs the select first: the existing blur-save runs once.
    await header.click(); await expect(header).toHaveAttribute("aria-expanded", "false");
    await expect(panel.locator('tr[data-object-id="1"]')).toBeHidden();
    await expect(panel.locator('tr[data-editor-id="1"]')).toBeHidden();
    await expect(siblings.locator("visible=true")).toHaveCount(0);
    expect(await handle!.evaluate((node) => node.isConnected), "editor stays mounted while hidden").toBe(true);
    if (process.env.KNX_CO_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_CO_CAPTURE_DIR, `co-collapse-${language}-${width}.png`) });
    await header.click(); await expect(header).toHaveAttribute("aria-expanded", "true");
    await expect(editor).toBeVisible();
    await expect(editor).toHaveValue("DPST-9-2");
    expect(await editor.evaluate((node, kept) => node === kept, handle), "the same editor returns").toBe(true);
    await expect(siblings.locator("visible=true")).toHaveCount(total);
    expect(state.requests.filter((r) => r === "POST /api/com-object-dpt")).toHaveLength(1);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    expect(state.unexpected).toEqual([]);
  });
}

// Issue #17 (ADR-0126): the server fills a product name's `{{0}}`; the table
// shows the result and the tooltip says where it came from. Fictional texts.
for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} ${width}px: a filled product name and heading say where their text came from`, async ({ page }) => {
    const m = language === "en" ? en : de;
    await page.setViewportSize({ width, height: 900 });
    const state = await intercept(page, language, 8);
    const heading = "Ground floor kitchen and dining area, north side";
    state.objects[0] = { ...state.objects[0], name: `Page 1 - ${heading}`,
      name_fill: { template: "Page 1 - {{0:Icon 1}}", source: "StoredValue" },
      channel: { ...state.objects[0].channel!, text: heading, text_fill: { template: "{{0:Page 1}}", source: "StoredValue" } } };
    state.objects[4] = { ...state.objects[4], name: "Dim {{0}}", name_fill: { template: "Dim {{0}}", source: "NoReference" } };
    await openProject(page, language); await openList(page, language);
    await page.locator(".devices-table tbody td:nth-child(3) button").first().click();
    const panel = page.locator(".com-table-panel");
    const header = panel.locator(".com-object-channel-summary", { hasText: heading });
    await expect(header.locator("strong")).toHaveAttribute("title",
      `${m["comTable.textFill.StoredValue"]}\n${m["comTable.textFill.template"].replace("{value}", "{{0:Page 1}}")}`);
    await header.click(); await expect(header).toHaveAttribute("aria-expanded", "true");
    const filled = panel.locator('tr[data-object-id="1"] .com-object-name');
    await expect(filled).toHaveText(`Page 1 - ${heading}`);
    await expect(filled).toHaveAttribute("data-text-fill", "StoredValue");
    if (process.env.KNX_CO_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_CO_CAPTURE_DIR, `co-text-fill-${language}-${width}-grouped.png`) });
    await panel.getByRole("button", { name: m["comTable.flat"], exact: true }).click();
    const unfilled = panel.locator('tr[data-object-id="5"] .com-object-name');
    await expect(unfilled).toHaveText("Dim {{0}}");
    await expect(unfilled).toHaveAttribute("title", new RegExp(m["comTable.textFill.NoReference"].replace(/[.*+?^${}()|[\]\\]/g, "\\$&")));
    // A name without a fill record has no tooltip.
    await expect(panel.locator('tr[data-object-id="2"] .com-object-name')).not.toHaveAttribute("title", /./);
    if (process.env.KNX_CO_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_CO_CAPTURE_DIR, `co-text-fill-${language}-${width}.png`) });
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    expect(state.unexpected).toEqual([]);
  });
}
