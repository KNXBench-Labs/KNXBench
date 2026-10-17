/** Explicit disclosure setup for browser cases testing content rather than initial folding. */
import { expect, type Page } from "@playwright/test";

export async function openNavigationSections(page: Page) {
  const navigation = page.locator('.workbench-panel-controls button[aria-expanded]').first();
  await expect(navigation).toBeVisible();
  if (await navigation.getAttribute("aria-expanded") === "false") await navigation.click();
  for (const toggle of await page.locator('.navigation-section-toggle').all()) {
    if (await toggle.getAttribute("aria-expanded") === "false") await toggle.click();
  }
}

export async function openExplorerBranches(page: Page) {
  await page.locator(".project-explorer, .workbench-panel-controls").first().waitFor({ state: "attached" });
  if (await page.locator(".workbench-panel-controls").count()) await openNavigationSections(page);
  const explorer = page.locator(".project-explorer");
  await expect(explorer).toBeAttached();
  await explorer.locator(".explorer-toolbar button").first().click();
}

export async function openParameterSections(page: Page) {
  await expect(page.locator('.parameter-section > summary:visible').first()).toBeVisible();
  for (const summary of await page.locator('.parameter-section > summary:visible').all()) {
    if (!await summary.locator("..").evaluate(node => node.hasAttribute("open"))) await summary.click();
  }
  for (const toggle of await page.locator('.parameter-group-toggle:visible').all()) {
    if (await toggle.getAttribute("aria-expanded") === "false") await toggle.click();
  }
}
