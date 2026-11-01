/** Captures the manual's diagnostic export and Report problem stills from real UI and simulator. */
// Production frontend + real HTTP handlers + the regular KNX simulator on loopback.
// Nothing here is mocked: every pixel comes from an actual preview, read and save.
import {test, expect, type Page, type Locator} from "@playwright/test";
import {readFileSync} from "node:fs";
import {resolve} from "node:path";
import { openFileSubmenu } from "./file-menu";

const target = process.env.KNX_DIAG_GUIDE_OUT;

async function dismissToasts(page: Page): Promise<void> {
  const buttons = page.getByRole("button", {name: "Dismiss", exact: true});
  // Start-up notices arrive one after another; keep dismissing until none is left.
  // Dispatch on the × itself: a coordinate click during the slide-in can land on the
  // toast's full-area "Open message" button instead.
  await expect(async () => {
    while (await buttons.count()) {
      const button = buttons.first();
      await button.dispatchEvent("click");
      await expect(button).not.toBeAttached({timeout: 2_000}).catch(() => undefined);
    }
    await expect(buttons).toHaveCount(0, {timeout: 750});
  }).toPass({timeout: 15_000});
}

async function shot(locator: Locator, name: string): Promise<void> {
  if (target) await locator.screenshot({path: resolve(target, `porcelain-diagnostic-export-${name}.png`), animations: "disabled"});
}

async function reportShot(locator: Locator, name: string): Promise<void> {
  if (target) await locator.screenshot({path: resolve(target, `porcelain-report-problem-${name}.png`), animations: "disabled"});
}

/** Clips the open File menu down to just below the hovered entry. */
async function menuShot(page: Page, entry: Locator, file: string): Promise<void> {
  await entry.hover();
  const menu = await page.locator(".file-menu").boundingBox();
  const item = await entry.boundingBox();
  if (!target) return;
  const right = Math.max(menu!.x + menu!.width, item!.x + item!.width);
  await page.screenshot({
    path: resolve(target, file),
    // The hover highlight fades in; finish transitions so reruns are byte-identical.
    animations: "disabled",
    clip: {x: 0, y: 0, width: Math.min(1280, right + 120), height: item!.y + item!.height + 120},
  });
}

test("step-by-step stills for the diagnostic export guide", async ({page}) => {
  test.skip(!target, "KNX_DIAG_GUIDE_OUT names the screenshot directory");
  // A real read plus the whole report wizard and its save outlasts the base 45 s.
  test.setTimeout(120_000);
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
  // The startup joke (holidays, late night) is picked once on mount: start on a neutral
  // weekday morning, then return to the real time for everything that follows.
  await page.clock.install({time: new Date("2026-11-04T10:30:00")});
  await page.goto("/");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "porcelain");
  await page.clock.setSystemTime(new Date());
  await dismissToasts(page);

  // Step 1: File menu entries (Report problem… first, then the device export).
  await page.locator(".file-menu > summary").click();
  await openFileSubmenu(page, "help");
  const report = page.locator(".file-menu [data-report-problem-open=menu]");
  await expect(report).toBeVisible();
  await menuShot(page, report, "porcelain-report-problem-0-file-menu.png");
  const entry = page.locator(".file-menu [data-diagnostics-open]");
  await expect(entry).toBeVisible();
  await menuShot(page, entry, "porcelain-diagnostic-export-1-file-menu.png");

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

  // Step 4: result and the hand-over to the Report problem wizard (DIAG-03).
  await dialog.locator("[data-diagnostics-start]").click();
  // The read itself may take a few seconds (the app allows up to 30 s plus connect time).
  await expect(dialog.locator("[data-diagnostics-result]")).toBeVisible({timeout: 45_000});
  await expect(dialog.locator("[data-diagnostics-result]")).toContainText("hardware-type: Read");
  await expect(dialog.getByRole("alert")).toHaveCount(0);
  await dialog.locator("[data-report-problem-open=diagnostics]").scrollIntoViewIfNeeded();
  await shot(dialog, "4-result-and-report");

  // The hand-over is real too: the stills must not show a button that does not work.
  // Walk the Report problem wizard it opens through all four steps (Alpha 19).
  // A taller window keeps each step's full list in one still.
  await page.setViewportSize({width: 1280, height: 1000});
  await dialog.locator("[data-report-problem-open=diagnostics]").click();
  const wizard = page.getByRole("dialog", { name: "Report a problem", exact: true });
  await expect(wizard).toBeVisible();
  await expect(wizard.locator("[data-report-section=operation] input")).toBeChecked();
  await wizard.locator(".project-wizard-steps").getByRole("button", { name: /Problem/ }).click();
  await expect(wizard.locator("[data-report-type=deviceReading]")).toBeChecked();
  await reportShot(wizard, "1-problem");

  await wizard.locator("[data-report-next]").click();
  await expect(wizard.locator("[data-report-section=operation] input")).toBeChecked();
  await expect(wizard.locator("[data-report-total]")).toBeVisible();
  await reportShot(wizard, "2-contents");

  await wizard.locator("[data-report-next]").click();
  await expect(wizard.locator("[data-report-show=legacyReports]")).toBeVisible();
  await reportShot(wizard, "3-preview");
  // Show content is real too: it displays the part exactly as it goes into the ZIP.
  await wizard.locator("[data-report-show=legacyReports]").click();
  await expect(wizard.locator("[data-report-text=legacyReports]")).toContainText("hardware-type");

  await wizard.locator("[data-report-next]").click();
  await wizard.locator("[data-report-description]").fill("Fictional example: the hardware type of the test device looks unexpected.");
  await expect(wizard.locator("[data-report-save]")).toBeDisabled();
  await wizard.locator("[data-report-consent]").check();
  const download = page.waitForEvent("download");
  await wizard.locator("[data-report-save]").click();
  const zip = readFileSync(await (await download).path());
  // The saved bundle carries the read's own recording, not only its short report.
  for (const name of ["manifest.json", "report.json", "recording.jsonl"]) expect(zip.includes(Buffer.from(name))).toBe(true);
  await expect(wizard.locator("[data-report-requested]")).toBeVisible();
  await wizard.locator("[data-report-file-confirm]").check();
  await expect(wizard.locator("a[data-report-mail]")).toBeVisible();
  await reportShot(wizard, "4-save-and-send");
});
