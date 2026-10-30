/** Settings workspace navigation, focus, sizing and acknowledged edits with fictional API data. */
import { expect, test, type Page } from "@playwright/test";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";
import { openNavigationSections } from "./disclosure-fixture";

type Language = "en" | "de";
const TREE: ProjectTree = {
  schema_version: 13, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
  server_incarnation: "settings-fixture", project_incarnation: 1, snapshot_revision: 1,
  group_address_style: "ThreeLevel", installations: [],
};

async function fixture(page: Page, language: Language, loaded: boolean, uiScale = 1) {
  let settings: Record<string, unknown> = {
    uiLanguage: language, theme: "graphite", uiThemePacks: {}, motionLevel: "off", uiScale,
    releaseChecksEnabled: false, achievementsEnabled: false,
    onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.18" },
  };
  const state = { unexpected: [] as string[], writes: [] as Record<string, unknown>[], failTheme: false };
  await page.route("**/*", async (route) => {
    const request = route.request(), url = new URL(request.url());
    const path = url.pathname, method = request.method();
    if (url.origin !== "http://127.0.0.1:4173") {
      state.unexpected.push(request.url()); return route.abort();
    }
    if (!path.startsWith("/api/")) return route.continue();
    const ok = (body: unknown) => route.fulfill({ json: body });
    const refuse = (status: number) => route.fulfill({ status, json: { error: "fictional refusal" } });
    const achievements = achievementsFixtureAnswer(method, path);
    if (achievements !== undefined) return ok(achievements);
    if (path === "/api/auth/status" && method === "GET") return ok({ required: false, authenticated: true });
    if (path === "/api/settings" && ["GET", "PUT"].includes(method)) {
      if (method === "PUT") {
        const payload = request.postDataJSON() as { settings: Record<string, unknown>; expectedSettings?: Record<string, unknown> };
        state.writes.push(payload.settings);
        if ("theme" in payload.settings) {
          expect(payload.expectedSettings).toEqual({ theme: settings.theme, uiThemePacks: settings.uiThemePacks });
          if (state.failTheme) { state.failTheme = false; return refuse(500); }
        }
        settings = { ...settings, ...payload.settings };
      }
      return ok({ schemaVersion: 1, conditionalPatchVersion: 1, status: "ok", settings });
    }
    if (path === "/api/project" && method === "GET") return loaded ? ok(TREE) : refuse(400);
    if (path === "/api/project/load-progress" && method === "GET") return ok(null);
    if (path === "/api/project/status-metrics" && method === "GET") return ok(statusMetricsFixtureBody(TREE));
    if (path === "/api/version" && method === "GET") return ok({ version: "0.1.0-alpha.18+settings-fixture" });
    if (path === "/api/legacy-password" && method === "GET") return ok({ available: true, remembered: false, problem: null });
    if (path === "/api/log" && method === "GET") return ok([]);
    if (path === "/api/bus/discover" && method === "POST") return ok({ interfaces: [] });
    if (["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items"].includes(path) && method === "GET") return ok([]);
    if (path === "/api/device-system2/status" && method === "GET") return ok(null);
    if (["/api/device-download/status", "/api/device-address/status"].includes(path) && method === "GET") return refuse(404);
    if (path === "/api/devices" && method === "GET") return ok({ schemaVersion: 1, serverIncarnation: "settings-fixture", snapshotRevision: 1, devices: [] });
    state.unexpected.push(`${method} ${path}`); return refuse(404);
  });
  await page.goto(loaded ? "/?view=editor" : "/");
  await expect(page.locator("html")).toHaveAttribute("lang", language);
  return { state, settings: () => settings };
}

