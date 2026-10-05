/** Browser proof for actual Appearance, with every settings request intercepted. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { expect, test, type Page } from "@playwright/test";
import { themePackFixture } from "../src/themePackFixtures";
import { canonicalJson } from "../src/canonicalJson";
import { parseThemePackText } from "../src/themePack";
import { readFile } from "node:fs/promises";
import { Buffer } from "node:buffer";

async function fixture(page: Page, installed = false) {
  const pack = themePackFixture();
  let settings: Record<string, unknown> = {
    theme: "graphite", accent: "mint", density: "compact", uiLanguage: "en",
    motionStyle: "apple", motionLevel: "off", uiThemePacks: installed ? { [pack.id]: pack } : {},
    foreign: { keep: ["synthetic", 7] },
  };
  const writes: { settings: Record<string, unknown>; expectedSettings: Record<string, unknown> }[] = [];
  const unexpected: string[] = [], errors: string[] = [];
  let conflicts = 0;
  let failNextWrite = false;
  const requests: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => { if (message.type() === "error" || message.type() === "warning") errors.push(message.text()); });
  await page.route("**/*", async (route) => {
    const request = route.request(), url = new URL(request.url());
    if (url.origin !== "http://127.0.0.1:4173") {
      unexpected.push(url.origin); await route.abort(); return;
    }
    if (url.pathname === "/api/settings") {
      requests.push(request.method());
      if (request.method() === "PUT") {
        const patch = request.postDataJSON() as (typeof writes)[number];
        if (!patch.expectedSettings) { unexpected.push("unconditional write"); await route.abort(); return; }
        writes.push(patch);
        if (failNextWrite) {
          failNextWrite = false;
          await route.fulfill({ status: 500, json: { message: "synthetic write failure" } }); return;
        }
        if (Object.entries(patch.expectedSettings).some(([key, value]) => canonicalJson(settings[key] ?? null) !== canonicalJson(value))) {
          conflicts += 1;
          await route.fulfill({ status: 409, json: { message: "synthetic conflict" } }); return;
        }
        settings = { ...settings, ...patch.settings };
      } else if (request.method() !== "GET") {
        unexpected.push(request.method()); await route.abort(); return;
      }
      await route.fulfill({ json: { schemaVersion: 1, conditionalPatchVersion: 1, status: "ok", settings } });
    } else if (url.pathname.startsWith("/api/")) {
      unexpected.push(url.pathname); await route.abort();
    } else await route.continue();
  });
  await page.goto("/e2e/theme-manager-fixture.html");
  await page.getByRole("button", { name: "Open settings", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Theme files", exact: true })).toBeVisible();
  await expect(page.locator("#theme-pack-import")).toBeEnabled();
  return { pack, writes, requests, settings: () => settings,
    failWrite: () => { failNextWrite = true; },
    peer: (patch: Record<string, unknown>) => { settings = { ...settings, ...patch }; },
    check: (expectedConflicts = 0, expectedFailures = 0) => {
      expect(unexpected).toEqual([]);
      expect(conflicts).toBe(expectedConflicts);
      // Chromium reports its deliberately intercepted HTTP error on console.
      // Account for that exact event/count; never hide other console errors.
      expect(errors).toEqual([
        ...Array.from({ length: expectedConflicts }, () =>
          "Failed to load resource: the server responded with a status of 409 (Conflict)"),
        ...Array.from({ length: expectedFailures }, () =>
          "Failed to load resource: the server responded with a status of 500 (Internal Server Error)"),
      ]);
    } };
}

async function importPack(page: Page, pack: ReturnType<typeof themePackFixture>, name = "synthetic.knx-theme.json") {
  // A user clicks the control to open the file chooser, so it holds focus;
  // `setInputFiles` alone would not focus it.
  await page.locator("#theme-pack-import").focus();
  await page.locator("#theme-pack-import").setInputFiles({ name, mimeType: "application/json", buffer: Buffer.from(JSON.stringify(pack)) });
}
const themeSelect = (page: Page) => page.getByRole("combobox", { name: "Theme", exact: true });
const CRT_ID = "user-modern-retro-green-crt";

test("the Theme dropdown is the only theme list: shipped CRT included, no preview cards, storage location shown", async ({ page }) => {
  const { writes, check } = await fixture(page);
  await expect(themeSelect(page).locator("option")).toHaveText(["System", "Porcelain", "Graphite", "Cupertino", "Modern Retro Green CRT"]);
  await expect(page.getByRole("button", { name: /^Preview/ })).toHaveCount(0);
  await expect(page.locator(".theme-manager-list")).toHaveCount(0);
  await expect(page.locator("[data-theme-storage]")).toContainText("settings.json");
  await expect(page.locator("[data-theme-storage]")).toContainText("~/.local/share/com.knxbench.knxbench-labs");
  await expect(page.locator("[data-theme-storage]")).toContainText("KNX_DATA_DIR");
  expect(writes).toEqual([]);
  check();
});

test("choosing the shipped CRT theme stores only its id, paints its tokens and survives a cold reload", async ({ page }) => {
  const { writes, settings, check } = await fixture(page);
  await themeSelect(page).selectOption(CRT_ID);
  await expect(page.getByTestId("saved-selection")).toHaveText(CRT_ID);
  await expect(page.locator("html")).toHaveAttribute("data-theme", CRT_ID);
  await expect(page.locator("html")).toHaveCSS("--knx-bg", "#050505");
  await expect(page.locator("body")).toHaveCSS("background-color", "rgb(5, 5, 5)");
  expect(writes).toEqual([{ settings: { theme: CRT_ID }, expectedSettings: { theme: "graphite", uiThemePacks: {} } }]);
  expect(settings().uiThemePacks).toEqual({});
  await expect(page.getByRole("button", { name: "Remove Modern Retro Green CRT", exact: true })).toHaveCount(0);
  await page.reload();
  await expect(page.getByTestId("saved-selection")).toHaveText(CRT_ID);
  await expect(page.locator("html")).toHaveCSS("--knx-bg", "#050505");
  expect(writes).toHaveLength(1);
  check();
});

test("actual Appearance rejects hostile file values without DOM effects or requests to the asset", async ({ page }) => {
  const { pack, writes, settings, check } = await fixture(page);
  const original = structuredClone(settings());
  pack.tokens["--knx-bg"] = 'url("https://invalid.example/synthetic-asset")';
  await importPack(page, pack, "hostile.knx-theme.json");
  await expect(page.getByText("The theme file was rejected. The saved theme and installed packs have not changed.", { exact: true })).toBeVisible();
  await expect(page.getByText("The token value is outside the permitted syntax or bounds.", { exact: true })).toBeVisible();
  await expect(page.locator("[data-theme-diagnostic=invalidValue]")).toContainText("tokens.--knx-bg");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "graphite");
  expect(await page.locator("html").evaluate((root) => (root as HTMLElement).style.getPropertyValue("--knx-bg"))).toBe("");
  expect(writes).toEqual([]);
  expect(settings()).toEqual(original);
  check();
});

