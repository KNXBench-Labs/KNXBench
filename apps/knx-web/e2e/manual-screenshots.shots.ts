/** Regenerates the manual's screenshots from a real server and a fictional project. */

// The run uses a release knx-server, the production frontend and the
// fictional project from tools/manual_sample_project.py, written to
// docs/assets/screenshots. Run it only in a loopback-only network namespace
// (see playwright.manual.config.ts); the application's start-up gateway
// search then finds nothing, as on a machine without a KNX installation.

import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const target = resolve(
  process.env.MANUAL_SCREENSHOT_DIR ?? fileURLToPath(new URL("../../../docs/assets/screenshots", import.meta.url)),
);
mkdirSync(target, { recursive: true });

async function shot(page: Page, name: string): Promise<void> {
  // Let transitions settle; the theme switch animates for 200 ms.
  await page.waitForTimeout(400);
  await page.screenshot({ path: `${target}/porcelain-${name}.png` });
}

async function dismissToasts(page: Page): Promise<void> {
  for (const dismiss of await page.getByRole("button", { name: "Dismiss" }).all()) {
    await dismiss.click();
  }
}

async function nav(page: Page, name: string): Promise<void> {
  await page.getByRole("navigation", { name: "Navigation" }).getByRole("button", { name, exact: true }).click();
  // Views share one scrolling main area; start every view at its top.
  await page.evaluate(() => {
    for (const element of document.querySelectorAll("*")) if (element.scrollTop) element.scrollTop = 0;
  });
}

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem("knx-desktop:theme", "porcelain");
    localStorage.setItem("knx-desktop:ui-language", "en");
  });
});

test("manual screenshots of the finished application", async ({ page }) => {
  const failures: string[] = [];
  page.on("pageerror", (error) => failures.push(String(error)));

  await page.goto("/");
  await expect(page.getByRole("button", { name: /Import ETS project/ })).toBeVisible();
  await shot(page, "welcome");

  await page.getByRole("button", { name: /Import ETS project/ }).first().click();
  const sample = page.getByRole("button", { name: "sample-house.knxproj" });
  await expect(sample).toBeVisible();
  await shot(page, "open-project-dialog");

  // Hold the import request back briefly so the progress banner is on
  // screen long enough to photograph; the request then reaches the real
  // server unchanged.
  await page.route("**/api/project/import", async (route) => {
    await new Promise((done) => setTimeout(done, 3000));
    await route.continue();
  }, { times: 1 });
  await sample.click();
  await expect(page.getByRole("status").filter({ hasText: /Loading|Starting|Reading/i }).first()).toBeVisible();
  await shot(page, "loading-progress");
  await expect(page.getByText("Project status")).toBeVisible({ timeout: 15_000 });
  await dismissToasts(page);
  await shot(page, "dashboard");

  await page.locator("summary", { hasText: "File" }).first().click();
  await expect(page.getByRole("button", { name: "New project…" })).toBeVisible();
  await shot(page, "file-menu");
  await page.getByRole("button", { name: "New project…" }).click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await shot(page, "new-project");
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);

  await nav(page, "Buildings");
  await expect(page.getByText("Living room").first()).toBeVisible();
  await shot(page, "buildings");

  await nav(page, "Topology");
  await expect(page.getByText("1.1 · Ground floor")).toBeVisible();
  await shot(page, "topology");

  await page.getByRole("button", { name: "1.1.1 Switch actuator ground floor" }).first().click();
  await expect(page.getByRole("tablist", { name: "Switch actuator ground floor" })).toBeVisible();
  await shot(page, "device-inspector");

  const tabs = page.getByRole("tablist", { name: "Switch actuator ground floor" });
  async function openTab(tab: string): Promise<void> {
    await tabs.getByRole("tab", { name: tab, exact: true }).click();
    await expect(tabs.getByRole("tab", { name: tab, exact: true })).toHaveAttribute("aria-selected", "true");
  }
  await openTab("Communication objects");
  await page.getByText("Channel A", { exact: true }).click();
  await page.getByText("Channel A: Switch", { exact: true }).click();
  await expect(page.getByText("0/0/1").first()).toBeVisible();
  await tabs.evaluate((el) => el.scrollIntoView({ block: "start" }));
  await shot(page, "device-tab-communication-objects");
  await openTab("Parameters");
  await expect(page.getByText("Delay after bus voltage recovery (s)").first()).toBeVisible();
  await tabs.evaluate((el) => el.scrollIntoView({ block: "start" }));
  await shot(page, "device-tab-parameters");
  await openTab("Product data");
  await page.getByText("More product data").first().click();
  await tabs.evaluate((el) => el.scrollIntoView({ block: "start" }));
  await shot(page, "device-tab-product-data");

  await nav(page, "Group addresses");
  await expect(page.getByText("Living room ceiling light").first()).toBeVisible();
  await shot(page, "group-addresses");

  // Opened from a line in the Topology view, so the batch has a target line.
  await nav(page, "Topology");
  await page.getByRole("button", { name: "Device · Ground floor" }).click();
  await page.getByText("Switch actuator 4-fold, DIN rail (1)").click();
  await page.getByLabel("Quantity").fill("3");
  await page.getByLabel(/Assign free addresses on the line/).check();
  await expect(page.getByText("Devices to create")).toBeVisible();
  await shot(page, "product-catalog");

  await page.keyboard.press("Control+k");
  await page.keyboard.type("light");
  await expect(page.getByRole("dialog", { name: "Search" })).toBeVisible();
  await shot(page, "search");
  await page.keyboard.press("Escape");

  await page.keyboard.press("Control+Shift+P");
  await expect(page.getByRole("dialog")).toBeVisible();
  await shot(page, "command-palette");
  await page.keyboard.press("Escape");

  await page.keyboard.press("F1");
  await expect(page.getByRole("heading", { name: "Help", level: 2 })).toBeVisible();
  await page.getByRole("button", { name: "Getting started" }).click();
  await expect(page.getByRole("heading", { name: "Getting started", level: 3 })).toBeVisible();
  await shot(page, "help-panel");
  await page.keyboard.press("Escape");

  await page.getByRole("button", { name: "Bus monitor", exact: true }).click();
  await expect(page.getByRole("heading", { name: /Bus monitor/ }).first()).toBeVisible();
  await shot(page, "bus-monitor");
  await page.keyboard.press("Escape");

  await page.getByRole("button", { name: "Log", exact: true }).click();
  await shot(page, "log");
  await page.keyboard.press("Escape");

  await page.locator(".workbench-navigation, aside").getByRole("button", { name: "Settings", exact: true }).first().click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await shot(page, "settings");
  await page.keyboard.press("Escape");

  expect(failures).toEqual([]);
});

test("manual screenshot of the sign-in page", async ({ page }) => {
  await page.goto("http://127.0.0.1:4811/");
  await expect(page.getByLabel(/Password/i)).toBeVisible();
  await shot(page, "login");
});
