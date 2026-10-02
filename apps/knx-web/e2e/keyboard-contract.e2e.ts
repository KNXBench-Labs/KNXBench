/** Tests keyboard scrolling, modal isolation and tooltip geometry in Chromium. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";

async function fixture(page: Page, lang = "en", scale = 1) {
  const unexpected: string[] = [];
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.route("**/api/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    let body: unknown = [];
    if (path === "/api/catalog/items") body = Array.from({ length: 40 }, (_, i) => ({ id: `fixture-${i}`, manufacturerId: "fixture", name: `Fixture catalog ${i}`, number: null, visibleDescription: null, productRefId: null, hardware2programRefId: null }));
    else if (path !== "/api/catalog/manufacturers") unexpected.push(path);
    await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
  });
  await page.goto(`/e2e/keyboard-fixture.html?lang=${lang}&scale=${scale}`);
  return () => { expect(unexpected).toEqual([]); expect(errors).toEqual([]); };
}

for (const kind of ["search", "palette", "catalog"]) {
  test(`${kind} keeps the active keyboard option visible and leaves focus on the combobox`, async ({ page }) => {
    await page.setViewportSize({ width: 640, height: 480 });
    const check = await fixture(page);
    await page.getByRole("button", { name: kind, exact: true }).click();
    const input = page.locator('input[role="combobox"]');
    if (kind === "search") await input.fill("fixture device");
    if (kind === "catalog") await expect(page.getByRole("listbox").getByRole("option")).toHaveCount(40);
    for (let i = 0; i < 30; i++) await input.press("ArrowDown");
    await expect(input).toBeFocused();
    const geometry = await input.evaluate((element) => {
      const option = document.getElementById(element.getAttribute("aria-activedescendant")!)!;
      const box = option.getBoundingClientRect();
      const panel = option.closest(".search-panel, .catalog-workspace")!.getBoundingClientRect();
      return { top: box.top, bottom: box.bottom, panelTop: panel.top, panelBottom: panel.bottom, viewport: innerHeight };
    });
    expect(geometry.top).toBeGreaterThanOrEqual(geometry.panelTop - 1);
    expect(geometry.bottom).toBeLessThanOrEqual(Math.min(geometry.panelBottom, geometry.viewport) + 1);
    if (kind === "catalog") await expect(page.locator(".catalog-create-row")).toHaveCount(0);
    check();
  });
}

test("modal background is inert, absent from AX, restored after nested close, and cannot steal focus", async ({ page }) => {
  const check = await fixture(page);
  const opener = page.getByRole("button", { name: "modal", exact: true });
  await opener.click();
  const background = page.locator("[data-background]");
  await expect(background).toHaveAttribute("inert", "");
  await expect(background).toHaveAttribute("aria-hidden", "true");
  await page.locator("#background-action").evaluate((el: HTMLElement) => el.focus());
  await expect(page.getByRole("button", { name: "Nested dialog", exact: true })).toBeFocused();
  const cdp = await page.context().newCDPSession(page);
  const ax = await cdp.send("Accessibility.getFullAXTree");
  expect(ax.nodes.filter((node) => !node.ignored && node.name?.value === "Background action")).toHaveLength(0);
  await page.getByRole("button", { name: "Nested dialog", exact: true }).click();
  await expect(page.getByRole("dialog", { name: "Nested fixture", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Close nested", exact: true }).click();
  await expect(background).toHaveAttribute("inert", "");
  await expect(page.getByRole("button", { name: "Nested dialog", exact: true })).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(background).not.toHaveAttribute("inert", "");
  await expect(background).not.toHaveAttribute("aria-hidden", "true");
  await expect(opener).toBeFocused();
  await cdp.detach();
  check();
});

for (const lang of ["en", "de"] as const) for (const width of [360, 640, 1440]) for (const scale of [1, 1.5]) {
  test(`help keeps its description and popup in the viewport: ${lang} ${width}px scale ${scale}`, async ({ page }) => {
    await page.setViewportSize({ width, height: 640 });
    const check = await fixture(page, lang, scale);
    const messages = lang === "de" ? de : en;
    const trigger = page.getByRole("button", { name: messages["help.tip.comFlags.label"], exact: true });
    const described = await trigger.getAttribute("aria-describedby");
    await trigger.focus();
    const bubble = page.locator(".help-tip-bubble.is-open");
    await expect(bubble).toHaveCount(1);
    await expect(bubble).toHaveText(messages["help.tip.comFlags.text"]);
    const box = await bubble.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.x).toBeGreaterThanOrEqual(0);
    expect(box!.y).toBeGreaterThanOrEqual(0);
    expect(box!.x + box!.width).toBeLessThanOrEqual(width + 1);
    expect(box!.y + box!.height).toBeLessThanOrEqual(641);
    const cdp = await page.context().newCDPSession(page);
    const ax = await cdp.send("Accessibility.getFullAXTree");
    const button = ax.nodes.find((node) => !node.ignored && node.role?.value === "button" && node.name?.value === messages["help.tip.comFlags.label"]);
    expect(button?.description?.value).toBe(messages["help.tip.comFlags.text"]);
    await trigger.press("Escape");
    await expect(trigger).toHaveAttribute("aria-describedby", described!);
    await expect(page.locator(`[id=${JSON.stringify(described)}]`)).toHaveText(messages["help.tip.comFlags.text"]);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await cdp.detach();
    check();
  });
}

test("a help tooltip within a modal retains its local accessible description", async ({ page }) => {
  const check = await fixture(page);
  await page.getByRole("button", { name: "modal", exact: true }).click();
  const trigger = page.getByRole("button", { name: en["help.tip.comFlags.label"], exact: true });
  await trigger.focus();
  const description = await trigger.evaluate((el) => document.getElementById(el.getAttribute("aria-describedby")!)!.closest("[inert], [aria-hidden='true']") === null);
  expect(description).toBe(true);
  await trigger.press("Escape");
  await expect(page.getByRole("dialog", { name: "Fixture modal", exact: true })).toBeVisible();
  await trigger.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  check();
});