test("importing installs and selects in one write; switching back to System keeps the pack and follows the OS", async ({ page }) => {
  const { pack, writes, settings, check } = await fixture(page);
  await importPack(page, pack);
  await expect(page.getByTestId("saved-selection")).toHaveText(pack.id);
  await expect(page.getByText("Theme saved.", { exact: true })).toBeVisible();
  expect(writes).toEqual([{ settings: { theme: pack.id, uiThemePacks: { [pack.id]: pack } }, expectedSettings: { theme: "graphite", uiThemePacks: {} } }]);
  const beforeReset = structuredClone(settings());
  await themeSelect(page).selectOption("system");
  await expect(page.getByTestId("saved-selection")).toHaveText("system");
  expect(settings()).toEqual({ ...beforeReset, theme: "system" });
  expect(writes[1]).toEqual({ settings: { theme: "system" }, expectedSettings: { theme: pack.id, uiThemePacks: { [pack.id]: pack } } });
  await page.emulateMedia({ colorScheme: "dark" });
  await expect(page.locator("html")).toHaveAttribute("data-theme", "graphite");
  await page.emulateMedia({ colorScheme: "light" });
  await expect(page.locator("html")).toHaveAttribute("data-theme", "porcelain");
  expect(writes).toHaveLength(2);
  check();
});

test("an uncertain HTTP500 import keeps the last confirmed palette and never replays the write", async ({ page }) => {
  const { pack, writes, requests, settings, failWrite, check } = await fixture(page);
  const original = structuredClone(settings());
  const baselineRequests = requests.length;
  failWrite();
  await importPack(page, pack);
  await expect(page.getByText("The theme could not be saved. Nothing changed; review the saved settings before trying again.", { exact: true })).toBeVisible();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "graphite");
  expect(await page.locator("html").evaluate((root) => (root as HTMLElement).style.getPropertyValue("--knx-bg"))).toBe("");
  await expect(page.getByText("Theme saved.", { exact: true })).toHaveCount(0);
  expect(settings()).toEqual(original);
  expect(writes).toHaveLength(1);
  expect(requests.slice(baselineRequests)).toEqual(["PUT", "GET"]);
  check(0, 1);
});

test("replacement Escape cancels and returns focus to the persistent import control", async ({ page }) => {
  const { pack, writes, check } = await fixture(page, true);
  await importPack(page, pack);
  const confirmation = page.getByRole("dialog", { name: "Replace theme pack", exact: true });
  await expect(confirmation).toBeVisible();
  await expect(confirmation.getByRole("button", { name: "Cancel replacement", exact: true })).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(confirmation).toHaveCount(0);
  await expect(page.getByRole("dialog", { name: "Settings", exact: true })).toBeVisible();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "graphite");
  await expect(page.locator("#theme-pack-import")).toBeFocused();
  expect(writes).toEqual([]);
  check();
});

