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
  // Legacy accordions (e.g. function controls) or the Parameters-tab area register.
  const areas = page.locator(".parameter-workspace-nav button:visible");
  await expect(page.locator('.parameter-section > summary:visible, .parameter-workspace-nav button:visible').first()).toBeVisible();
  for (const summary of await page.locator('.parameter-section > summary:visible').all()) {
    if (!await summary.locator("..").evaluate(node => node.hasAttribute("open"))) await summary.click();
  }
  for (const toggle of await page.locator('.parameter-group-toggle:visible').all()) {
    if (await toggle.getAttribute("aria-expanded") === "false") await toggle.click();
  }
  // The register shows one area at a time; with several, a case must choose its area explicitly.
  const count = await areas.count();
  if (count > 1) throw new Error(`parameter workspace has ${count} areas; open the intended area explicitly`);
  if (count === 1 && await areas.first().getAttribute("aria-current") !== "page") await areas.first().click();
}
