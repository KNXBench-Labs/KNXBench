/** Captures the manual's step-by-step diagnostic export stills from the real UI and simulator. */
// Production frontend + real HTTP handlers + the regular KNX simulator on loopback.
// Nothing here is mocked: every pixel comes from an actual preview, read and save.
import {test, expect, type Page, type Locator} from "@playwright/test";
import {resolve} from "node:path";

const target = process.env.KNX_DIAG_GUIDE_OUT;

async function dismissToasts(page: Page): Promise<void> {
  const buttons = page.getByRole("button", {name: "Dismiss", exact: true});
  // Toasts slide in; a forced click avoids waiting on the entrance animation.
  for (let attempt = 0; attempt < 5 && (await buttons.count()); attempt++) {
    await buttons.first().click({force: true});
  }
  await expect(buttons).toHaveCount(0);
}

async function shot(locator: Locator, name: string): Promise<void> {
  if (target) await locator.screenshot({path: resolve(target, `porcelain-diagnostic-export-${name}.png`), animations: "disabled"});
}

test("step-by-step stills for the diagnostic export guide", async ({page}) => {
  test.skip(!target, "KNX_DIAG_GUIDE_OUT names the screenshot directory");
  await page.setViewportSize({width: 1280, height: 860});
  await page.addInitScript(() => {
    localStorage.setItem("knx-desktop:theme", "porcelain");
    localStorage.setItem("knx-desktop:ui-language", "en");
  });
  await page.route("**/api/settings", async (route) => {
    if (route.request().method() !== "GET") {
      await route.continue();
      return;
    }
    const response = await route.fetch();
    const body = await response.json();
    body.settings.uiLanguage = "en";
    await route.fulfill({response, json: body});
  });
  await page.goto("/");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "porcelain");
  await dismissToasts(page);

  // Step 1: File menu entry.
  await page.locator(".file-menu > summary").click();
  const entry = page.locator(".file-menu [data-diagnostics-open]");
  await expect(entry).toBeVisible();
  await entry.hover();
  const menu = await page.locator(".file-menu").boundingBox();
  const item = await entry.boundingBox();
  if (target) {
    const right = Math.max(menu!.x + menu!.width, item!.x + item!.width);
    await page.screenshot({
      path: resolve(target, "porcelain-diagnostic-export-1-file-menu.png"),
      // The hover highlight fades in; finish transitions so reruns are byte-identical.
      animations: "disabled",
      clip: {x: 0, y: 0, width: Math.min(1280, right + 120), height: item!.y + item!.height + 120},
    });
  }

  // Step 2: device, interface, preview button.
  await entry.click();
  const dialog = page.getByRole("dialog", {name: "Export diagnostic data", exact: true});
  await expect(dialog).toBeVisible();
  await dialog.locator("[data-diagnostics-device]").selectOption("1");
  await dialog.locator("[data-diagnostics-gateway]").fill("192.0.2.10:3671");
  await shot(dialog, "2-device-and-interface");

  // Step 3: preview, read consent, start button.
  await dialog.locator("[data-diagnostics-preview]").click();
  await expect(dialog.locator("[data-diagnostics-review]")).toContainText("1.1.25");
  await dialog.locator("[data-diagnostics-read-consent]").check();
  await dialog.locator("[data-diagnostics-start]").scrollIntoViewIfNeeded();
  await expect(dialog.locator("[data-diagnostics-start]")).toBeEnabled();
  await shot(dialog, "3-review-and-read");

  // Step 4: result, save consent, save button.
  await dialog.locator("[data-diagnostics-start]").click();
  await expect(dialog.locator("[data-diagnostics-result]")).toBeVisible();
  await expect(dialog.locator("[data-diagnostics-result]")).toContainText("hardware-type: Read");
  await expect(dialog.getByRole("alert")).toHaveCount(0);
  await dialog.locator("[data-diagnostics-share-consent]").check();
  await dialog.locator("[data-diagnostics-save]").scrollIntoViewIfNeeded();
  await shot(dialog, "4-result-and-save");

  // The save is real too: the stills must not show a button that does not work.
  const download = page.waitForEvent("download");
  await dialog.locator("[data-diagnostics-save]").click();
  expect((await download).suggestedFilename()).toMatch(/^knxbench-device-diagnostics-[a-f0-9]{12}\.zip$/);
});
