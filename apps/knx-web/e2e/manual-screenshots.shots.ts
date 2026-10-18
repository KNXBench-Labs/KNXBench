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

async function shot(page: Page, name: string, theme = "porcelain"): Promise<void> {
  const viewport = page.viewportSize();
  // Keep the complete history notices and expanded DPT controls in the frame.
  const tall = ["project-history", "dpt-selection", "message-details"].includes(name);
  if (tall && viewport) await page.setViewportSize({ ...viewport, height: 1080 });
  try {
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    // Let finite transitions and newly arriving notifications settle first.
    await page.waitForTimeout(400);
    await dismissToasts(page);
    await page.screenshot({ path: `${target}/${theme}-${name}.png`, animations: "disabled" });
    console.log(`Captured ${theme}-${name}.png`);
  } finally {
    if (tall && viewport) await page.setViewportSize(viewport);
  }
}

async function dismissToasts(page: Page): Promise<void> {
  const buttons = page.getByRole("button", { name: "Dismiss" });
  // Wait for the clicked node, not a queue count: another real notification
  // may arrive while this one leaves. Reacquire only after its own removal.
  while (await buttons.count()) {
    const button = await buttons.first().elementHandle();
    if (!button) continue;
    await button.click();
    await expect.poll(() => button.evaluate((node) => node.isConnected)).toBe(false);
    await button.dispose();
  }
}

async function nav(page: Page, name: string): Promise<void> {
  await page.getByRole("navigation", { name: "Navigation" }).getByRole("button", { name, exact: true }).click();
  // Views share one scrolling main area; start every view at its top.
  await page.evaluate(() => {
    for (const element of document.querySelectorAll("*")) if (element.scrollTop) element.scrollTop = 0;
  });
}

// Since Alpha 14 navigation blocks, explorer branches, address ranges and
// parameter areas start closed. Open them the way a reader would.
async function unfoldNavigation(page: Page, title: string): Promise<void> {
  const toggle = page.locator(".navigation-section-toggle", { hasText: title }).first();
  if ((await toggle.getAttribute("aria-expanded")) === "false") await toggle.click();
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
}

async function openWorkbench(page: Page): Promise<void> {
  await unfoldNavigation(page, "Workspaces");
  await unfoldNavigation(page, "Project explorer");
  await page.locator(".project-explorer").getByRole("button", { name: "Expand all", exact: true }).click();
}

async function expandAddresses(page: Page): Promise<void> {
  await page.locator("[data-ga-expand-all]").click();
}

async function openParameterArea(page: Page, field: string): Promise<void> {
  const areas = page.locator(".parameter-workspace-nav button:visible");
  await expect(areas.first()).toBeVisible();
  for (const area of await areas.all()) {
    await area.click();
    if (await page.getByText(field).first().isVisible()) return;
  }
  throw new Error(`no parameter area shows ${field}`);
}

async function setTheme(page: Page, theme: string): Promise<void> {
  await page.getByRole("button", { name: "Settings", exact: true }).first().click();
  const settings = page.getByRole("dialog");
  await settings.getByLabel("Theme", { exact: true }).selectOption(theme);
  await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
  await expect(settings.getByRole("status").filter({ hasText: "Theme saved." })).toBeVisible();
  await settings.focus();
  await expect(settings).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(settings).toBeHidden();
  // The next workflow recording must not begin halfway through the old palette.
  await page.waitForTimeout(400);
}

async function history(page: Page): Promise<void> {
  await page.locator("summary", { hasText: "File" }).first().click();
  await page.getByRole("button", { name: "Project history", exact: true }).click();
  await expect(page.getByRole("dialog", { name: "Project history", exact: true })).toBeVisible();
}

