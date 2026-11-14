/** Devices navigation browser contracts with strictly intercepted, fictional API data. */
import { openExplorerBranches, openNavigationSections, openParameterSections } from "./disclosure-fixture";
import { expect, test, type Locator, type Page } from "@playwright/test";
import { join } from "node:path";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import type { ParameterPanel, LineScanResultsResponse } from "../src/api";
import type { ComObjectNode } from "../src/bindings/ComObjectNode";
import type { DeviceNode } from "../src/bindings/DeviceNode";
import type { DeviceProductNode } from "../src/bindings/DeviceProductNode";
import { product as databaseProduct } from "./fixtures/product-details";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} stopping comparison resumes focus after Settings at ${width}px`, async ({ page }, testInfo) => {
    const m = language === "de" ? de : en;
    await page.setViewportSize({ width, height: 1000 });
    const state = await intercept(page, language);
    state.tree.comparison_revisions = { 1: 1, 2: 1, 3: 1 };
    let finish!: () => void;
    const held = new Promise<void>(resolve => { finish = resolve; });
    let settled!: () => void;
    const routeSettled = new Promise<void>(resolve => { settled = resolve; });
    let compareRequests = 0;
    await page.route("**/api/device-compare", async route => {
      compareRequests++;
      state.requests.push("POST /api/device-compare");
      await held;
      try {
        await route.fulfill({ status: 503, contentType: "application/json", body: JSON.stringify({ error: "Fictional held comparison completed" }) });
      } finally { settled(); }
    });
    try {
      await openProject(page, language); await openList(page, language);
      const opener = page.getByRole("button", { name: m["compare.all"], exact: true });
      await opener.click();
      const comparison = page.locator(".device-comparison-dialog");
      await comparison.locator("input").fill("192.0.2.17:3671");
      await comparison.getByRole("button", { name: m["compare.confirm"], exact: true }).click();
      const cancel = comparison.getByRole("button", { name: m["deviceChecks.cancel"], exact: true });
      await cancel.click();
      await expect(cancel).toBeDisabled();
      await comparison.evaluate(node => node.setAttribute("data-retained-stopping", "same"));
      await page.keyboard.press("Control+Shift+P");
      const palette = page.getByRole("dialog").getByRole("combobox");
      await palette.fill(m["toolbar.settings"]); await palette.press("Enter");
      await expect(page.locator("#settings-panel-title")).toBeFocused();
      await page.getByRole("button", { name: m["settings.back"], exact: true }).click();
      await expect(comparison).toBeVisible();
      await expect(comparison).toHaveAttribute("data-retained-stopping", "same");
      await expect(cancel).toBeDisabled();
      expect(compareRequests).toBe(1);
      expect(await comparison.evaluate(node => node.contains(document.activeElement)), "stopping comparison must resume focus inside its modal").toBe(true);
      await page.keyboard.press("Tab");
      expect(await comparison.evaluate(node => node.contains(document.activeElement))).toBe(true);
      expect(await page.locator(".workbench-toolbar").evaluate(node => !!node.closest("[inert]"))).toBe(true);
      await testInfo.attach("stopping-comparison-focus", { body: await page.screenshot(), contentType: "image/png" });
      finish(); await routeSettled;
      await comparison.getByRole("button", { name: m["compare.close"], exact: true }).click();
      await expect(comparison).toHaveCount(0);
      await expect(opener).toBeFocused();
      expect(state.unexpected).toEqual([]);
    } finally {
      finish();
      if (compareRequests > 0) await routeSettled;
    }
  });
}

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} complete product database metadata at ${width}px`, async ({ page }, testInfo) => {
    const m = language === "de" ? de : en;
    await page.setViewportSize({ width, height: 1000 });
    const state = await intercept(page, language, 3, width === 1440 ? "graphite" : "porcelain");
    state.detailProduct = structuredClone(databaseProduct);
    await openProject(page, language); await openList(page, language);
    const firstRow = page.locator(".devices-table tbody tr").filter({ hasText: "Actuator 1" });
    await expect(firstRow).toContainText(databaseProduct.catalog!.manufacturer_name!);
    await firstRow.getByRole("button", { name: "Actuator 1", exact: true }).click();
    await page.getByRole("tab", { name: m["deviceIdentity.tab"], exact: true }).click();
    const achievement = page.locator(".toast--achievement .toast-dismiss");
    if (await achievement.count()) {
      await achievement.first().click();
      await expect(page.locator(".toast--achievement")).toHaveCount(0);
    }
    const identity = page.locator(".device-identity");
    await expect(identity).toContainText("Fictional Devices");
    await expect(identity).toContainText("P-rich");
    await identity.locator(".identity-more > summary").click();
    const records = identity.locator(".product-database-details");
    const sections = databaseProduct.catalog!.details!;
    await expect(records.locator("details")).toHaveCount(sections.length);
    await expect(records.locator("dd")).toHaveCount(sections.reduce((count, section) => count + section.fields.length, 0));
    await testInfo.attach(`product-overview-${language}-${width}`, { body: await identity.screenshot(), contentType: "image/png" });
    for (const [index, section] of sections.entries()) {
      const record = records.locator("details").nth(index);
      await record.locator("summary").click();
      await expect(record.locator("dd")).toHaveText(section.fields.map(field => field.value));
      expect(await record.evaluate(node => node.scrollWidth - node.clientWidth)).toBeLessThanOrEqual(1);
      if (["hardware", "application", "catalogItem", "source", "additionalAttributes", "package"].includes(section.kind)) {
        await record.scrollIntoViewIfNeeded();
        await testInfo.attach(`product-${section.kind}-${index}-${language}-${width}`, { body: await record.screenshot(), contentType: "image/png" });
      }
      await record.locator("summary").click();
    }
    const bounds = await records.boundingBox();
    expect(bounds!.x).toBeGreaterThanOrEqual(0);
    expect(bounds!.x + bounds!.width).toBeLessThanOrEqual(width + 1);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    expect(state.requests.filter(request => /^(POST|PUT|DELETE) /.test(request))).toEqual([
      "POST /api/bus/discover", "POST /api/project/new",
      // Existing local achievement bookkeeping for opening the project/list/device; no project edit.
      "POST /api/achievements/record", "POST /api/achievements/record", "POST /api/achievements/record",
    ]);
    expect(state.unexpected).toEqual([]);
  });
}

test("unknown manufacturer IDs and long product metadata remain readable", async ({ page }) => {
  await page.setViewportSize({ width: 400, height: 900 });
  const state = await intercept(page, "en");
  state.detailProduct = structuredClone(databaseProduct);
  state.detailProduct.catalog!.manufacturer_name = null;
  state.detailProduct.catalog!.manufacturer_id = "M-FFFF";
  const long = "UnbrokenSourceLiteral".repeat(60);
  state.detailProduct.catalog!.details!.push({ kind: "source", id: long, fields: [{ key: "sample", value: long }] });
  await openProject(page, "en"); await openList(page, "en");
  await page.getByRole("button", { name: "Actuator 1", exact: true }).click();
  await page.getByRole("tab", { name: en["deviceIdentity.tab"], exact: true }).click();
  const identity = page.locator(".device-identity");
  await expect(identity.locator(".identity-fields").first()).toContainText("M-FFFF");
  await identity.locator(".identity-more > summary").click();
  const record = identity.locator(".product-database-details details").last();
  await record.locator("summary").click();
  await expect(record.locator("dd")).toHaveText(long);
  expect(await record.evaluate(node => node.scrollWidth - node.clientWidth)).toBeLessThanOrEqual(1);
  expect(state.unexpected).toEqual([]);
});

const DENSITY_PALETTES = ["porcelain", "graphite", "cupertino", "lcars", "rail", "rail-dark", "technical-light", "technical-dark", "user-modern-retro-green-crt"] as const;

