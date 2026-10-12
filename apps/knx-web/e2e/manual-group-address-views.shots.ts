/** Captures the manual's four group-address presentation screenshots from a real server. */

// Same setup as manual-screenshots.shots.ts (fresh knx-server, production
// frontend, fictional tools/manual_sample_project.py, loopback-only network
// namespace), but kept separate: that spec's set is pinned to the published
// Alpha 9 image, while these views exist only in source builds after it.
// The few helpers are repeated on purpose so the pinned spec stays unchanged.

import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const target = resolve(
  process.env.MANUAL_SCREENSHOT_DIR ?? fileURLToPath(new URL("../../../docs/assets/screenshots", import.meta.url)),
);
mkdirSync(target, { recursive: true });

async function dismissToasts(page: Page): Promise<void> {
  const buttons = page.getByRole("button", { name: "Dismiss" });
  while (await buttons.count()) {
    const button = await buttons.first().elementHandle();
    if (!button) continue;
    await button.click();
    await expect.poll(() => button.evaluate((node) => node.isConnected)).toBe(false);
    await button.dispose();
  }
}

async function shot(page: Page, name: string): Promise<void> {
  await expect(page.locator("html")).toHaveAttribute("data-theme", "porcelain");
  await page.waitForTimeout(400);
  await dismissToasts(page);
  // Clicks can scroll the shared main area; every shot starts at the top.
  await page.evaluate(() => {
    for (const element of document.querySelectorAll("*")) if (element.scrollTop) element.scrollTop = 0;
  });
  await page.screenshot({ path: `${target}/porcelain-${name}.png` });
  console.log(`Captured porcelain-${name}.png`);
}

const view = (page: Page, value: string) => page.locator("[data-ga-view]").selectOption(value);
const scopeButton = (page: Page, name: string) =>
  page.locator(".ga-workspace [data-ga-scope]").filter({ hasText: name }).first();

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem("knx-desktop:theme", "porcelain");
    localStorage.setItem("knx-desktop:ui-language", "en");
  });
});

test("manual screenshots of the four group-address presentations", async ({ page }) => {
  const failures: string[] = [];
  page.on("pageerror", (error) => failures.push(String(error)));

  await page.goto("/");
  const guide = page.getByRole("dialog", { name: "Welcome to KNXBench" });
  await expect(guide).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(guide).toBeHidden();
  await page.getByRole("button", { name: /Import ETS project/ }).first().click();
  await page.getByRole("button", { name: "sample-house.knxproj" }).click();
  await expect(page.getByText("Project status")).toBeVisible({ timeout: 15_000 });
  await dismissToasts(page);

  await page.getByRole("navigation", { name: "Navigation" }).getByRole("button", { name: "Group addresses", exact: true }).click();
  const workspace = page.locator(".ga-workspace");
  await expect(workspace).toBeVisible();
  // A fresh settings directory: the tree table is the default.
  await expect(page.locator("[data-ga-view]")).toHaveValue("tree");

  // Tree table: one address selected, Lighting / Status collapsed to show the disclosure.
  await page.getByRole("button", { name: "0/0/2", exact: true }).first().click();
  await expect(page.locator(".workbench-pane-right").getByRole("textbox", { name: "Name", exact: true })).toHaveValue("Kitchen light");
  const status = workspace.locator("[data-ga-expand]").filter({ hasText: "Status" }).first();
  await status.click();
  await expect(status).toHaveAttribute("aria-expanded", "false");
  await shot(page, "ga-view-tree");

  // Column browser: Lighting → Switching; the selection is still Kitchen light.
  await view(page, "columns");
  await scopeButton(page, "Lighting").click();
  await scopeButton(page, "Switching").click();
  await expect(page.locator("[data-ga-current-scope]")).toHaveText("Switching");
  await shot(page, "ga-view-columns");

  // Address atlas, same scope and selection.
  await view(page, "atlas");
  await expect(workspace.locator("[data-ga-atlas]")).toBeVisible();
  await expect(workspace.locator('[data-ga-atlas] [aria-pressed="true"]')).toContainText("Kitchen light");
  await shot(page, "ga-view-atlas");

  // Connection matrix over all ranges.
  await page.locator(".ga-breadcrumb button").click();
  await view(page, "matrix");
  await expect(workspace.locator("[data-ga-matrix]")).toBeVisible();
  await expect(workspace.locator("[data-ga-cell]").first()).toBeVisible();
  // Fold the column chooser (open by default) so the grid itself is in frame.
  await workspace.locator(".ga-column-picker summary").click();
  await shot(page, "ga-view-matrix");

  await view(page, "tree");
  expect(failures).toEqual([]);
});
