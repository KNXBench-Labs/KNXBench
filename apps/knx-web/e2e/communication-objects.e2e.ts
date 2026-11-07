/** Built-app communication table UX, mutation boundaries and synthetic load checks. */
import { expect, test } from "@playwright/test";
import { join } from "node:path";
import { writeFileSync } from "node:fs";

import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { intercept, openList, openProject } from "./communication-fixture";
import type { ComObjectProjectFolder } from "../src/bindings/ComObjectProjectFolder";
import type { ProjectTreeNode } from "../src/bindings/ProjectTreeNode";
import type { ProductFolderCaption } from "../src/bindings/ProductFolderCaption";
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

// WP-06a: a saved project folder shows by its declared ids; a withheld one says why.
for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} ${width}px: a saved project folder and a withheld one are named`, async ({ page }) => {
    const m = language === "en" ? en : de;
    await page.setViewportSize({ width, height: 900 });
    const state = await intercept(page, language, 8);
    state.objects[0] = { ...state.objects[0], project_folder: { status: "Assigned", key: "Channel:4:CH-1/Folder:4:PB-3",
      path: [{ kind: "Channel", ref_id: "CH-1" }, { kind: "Folder", ref_id: "PB-3" }] } };
    state.objects[1] = { ...state.objects[1], project_folder: { status: "Ambiguous", key: null, path: [] } };
    state.objects[2] = { ...state.objects[2], project_folder: { status: "NoFolder", key: null, path: [] } };
    await openProject(page, language); await openList(page, language);
    await page.locator(".devices-table tbody td:nth-child(3) button").first().click();
    const panel = page.locator(".com-table-panel");
    const requestStart = state.requests.length;
    await panel.getByRole("button", { name: m["comTable.flat"], exact: true }).click();
    const note = (id: number) => panel.locator(`tr[data-object-id="${id}"] .com-object-project-folder`);
    await expect(note(1)).toHaveText(m["comTable.projectFolder"].replace("{value}", "PB-3"));
    await expect(note(1)).toHaveAttribute("title", m["comTable.projectFolderTitle"].replace("{value}", "CH-1 › PB-3"));
    await expect(note(2)).toHaveText(m["comTable.projectFolderRefused.Ambiguous"]);
    await expect(note(3)).toHaveCount(0);
    await expect(note(4)).toHaveCount(0);
    await panel.getByRole("searchbox", { name: m["comTable.search"] }).fill("PB-3");
    await expect(panel.locator("tr.com-object-summary:visible")).toHaveCount(1);
    await expect(panel.locator('tr.com-object-summary[data-object-id="1"]')).toBeVisible();
    if (process.env.KNX_CO_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_CO_CAPTURE_DIR, `co-project-folder-${language}-${width}.png`) });
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    // Showing and searching folders is view-only: no write reaches the server.
    expect(state.requests.slice(requestStart).filter(r => !r.startsWith("GET ") && r !== "POST /api/achievements/record")).toEqual([]);
    expect(state.unexpected).toEqual([]);
  });
}

// WP-06 folder grouping: groups by the opaque saved-folder key; equal ids under
// different keys stay apart and say so visibly; everything else is the fallback.
const F1 = "folder:Channel:4:CH-1/Folder:4:PB-3", F2 = "folder:Channel:4:CH-2/Folder:4:PB-3";
for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} ${width}px: grouping by saved project folder keeps colliding ids apart and stays view-only`, async ({ page }) => {
    const m = language === "en" ? en : de;
    await page.setViewportSize({ width, height: 900 });
    const state = await intercept(page, language, 8);
    const pb3 = (ch: string) => ({ status: "Assigned" as const, key: `Channel:4:${ch}/Folder:4:PB-3`,
      path: [{ kind: "Channel", ref_id: ch }, { kind: "Folder", ref_id: "PB-3" }] });
    state.objects[0] = { ...state.objects[0], project_folder: pb3("CH-1") };
    state.objects[1] = { ...state.objects[1], project_folder: pb3("CH-2") };
    state.objects[2] = { ...state.objects[2], project_folder: { status: "Ambiguous", key: null, path: [] } };
    state.objects[3] = { ...state.objects[3], project_folder: { status: "NoFolder", key: null, path: [] } };
    state.objects[4] = { ...state.objects[4], project_folder: pb3("CH-1") };
    await openProject(page, language); await openList(page, language);
    await page.locator(".devices-table tbody td:nth-child(3) button").first().click();
    const panel = page.locator(".com-table-panel");
    const before = JSON.stringify(state.tree), requestStart = state.requests.length;
    const views = panel.locator(".com-table-views button");
    await expect(views).toHaveText([m["comTable.grouped"], m["comTable.byFolder"], m["comTable.flat"]]);
    await panel.getByRole("button", { name: m["comTable.byFolder"], exact: true }).click();
    await expect(views.nth(0)).toHaveAttribute("aria-pressed", "false");
    await expect(views.nth(1)).toHaveAttribute("aria-pressed", "true");
    await expect(views.nth(2)).toHaveAttribute("aria-pressed", "false");
    await expect(panel.locator("thead th")).toHaveCount(6);
    await expect(panel.locator(".com-folder-view-note")).toHaveText(m["comTable.folderViewNote"]);
    const headers = panel.locator("tr.com-object-channel");
    expect(await headers.evaluateAll(rows => rows.map(r => [r.getAttribute("data-group-key"), r.hasAttribute("data-channel-key")])))
      .toEqual([[F1, false], [F2, false], ["folder-none", false]]);
    const header = (key: string) => panel.locator(`tr.com-object-channel[data-group-key="${key}"] .com-object-channel-summary`);
    for (const key of [F1, F2, "folder-none"]) await expect(header(key)).toHaveAttribute("aria-expanded", "false");
    // Same Folder id, different saved paths: visibly distinct without a tooltip.
    await expect(header(F1).locator("strong")).toHaveText("PB-3");
    await expect(header(F2).locator("strong")).toHaveText("PB-3");
    await expect(header(F1).locator(".com-object-folder-path")).toBeVisible();
    await expect(header(F1).locator(".com-object-folder-path")).toHaveText(m["comTable.folderPath"].replace("{value}", "CH-1 › PB-3"));
    await expect(header(F2).locator(".com-object-folder-path")).toHaveText(m["comTable.folderPath"].replace("{value}", "CH-2 › PB-3"));
    await expect(header("folder-none").locator("strong")).toHaveText(m["comTable.noFolderGroup"]);
    await expect(panel.locator("tr.com-object-summary:visible")).toHaveCount(0);
    await header(F1).click(); await header("folder-none").click();
    expect(await panel.locator("tr.com-object-summary:visible").evaluateAll(rows => rows.map(r => r.getAttribute("data-object-id"))))
      .toEqual(["1", "5", "3", "4", "6", "7", "8"]);
    await expect(panel.locator('tr[data-object-id="1"]')).toHaveAttribute("data-group-key", F1);
    await expect(panel.locator('tr[data-object-id="1"]')).toHaveAttribute("data-channel-key", "channel:channel-0");
    await expect(panel.locator('tr[data-object-id="3"] .com-object-project-folder')).toHaveText(m["comTable.projectFolderRefused.Ambiguous"]);
    if (process.env.KNX_CO_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_CO_CAPTURE_DIR, `co-folder-groups-${language}-${width}.png`) });
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    await header("folder-none").click();
    await panel.getByRole("searchbox", { name: m["comTable.search"] }).fill("PB-3");
    await expect(header(F2)).toHaveAttribute("aria-expanded", "true");
    await expect(panel.locator('tr.com-object-channel[data-group-key="folder-none"]')).toBeHidden();
    expect(await panel.locator("tr.com-object-summary:visible").evaluateAll(rows => rows.map(r => r.getAttribute("data-object-id"))))
      .toEqual(["1", "5", "2"]);
    if (process.env.KNX_CO_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_CO_CAPTURE_DIR, `co-folder-search-${language}-${width}.png`) });
    await panel.getByRole("button", { name: m["comTable.resetFilters"], exact: true }).click();
    await expect(header(F1)).toHaveAttribute("aria-expanded", "true");
    await expect(header(F2)).toHaveAttribute("aria-expanded", "false");
    await expect(header("folder-none")).toHaveAttribute("aria-expanded", "false");
    // The channel view keeps its own collapse state.
    await panel.getByRole("button", { name: m["comTable.grouped"], exact: true }).click();
    await expect(panel.locator(".com-object-channel-summary").first()).toHaveAttribute("aria-expanded", "false");
    await expect(panel.locator(".com-folder-view-note")).toHaveCount(0);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    expect(JSON.stringify(state.tree)).toBe(before);
    expect(state.requests.slice(requestStart).filter(r => !r.startsWith("GET ") && r !== "POST /api/achievements/record")).toEqual([]);
    expect(state.unexpected).toEqual([]);
  });
}
test("a refused editor survives switching through the folder view with exactly one save", async ({ page }) => {
  const state = await intercept(page, "en"); await openProject(page, "en"); await openList(page, "en");
  await page.locator(".devices-table tbody td:nth-child(3) button").first().click();
  const panel = page.locator(".com-table-panel");
  await panel.getByRole("button", { name: en["comTable.flat"], exact: true }).click();
  await panel.locator('[data-object-id="1"] summary').click();
  const editor = panel.locator('[data-editor-id="1"] .dpt-field select');
  const before = JSON.stringify(state.tree); state.refuseDpt = true;
  await editor.focus(); await editor.selectOption("DPST-9-2");
  const handle = await editor.elementHandle();
  // Focusing the search field triggers the existing input blur-save gesture.
  await panel.getByRole("searchbox").fill("Object 2");
  await expect(panel.locator('[data-editor-id="1"] .field-error')).toHaveText("Fictional refusal");
  for (const view of ["comTable.byFolder", "comTable.grouped", "comTable.byFolder"] as const) {
    await panel.getByRole("button", { name: en[view], exact: true }).click();
    await expect(panel.locator('[data-editor-id="1"] .field-error')).toHaveText("Fictional refusal");
    expect(await editor.evaluate((node, kept) => node === kept, handle), "the same editor stays mounted").toBe(true);
  }
  expect(state.requests.filter(r => r === "POST /api/com-object-dpt")).toHaveLength(1);
  expect(JSON.stringify(state.tree)).toBe(before); expect(state.unexpected).toEqual([]);
});

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} ${width}px: saved schema20 literal folder text is inert, searchable and never identity`, async ({ page }) => {
    const m = language === "en" ? en : de;
    await page.setViewportSize({ width, height: 900 });
    const state = await intercept(page, language, 8);
    const literal = '<img src=x onerror="alert(1)"> {{ChNo}} & {{0}}';
    const folder = (channel: string, text: string): ComObjectProjectFolder => ({
      status: "Assigned", key: `Channel:4:${channel}/Folder:4:PB-3`,
      path: [{ kind: "Channel", ref_id: channel, text: "Channel-only secret" }, { kind: "Folder", ref_id: "PB-3", text }],
    });
    const fallback = (ref_id: string, text?: string): ComObjectProjectFolder => ({
      status: "Assigned", key: `Folder:4:${ref_id}`, path: [{ kind: "Folder", ref_id, text }],
    });
    state.objects[0] = { ...state.objects[0], project_folder: folder("CH-1", literal) };
    state.objects[1] = { ...state.objects[1], activation: "NotEvaluated", project_folder: folder("CH-2", literal) };
    state.objects[2] = { ...state.objects[2], project_folder: folder("CH-1", "Captiononly {{ChNo}}") };
    state.objects[3] = { ...state.objects[3], project_folder: fallback("PB-4", "   ") };
    state.objects[4] = { ...state.objects[4], project_folder: fallback("PB-5", "") };
    state.objects[5] = { ...state.objects[5], activation: "NotEvaluated", project_folder: fallback("PB-6") };
    state.objects[6] = { ...state.objects[6], project_folder: { status: "ModuleObject", key: null, path: [] } };
    state.objects[7] = { ...state.objects[7], project_folder: { status: "NoFolder", key: null, path: [] } };
    await openProject(page, language); await openList(page, language);
    await page.locator(".devices-table tbody td:nth-child(3) button").first().click();
    const panel = page.locator(".com-table-panel");
    const before = JSON.stringify({ tree: state.tree, objects: state.objects }), requestStart = state.requests.length;
    const header = (key: string) => panel.locator(`tr.com-object-channel[data-group-key="${key}"] .com-object-channel-summary`);
    const search = panel.getByRole("searchbox", { name: m["comTable.search"] });
    await panel.getByRole("button", { name: m["comTable.byFolder"], exact: true }).click();
    expect(await panel.locator("tr.com-object-channel").evaluateAll(rows => rows.map(r => r.getAttribute("data-group-key"))))
      .toEqual([F1, F2, "folder:Folder:4:PB-4", "folder:Folder:4:PB-5", "folder:Folder:4:PB-6", "folder-none"]);
    for (const [key, channel] of [[F1, "CH-1"], [F2, "CH-2"]]) {
      await expect(header(key).locator("strong")).toHaveText(`${literal} (PB-3)`);
      await expect(header(key).locator(".com-object-folder-path")).toBeVisible();
      await expect(header(key).locator(".com-object-folder-path")).toHaveText(m["comTable.folderPath"].replace("{value}", `${channel} › PB-3`));
    }
    for (const id of [4, 5, 6]) await expect(header(`folder:Folder:4:PB-${id}`).locator("strong")).toHaveText(`PB-${id}`);
    await expect(panel.locator(".com-folder-view-note")).toHaveText(m["comTable.folderViewNote"]);
    await expect(panel.locator("img, script, b, [data-text-fill]")).toHaveCount(0);
    await header(F1).focus(); await page.keyboard.press("Enter");
    await expect(header(F1)).toHaveAttribute("aria-expanded", "true");
    await expect(panel.locator('tr[data-object-id="1"]')).toBeVisible();
    await expect(panel.locator('tr[data-object-id="3"]')).toBeVisible();
    await expect(panel.locator('tr[data-object-id="1"] .com-object-project-folder')).toContainText(m["comTable.projectFolder"].replace("{value}", `${literal} (PB-3)`));
    await expect(panel.locator('tr[data-object-id="1"] .com-object-folder-path')).toHaveText(m["comTable.folderPath"].replace("{value}", "CH-1 › PB-3"));
    if (process.env.KNX_CO_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_CO_CAPTURE_DIR, `co-saved-text-${language}-${width}.png`) });
    await search.fill("{{ChNo}}");
    expect(await panel.locator("tr.com-object-summary:visible").evaluateAll(rows => rows.map(r => r.getAttribute("data-object-id"))))
      .toEqual(["1", "3", "2"]);
    await search.fill("Captiononly");
    await expect(panel.locator("tr.com-object-summary:visible")).toHaveCount(1);
    await expect(panel.locator('tr[data-object-id="3"]')).toBeVisible();
    await expect(header(F1).locator(".com-object-channel-count")).toHaveText(m["comTable.count.other"].replace("{shown}", "1").replace("{total}", "2"));
    await search.fill("PB-3");
    await expect(panel.locator("tr.com-object-summary:visible")).toHaveCount(3);
    await search.fill("Channel-only secret");
    await expect(panel.locator("tr.com-object-summary:visible")).toHaveCount(0);
    await panel.getByRole("button", { name: m["comTable.resetFilters"], exact: true }).click();
    await expect(header(F1)).toHaveAttribute("aria-expanded", "true");
    await expect(header(F2)).toHaveAttribute("aria-expanded", "false");
    await panel.getByRole("button", { name: m["comTable.flat"], exact: true }).click();
    await expect(panel.locator("thead th")).toHaveCount(7);
    await expect(panel.locator('tr[data-object-id="7"] .com-object-project-folder')).toHaveText(m["comTable.projectFolderRefused.ModuleObject"]);
    await expect(panel.locator('tr[data-object-id="8"] .com-object-project-folder')).toHaveCount(0);
    await search.fill("PB-6");
    await expect(panel.locator('tr[data-object-id="6"]')).toBeVisible();
    await expect(panel.locator("tr.com-object-summary:visible")).toHaveCount(1);
    await expect(panel.locator('tr[data-object-id="6"] .com-object-project-folder')).toHaveText(m["comTable.projectFolder"].replace("{value}", "PB-6"));
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    expect(JSON.stringify({ tree: state.tree, objects: state.objects })).toBe(before);
    expect(state.requests.slice(requestStart).filter(r => !r.startsWith("GET ") && r !== "POST /api/achievements/record")).toEqual([]);
    expect(state.unexpected).toEqual([]);
  });
}

const declaredFolderCaption = (text: string, refId = "PB-3"): ProductFolderCaption => ({
  source: "ProductDeclaration", text, application_program_id: "M-FA01_A-0100-01-0000",
  parameter_block_id: `M-FA01_A-0100-01-0000_${refId}`, source_sha256: "ab".repeat(32),
});
for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} ${width}px: literal product folder declarations respect saved precedence and stay view-only`, async ({ page }) => {
    const m = language === "en" ? en : de;
    await page.setViewportSize({ width, height: 900 });
    const state = await intercept(page, language, 10);
    const literal = '<img src=x onerror="alert(1)"> & Declarationonly';
    const product = declaredFolderCaption(literal);
    const hidden = declaredFolderCaption("Unshown-product-secret");
    const assigned = (key: string, path: ProjectTreeNode[]): ComObjectProjectFolder => ({ status: "Assigned", key, path });
    const productPath = (channel: string): ProjectTreeNode[] => [
      { kind: "Channel", ref_id: channel, product_caption: declaredFolderCaption("Channel-product-secret") },
      { kind: "Folder", ref_id: "PB-3", product_caption: product },
    ];
    const future: ProjectTreeNode = { kind: "Folder", ref_id: "PB-U" };
    const malformed: ProjectTreeNode = { kind: "Folder", ref_id: "PB-M" };
    // These deliberately malformed JSON additions must not gain authority through a typed base DTO.
    Object.defineProperty(future, "product_caption", { value: { ...hidden, text: "Future-product-secret", source: "FutureDeclaration" }, enumerable: true });
    Object.defineProperty(malformed, "product_caption", { value: { source: "ProductDeclaration", text: "Malformed-product-secret" }, enumerable: true });
    const folders: ComObjectProjectFolder[] = [
      assigned("Channel:4:CH-1/Folder:4:PB-3", productPath("CH-1")),
      assigned("Channel:4:CH-2/Folder:4:PB-3", productPath("CH-2")),
      assigned("saved", [{ kind: "Folder", ref_id: "PB-S", text: "Saved wins", product_caption: hidden }]),
      assigned("empty", [{ kind: "Folder", ref_id: "PB-E", text: "", product_caption: hidden }]),
      assigned("space", [{ kind: "Folder", ref_id: "PB-W", text: " \t\n", product_caption: hidden }]),
      assigned("legacy", [{ kind: "Folder", ref_id: "PB-L" }]),
      assigned("future", [future]),
      { status: "ModuleObject", key: null, path: [{ kind: "Folder", ref_id: "REFUSED-ID", product_caption: declaredFolderCaption("Refused-product-secret") }] },
      assigned("channel-only", [{ kind: "Channel", ref_id: "CH-SECRET", product_caption: declaredFolderCaption("Channel-product-secret") }]),
      assigned("malformed", [malformed]),
    ];
    folders.forEach((folder, index) => {
      state.objects[index] = { ...state.objects[index], activation: "Active", project_folder: folder };
    });
    await openProject(page, language); await openList(page, language);
    await page.locator(".devices-table tbody td:nth-child(3) button").first().click();
    const panel = page.locator(".com-table-panel");
    const before = JSON.stringify({ tree: state.tree, objects: state.objects }), requestStart = state.requests.length;
    const header = (key: string) => panel.locator(`tr.com-object-channel[data-group-key="${key}"] .com-object-channel-summary`);
    const row = (id: number) => panel.locator(`tr.com-object-summary[data-object-id="${id}"]`);
    const search = panel.getByRole("searchbox", { name: m["comTable.search"] });
    await panel.getByRole("button", { name: m["comTable.byFolder"], exact: true }).click();
    expect(await panel.locator("tr.com-object-channel").evaluateAll(rows => rows.map(node => node.getAttribute("data-group-key"))))
      .toEqual([F1, F2, "folder:saved", "folder:empty", "folder:space", "folder:legacy", "folder:future", "folder:malformed", "folder-none"]);
    for (const [key, channel] of [[F1, "CH-1"], [F2, "CH-2"]]) {
      await expect(header(key).locator("strong")).toHaveText(`${literal} (PB-3)`);
      const badge = header(key).locator('[data-folder-caption-source="ProductDeclaration"]');
      await expect(badge).toBeVisible();
      await expect(badge).toHaveText(m["comTable.productFolderSource"].replace("{id}", "PB-3"));
      await expect(badge).toHaveAttribute("title", m["comTable.productFolderTitle"]
        .replace("{program}", product.application_program_id).replace("{block}", product.parameter_block_id).replace("{sha256}", product.source_sha256));
      await expect(header(key).locator(".com-object-folder-path")).toBeVisible();
      await expect(header(key).locator(".com-object-folder-path")).toHaveText(m["comTable.folderPath"].replace("{value}", `${channel} › PB-3`));
    }
    for (const [key, label] of [["saved", "Saved wins (PB-S)"], ["empty", "PB-E"], ["space", "PB-W"], ["legacy", "PB-L"], ["future", "PB-U"], ["malformed", "PB-M"]]) {
      await expect(header(`folder:${key}`).locator("strong")).toHaveText(label);
      await expect(header(`folder:${key}`).locator("[data-folder-caption-source]")).toHaveCount(0);
    }
    await expect(panel.locator("img, script, [data-text-fill]")).toHaveCount(0);
    await expect(panel.locator(".com-folder-view-note")).toHaveText(m["comTable.folderViewNote"]);
    await header(F1).focus(); await page.keyboard.press("Enter");
    await expect(header(F1)).toHaveAttribute("aria-expanded", "true");
    await expect(row(1)).toBeVisible();
    await expect(row(1).locator("[data-folder-caption-source]")).toHaveText(m["comTable.productFolderSource"].replace("{id}", "PB-3"));
    await expect(row(1).locator(".com-object-folder-path")).toHaveText(m["comTable.folderPath"].replace("{value}", "CH-1 › PB-3"));
    if (process.env.KNX_CO_CAPTURE_DIR) await panel.screenshot({ path: join(process.env.KNX_CO_CAPTURE_DIR, `co-product-folder-${language}-${width}.png`) });
    await search.fill("Declarationonly");
    expect(await panel.locator("tr.com-object-summary:visible").evaluateAll(rows => rows.map(node => node.getAttribute("data-object-id")))).toEqual(["1", "2"]);
    await search.fill("PB-3");
    await expect(panel.locator("tr.com-object-summary:visible")).toHaveCount(2);
    for (const query of ["Unshown-product-secret", "Future-product-secret", "Malformed-product-secret", "Channel-product-secret", "Refused-product-secret",
      "REFUSED-ID", "CH-SECRET", product.application_program_id, product.source_sha256]) {
      await search.fill(query);
      await expect(panel.locator("tr.com-object-summary:visible")).toHaveCount(0);
    }
    await panel.getByRole("button", { name: m["comTable.resetFilters"], exact: true }).click();
    await expect(header(F1)).toHaveAttribute("aria-expanded", "true");
    await expect(header(F2)).toHaveAttribute("aria-expanded", "false");
    await panel.getByRole("button", { name: m["comTable.flat"], exact: true }).click();
    await expect(row(2).locator("[data-folder-caption-source]")).toBeVisible();
    await expect(row(2).locator(".com-object-folder-path")).toHaveText(m["comTable.folderPath"].replace("{value}", "CH-2 › PB-3"));
    await expect(row(8).locator(".com-object-project-folder")).toHaveText(m["comTable.projectFolderRefused.ModuleObject"]);
    await expect(panel.locator("img, script")).toHaveCount(0);
    await search.fill("PB-L");
    await expect(panel.locator("tr.com-object-summary:visible")).toHaveCount(1);
    await expect(row(6)).toBeVisible();
    await expect(row(6).locator("[data-folder-caption-source]")).toHaveCount(0);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    expect(JSON.stringify({ tree: state.tree, objects: state.objects })).toBe(before);
    expect(state.requests.slice(requestStart).filter(request => !request.startsWith("GET ") && request !== "POST /api/achievements/record")).toEqual([]);
    expect(state.unexpected).toEqual([]);
  });
}