for (const language of ["en", "de"] as const) for (const density of ["compact", "comfortable"] as const) {
  for (const theme of DENSITY_PALETTES) test(`WP41 ${theme} ${density} ${language}: shared slots, table room and real state marks`, async ({ page }, info) => {
    const m = language === "en" ? en : de;
    await page.setViewportSize({ width: 1440, height: 1000 });
    const state = await intercept(page, language, 24, theme, true, true, { density, study: true });
    await openProject(page, language); await openList(page, language);
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    await expect(page.locator("html")).toHaveAttribute("data-density", density);
    await page.evaluate(() => document.fonts.ready);
    const table = page.locator(".devices-table");
    await expect(table.locator("thead th")).toHaveCount(13);
    await expect(table.locator("tbody tr")).toHaveCount(24);
    await expect(table.locator('tbody tr[data-rename-id="1"] .programming-chip[data-part="parameters"]')).toBeVisible();
    const scrolling = await page.locator(".devices-table-scroll").evaluate(node => {
      const rect = node.getBoundingClientRect();
      const head = node.querySelector("thead")!.getBoundingClientRect();
      node.scrollLeft = 100;
      return { body: rect.height - head.height, local: node.scrollWidth > node.clientWidth, moved: node.scrollLeft };
    });
    expect(scrolling.body).toBeGreaterThanOrEqual(300);
    expect(scrolling.local).toBe(true); expect(scrolling.moved).toBeGreaterThan(0);
    expect(await table.locator("tbody tr").first().evaluate(node => getComputedStyle(node.querySelector("td")!).fontSize)).toBe("13px");
    await info.attach("wp41-devices", { body: await page.screenshot(), contentType: "image/png" });
    await dismissStudyNotifications(page);
    const select = page.getByRole("checkbox", { name: m["devices.select"].replace("{name}", "Actuator 1"), exact: true });
    await select.check();
    await expect(select).toBeChecked();
    await expect(page.locator(".device-workspace")).toHaveCount(0); // Checkbox does not activate the editor.
    await table.locator("tbody td:nth-child(3) button").filter({ hasText: /^Actuator 1$/ }).click();
    const inspector = page.locator(".workbench-pane-right .inspector");
    const address = inspector.locator(".individual-address-field .address-editor");
    await expect(address).toBeVisible();
    expect((await address.boundingBox())!.width).toBeCloseTo(160, 0);
    const name = inspector.locator("input.value-slot-text[data-rename-name]");
    expect((await name.boundingBox())!.width).toBeLessThanOrEqual(384);
    expect((await name.boundingBox())!.width).toBeGreaterThan(160);
    const textFields = inspector.locator("input.value-slot-text, textarea.value-slot-text");
    await expect(textFields).toHaveCount(2);
    for (const field of await textFields.all()) {
      const width = (await field.boundingBox())!.width;
      expect(width).toBeGreaterThan(160); expect(width).toBeLessThanOrEqual(384);
    }
    await expect(inspector.locator('.device-programming-parts [data-pending="true"]')).toHaveCount(1);
    const addressInput = address.locator("input");
    await addressInput.fill("256"); await addressInput.press("Enter");
    await expect(inspector.locator(".individual-address-field .field-error")).toBeVisible();
    await expect(addressInput).toHaveValue("256");
    await info.attach("wp41-inspector", { body: await inspector.screenshot(), contentType: "image/png" });
    await addressInput.fill("2"); await addressInput.press("Enter");
    await expect(inspector.locator(".individual-address-field .field-error")).toHaveCount(0);
    await page.getByRole("tab", { name: m["workbench.parameters"], exact: true }).click();
    const workspace = page.locator(".parameter-workspace");
    await workspace.locator(".parameter-workspace-nav button").first().click();
    const mode = workspace.locator('[data-parameter-row="Mode"] select');
    const scalar = workspace.locator('[data-parameter-row="Study-number"] .parameter-field-control');
    expect((await mode.boundingBox())!.width).toBeCloseTo(160, 0);
    expect((await scalar.boundingBox())!.width).toBeCloseTo(160, 0);
    await expect(scalar.locator(".parameter-field-suffix")).toHaveText("s");
    await expect(scalar.locator("input")).toBeDisabled();
    await expect(scalar.locator("input")).toHaveValue("5");
    await expect(workspace.locator('[data-parameter-row="Study-number"] .parameter-field-caption')).not.toBeEmpty();
    const text = workspace.locator('[data-parameter-row="Study-text"] .parameter-field-text input');
    expect((await text.boundingBox())!.width).toBeLessThanOrEqual(384);
    expect((await text.boundingBox())!.width).toBeGreaterThan(160);
    await expect(text).toHaveValue("Fictional engineering description");
    for (const control of [mode, scalar.locator("input")]) {
      const box = (await control.boundingBox())!;
      expect(box.height).toBeGreaterThanOrEqual(density === "compact" ? 32 : 40);
      await expect(control).toHaveCSS("font-size", "13px");
    }
    const label = workspace.locator('[data-parameter-row="Mode"] .parameter-field-name');
    await expect(label).toContainText(language === "de" ? "Busspannungsversorgung" : "voltage recovery");
    expect(await label.evaluate(node => node.scrollHeight - node.clientHeight)).toBeLessThanOrEqual(1);
    await expectSourceNotice(workspace, "Information", "Fictional source warning");
    await expectSourceNotice(workspace, "Error", "Fictional source error");
    await mode.focus(); await page.keyboard.press("Tab"); await page.keyboard.press("Shift+Tab");
    await expect(mode).toBeFocused();
    expect(await mode.evaluate(node => node.matches(":focus-visible"))).toBe(true);
    expect(await mode.evaluate(node => getComputedStyle(node).boxShadow)).not.toBe("none");
    await info.attach("wp41-parameters-focused", { body: await workspace.screenshot(), contentType: "image/png" });
    await page.locator(".device-editor-navigation button").first().click();
    await expect(select).toBeChecked();
    expect(state.requests.filter(request => /^(POST|PUT|DELETE) /.test(request)
      && request !== "POST /api/bus/discover" && request !== "POST /api/project/new" && request !== "POST /api/achievements/record")).toEqual([]);
    expect(state.unexpected).toEqual([]);
  });

  for (const theme of ["technical-light", "technical-dark"]) test(`WP41 ${theme} ${density} ${language}: narrow panes bound slots and retain labels`, async ({ page }, info) => {
    const m = language === "en" ? en : de;
    await page.setViewportSize({ width: 400, height: 1000 });
    const state = await intercept(page, language, 24, theme, true, true, { density, study: true });
    await openProject(page, language); await openList(page, language);
    expect((await page.locator(".devices-table-scroll").boundingBox())!.height).toBeGreaterThanOrEqual(240);
    await page.locator(".devices-table tbody td:nth-child(3) button").filter({ hasText: /^Actuator 1$/ }).click();
    await page.getByRole("tab", { name: m["workbench.parameters"], exact: true }).click();
    const workspace = page.locator(".parameter-workspace");
    await workspace.locator(".parameter-workspace-nav button").first().click();
    const mode = workspace.locator('[data-parameter-row="Mode"] select');
    const label = workspace.locator('[data-parameter-row="Mode"] .parameter-field-name');
    const [labelBox, modeBox] = [(await label.boundingBox())!, (await mode.boundingBox())!];
    expect(modeBox.y).toBeGreaterThanOrEqual(labelBox.y + labelBox.height);
    expect(modeBox.width).toBeCloseTo(160, 0);
    for (const editor of await workspace.locator(".parameter-field > input, .parameter-field > select, .parameter-field-control").all()) {
      if (!await editor.isVisible()) continue;
      const box = (await editor.boundingBox())!;
      expect(box.x).toBeGreaterThanOrEqual(0); expect(box.x + box.width).toBeLessThanOrEqual(401);
    }
    await expectSourceNotice(workspace, "Information", "Fictional source warning");
    await expectSourceNotice(workspace, "Error", "retained evidence");
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    await info.attach("wp41-narrow-parameters", { body: await workspace.screenshot(), contentType: "image/png" });
    expect(state.unexpected).toEqual([]);
  });
}

