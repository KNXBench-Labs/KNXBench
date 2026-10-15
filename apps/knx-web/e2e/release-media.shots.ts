/** Captures Alpha 12 candidate media from the real server and a fictional project. */
import {expect, test, type Page} from "@playwright/test";
import {mkdirSync} from "node:fs";
import {resolve} from "node:path";

const target = process.env.KNX_RELEASE_MEDIA_DIR;
if (!target) throw new Error("set KNX_RELEASE_MEDIA_DIR to a task-owned capture directory");
mkdirSync(target, {recursive: true});

async function dismissToasts(page: Page): Promise<void> {
  const buttons = page.getByRole("button", {name: "Dismiss", exact: true});
  while (await buttons.count()) {
    const node = await buttons.first().elementHandle();
    if (!node) continue;
    await node.click();
    await expect.poll(() => node.evaluate(element => element.isConnected)).toBe(false);
    await node.dispose();
  }
}

async function shot(page: Page, name: string): Promise<void> {
  await dismissToasts(page);
  await page.waitForTimeout(400);
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
  await page.screenshot({path: resolve(target!, name + ".png")});
}

async function nav(page: Page, name: string): Promise<void> {
  await page.getByRole("navigation", {name: "Navigation", exact: true})
    .getByRole("button", {name, exact: true}).click();
}

async function theme(page: Page, value: string): Promise<void> {
  await page.getByRole("button", {name: "Settings", exact: true}).first().click();
  const settings = page.getByRole("dialog");
  await settings.getByLabel("Theme", {exact: true}).selectOption(value);
  await expect(page.locator("html")).toHaveAttribute("data-theme", value);
  await expect(settings.getByRole("status").filter({hasText: "Theme saved."})).toBeVisible();
  await settings.focus();
  await page.keyboard.press("Escape");
  await expect(settings).toBeHidden();
}

test("Alpha 12 website and README stills, real application without hardware", async ({page}) => {
  const errors: string[] = [];
  const forbidden: string[] = [];
  page.on("pageerror", error => errors.push(error.message));
  page.on("request", request => {
    const path = new URL(request.url()).pathname;
    if (/\/api\/(support\/diagnose|support\/export|device-download\/start|device-address\/start)/.test(path)) {
      forbidden.push(path);
    }
  });
  // This test must run in the isolated loopback-only namespace documented in the config.
  // No application responses are replaced; the sample is imported by the actual server.
  await page.addInitScript(() => {
    localStorage.setItem("knx-desktop:theme", "graphite");
    localStorage.setItem("knx-desktop:ui-language", "en");
  });
  await page.goto("/");
  const guide = page.getByRole("dialog", {name: "Welcome to KNXBench", exact: true});
  await expect(guide).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(guide).toBeHidden();
  await page.getByRole("button", {name: /Import ETS project/}).first().click();
  await page.getByRole("button", {name: "sample-house.knxproj", exact: true}).click();
  await expect(page.getByText("Project status", {exact: true})).toBeVisible({timeout: 20_000});
  await dismissToasts(page);

  await nav(page, "Group addresses");
  await page.locator("[data-ga-expand-all]").click();
  await page.getByRole("button", {name: "0/0/2", exact: true}).first().click();
  await expect(page.locator(".workbench-pane-right").getByRole("textbox", {name: "Name", exact: true}))
    .toHaveValue("Kitchen light");
  await shot(page, "graphite-addresses");

  await nav(page, "Devices");
  await expect(page.getByRole("button", {name: "Compare all shown", exact: true})).toBeVisible();
  await shot(page, "graphite-devices");

  await page.getByRole("button", {name: "1.1.1 Switch actuator ground floor", exact: true}).first().click();
  const tabs = page.getByRole("tablist", {name: "Switch actuator ground floor", exact: true});
  await tabs.getByRole("tab", {name: "Instructions", exact: true}).click();
  const instructions = page.locator(".device-instructions");
  await expect(instructions).toBeVisible();
  await instructions.locator("textarea").fill("Sample house — kitchen lighting.\nLabel the outgoing circuits before handover.");
  await instructions.getByRole("button", {name: "Preview change", exact: true}).click();
  const preview = instructions.getByRole("region", {name: "Change preview", exact: true});
  await expect(preview).toContainText("Label the outgoing circuits");
  await preview.getByRole("button", {name: "Apply change", exact: true}).click();
  await expect(instructions.getByText("Native override", {exact: true})).toBeVisible();
  await shot(page, "graphite-instructions");

  await page.locator(".file-menu > summary").click();
  await page.locator(".file-menu [data-support-open]").click();
  const support = page.getByRole("dialog", {name: "Device support report", exact: true});
  await expect(support).toBeVisible();
  await support.locator("[data-support-preview]").click();
  await expect(support.locator("[data-support-preview-content]")).toBeVisible();
  await expect(support.locator("[data-support-save]")).toBeDisabled();
  await support.locator("[data-support-preview-content]").scrollIntoViewIfNeeded();
  await shot(page, "graphite-support");
  await support.getByRole("button", {name: "Close", exact: true}).click();

  for (const value of ["porcelain", "lcars", "user-modern-retro-green-crt"]) {
    await theme(page, value);
    await nav(page, "Group addresses");
    await page.locator("[data-ga-expand-all]").click();
    await page.getByRole("button", {name: "0/0/2", exact: true}).first().click();
    await shot(page, value + "-addresses");
  }
  await theme(page, "porcelain");
  await nav(page, "Topology");
  await shot(page, "porcelain-topology");
  await page.getByRole("button", {name: "1.1.1 Switch actuator ground floor", exact: true}).first().click();
  await tabs.getByRole("tab", {name: "Parameters", exact: true}).click();
  await shot(page, "porcelain-parameters");
  expect(errors).toEqual([]);
  expect(forbidden).toEqual([]);
});