// WP-06: the optional priority column is off by default, a remembered view
// preference, and shows the server's effective value with its winning source.
for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} ${width}px: the optional priority column shows effective values and stays view-only`, async ({ page }) => {
    const m = language === "en" ? en : de;
    const theme = width === 1440 ? "graphite" : "porcelain";
    await page.setViewportSize({ width, height: 900 });
    const state = await intercept(page, language, 8, theme);
    state.objects[0] = { ...state.objects[0], effective_priority: { status: "resolved", value: "Alert", source: { kind: "product", layer: "ProgramRef", omission: "AttributeOmitted" } } };
    state.objects[1] = { ...state.objects[1], effective_priority: { status: "resolved", value: "Low", source: { kind: "userEdit" } } };
    state.objects[2] = { ...state.objects[2], effective_priority: { status: "resolved", value: "High", source: { kind: "instance" } } };
    await openProject(page, language); await openList(page, language);
    await page.locator(".devices-table tbody td:nth-child(3) button").first().click();
    const panel = page.locator(".com-table-panel");
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    await expect(panel.locator("thead th")).toHaveCount(6);
    await expect(panel.getByRole("combobox", { name: m["comTable.priorityFilter"] })).toHaveCount(0);
    const before = JSON.stringify(state.tree), requestStart = state.requests.length;
    const toggle = panel.getByRole("button", { name: m["comTable.priorityColumn"], exact: true });
    await expect(toggle).toHaveAttribute("aria-pressed", "false");
    await toggle.click();
    await expect(toggle).toHaveAttribute("aria-pressed", "true");
    await expect(panel.locator('th[data-column="priority"]')).toContainText(m["comTable.priority"]);
    await panel.getByRole("button", { name: m["comTable.flat"], exact: true }).click();
    await expect(panel.locator("thead th")).toHaveCount(8);
    const cell = (id: number) => panel.locator(`tr[data-object-id="${id}"] .com-object-priority`);
    await expect(cell(1)).toHaveAttribute("data-priority", "Alert");
    await expect(cell(1).locator(".com-object-priority-source")).toHaveText(m["comTable.prioritySource.product"]);
    await expect(cell(2).locator(".com-object-priority-source")).toHaveText(m["comTable.prioritySource.userEdit"]);
    await expect(cell(3)).toHaveAttribute("data-priority", "High");
    await expect(cell(4)).toHaveAttribute("data-priority", "unresolved");
    await expect(cell(4)).toHaveAttribute("title", new RegExp(m["inspector.effectivePriority.unresolved.InstanceUnavailable"].replace(/[.*+?^${}()|[\]\\]/g, "\\$&")));
    await panel.locator('th[data-column="priority"] button').click();
    await expect(panel.locator('th[data-column="priority"]')).toHaveAttribute("aria-sort", "ascending");
    await expect(panel.locator("tr[data-object-id]:visible").first()).toHaveAttribute("data-object-id", "2");
    await panel.getByRole("combobox", { name: m["comTable.priorityFilter"] }).selectOption("unresolved");
    await expect(panel.locator("tr[data-object-id]:visible")).toHaveCount(5);
    const alignment = await panel.locator("table").evaluate(table => {
      const head = [...table.querySelectorAll("thead th")].map(n => n.getBoundingClientRect());
      const row = [...table.querySelector("tr[data-object-id]:not([hidden])")!.children].map(n => n.getBoundingClientRect());
      return head.map((h, i) => Math.abs(h.left - row[i].left));
    });
    expect(alignment.every(delta => delta < 1)).toBe(true);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    if (process.env.KNX_CO_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_CO_CAPTURE_DIR, `co-priority-${language}-${width}.png`) });
    await toggle.click();
    await expect(panel.locator("thead th")).toHaveCount(7);
    await expect(panel.locator("tr[data-object-id]:visible")).toHaveCount(8);
    // Only the view preference was written; the project is untouched.
    expect(state.settings.comTablePriorityColumn).toBe(false);
    expect(state.requests.slice(requestStart).filter(r => !r.startsWith("GET ") && r !== "POST /api/achievements/record")).toEqual(["PUT /api/settings", "PUT /api/settings"]);
    expect(JSON.stringify(state.tree)).toBe(before);
    expect(state.unexpected).toEqual([]);
  });
}