for (const language of ["en", "de"] as const) test(`WP41 ${language}: saved/resized columns and draft survive Settings; active focus survives in-place refresh`, async ({ page }, info) => {
  const m = language === "en" ? en : de;
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.addInitScript(() => {
    const key = "knxbench:table-column-widths:v1:devices";
    // Init scripts also run on reload: seed an absent preference, never overwrite a resize.
    if (localStorage.getItem(key) === null) localStorage.setItem(key, JSON.stringify({ name: 280, description: 220 }));
  });
  const state = await intercept(page, language, 24, "technical-light", true, true, { density: "compact", study: true });
  await openProject(page, language); await openList(page, language);
  const table = page.locator('[data-table-id="devices"]');
  const stored = () => page.evaluate(() => JSON.parse(localStorage.getItem("knxbench:table-column-widths:v1:devices")!));
  expect(await stored()).toEqual({ name: 280, description: 220 });
  const renderedName = () => table.locator("col").nth(2).evaluate(node => node.getBoundingClientRect().width);
  const initialRenderedName = await renderedName();
  await table.getByRole("separator").nth(2).press("ArrowRight");
  const resized = await stored(); expect(resized.name).toBe(288); expect(resized.description).toBe(220);
  await expect.poll(renderedName).toBeCloseTo(initialRenderedName + 8, 0);
  const selected = page.getByRole("checkbox", { name: m["devices.select"].replace("{name}", "Actuator 1"), exact: true });
  await selected.check();
  await table.locator("tbody td:nth-child(3) button").filter({ hasText: /^Actuator 1$/ }).click();
  await page.getByRole("tab", { name: m["workbench.parameters"], exact: true }).click();
  const workspace = page.locator(".parameter-workspace");
  await workspace.locator(".parameter-workspace-nav button").first().click();
  await workspace.locator('[data-checked-for="Study-date"]').click();
  const checked = workspace.locator('[data-ets-id="Study-date"] .parameter-checked-editor');
  const draft = checked.locator('input[type="text"]');
  await draft.fill("2026-12-24");
  await draft.evaluate(node => node.setAttribute("data-wp41-retained", "same"));
  await page.keyboard.press("Tab"); await page.keyboard.press("Shift+Tab");
  await expect(draft).toBeFocused();
  const before = state.requests.length;
  for (const [theme, density] of [["technical-dark", "comfortable"], ["graphite", "compact"], ["technical-light", "comfortable"]]) {
    await page.keyboard.press("Control+Shift+P");
    const palette = page.getByRole("dialog").getByRole("combobox");
    await palette.fill(m["toolbar.settings"]); await palette.press("Enter");
    await page.getByRole("combobox", { name: m["settings.theme"], exact: true }).selectOption(theme);
    await page.getByRole("combobox", { name: m["appearance.density"], exact: true }).selectOption(density);
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    await expect(page.locator("html")).toHaveAttribute("data-density", density);
    await page.getByRole("button", { name: m["settings.back"], exact: true }).click();
    await expect(draft).toHaveAttribute("data-wp41-retained", "same");
    await expect(draft).toHaveValue("2026-12-24");
    // Settings is page navigation; its Back action does not promise inline focus restoration.
    expect((await draft.boundingBox())!.width).toBeCloseTo(160, 0);
    await expect(workspace.locator('.parameter-workspace-nav [aria-current="page"]')).toHaveCount(1);
    expect(await stored()).toEqual(resized);
    await info.attach(`wp41-switch-${theme}-${density}`, { body: await workspace.screenshot(), contentType: "image/png" });
  }
  // Re-enter by real pointer/keyboard input before testing an in-place update.
  await draft.click(); await page.keyboard.press("Tab"); await page.keyboard.press("Shift+Tab");
  await expect(draft).toBeFocused();
  for (const [theme, density] of [["technical-dark", "compact"], ["technical-light", "comfortable"]]) {
    // Simulated change in the server's record; the production refresh applies it.
    // No focusing, clicking or locator.press between these before/after assertions.
    state.settings = { ...state.settings, theme, density };
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme, { timeout: 10_000 });
    await expect(page.locator("html")).toHaveAttribute("data-density", density);
    await expect(draft).toHaveAttribute("data-wp41-retained", "same");
    await expect(draft).toHaveValue("2026-12-24");
    await expect(draft).toBeFocused();
    expect(await draft.evaluate(node => node.matches(":focus-visible"))).toBe(true);
    await expect(draft).toHaveCSS("outline-style", "solid");
    await expect(draft).toHaveCSS("outline-width", "2px");
    await page.keyboard.press("End"); await page.keyboard.press("Backspace");
    await expect(draft).toHaveValue("2026-12-2");
    await page.keyboard.type("4"); await expect(draft).toHaveValue("2026-12-24");
    expect(await stored()).toEqual(resized);
    await info.attach(`wp41-active-${theme}-${density}`, { body: await workspace.screenshot(), contentType: "image/png" });
  }
  await expect.poll(() => ({ theme: state.settings.theme, density: state.settings.density }))
    .toEqual({ theme: "technical-light", density: "comfortable" });
  await expect.poll(() => page.evaluate(() => JSON.parse(localStorage.getItem("knx-desktop:settings-cache")!)))
    .toMatchObject({ schemaVersion: 1, settings: { theme: "technical-light", density: "comfortable" } });
  expect(state.requests.slice(before).filter(request => /^(POST|DELETE) /.test(request) && request !== "POST /api/achievements/record")).toEqual([]);
  expect(state.tree.snapshot_revision).toBe(1); expect(state.tree.can_undo).toBe(false);
  await page.locator(".device-editor-navigation button").first().click();
  await expect(selected).toBeChecked();
  const finalRenderedName = await renderedName();
  await table.getByRole("separator").nth(2).press("ArrowLeft");
  await expect.poll(renderedName).toBeCloseTo(finalRenderedName - 8, 0);
  await table.getByRole("separator").nth(2).press("ArrowRight");
  await expect.poll(renderedName).toBeCloseTo(finalRenderedName, 0);
  expect(await stored()).toEqual(resized);
  expect(state.settings).toMatchObject({ theme: "technical-light", density: "comfortable" });
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "technical-light");
  await expect(page.locator("html")).toHaveAttribute("data-density", "comfortable");
  expect(await stored()).toEqual(resized);
  expect(state.unexpected).toEqual([]);
});

for (const [theme, density, language, width, uiScale] of [
  ["technical-light", "compact", "en", 1440, 0.8],
  ["technical-light", "comfortable", "de", 400, 1.5],
  ["technical-dark", "compact", "de", 400, 0.8],
  ["technical-dark", "comfortable", "en", 1440, 1.5],
] as const) test(`WP41 ${theme} ${density} ${language}: scale ${uiScale} at ${width}px and constrained inspector`, async ({ page }, info) => {
  const m = language === "en" ? en : de;
  await page.setViewportSize({ width, height: 1000 });
  const state = await intercept(page, language, 24, theme, true, true, { density, study: true, uiScale });
  await openProject(page, language); await openList(page, language);
  await expect(page.locator("html")).toHaveCSS("--app-ui-scale", String(uiScale));
  const tableScroll = page.locator(".devices-table-scroll");
  expect((await tableScroll.boundingBox())!.height / uiScale).toBeGreaterThanOrEqual(160);
  await page.locator('tr[data-rename-id="1"] td:nth-child(3) button').click();
  const inspector = page.locator(".workbench-pane-right .inspector");
  // Host-size constraint on the real consumer, not a new inspector implementation.
  await inspector.evaluate(node => { (node as HTMLElement).style.inlineSize = "min(240px, 100%)"; });
  const name = inspector.locator("input[data-rename-name]");
  await name.scrollIntoViewIfNeeded();
  const label = inspector.locator(".rename-name-field .inspector-field");
  await expect(label).toHaveText(m["inspector.name"]);
  // The caption is an anonymous grid text item, not an invented wrapper element.
  const labelBox = await label.evaluate(node => {
    const text = Array.from(node.childNodes).find(child => child.nodeType === Node.TEXT_NODE && child.textContent?.trim())!;
    const range = document.createRange(); range.selectNodeContents(text);
    const box = range.getBoundingClientRect(); return { y: box.y, height: box.height };
  });
  const nameBox = (await name.boundingBox())!;
  expect(nameBox.y).toBeGreaterThanOrEqual(labelBox.y + labelBox.height - 1);
  await expect(name).toHaveValue("Actuator 1");
  const pane = (await inspector.boundingBox())!;
  for (const field of await inspector.locator("input.value-slot-text, .address-editor").all()) {
    const box = (await field.boundingBox())!;
    expect(box.width).toBeGreaterThan(0);
    expect(box.x).toBeGreaterThanOrEqual(pane.x - 1);
    expect(box.x + box.width).toBeLessThanOrEqual(pane.x + pane.width + 1);
  }
  expect((await inspector.locator(".address-editor").boundingBox())!.width / uiScale).toBeCloseTo(160, 0);
  await inspector.screenshot({ path: info.outputPath("wp41-scale-inspector.png") });
  await page.getByRole("tab", { name: m["workbench.parameters"], exact: true }).click();
  const workspace = page.locator(".parameter-workspace");
  await workspace.locator(".parameter-workspace-nav button").first().click();
  const mode = workspace.locator('[data-parameter-row="Mode"] select');
  expect((await mode.boundingBox())!.width / uiScale).toBeCloseTo(160, 0);
  const expectedHeight = density === "compact" ? 32 : 40;
  await expect(mode).toHaveCSS("min-height", `${expectedHeight}px`);
  // Chromium quantizes scaled DOMRects; retain the exact CSS contract and a 1/64px bound.
  expect((await mode.boundingBox())!.height / uiScale).toBeGreaterThanOrEqual(expectedHeight - 1 / 64);
  await expectSourceNotice(workspace, "Information", "Fictional source warning");
  await expectSourceNotice(workspace, "Error", "retained evidence");
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
  await workspace.screenshot({ path: info.outputPath("wp41-scale-parameters.png") });
  expect(state.tree.snapshot_revision).toBe(1); expect(state.tree.can_undo).toBe(false);
  expect(state.requests.filter(request => /^(POST|PUT|DELETE) /.test(request)
    && !["POST /api/bus/discover", "POST /api/project/new", "POST /api/achievements/record"].includes(request))).toEqual([]);
  expect(state.unexpected).toEqual([]);
});