async function palette(page: Page, label: string) {
  await page.keyboard.press("Control+Shift+P");
  const input = page.getByRole("dialog").getByRole("combobox");
  await input.fill(label);
  await input.press("Enter");
}

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) for (const loaded of [false, true]) {
  test(`${language} ${width} ${loaded ? "project" : "projectless"}: continuous settings page, jumps and return`, async ({ page }, testInfo) => {
    const m = language === "de" ? de : en;
    await page.setViewportSize({ width, height: 900 });
    await page.emulateMedia({ reducedMotion: "reduce" });
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    const { state } = await fixture(page, language, loaded);
    const properties = page.locator(".workbench-panel-controls").getByRole("button", { name: m["workbench.properties"], exact: true });
    // Both prior states in each locale, including the project's collapsed inspector.
    const collapsed = loaded;
    if (collapsed) await properties.click();
    const gear = page.locator(".workbench-toolbar").getByRole("button", { name: m["toolbar.settings"], exact: true });
    if (width === 400) await palette(page, m["toolbar.settings"]);
    else {
      await openNavigationSections(page);
      await page.locator(".diagnostic-navigation").getByRole("button", { name: m["toolbar.settings"], exact: true }).click();
    }
    const workspace = page.getByRole("region", { name: m["settings.title"], exact: true });
    const title = workspace.getByRole("heading", { name: m["settings.title"], level: 1, exact: true });
    await expect(page.locator(".workbench-center .settings-workspace")).toBeVisible();
    await expect(page.getByRole("dialog", { name: m["settings.title"], exact: true })).toHaveCount(0);
    await expect(title).toBeFocused();
    await page.keyboard.press("Shift+Tab");
    await expect(workspace.getByRole("button", { name: m["settings.back"], exact: true })).toBeFocused();
    await page.keyboard.press("Shift+Tab");
    expect(await workspace.evaluate((node) => node.contains(document.activeElement))).toBe(false);
    await expect(properties).toBeDisabled();
    await expect(properties).toHaveAttribute("aria-expanded", String(!collapsed));
    await expect(page.locator(".workbench-pane-right")).toHaveCount(0);
    if (width === 1440) {
      await expect(page.locator(".workbench-pane-left")).toBeVisible();
      await expect(page.locator('.workbench-navigation [aria-current="page"]')).toHaveText(m["toolbar.settings"]);
    }
    await page.keyboard.press("Escape");
    await expect(workspace).toBeVisible();
    await gear.click();
    await expect(page.locator(".settings-workspace")).toHaveCount(1);
    const jump = workspace.getByRole("navigation", { name: m["settings.jumpNavigation"], exact: true });
    await expect(jump.getByRole("link")).toHaveCount(8);
    if (loaded) await testInfo.attach("settings-top", { body: await page.screenshot(), contentType: "image/png" });
    const initialUrl = page.url();
    const links = jump.getByRole("link");
    for (let index = 0; index < 8; index++) {
      const link = links.nth(index), id = await link.getAttribute("href");
      await link.focus();
      await link.press("Enter");
      const heading = workspace.locator(id!);
      await expect(heading).toBeFocused();
      await expect(heading).toBeInViewport();
      expect(page.url()).toBe(initialUrl);
    }
    // Every section stays mounted; jumping never changes to a category subpage.
    await expect(workspace.locator(":scope > .settings-section")).toHaveCount(8);
    await expect(workspace.locator(".settings-section:visible")).toHaveCount(8);
    await expect(workspace.locator(".settings-section-debug input")).toBeInViewport();
    const bounds = await workspace.boundingBox();
    expect(bounds!.x).toBeGreaterThanOrEqual(0);
    expect(bounds!.x + bounds!.width).toBeLessThanOrEqual(width + 1);
    expect(await workspace.evaluate((node) => node.scrollWidth - node.clientWidth)).toBeLessThanOrEqual(1);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    if (loaded) await testInfo.attach("settings-bottom", { body: await page.screenshot(), contentType: "image/png" });
    const back = workspace.getByRole("button", { name: m["settings.back"], exact: true });
    await back.click();
    await expect(workspace).toHaveCount(0);
    await expect(properties).toBeEnabled();
    await expect(properties).toHaveAttribute("aria-expanded", String(!collapsed));
    await expect(page.locator(".workbench-pane-right")).toHaveCount(collapsed ? 0 : 1);
    await expect(page.locator(loaded ? ".dashboard" : ".welcome-workspace")).toBeVisible();
    await gear.click();
    await palette(page, m["workbench.catalog"]);
    await expect(workspace).toHaveCount(0);
    await expect(page.locator(".catalog-workspace")).toBeVisible();
    expect(state.writes).toEqual([]);
    expect(state.unexpected).toEqual([]);
    expect(errors).toEqual([]);
  });
}