async function captureReleaseDetails(page: Page): Promise<void> {
  await history(page);
  const panel = page.getByRole("dialog", { name: "Project history", exact: true });
  await panel.getByLabel("Version name", { exact: true }).fill("Before the kitchen redesign");
  await panel.getByRole("button", { name: "Create version", exact: true }).click();
  await expect(panel.getByText("Before the kitchen redesign", { exact: true })).toBeVisible();
  await shot(page, "project-history");
  await panel.getByRole("button", { name: "Close", exact: true }).click();
  await dismissToasts(page);

  await setTheme(page, "graphite");
  for (const [view, name] of [["Overview", "dashboard"], ["Devices", "devices"], ["Topology", "topology"]]) {
    await nav(page, view);
    await shot(page, name, "graphite");
  }
  await page.locator("button.tree-label").filter({ hasText: /^Sample house$/ }).click();
  await expect(page.locator(".workbench-pane-right").getByRole("textbox", { name: "Name", exact: true })).toBeVisible();
  await shot(page, "installation-inspector", "graphite");

  await nav(page, "Group addresses");
  await expandAddresses(page);
  await page.getByRole("button", { name: "0/0/2", exact: true }).first().click();
  await expect(page.locator(".workbench-pane-right").getByRole("textbox", { name: "Name", exact: true })).toHaveValue("Kitchen light");
  await shot(page, "group-addresses", "graphite");

  await page.getByRole("button", { name: "1.1.1 Switch actuator ground floor", exact: true }).first().click();
  const tabs = page.getByRole("tablist", { name: "Switch actuator ground floor" });
  await tabs.getByRole("tab", { name: "Communication objects", exact: true }).click();
  await page.locator(".com-object-channel-summary").filter({ hasText: "Channel A" }).click();
  await page.locator('summary[aria-label="Edit object 0"]').click();
  const dpt = page.locator(".dpt-field select");
  await expect(dpt).toBeVisible();
  await expect(dpt.locator("option").filter({ hasText: "Switch" }).first()).toHaveCount(1);
  await dpt.scrollIntoViewIfNeeded();
  await shot(page, "dpt-selection", "graphite");
  await tabs.getByRole("tab", { name: "Parameters", exact: true }).click();
  await page.locator(".parameter-workspace-nav button", { hasText: "Channel A" }).first().click();
  await expect(page.locator(".parameter-workspace-nav button[aria-current=\"page\"]")).toContainText("Channel A");
  await shot(page, "device-parameters", "graphite");

  await history(page);
  await expect(panel.getByText("Before the kitchen redesign", { exact: true })).toBeVisible();
  await shot(page, "project-history", "graphite");
  await panel.getByRole("button", { name: "Close", exact: true }).click();
  await page.getByRole("button", { name: "Log", exact: true }).click();
  await page.getByLabel("Search session log", { exact: true }).fill("ui:toast:achievement");
  const message = page.locator('[data-log-key^="ui:"] .log-entry-open').first();
  await expect(message).toBeVisible();
  await message.click();
  const details = page.getByRole("dialog", { name: "Message details", exact: true });
  await expect(details).toBeVisible();
  await expect(details.getByRole("button", { name: "Copy all details", exact: true })).toBeVisible();
  expect(await details.innerText()).not.toMatch(/\/home\/|\/mnt\/|password=/i);
  await shot(page, "message-details", "graphite");
  await page.keyboard.press("Escape");

  for (const theme of ["user-modern-retro-green-crt", "lcars"]) {
    await setTheme(page, theme);
    await nav(page, "Group addresses");
    await expandAddresses(page);
    await page.getByRole("button", { name: "0/0/2", exact: true }).first().click();
    await shot(page, "group-addresses", theme);
  }
  await setTheme(page, "porcelain");
}

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    if (!localStorage.getItem("knx-desktop:theme")) localStorage.setItem("knx-desktop:theme", "porcelain");
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

  await openWorkbench(page);
  await nav(page, "Buildings");
  await expect(page.getByText("Living room").first()).toBeVisible();
  await shot(page, "buildings");

  await nav(page, "Topology");
  await expect(page.getByRole("button", { name: "1.1 · Ground floor", exact: true })).toBeVisible();
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
  await page.locator(".com-object-channel-summary").filter({ hasText: "Channel A" }).click();
  await page.locator('summary[aria-label="Edit object 0"]').click();
  await expect(page.getByText("0/0/1").first()).toBeVisible();
  await tabs.evaluate((el) => el.scrollIntoView({ block: "start" }));
  await shot(page, "device-tab-communication-objects");
  await openTab("Parameters");
  await openParameterArea(page, "Delay after bus voltage recovery (s)");
  await tabs.evaluate((el) => el.scrollIntoView({ block: "start" }));
  await shot(page, "device-tab-parameters");
  await openTab("Product data");
  await page.getByText("More product data").first().click();
  await tabs.evaluate((el) => el.scrollIntoView({ block: "start" }));
  await shot(page, "device-tab-product-data");

  await nav(page, "Group addresses");
  await expandAddresses(page);
  await expect(page.getByText("Living room ceiling light").first()).toBeVisible();
  await shot(page, "group-addresses");

  await nav(page, "Devices");
  await expect(page.getByRole("heading", { name: "Devices", exact: true })).toBeVisible();
  await shot(page, "devices");

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

  await unfoldNavigation(page, "Diagnostics");
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
  await captureReleaseDetails(page);
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
  await page.evaluate(() => localStorage.setItem("knx-desktop:theme", "graphite"));
  await page.reload();
  await expect(page.getByLabel(/Password/i)).toBeVisible();
  // The release being photographed; a stale frontend would show its own version.
  const version = process.env.MANUAL_EXPECTED_VERSION ?? "0.1.0-alpha.14";
  await expect(page.getByText(version, { exact: false })).toBeVisible();
  await shot(page, "login", "graphite");
});