const devices = (count: number): DeviceNode[] => Array.from({ length: count }, (_, index) => ({
  id: index + 1, name: `Actuator ${index + 1}`, address: index < 2 ? `1.1.${index === 0 ? 2 : 10}` : null,
  description: `Fictional device ${index + 1}`, com_object_count: 0,
}));
function fixtureTree(nodes: DeviceNode[]): ProjectTree {
  return { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
    group_address_style: "ThreeLevel", server_incarnation: "devices-browser", snapshot_revision: 1,
    installations: [{ id: 1, name: "House", topology: [{ id: 1, name: "Area", address: 1,
      lines: [{ id: 2, name: "Ground floor", address: 1, devices: nodes }] }], unassigned: [],
      buildings: [{ id: 3, name: "Kitchen", kind: "Room", children: [], devices: nodes.slice(0, 1) }], group_ranges: [],
      group_addresses: [{ id: 500, name: "Kitchen light", address: "1/1/1", range: null, dpts: [],
        links: [{ device_id: 1, device_name: "Actuator 1", device_address: "1.1.2", com_object_id: 20,
          com_object_number: 1, com_object_name: "Switch", direction: "Receive" }] }],
    }] };
}
/** Dismiss transient first-project notices through their real UI before durable visual capture. */
async function dismissStudyNotifications(page: Page) {
  const dismiss = page.locator(".toast:not(.toast--leaving) .toast-dismiss");
  for (let round = 0; round < 20 && await dismiss.count() > 0; round++) await dismiss.first().click();
  await expect(page.locator(".toast")).toHaveCount(0);
}

/** Source notices must remain visibly rendered and reachable, not merely present in the DOM. */
async function expectSourceNotice(workspace: Locator, hint: "Information" | "Error", text: string) {
  const row = workspace.locator(`.parameter-workspace-separator[data-ui-hint="${hint}"]`);
  const message = row.locator("p");
  await expect(row).toBeVisible(); await expect(message).toBeVisible();
  await expect(message).toContainText(text);
  await message.scrollIntoViewIfNeeded();
  await expect(message).toBeInViewport({ ratio: 1 });
  const bounds = (await message.boundingBox())!;
  const pane = (await workspace.boundingBox())!;
  expect(bounds.width).toBeGreaterThan(0); expect(bounds.height).toBeGreaterThan(0);
  expect(bounds.x).toBeGreaterThanOrEqual(pane.x - 1);
  expect(bounds.x + bounds.width).toBeLessThanOrEqual(pane.x + pane.width + 1);
  if (hint === "Error") {
    const paint = await message.evaluate(node => {
      const style = getComputedStyle(node); const probe = document.createElement("span");
      probe.style.color = style.getPropertyValue("--knx-error-color").trim();
      return { actual: style.color, expected: probe.style.color };
    });
    expect(paint.actual).toBe(paint.expected);
  }
}