test("replacement confirmation traps keyboard focus and Enter on Cancel never writes", async ({ page }) => {
  const { pack, writes, check } = await fixture(page, true);
  await importPack(page, pack);
  const dialog = page.getByRole("dialog", { name: "Replace theme pack", exact: true });
  const cancel = dialog.getByRole("button", { name: "Cancel replacement", exact: true });
  const confirm = dialog.getByRole("button", { name: "Replace and apply", exact: true });
  await expect(cancel).toBeFocused();
  await page.keyboard.press("Shift+Tab");
  await expect(confirm).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(cancel).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(dialog).toHaveCount(0);
  await expect(page.locator("#theme-pack-import")).toBeFocused();
  expect(writes).toEqual([]);
  check();
});

test("an imported theme persists across cold reload and the actual download reimports exactly", async ({ page }) => {
  const { pack, writes, settings, check } = await fixture(page);
  await importPack(page, pack);
  await expect(page.getByTestId("saved-selection")).toHaveText(pack.id);
  expect(settings().foreign).toEqual({ keep: ["synthetic", 7] });
  await page.reload();
  await expect(page.getByTestId("saved-selection")).toHaveText(pack.id);
  await page.getByRole("button", { name: "Open settings", exact: true }).click();
  await expect(themeSelect(page)).toHaveValue(pack.id);
  const downloadEvent = page.waitForEvent("download");
  await page.getByRole("button", { name: `Export ${pack.name}`, exact: true }).click();
  const download = await downloadEvent;
  expect(download.suggestedFilename()).toBe(`${pack.id}.knx-theme.json`);
  const path = await download.path();
  expect(path).not.toBeNull();
  const text = await readFile(path!, "utf8");
  expect(text).toBe(canonicalJson(pack, 2) + "\n");
  expect(parseThemePackText(text)).toEqual({ ok: true, pack });
  expect(writes).toHaveLength(1);
  check();
});

test("confirmed removal of the selected pack falls back to System and retains unrelated settings", async ({ page }) => {
  const { pack, writes, settings, check } = await fixture(page, true);
  await themeSelect(page).selectOption(pack.id);
  await expect(page.getByTestId("saved-selection")).toHaveText(pack.id);
  await page.getByRole("button", { name: `Remove ${pack.name}`, exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Remove theme pack", exact: true });
  await expect(dialog.getByRole("button", { name: "Cancel removal", exact: true })).toBeFocused();
  expect(writes).toHaveLength(1);
  await dialog.getByRole("button", { name: "Remove pack", exact: true }).click();
  await expect(page.getByTestId("saved-selection")).toHaveText("system");
  expect(writes[1]).toEqual({ settings: { theme: "system", uiThemePacks: {} }, expectedSettings: { theme: pack.id, uiThemePacks: { [pack.id]: pack } } });
  expect(settings().accent).toBe("mint");
  expect(settings().density).toBe("compact");
  expect(settings().foreign).toEqual({ keep: ["synthetic", 7] });
  await expect(themeSelect(page).locator(`option[value="${pack.id}"]`)).toHaveCount(0);
  await expect(page.locator("html")).toHaveAttribute("data-theme", "porcelain");
  check();
});

test("a conflicting import rolls back, then the existing peer refresh takes over without a second write", async ({ page }) => {
  const { pack, writes, peer, check } = await fixture(page);
  peer({ theme: "porcelain" });
  await importPack(page, pack);
  await expect(page.getByText("Theme settings changed elsewhere. The write was not retried; the theme shown is the saved one.", { exact: true })).toBeVisible();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "graphite");
  await page.evaluate(() => window.dispatchEvent(new Event("focus")));
  await expect(page.getByTestId("saved-selection")).toHaveText("porcelain");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "porcelain");
  expect(writes).toHaveLength(1);
  check(1);
});

test("the theme file actions use the existing engineering tokens with wrapped actions", async ({ page }, testInfo) => {
  const { pack, check } = await fixture(page, true);
  await themeSelect(page).selectOption(pack.id);
  const actions = page.locator(".theme-manager .theme-manager-actions");
  await expect(actions).toHaveCSS("flex-wrap", "wrap");
  await actions.scrollIntoViewIfNeeded();
  await testInfo.attach("Appearance desktop", { body: await page.screenshot(), contentType: "image/png" });
  check();
});

test("narrow Appearance remains scrollable and closing Settings restores its external opener", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  const { writes, check } = await fixture(page);
  const hint = page.locator("[data-theme-storage]");
  await hint.scrollIntoViewIfNeeded();
  await expect(hint).toBeInViewport();
  expect(await page.locator("html").evaluate((root) => root.scrollWidth <= window.innerWidth)).toBe(true);
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog", { name: "Settings", exact: true })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Open settings", exact: true })).toBeFocused();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "graphite");
  expect(writes).toEqual([]);
  check();
});
