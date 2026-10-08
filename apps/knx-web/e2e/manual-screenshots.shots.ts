/** Regenerates the manual's screenshots from a real server and a fictional project. */

// The run uses a freshly built knx-server, the production frontend and the
// fictional project from tools/manual_sample_project.py, written to
// docs/assets/screenshots. Run it only in a loopback-only network namespace
// (see playwright.manual.config.ts); the application's start-up gateway
// search then finds nothing, as on a machine without a KNX installation.

import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { recordManualGif } from "./manual-recording";

const target = resolve(
  process.env.MANUAL_SCREENSHOT_DIR ?? fileURLToPath(new URL("../../../docs/assets/screenshots", import.meta.url)),
);
mkdirSync(target, { recursive: true });

async function shot(page: Page, name: string): Promise<void> {
  // Let transitions settle; the theme switch animates for 200 ms.
  await page.waitForTimeout(400);
  await page.screenshot({ path: `${target}/porcelain-${name}.png` });
  console.log(`Captured porcelain-${name}.png`);
}

async function dismissToasts(page: Page): Promise<void> {
  const buttons = page.getByRole("button", { name: "Dismiss" });
  // Reacquire after every dismissal: a snapshot of nth locators skips rows
  // when the preceding toast leaves (achievements now animate their exit).
  while (await buttons.count()) {
    const before = await buttons.count();
    await buttons.first().click();
    await expect.poll(() => buttons.count()).toBeLessThan(before);
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
  // ADR-0084: a fresh data directory is a first start, so the guide opens
  // over the welcome page. Photograph it, then close it as a user would;
  // every later screenshot is taken with it seen.
  const guide = page.getByRole("dialog", { name: "Welcome to KNXBench" });
  await expect(guide).toBeVisible();
  await shot(page, "onboarding-guide");
  await page.keyboard.press("Escape");
  await expect(guide).toBeHidden();
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
  await page.getByRole("button", { name: "Analyze support gaps…", exact: true }).click();
  const analysis = page.getByRole("dialog", { name: "Analyze & contribute evidence", exact: true });
  await expect(analysis).toBeVisible();
  await shot(page, "support-gap-analysis");
  await analysis.getByRole("button", { name: "Close", exact: true }).click();
  await expect(analysis).toBeHidden();
  await page.locator("summary", { hasText: "File" }).first().click();
  await page.getByRole("button", { name: "New project…" }).click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await shot(page, "new-project");
  const projectWizard = page.getByRole("dialog", { name: "New project", exact: true });
  for (const step of ["topology", "building", "groups", "review"]) {
    await projectWizard.getByRole("button", { name: "Next", exact: true }).click();
    await shot(page, `project-wizard-${step}`);
  }
  await projectWizard.getByRole("button", { name: "Cancel", exact: true }).click();
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

  await nav(page, "Product catalog");
  await expect(page.getByText("Switch actuator 4-fold, DIN rail (1)")).toBeVisible();
  await shot(page, "product-catalog");

  // The explorer's Add device action opens the wizard; the topology + opens the catalog.
  await nav(page, "Topology");
  await recordManualGif(page, "add-device", async () => {
    const line = page.locator("li").filter({
      has: page.locator("button.tree-label", { hasText: /^Line 1: Ground floor$/ }),
    }).last();
    await line.getByRole("button", { name: "+ Add device", exact: true }).click();
    const deviceWizard = page.getByRole("dialog", { name: "Add device", exact: true });
    await expect(deviceWizard).toBeVisible();
    await shot(page, "device-wizard-product");
    await page.waitForTimeout(1000);
    await deviceWizard.getByRole("button", { name: /Switch actuator 4-fold, DIN rail/ }).click();
    await deviceWizard.getByRole("button", { name: "Next", exact: true }).click();
    await shot(page, "device-wizard-placement");
    await page.waitForTimeout(1000);
    await deviceWizard.getByRole("button", { name: "Next", exact: true }).click();
    await deviceWizard.getByLabel("Device name", { exact: true }).fill("Practice actuator");
    await deviceWizard.getByLabel(/Assign free addresses on the line/).check();
    await page.waitForTimeout(1000);
    await deviceWizard.getByRole("button", { name: "Next", exact: true }).click();
    await expect(deviceWizard.getByText("1 device will be created")).toBeVisible();
    await shot(page, "device-wizard-review");
    await page.waitForTimeout(1500);
    await deviceWizard.getByRole("button", { name: "Create device", exact: true }).click();
    await expect(deviceWizard.getByText("1 device created.", { exact: true })).toBeVisible();
    await shot(page, "device-wizard-result");
    await page.waitForTimeout(1500);
    await deviceWizard.getByRole("button", { name: "Done", exact: true }).click();
    await expect(deviceWizard).toBeHidden();
    const tree = await (await page.request.get("/api/project")).json();
    expect(tree.installations[0].topology.flatMap((area: { lines: { devices: { name: string }[] }[] }) =>
      area.lines.flatMap((line) => line.devices)).filter((device: { name: string }) => device.name === "Practice actuator")).toHaveLength(1);
  });

  await recordManualGif(page, "search", async () => {
    await page.keyboard.press("Control+k");
    await expect(page.getByRole("dialog", { name: "Search" })).toBeVisible();
    await page.waitForTimeout(1000);
    await page.keyboard.type("light", { delay: 180 });
    await shot(page, "search");
    await page.waitForTimeout(2000);
    await page.keyboard.press("Escape");
  });

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

  // Exercise the first-project tutorial through real UI save/open operations.
  await page.getByRole("button", { name: "Save", exact: true }).first().click();
  const save = page.getByRole("dialog", { name: "Save as", exact: true });
  await expect(save).toBeVisible();
  await save.getByPlaceholder("filename").fill("sample-after-docs.knxdb");
  await save.getByRole("button", { name: "Save", exact: true }).click();
  await expect(save).toBeHidden();
  await expect.poll(async () => (await (await page.request.get("/api/project")).json()).is_modified).toBe(false);
  await recordManualGif(page, "new-project", async () => {
    await page.locator("summary", { hasText: "File" }).first().click();
    await page.getByRole("button", { name: "New project…", exact: true }).click();
    const wizard = page.getByRole("dialog", { name: "New project", exact: true });
    await wizard.getByLabel("Project name", { exact: true }).fill("Practice house");
    await page.waitForTimeout(1200);
    for (let step = 0; step < 4; step++) {
      await wizard.getByRole("button", { name: "Next", exact: true }).click();
      await page.waitForTimeout(1000);
    }
    await wizard.getByRole("button", { name: "Create project", exact: true }).click();
    const created = page.getByRole("dialog", { name: "Project created", exact: true });
    await expect(created.getByText(/It is not saved yet/)).toBeVisible();
    await page.waitForTimeout(1600);
    await created.getByRole("button", { name: "Done", exact: true }).click();
  });
  await page.getByRole("button", { name: "Save", exact: true }).first().click();
  await expect(save).toBeVisible();
  await save.getByPlaceholder("filename").fill("practice-house.knxdb");
  await save.getByRole("button", { name: "Save", exact: true }).click();
  await expect(save).toBeHidden();
  await expect.poll(async () => (await (await page.request.get("/api/project")).json()).is_modified).toBe(false);
  await page.locator("summary", { hasText: "File" }).first().click();
  await page.getByRole("button", { name: "Open (.knxdb)…", exact: true }).click();
  await page.getByRole("button", { name: "practice-house.knxdb", exact: true }).click();
  await expect(page.locator("button.tree-label", { hasText: /^Line 1: Line 1\.1$/ })).toBeVisible();
  const reopened = await (await page.request.get("/api/project")).json();
  expect(reopened.is_modified).toBe(false);
  expect(reopened.installations[0].topology[0].lines).toHaveLength(1);
  await dismissToasts(page);
  await shot(page, "practice-project-reopened");
  console.log("First-project tutorial: created, saved and reopened through the real UI");
  expect(failures).toEqual([]);
});

test("manual screenshot of the sign-in page", async ({ page }) => {
  await page.goto("http://127.0.0.1:4811/");
  await expect(page.getByLabel(/Password/i)).toBeVisible();
  await shot(page, "login");
});