async function intercept(page: Page, language: "en" | "de", count = 3, theme = "porcelain", parameterEditing = false, parameterWorkspace = false,
  appearance: { density: "compact" | "comfortable"; study?: boolean; uiScale?: number } = { density: "compact" }) {
  const nodes = devices(count); const tree = fixtureTree(nodes);
  if (appearance.study) nodes[0].pending_programming = { address: false, application: false, parameters: true, group_addresses: false };
  const state = { tree, nodes, unexpected: [] as string[], requests: [] as string[], polls: 0, failCatalogue: false, detailProduct: null as DeviceProductNode | null,
    settings: { uiLanguage: language, theme, density: appearance.density, ...(appearance.study ? { motionLevel: "off", accent: "violet", releaseChecksEnabled: false, ...(appearance.uiScale === undefined ? {} : { uiScale: appearance.uiScale }) } : {}) } as Record<string, unknown> };
  let mode = "0";
  tree.project_incarnation = 1;
  const panel = (): ParameterPanel => ({ programId: "A-FIXTURE", sourceLanguage: "en", tree: null, stale: [], diagnostics: [],
    sections: [{ scope: null, fields: [{ etsId: "Mode", name: "Mode", text: "Mode", nameLanguage: null, textLanguage: null, kind: "Restriction", value: mode, valueSource: "Stored", editable: true, min: null, max: null,
      enumOptions: [{ value: "0", text: "Disabled", language: null }, { value: "1", text: "Rocker", language: null }, { value: "2", text: "Individual buttons", language: null }], displayOrder: null, access: "ReadWrite", writeEtsId: "Mode", group: { key: "buttons", label: "Buttons" }, controlsComObjects: true, controlledComObjectIds: mode === "0" ? [] : [20] }] }] });
  const shownPanel = (): ParameterPanel => {
    const data = panel();
    if (!parameterWorkspace) return data;
    const modeField = data.sections[0].fields[0];
    data.sections[0].fields.push(...["3 / 4", "5 / 6", "7 / 8"].map((pair, index) => ({ ...modeField,
      etsId: `Pair-${index}`, writeEtsId: `Pair-${index}`, text: `Function buttons ${pair} (left / right)` })));
    data.sections[0].fields.push({ ...modeField, etsId: "Delay", writeEtsId: null, editable: false,
      kind: "Number", text: "Delay after bus voltage recovery (seconds)", value: "5", min: "0", max: "255",
      enumOptions: [], group: { key: "timing", label: "Timing and start-up" } });
    data.sections[0].fields.push({ ...modeField, etsId: "Ungrouped", writeEtsId: null, editable: false,
      kind: "Text", text: "Engineering note", value: "Fictional ungrouped parameter", enumOptions: [], group: null });
    if (appearance.study) {
      modeField.text = language === "de" ? "Bedienverhalten nach Wiederkehr der Busspannungsversorgung mit langer Beschriftung"
        : "Operating behavior after bus voltage recovery with a deliberately long engineering label";
      data.sections[0].fields.push(
        { ...modeField, etsId: "Study-number", kind: "Number", enumOptions: [], value: "5", suffixText: "s", editable: false, writeEtsId: null },
        { ...modeField, etsId: "Study-text", kind: "Text", enumOptions: [], value: "Fictional engineering description", editable: false, writeEtsId: null },
        { ...modeField, etsId: "Study-date", kind: "Date", enumOptions: [], value: "2026-11-14", writeEtsId: "Study-date", min: null, max: null });
      data.sections[0].separators = [
        { position: 0, group: modeField.group!, elementId: "study-information", text: "Fictional source warning — do not hide", uiHint: "Information", access: null, blockAccess: null },
        { position: 1, group: modeField.group!, elementId: "study-error", text: "Fictional source error — retained evidence", uiHint: "Error", access: null, blockAccess: null },
      ];
    }
    data.sections.push({ scope: { moduleNode: 7, moduleId: "Channel A", moduleDefId: "MD-A" },
      fields: [{ ...modeField, etsId: "Module-mode", writeEtsId: null, editable: false,
        text: "Operating mode", group: { key: "buttons", label: "Buttons" } }] });
    return data;
  };
  const configuredObject = (): ComObjectNode => ({
    saved_instance_priority: { status: "unavailable", reason: "NotCaptured" }, effective_priority: { status: "unresolved", reason: "InstanceUnavailable", source: null, raw: null },
 id: 20, number: 1, name: mode === "1" ? "Rocker output" : "Left output", function_text: "Switch", description: null, description_layer: null, dpt: "DPST-1-1", dpt_layer: "Program", program_dpt: null, dpt_text: null, is_active: true, activation: "Active", channel: null, read: false, write: true, transmit: false, communication: true, update: false, read_on_init: false, links: [] });
  // Like the real server: no project until the test creates one (ADR-0127).
  let projectOpen = false;
  await page.route("**/api/**", (route) => {
    const url = new URL(route.request().url()); const path = url.pathname; const method = route.request().method();
    state.requests.push(`${method} ${path}`);
    let body: unknown; let status = 200;
    if (path === "/api/auth/status" && method === "GET") body = { required: false, authenticated: true };
    else if (path === "/api/settings" && method === "GET") body = { schemaVersion: 1, ...(appearance.study ? { conditionalPatchVersion: 1 } : {}), status: "ok", settings: state.settings };
    // The acknowledged study document enables the first-run build metadata read.
    else if (appearance.study && path === "/api/version" && method === "GET") body = { version: "0.0.0-fixture" };
    else if (appearance.study && path === "/api/settings" && method === "PUT") {
      const payload = route.request().postDataJSON() as { settings: Record<string, unknown>; expectedSettings?: Record<string, unknown> };
      expect(Object.keys(payload.settings).every(key => ["theme", "density"].includes(key))).toBe(true);
      for (const [key, value] of Object.entries(payload.expectedSettings ?? {})) expect(state.settings[key] ?? null).toEqual(value);
      state.settings = { ...state.settings, ...payload.settings };
      body = { schemaVersion: 1, conditionalPatchVersion: 1, status: "ok", settings: state.settings };
    }
    else if (path === "/api/project" && method === "GET") { if (projectOpen) body = state.tree; else { status = 400; body = { error: "no project open" }; } }
    else if (path === "/api/project/new" && method === "POST") { projectOpen = true; body = state.tree; }
    else if (path === "/api/project/load-progress" && method === "GET") body = null;
    else if (path === "/api/project/status-metrics" && method === "GET") body = statusMetricsFixtureBody(state.tree);
    else if (path === "/api/log" && method === "GET") body = [];
    else if (path === "/api/legacy-password" && method === "GET") body = { available: true, remembered: false, problem: null };
    else if (path === "/api/bus/discover" && method === "POST") body = { interfaces: [] };
    else if (path === "/api/product-languages" && method === "GET") body = [];
    else if (path === "/api/device-system2/status" && method === "GET") body = null;
    else if (["/api/device-download/status", "/api/device-download/batch/status", "/api/device-address/status"].includes(path) && method === "GET") { status = 404; body = { error: "no operation in this fixture" }; }
    else if (path === "/api/devices" && method === "GET") {
      if (state.failCatalogue) { status = 500; body = { error: "fixture product database unavailable" }; }
      else body = { schemaVersion: 1, serverIncarnation: "devices-browser", snapshotRevision: state.tree.snapshot_revision,
        devices: nodes.map((device) => ({ id: device.id, device, productRef: "Fictional-P-1", resolution: "Resolved",
          manufacturerId: "Fictional-M-1", manufacturerName: "Fictional Devices", productText: "Switch actuator", orderNumber: "SA-4",
          productTextLanguage: null, productSourceLanguage: null, system: { state: "family", mask: 0x07b0, family: "systemB" } })) };
    } else if (/^\/api\/device\/\d+$/.test(path) && method === "GET") {
      const device = nodes.find((node) => node.id === Number(path.split("/").at(-1)));
      if (!device) { status = 400; body = { error: "device not found" }; }
      else body = { ...device, com_objects: parameterEditing && mode !== "0" ? [configuredObject()] : [], product: state.detailProduct ?? { resolution: "NoReference", product_ref: null, program_ref: null, catalog: null } };
    } else if (appearance.study && path === "/api/device/1/parameter-editor-source" && method === "GET") {
      expect(url.searchParams.get("etsId")).toBe("Study-date");
      body = { device: 1, parameterRefId: "Study-date", serverIncarnation: "devices-browser", projectIncarnation: 1,
        snapshotRevision: state.tree.snapshot_revision, evidence: { status: "declared", source: {
          sourceSha256: "a".repeat(64), programId: "A-FIXTURE", parameterTypeId: "T-date", schema: 23,
          child: "TypeDate", attributes: { Encoding: "DPT 11" }, parentAttributes: { Id: "T-date" },
        } } };
    } else if (/^\/api\/device\/\d+\/parameters$/.test(path) && method === "GET") body = parameterEditing ? shownPanel() : { programId: null, sections: [], stale: [], diagnostics: [] };
    else if (parameterEditing && /^\/api\/device\/\d+\/parameters$/.test(path) && method === "POST") {
      const write = route.request().postDataJSON();
      expect(write.expectedValue).toBe(mode); expect(write.etsId).toBe("Mode");
      mode = write.raw; state.tree = { ...state.tree, snapshot_revision: state.tree.snapshot_revision! + 1, can_undo: true, is_modified: true };
      body = { ...shownPanel(), tree: state.tree };
    }
    else if (path === "/api/bus/monitor/telegrams" && method === "GET") {
      state.polls++;
      body = { sessionId: 1, serverIncarnation: "devices-browser", contextStatus: "current", projectOpen: true,
        status: "active", nextSince: 1, droppedBefore: 0, telegrams: Number(url.searchParams.get("since")) === 0 ? [{
          seq: 1, timestamp: "2026-10-08T12:00:00Z", source: "1.1.2", destination: "1/1/1", destinationName: "Kitchen light",
          service: "GroupValueWrite", rawPayload: "01", decoded: null, control: null,
        }] : [] };
    } else if (path === "/api/device-readiness" && method === "GET") {
      const selector = url.searchParams.get("deviceId");
      const selected = selector === null ? nodes : nodes.filter((node) => node.id === Number(selector));
      if (selector !== null) expect([...url.searchParams.keys()]).toEqual(["deviceId"]);
      if (selector !== null && selected.length !== 1) {
        state.unexpected.push(`${method} ${url.pathname}${url.search}`);
        status = 404; body = { error: "device not found" };
      } else body = { devices: selected.map((node) => ({
        address: node.address, name: node.name, programRef: "fictional", readiness: "untested", category: null, detail: null, steps: 1, octets: 1,
      })), counts: { untested: selected.length } };
    }
    else if (path === "/api/device-address/availability" && method === "GET") body = { startAvailable: false, reason: "Device-specific recovery not implemented" };
    else if (path === "/api/bus/scan/results" && method === "GET") body = { sessionId: 4, serverIncarnation: "devices-browser", transport: "udp", skippedFrames: null, disconnectConfirmed: true, cleanupError: null, status: "completed", error: null,
      nextSince: 1, completedCount: 1, totalCount: 1, omittedAddresses: [], excludedAddresses: [],
      results: [{ address: "1.1.2", outcome: { kind: "occupied", maskVersion: 1793 } }] } satisfies LineScanResultsResponse;
    else if (path === "/api/bus/scan/comparison-bound" && method === "GET") body = { unexpected: [], missing: [], excludedInProject: [] };
    else if (path === "/api/bus/activity" && method === "GET") body = { serverIncarnation: "fixture", coverage: "partial", historyState: "disabled", sessions: [], oneShot: [], oneShotDropped: 0, busyLocks: [], untracked: [] };
    else if (achievementsFixtureAnswer(method, path) !== undefined) body = achievementsFixtureAnswer(method, path);
    else {
      state.unexpected.push(`${method} ${path}`); status = 404; body = { error: "strict devices fixture only" };
    }
    return route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
  });
  return state;
}
async function openProject(page: Page, language: "en" | "de", expand = true) {
  const m = language === "en" ? en : de;
  await page.goto("/");
  await page.getByRole("button", { name: m["toolbar.newProject"], exact: true }).first().click();
  await page.getByRole("dialog", { name: m["newProject.title"], exact: true }).getByRole("button", { name: m["newProject.create"], exact: true }).click();
  await page.getByRole("dialog", { name: m["projectWizard.done.title"], exact: true }).getByRole("button", { name: m["projectWizard.done.close"], exact: true }).click();
  if (expand) { await openNavigationSections(page); await openExplorerBranches(page); }
}
async function openList(page: Page, language: "en" | "de") {
  const m = language === "en" ? en : de;
  await page.keyboard.press("Control+Shift+P");
  await page.getByRole("combobox").fill(m["workbench.devices"]);
  await page.getByRole("combobox").press("Enter");
  await expect(page.locator(".devices-workspace")).toBeVisible();
}
for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} source-group parameter workspace at ${width}px`, async ({ page }, testInfo) => {
    const m = language === "de" ? de : en;
    await page.setViewportSize({ width, height: 1000 });
    const state = await intercept(page, language, 3, width === 1440 ? "graphite" : "porcelain", true, true);
    await openProject(page, language); await openList(page, language);
    await page.locator(".devices-table tbody td:nth-child(3) button").filter({ hasText: /^Actuator 1$/ }).click();
    await page.getByRole("tab", { name: m["workbench.parameters"], exact: true }).click();
    const workspace = page.locator(".parameter-workspace");
    const nav = workspace.locator(".parameter-workspace-nav button");
    await expect(nav).toHaveCount(4);
    // Closed by default: areas are listed, none opens until chosen.
    await expect(workspace.locator('.parameter-workspace-nav [aria-current="page"]')).toHaveCount(0);
    await expect(workspace.locator(".parameter-workspace-row:visible")).toHaveCount(0);
    await expect(workspace.locator(".parameter-workspace-closed")).toBeVisible();
    if (process.env.KNX_DEVICES_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_DEVICES_CAPTURE_DIR, `parameter-workspace-closed-${language}-${width}.png`) });
    await nav.first().click();
    await expect(nav.first()).toHaveAttribute("aria-current", "page");
    await expect(workspace.locator(".parameter-workspace-row:visible")).toHaveCount(4);
    const mode = workspace.locator('[data-parameter-row="Mode"] select');
    const identity = await mode.evaluate(node => { node.setAttribute("data-retained-editor", "yes"); return (node as HTMLSelectElement).value; });
    await nav.nth(1).click();
    await expect(mode).toBeHidden();
    await expect(workspace.locator('[data-parameter-row="Delay"] input')).toHaveValue("5");
    await workspace.locator('[data-parameter-row="Delay"] .parameter-workspace-info').click();
    await expect(workspace.locator(".parameter-workspace-context")).toContainText("255");
    await nav.first().click();
    await expect(mode).toHaveAttribute("data-retained-editor", "yes");
    await expect(mode).toHaveValue(identity);
    const postsBefore = state.requests.filter(request => request.startsWith("POST ")).length;
    const search = workspace.getByRole("searchbox");
    await search.fill("Delay");
    await expect(workspace.locator(".parameter-workspace-row:visible")).toHaveCount(1);
    await search.fill("not-present");
    await expect(workspace.locator(".parameter-workspace-empty")).toBeVisible();
    await search.fill("");
    await expect(mode).toHaveAttribute("data-retained-editor", "yes");
    await expect(mode).toHaveValue(identity);
    expect(state.requests.filter(request => request.startsWith("POST ")).length).toBe(postsBefore);
    await mode.selectOption("2");
    await expect(mode).toHaveValue("2");
    await workspace.locator('[data-parameter-row="Mode"] .parameter-workspace-info').click();
    const metadata = workspace.locator(".parameter-workspace-context");
    const metadataValue = (label: string) => metadata.getByText(label, { exact: true }).locator("xpath=following-sibling::dd[1]");
    await expect(metadataValue(m["parameters.workspace.access"])).toHaveText(language === "de" ? "Lesen und Schreiben" : "ReadWrite");
    await expect(metadataValue(m["parameters.workspace.access"])).toHaveAttribute("title", "ReadWrite");
    await expect(metadataValue(m["parameters.workspace.type"])).toHaveText(language === "de" ? "Auswahl" : "Restriction");
    await expect(metadataValue(m["parameters.workspace.type"])).toHaveAttribute("title", "Restriction");
    await expect(metadataValue(m["parameters.workspace.source"])).toHaveText(language === "de" ? "Im Projekt gespeichert" : "Stored");
    await expect(metadataValue(m["parameters.workspace.source"])).toHaveAttribute("title", "Stored");
    const normalBadge = workspace.locator('[data-parameter-row="Mode"] .provenance-badge');
    await expect(normalBadge).toBeHidden();
    const dimensions = await workspace.evaluate(node => {
      const box = node.getBoundingClientRect();
      const row = node.querySelector('.parameter-workspace-row:not([hidden])')!.getBoundingClientRect();
      return { width: box.width, rowHeight: row.height, overflow: node.scrollWidth - node.clientWidth };
    });
    expect(dimensions.overflow).toBeLessThanOrEqual(1);
    if (dimensions.width > 580) expect(dimensions.rowHeight).toBeLessThan(100);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    if (process.env.KNX_DEVICES_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_DEVICES_CAPTURE_DIR, `parameter-workspace-${language}-${width}.png`) });
    await metadata.scrollIntoViewIfNeeded();
    await testInfo.attach(`parameter-metadata-${language}-${width}`, { body: await metadata.screenshot(), contentType: "image/png" });
    expect(state.unexpected).toEqual([]);
  });

  test(`${language} workbench configuration refreshes objects at ${width}px`, async ({ page }) => {
    const m = language === "de" ? de : en;
    await page.setViewportSize({ width, height: 900 });
    const state = await intercept(page, language, 3, width === 1440 ? "graphite" : "porcelain", true);
    await openProject(page, language, false);
    await expect(page.locator('.navigation-section-toggle[aria-expanded="true"]')).toHaveCount(0);
    await expect(page.locator('.project-explorer .tree-toggle[aria-expanded="true"]')).toHaveCount(0);
    await expect(page.locator('details[open]')).toHaveCount(0);
    if (process.env.KNX_DEVICES_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_DEVICES_CAPTURE_DIR, `collapsed-defaults-${language}-${width}.png`) });
    await openNavigationSections(page);
    const explorer = page.locator(".navigation-section").filter({ has: page.locator(".project-explorer") });
    const navigation = page.getByRole("button", { name: m["workbench.navigation"], exact: true });
    if (await navigation.getAttribute("aria-expanded") === "false") await navigation.click();
    const geometry = await explorer.evaluate(node => {
      const rows = []; let current: Element | null = node;
      while (current) { const style = getComputedStyle(current), box = current.getBoundingClientRect();
        rows.push({ cls: current.className, height: box.height, width: box.width, flex: style.flex, display: style.display, overflow: style.overflow }); current = current.parentElement; }
      return rows;
    });
    await expect(explorer, JSON.stringify(geometry)).toBeVisible();
    const before = (await explorer.boundingBox())!.height;
    await explorer.locator(".navigation-section-toggle").click();
    await expect(explorer.locator(".project-explorer")).toBeHidden();
    expect((await explorer.boundingBox())!.height).toBeLessThan(before);
    await explorer.locator(".navigation-section-toggle").click();
    await expect(explorer.locator(".project-explorer")).toBeVisible();
    await page.locator(".file-menu > summary").click();
    await expect(page.locator(".file-submenu-trigger > span:first-child")).toHaveText([m["fileMenu.exchange"], m["fileMenu.history"], m["fileMenu.help"]]);
    await page.keyboard.press("Escape");
    await openList(page, language);
    await page.locator(".devices-table tbody td:nth-child(3) button").filter({ hasText: /^Actuator 1$/ }).click();
    await expect(page.locator(".device-function-controls")).toHaveCount(0);
    await page.getByRole("tab", { name: m["workbench.parameters"], exact: true }).click();
    const parameterPanel = page.locator('[role="tabpanel"]:visible');
    // Closed by default on the first untouched visit: the area register is the folded overview.
    await expect(parameterPanel.locator('.parameter-workspace-nav [aria-current="page"]')).toHaveCount(0);
    await expect(parameterPanel.locator(".parameter-workspace-closed")).toBeVisible();
    await expect(parameterPanel.locator(".parameter-field select")).toBeHidden();
    await openParameterSections(page);
    await expect(parameterPanel.locator('.parameter-workspace-nav [aria-current="page"]')).toHaveCount(1);
    await expect(parameterPanel.locator(".parameter-field select")).toHaveValue("0");
    await parameterPanel.locator(".parameter-field select").selectOption("1");
    await page.getByRole("tab", { name: m["inspector.communicationObjects"], exact: true }).click();
    await expect(page.locator(".com-table-count")).toContainText("1");
    await page.locator(".com-object-channel-summary").click();
    await expect(page.locator(".com-object-name")).toHaveText("Rocker output");
    await page.locator('.com-object-detail > summary').click();
    // Issue #46: the object-local function control sits in the closed object settings.
    await expect(page.locator(".com-object-functions select")).toBeHidden();
    await page.locator(".com-object-more > summary").click();
    const mode = page.locator(".com-object-functions select");
    await expect(mode).toBeVisible();
    await expect(mode).toHaveValue("1");
    const positions = await page.locator(".com-object-type-fields").evaluate(node => {
      const dpt = node.querySelector('.dpt-field')!.getBoundingClientRect();
      const functions = node.querySelector('.com-object-functions')!.getBoundingClientRect();
      return { dptBottom: dpt.bottom, functionTop: functions.top, left: functions.left, right: functions.right };
    });
    expect(positions.functionTop).toBeGreaterThanOrEqual(positions.dptBottom);
    expect(positions.left).toBeGreaterThanOrEqual(0);
    expect(positions.right).toBeLessThanOrEqual(width);
    await mode.selectOption("2");
    await expect(page.locator(".com-object-name")).toHaveText("Left output");
    if (process.env.KNX_DEVICES_CAPTURE_DIR) {
      // Show the object-local control itself, not the page top above it.
      await mode.scrollIntoViewIfNeeded();
      await page.screenshot({ path: join(process.env.KNX_DEVICES_CAPTURE_DIR, `workbench-functions-${language}-${width}.png`) });
      await page.locator(".com-object-edit-fields").screenshot({ path: join(process.env.KNX_DEVICES_CAPTURE_DIR, `workbench-functions-editor-${language}-${width}.png`) });
    }
    await page.getByRole("tab", { name: m["workbench.parameters"], exact: true }).click();
    // The area chosen on the first visit stays open, and the object-local write shows in the shared row.
    await expect(parameterPanel.locator('.parameter-workspace-nav [aria-current="page"]')).toHaveCount(1);
    await expect(parameterPanel.locator(".parameter-workspace-nav button").filter({ hasText: "Buttons" })).toHaveAttribute("aria-current", "page");
    const parameterMode = parameterPanel.locator(".parameter-field select");
    await expect(parameterMode).toBeVisible();
    await expect(parameterMode).toHaveValue("2");
    const search = parameterPanel.getByRole("searchbox");
    await search.fill("no-match");
    await expect(parameterMode).toBeHidden();
    await search.fill("");
    await expect(parameterMode).toBeVisible();
    await expect(parameterMode).toHaveValue("2");
    if (process.env.KNX_DEVICES_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_DEVICES_CAPTURE_DIR, `workbench-parameters-${language}-${width}.png`) });
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    expect(state.unexpected).toEqual([]);
  });
}

for (const language of ["en", "de"] as const) {
  const m = language === "en" ? en : de;
  for (const width of [1440, 400]) {
    test(`${language} device list/editor retains filter, sort, scroll and multi-selection at ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 850 });
      const theme = width === 1440 && language === "en" ? "graphite" : "porcelain";
      const state = await intercept(page, language, 3, theme); await openProject(page, language); await openList(page, language);
      await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
      await expect(page.locator(".devices-table tbody tr")).toHaveCount(3);
      await expect(page.locator(".devices-table")).toContainText("Fictional Devices");
      if (process.env.KNX_DEVICES_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_DEVICES_CAPTURE_DIR, `devices-${language}-${width}.png`) });
      await page.getByRole("searchbox", { name: m["devices.filter"] }).fill("Actuator");
      await page.locator(".devices-table th button").filter({ hasText: new RegExp(`^${m["workbench.name"]}`) }).click();
      await page.getByRole("checkbox", { name: m["devices.select"].replace("{name}", "Actuator 1"), exact: true }).check();
      const scroller = page.locator(".devices-table-scroll");
      // Defaults fit the desktop pane; explicitly widen before testing scroll memory.
      await page.locator('[data-table-id="devices"]').getByRole("separator").last().press("End");
      await scroller.evaluate((node) => { node.scrollLeft = 180; });
      const scrollLeft = await scroller.evaluate((node) => node.scrollLeft);
      const before = state.requests.length;
      await page.locator(".devices-table-scroll").evaluate((node) => {
        node.addEventListener("click", () => { (window as unknown as { devicesReturnScroll: number }).devicesReturnScroll = node.scrollLeft; }, { once: true });
      });
      await page.locator(".devices-table tbody td:nth-child(3) button").filter({ hasText: /^Actuator 1$/ }).click();
      await expect(page.locator(".device-workspace h2")).toHaveText("Actuator 1");
      await expect(page.locator(".workbench-center .structure-workspace")).toHaveCount(0);
      await expect(page.locator(".workbench-center .device-workspace")).toHaveCount(1);
      await page.locator(".device-editor-navigation button").first().click();
      await expect(page.getByRole("searchbox", { name: m["devices.filter"] })).toHaveValue("Actuator");
      await expect(page.locator('.devices-table th[aria-sort="ascending"]')).toContainText(m["workbench.name"]);
      await expect(page.getByRole("checkbox", { name: m["devices.select"].replace("{name}", "Actuator 1"), exact: true })).toBeChecked();
      // Clicking a off-screen name can scroll its cell into view; capture the
      // actual post-click list position before it is hidden in the editor.
      expect(await scroller.evaluate((node) => node.scrollLeft)).toBe(await page.evaluate(() => (window as unknown as { devicesReturnScroll: number }).devicesReturnScroll));
      expect(scrollLeft).toBeGreaterThan(0);
      expect(state.requests.slice(before).every((request) => request.startsWith("GET ") || request === "POST /api/achievements/record")).toBe(true);
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth - innerWidth);
      const boxes = await page.locator(".workbench-panel-controls, .bulk-action-toolbar, .bulk-action-toolbar select, .bulk-action-toolbar button, .devices-toolbar, .workbench-center, .workbench-status").evaluateAll((nodes) => nodes.map((node) => {
        const box = node.getBoundingClientRect(); return { tag: node.tagName, cls: node.className, x: box.x, width: box.width, right: box.right };
      }));
      if (process.env.KNX_DEVICES_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_DEVICES_CAPTURE_DIR, `devices-selected-${language}-${width}.png`) });
      expect(overflow, JSON.stringify(boxes)).toBeLessThanOrEqual(1);
      expect(state.unexpected).toEqual([]);
    });
  }
  test(`${language} GA table and inspector device links share the central editor`, async ({ page }) => {
    const state = await intercept(page, language); await openProject(page, language);
    await page.getByRole("navigation", { name: m["workbench.navigation"], exact: true }).getByRole("button", { name: m["workbench.addresses"], exact: true }).click();
    // Issue #11: address lists start closed; this case works on rows, so open them like a user.
    // Issue #11: every installation's tree opens compact; open each one.
    await expect(page.locator("[data-ga-expand-all]").first()).toBeVisible();
    for (const button of await page.locator("[data-ga-expand-all]").all()) await button.click();
    await page.locator(".address-table .table-select").first().click();
    // The Inspector lists the address's members (one link here), like the table.
    await expect(page.locator(".inspector .ga-members .device-link")).toHaveCount(1);
    await page.locator(".ga-workspace .address-links-panel .device-link").click();
    await expect(page.locator(".device-workspace h2")).toHaveText("Actuator 1");
    await page.locator(".device-editor-navigation button").first().click();
    await page.locator(".inspector .ga-members .device-link").click();
    await expect(page.locator(".device-workspace h2")).toHaveText("Actuator 1");
    expect(state.unexpected).toEqual([]);
  });
}
for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  const m = language === "de" ? de : en;
  test(`${language} repeated diagnostic navigation stays selected at ${width}px (#23)`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    const theme = width === 1440 ? "porcelain" : "graphite";
    const state = await intercept(page, language, 3, theme);
    await openProject(page, language);
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    const sidebar = page.locator(".diagnostic-navigation");
    if (!await sidebar.isVisible()) {
      await page.getByRole("button", { name: m["workbench.navigation"], exact: true }).click();
    }
    const topology = page.getByRole("navigation", { name: m["workbench.navigation"], exact: true })
      .getByRole("button", { name: m["workbench.topology"], exact: true });
    await topology.click();
    const monitor = sidebar.getByRole("button", { name: m["toolbar.busMonitor"], exact: true });
    await monitor.click();
    await page.locator(".bus-diagnostics-tabs").getByRole("button", { name: m["addressProgramming.tab"], exact: true }).click();
    const address = page.locator('.address-programming-config input[name="address"]');
    await address.fill("1.1.2");
    for (let i = 0; i < 3; i++) {
      await monitor.click();
      await expect(monitor).toHaveAttribute("aria-current", "page");
      await expect(topology).not.toHaveAttribute("aria-current", "page");
      await expect(address).toBeVisible();
      await expect(address).toHaveValue("1.1.2");
    }
    if (process.env.KNX_ISSUE23_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_ISSUE23_CAPTURE_DIR, `monitor-${language}-${width}.png`) });
    const log = sidebar.getByRole("button", { name: m["toolbar.log"], exact: true });
    await log.click();
    const search = page.locator(".log-panel").getByRole("searchbox");
    await search.fill("keep my filter");
    for (let i = 0; i < 3; i++) {
      await log.click();
      await expect(log).toHaveAttribute("aria-current", "page");
      await expect(monitor).not.toHaveAttribute("aria-current", "page");
      await expect(search).toHaveValue("keep my filter");
    }
    await page.locator(".settings-navigation").getByRole("button", { name: m["toolbar.settings"], exact: true }).click();
    await expect(page.locator(".settings-panel")).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.locator(".settings-panel")).toBeVisible();
    if (await page.getByRole("button", { name: m["workbench.navigation"], exact: true }).getAttribute("aria-expanded") === "false") await page.getByRole("button", { name: m["workbench.navigation"], exact: true }).click();
    await openNavigationSections(page);
    await expect(log).not.toHaveAttribute("aria-current", "page");
    await page.getByRole("button", { name: m["settings.back"], exact: true }).click();
    await expect(page.locator(".settings-panel")).toHaveCount(0);
    if (await page.getByRole("button", { name: m["workbench.navigation"], exact: true }).getAttribute("aria-expanded") === "false") await page.getByRole("button", { name: m["workbench.navigation"], exact: true }).click();
    await expect(log).toHaveAttribute("aria-current", "page");
    await expect(search).toHaveValue("keep my filter");
    if (process.env.KNX_ISSUE23_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_ISSUE23_CAPTURE_DIR, `log-${language}-${width}.png`) });
    await topology.click();
    await expect(topology).toHaveAttribute("aria-current", "page");
    await expect(page.locator(".log-panel")).toHaveCount(0);
    await expect(page.locator(".bus-diagnostics-panel")).toBeHidden();
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    expect(state.requests.some((request) => /^POST .*\/(?:start|stop|plan|write|estimate|device-compare)$/.test(request))).toBe(false);
    expect(state.unexpected).toEqual([]);
  });
}