for (const language of ["en", "de"] as const) test(`${language}: settings edits apply immediately, failures remain visible, language edits keep focus`, async ({ page }) => {
  const m = language === "de" ? de : en;
  const { state, settings } = await fixture(page, language, false);
  await palette(page, m["toolbar.settings"]);
  const workspace = page.locator(".settings-workspace");
  const theme = workspace.getByRole("combobox", { name: m["settings.theme"], exact: true });
  await theme.selectOption("porcelain");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "porcelain");
  await expect(workspace.getByRole("status").filter({ hasText: m["themePack.manager.saved"] })).toBeVisible();
  state.failTheme = true;
  await theme.selectOption("graphite");
  await expect(workspace.getByRole("status").filter({ hasText: m["themePack.manager.applyFailed"] })).toBeVisible();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "porcelain");
  expect(settings().theme).toBe("porcelain");
  const languageSelect = workspace.getByRole("combobox", { name: m["settings.uiLanguage"], exact: true });
  await languageSelect.focus();
  const nextLanguage = language === "en" ? "de" : "en", next = nextLanguage === "de" ? de : en;
  await languageSelect.selectOption(nextLanguage);
  await expect(page.locator("html")).toHaveAttribute("lang", nextLanguage);
  await expect(workspace.getByRole("combobox", { name: next["settings.uiLanguage"], exact: true })).toBeFocused();
  await expect(workspace.getByRole("heading", { level: 1 })).toHaveText(next["settings.title"]);
  await expect.poll(() => settings().uiLanguage).toBe(nextLanguage);
  expect(state.writes).toEqual([{ theme: "porcelain" }, { theme: "graphite" }, { uiLanguage: nextLanguage }]);
  expect(state.unexpected).toEqual([]);
});

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) for (const scale of [0.8, 1.5]) {
  test(`${language} ${width} zoom ${scale}: settings sections remain reachable without horizontal overflow`, async ({ page }, testInfo) => {
    const m = language === "de" ? de : en;
    await page.setViewportSize({ width, height: 900 });
    const { state } = await fixture(page, language, false, scale);
    await palette(page, m["toolbar.settings"]);
    const workspace = page.locator(".settings-workspace");
    await expect(workspace.getByRole("heading", { level: 1 })).toBeFocused();
    const jump = workspace.getByRole("navigation", { name: m["settings.jumpNavigation"], exact: true });
    await jump.getByRole("link").last().click();
    const target = workspace.locator("#settings-debug");
    await expect(target).toBeFocused();
    await expect(target).toBeInViewport();
    await expect(workspace.locator(".settings-section-debug input")).toBeInViewport();
    const geometry = await workspace.evaluate((node) => {
      const right = node.getBoundingClientRect().right;
      return { overflow: node.scrollWidth - node.clientWidth, offenders: [...node.querySelectorAll("*")]
        .filter((child) => child.getBoundingClientRect().right > right + 1)
        .map((child) => ({ tag: child.tagName, className: child.className, width: child.getBoundingClientRect().width,
          minWidth: getComputedStyle(child).minWidth, columns: getComputedStyle(child).gridTemplateColumns })) };
    });
    expect(geometry.overflow, JSON.stringify(geometry.offenders)).toBeLessThanOrEqual(1);
    if (width === 400 && scale === 1.5) {
      const languageSection = workspace.locator("#settings-language-data").locator("..");
      await languageSection.scrollIntoViewIfNeeded();
      await testInfo.attach("settings-zoom-language", { body: await languageSection.screenshot(), contentType: "image/png" });
    }
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    expect(state.writes).toEqual([]);
    expect(state.unexpected).toEqual([]);
  });
}