test("monitor source navigation keeps capture/polling alive with no reconnect, stop or write", async ({ page }) => {
  const state = await intercept(page, "en"); await openProject(page, "en");
  await page.locator(".diagnostic-navigation").getByRole("button", { name: en["toolbar.busMonitor"], exact: true }).click();
  await expect(page.locator(".bus-monitor-table .device-link")).toHaveCount(1);
  const before = state.requests.length;
  const settingsPolls = state.polls;
  await page.locator(".workbench-toolbar").getByRole("button", { name: en["toolbar.settings"], exact: true }).click();
  await expect(page.locator(".bus-diagnostics-panel")).toBeHidden();
  await expect.poll(() => state.polls).toBeGreaterThan(settingsPolls);
  await page.getByRole("button", { name: en["settings.back"], exact: true }).click();
  await expect(page.locator(".bus-monitor-table .device-link")).toHaveCount(1);
  const polls = state.polls;
  await page.locator(".bus-monitor-table .device-link").click();
  await expect(page.locator(".device-workspace h2")).toHaveText("Actuator 1");
  await expect.poll(() => state.polls).toBeGreaterThan(polls);
  await page.locator(".device-editor-navigation button").first().click();
  await expect(page.locator(".bus-monitor-table tbody tr")).toHaveCount(1);
  await expect(page.locator('.bus-monitor-table tr[aria-selected="true"]')).toHaveCount(0);
  expect(state.requests.slice(before).every((request) => request.startsWith("GET ") || request === "POST /api/achievements/record")).toBe(true);
  expect(state.requests.some((request) => /monitor\/(start|stop)|bus\/write/.test(request))).toBe(false);
  expect(state.unexpected).toEqual([]);
});
for (const panel of ["download", "address", "checks", "scan"] as const) {
  test(`${panel} links open the device editor without invoking its hardware operation`, async ({ page }) => {
    const state = await intercept(page, "en"); await openProject(page, "en");
    await page.locator(".diagnostic-navigation").getByRole("button", { name: en["toolbar.busMonitor"], exact: true }).click();
    const labels = { download: en["deviceDownload.title"], address: en["addressProgramming.tab"], checks: en["deviceChecks.tab"], scan: en["lineScan.title"] };
    await page.locator(".bus-diagnostics-tabs").getByRole("button", { name: labels[panel], exact: true }).click();
    if (panel === "download") await page.locator(".device-download-config select[data-download-device]").selectOption({ label: "1.1.2 — Actuator 1" });
    if (panel === "address") await page.locator('.address-programming-config input[name="address"]').fill("1.1.2");
    const selector = { download: ".device-download-panel .device-link", address: ".address-programming-panel .device-link",
      checks: ".device-checks-readiness .device-link", scan: ".line-scan-results .device-link" }[panel];
    await page.locator(selector).first().click(); await expect(page.locator(".device-workspace h2")).toHaveText("Actuator 1");
    expect(state.requests.some((request) => /^POST .*\/(?:start|plan|write|estimate|device-compare)$/.test(request))).toBe(false);
    expect(state.unexpected).toEqual([]);
  });
}
test("catalogue failures retain every project device and explain unavailable product data", async ({ page }) => {
  const state = await intercept(page, "en"); state.failCatalogue = true;
  await openProject(page, "en"); await openList(page, "en");
  await expect(page.locator(".devices-workspace [role=alert]")).toContainText("product database unavailable");
  await expect(page.locator(".devices-table tbody tr")).toHaveCount(3);
  expect(state.unexpected).toEqual([]);
});
test("measures a 5000-device production table and keeps the last device searchable", { tag: "@exclusive" }, async ({ page }, info) => {
  test.setTimeout(120_000);
  const state = await intercept(page, "en", 5000); await openProject(page, "en");
  const started = Date.now(); await openList(page, "en");
  await expect(page.locator(".devices-table tbody tr")).toHaveCount(5000);
  const renderMs = Date.now() - started;
  const filterStarted = Date.now(); await page.getByRole("searchbox", { name: en["devices.filter"] }).fill("Actuator 5000");
  await expect(page.locator(".devices-table tbody tr")).toHaveCount(1);
  await expect(page.locator(".devices-table")).toContainText("Actuator 5000");
  await info.attach("devices-performance", { body: JSON.stringify({ devices: 5000, renderMs, filterMs: Date.now() - filterStarted }), contentType: "application/json" });
  expect(state.requests.filter((request) => request === "GET /api/devices")).toHaveLength(1);
  expect(state.unexpected).toEqual([]);
});


for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} retained comparison yields Settings and resumes modal ownership at ${width}px`, async ({ page }, testInfo) => {
    const m = language === "de" ? de : en;
    await page.setViewportSize({ width, height: 1000 });
    const state = await intercept(page, language);
    await openProject(page, language); await openList(page, language);
    const opener = page.getByRole("button", { name: m["compare.all"], exact: true });
    await opener.click();
    const comparison = page.locator(".device-comparison-dialog");
    const gateway = comparison.locator("input");
    await gateway.fill("192.0.2.17:3671");
    // The compare-local transport choice sits between the gateway field and Confirm.
    await page.keyboard.press("Tab");
    await expect(comparison.getByRole("combobox")).toBeFocused();
    // This is the last real control used, not merely the dialog's default target.
    await page.keyboard.press("Tab");
    await expect(comparison.getByRole("button", { name: m["compare.confirm"], exact: true })).toBeFocused();
    await comparison.evaluate(node => { node.setAttribute("data-retained-modal", "same"); });
    async function settingsViaPalette() {
      await page.keyboard.press("Control+Shift+P");
      const palette = page.getByRole("dialog").getByRole("combobox");
      await palette.fill(m["toolbar.settings"]); await palette.press("Enter");
      await expect(page.locator("#settings-panel-title")).toBeFocused();
    }
    await settingsViaPalette();
    const workspace = page.locator(".settings-workspace");
    await expect(comparison).toBeHidden();
    await expect(comparison).toHaveAttribute("data-retained-modal", "same");
    await expect(gateway).toHaveValue("192.0.2.17:3671");
    // Pointer activation and the following native Tab exercise the normal page,
    // rather than inferring usability from the absence of an inert attribute.
    await workspace.locator('a[href="#settings-autosave"]').click();
    await expect(workspace.locator("#settings-autosave")).toBeFocused();
    await page.keyboard.press("Tab");
    await expect(workspace.getByRole("checkbox", { name: m["settings.autosaveEnabled"], exact: true })).toBeFocused();
    await workspace.getByRole("button", { name: m["settings.back"], exact: true }).click();
    await expect(comparison).toBeVisible();
    await expect(comparison.getByRole("button", { name: m["compare.confirm"], exact: true })).toBeFocused();
    await expect(gateway).toHaveValue("192.0.2.17:3671");
    await page.keyboard.press("Tab");
    await expect(comparison.getByRole("button", { name: m["deviceChecks.cancel"], exact: true })).toBeFocused();
    await page.keyboard.press("Tab"); await expect(gateway).toBeFocused();
    await page.keyboard.press("Shift+Tab");
    await expect(comparison.getByRole("button", { name: m["deviceChecks.cancel"], exact: true })).toBeFocused();
    // Resumed isolation must still block ordinary toolbar pointer actions.
    const toolbarSettings = page.locator(".workbench-toolbar").getByRole("button", { name: m["toolbar.settings"], exact: true, includeHidden: true });
    expect(await toolbarSettings.isVisible()).toBe(true);
    await expect(toolbarSettings.click({ trial: true, timeout: 300 })).rejects.toThrow();
    await settingsViaPalette();
    await testInfo.attach("settings-paused-comparison", { body: await page.screenshot(), contentType: "image/png" });
    // Left navigation is usable while the comparison stays mounted and paused.
    await openNavigationSections(page);
    await page.locator(".workbench-navigation").getByRole("button", { name: m["workbench.overview"], exact: true }).click();
    await expect(workspace).toHaveCount(0);
    await expect(page.locator(".dashboard")).toBeVisible();
    await expect(comparison).toBeHidden();
    await toolbarSettings.click();
    await expect(page.locator("#settings-panel-title")).toBeFocused();
    await workspace.getByRole("button", { name: m["settings.back"], exact: true }).click();
    await expect(page.locator(".dashboard")).toBeVisible();
    await expect(comparison).toBeHidden();
    await openNavigationSections(page);
    await page.locator(".workbench-navigation").getByRole("button", { name: m["workbench.devices"], exact: true }).click();
    await expect(comparison).toHaveAttribute("data-retained-modal", "same");
    await expect(gateway).toHaveValue("192.0.2.17:3671");
    await expect(comparison.getByRole("button", { name: m["deviceChecks.cancel"], exact: true })).toBeFocused();
    await testInfo.attach("comparison-resumed", { body: await page.screenshot(), contentType: "image/png" });
    await page.keyboard.press("Escape");
    await expect(comparison).toHaveCount(0);
    await expect(opener).toBeFocused();
    // Closing the resumed modal releases navigation, with its original trigger restored.
    await page.keyboard.press("Tab");
    await expect(page.locator(".settings-workspace")).toHaveCount(0);
    await openNavigationSections(page);
    await page.locator(".workbench-navigation").getByRole("button", { name: m["workbench.overview"], exact: true }).click();
    await toolbarSettings.click();
    await expect(page.locator("#settings-panel-title")).toBeFocused();
    expect(state.requests.filter(request => /compare|device-read|device-write/.test(request))).toEqual([]);
    expect(state.unexpected).toEqual([]);
  });
}
