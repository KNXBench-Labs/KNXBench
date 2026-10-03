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
  await expect(page.getByRole("heading", { name: "Theme packs", exact: true })).toBeVisible();
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

async function importPack(page: Page, pack: ReturnType<typeof themePackFixture>) {
  await page.locator("#theme-pack-import").setInputFiles({ name: "synthetic.knx-theme.json", mimeType: "application/json", buffer: Buffer.from(JSON.stringify(pack)) });
  await expect(page.locator("html")).toHaveAttribute("data-theme", pack.id);
}

test("actual Appearance imports and cancels a draft under Strict Mode without writes", async ({ page }) => {
  const { pack, writes, check } = await fixture(page);
  await importPack(page, pack);
  await expect(page.getByTestId("saved-selection")).toHaveText("graphite");
  await expect(page.locator("html")).toHaveCSS("--knx-bg", pack.tokens["--knx-bg"]);
  expect(writes).toEqual([]);
  await page.getByRole("button", { name: "Cancel preview", exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "graphite");
  expect(await page.locator("html").evaluate((element) => (element as HTMLElement).style.getPropertyValue("--knx-bg"))).toBe("");
  expect(writes).toEqual([]);
  check();
});

test("actual Appearance rejects hostile file values without DOM effects or requests to the asset", async ({ page }) => {
  const { pack, writes, settings, check } = await fixture(page);
  const original = structuredClone(settings());
  pack.tokens["--knx-bg"] = 'url("https://invalid.example/synthetic-asset")';
  await page.locator("#theme-pack-import").setInputFiles({ name: "hostile.knx-theme.json", mimeType: "application/json", buffer: Buffer.from(JSON.stringify(pack)) });
  await expect(page.getByText("The theme file was rejected. The saved theme and installed packs have not changed.", { exact: true })).toBeVisible();
  await expect(page.getByText("The token value is outside the permitted syntax or bounds.", { exact: true })).toBeVisible();
  await expect(page.locator("[data-theme-diagnostic=invalidValue]")).toContainText("tokens.--knx-bg");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "graphite");
  expect(await page.locator("html").evaluate((root) => (root as HTMLElement).style.getPropertyValue("--knx-bg"))).toBe("");
  await expect(page.getByRole("button", { name: "Apply theme", exact: true })).toHaveCount(0);
  expect(writes).toEqual([]);
  expect(settings()).toEqual(original);
  check();
});

test("explicit System reset keeps installed contents and responds to OS changes", async ({ page }) => {
  const { pack, writes, settings, check } = await fixture(page, true);
  await page.locator(`[data-theme-id="${pack.id}"]`).getByRole("button", { name: `Preview ${pack.name}`, exact: true }).click();
  await page.getByRole("button", { name: "Apply theme", exact: true }).click();
  await expect(page.getByTestId("saved-selection")).toHaveText(pack.id);
  const beforeReset = structuredClone(settings());
  await page.getByRole("button", { name: "Use system theme", exact: true }).click();
  await expect(page.getByTestId("saved-selection")).toHaveText("system");
  await expect(page.getByText("Theme saved.", { exact: true })).toBeVisible();
  expect(settings()).toEqual({ ...beforeReset, theme: "system" });
  expect(writes[1]).toEqual({ settings: { theme: "system" }, expectedSettings: { theme: pack.id, uiThemePacks: { [pack.id]: pack } } });
  expect(writes).toHaveLength(2);
  await page.emulateMedia({ colorScheme: "dark" });
  await expect(page.locator("html")).toHaveAttribute("data-theme", "graphite");
  await page.emulateMedia({ colorScheme: "light" });
  await expect(page.locator("html")).toHaveAttribute("data-theme", "porcelain");
  expect(settings()).toEqual({ ...beforeReset, theme: "system" });
  expect(writes).toHaveLength(2);
  check();
});

test("an uncertain HTTP500 Apply restores the last confirmed palette and never replays the write", async ({ page }) => {
  const { pack, writes, requests, settings, failWrite, check } = await fixture(page);
  const original = structuredClone(settings());
  await importPack(page, pack);
  const baselineRequests = requests.length;
  failWrite();
  await page.getByRole("button", { name: "Apply theme", exact: true }).click();
  await expect(page.getByText("The theme could not be saved. Preview was cancelled; review the acknowledged settings before trying again.", { exact: true })).toBeVisible();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "graphite");
  expect(await page.locator("html").evaluate((root) => (root as HTMLElement).style.getPropertyValue("--knx-bg"))).toBe("");
  await expect(page.getByRole("button", { name: "Apply theme", exact: true })).toHaveCount(0);
  await expect(page.getByText("Theme saved.", { exact: true })).toHaveCount(0);
  expect(settings()).toEqual(original);
  expect(writes).toHaveLength(1);
  expect(requests.slice(baselineRequests)).toEqual(["PUT", "GET"]);
  check(0, 1);
});

test("replacement Escape cancels its draft and returns focus to the persistent import control", async ({ page }) => {
  const { pack, writes, check } = await fixture(page, true);
  await importPack(page, pack);
  const apply = page.getByRole("button", { name: "Apply theme", exact: true });
  await apply.click();
  const confirmation = page.getByRole("dialog", { name: "Replace theme pack", exact: true });
  await expect(confirmation).toBeVisible();
  const cancel = confirmation.getByRole("button", { name: "Cancel replacement", exact: true });
  await expect(cancel).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(confirmation).toHaveCount(0);
  await expect(page.getByRole("dialog", { name: "Settings", exact: true })).toBeVisible();
  await expect(apply).toHaveCount(0);
  await expect(page.locator("html")).toHaveAttribute("data-theme", "graphite");
  await expect(page.locator("#theme-pack-import")).toBeFocused();
  expect(writes).toEqual([]);
  check();
});

test("explicit Apply persists across cold reload and the actual download reimports exactly", async ({ page }) => {
  const { pack, writes, settings, check } = await fixture(page);
  await importPack(page, pack);
  await page.getByRole("button", { name: "Apply theme", exact: true }).click();
  await expect(page.getByTestId("saved-selection")).toHaveText(pack.id);
  expect(writes).toEqual([{ settings: { theme: pack.id, uiThemePacks: { [pack.id]: pack } }, expectedSettings: { theme: "graphite", uiThemePacks: {} } }]);
  expect(settings().foreign).toEqual({ keep: ["synthetic", 7] });
  await page.reload();
  await expect(page.getByTestId("saved-selection")).toHaveText(pack.id);
  await page.getByRole("button", { name: "Open settings", exact: true }).click();
  const row = page.locator(`[data-theme-id="${pack.id}"]`);
  await expect(row).toContainText("Selected (saved)");
  const downloadEvent = page.waitForEvent("download");
  await row.getByRole("button", { name: `Export ${pack.name}`, exact: true }).click();
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

test("replacement confirmation traps keyboard focus and Escape never writes", async ({ page }) => {
  const { pack, writes, check } = await fixture(page, true);
  await importPack(page, pack);
  await page.getByRole("button", { name: "Apply theme", exact: true }).click();
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

test("confirmed active removal falls back to System and retains unrelated settings", async ({ page }) => {
  const { pack, writes, settings, check } = await fixture(page, true);
  const row = page.locator(`[data-theme-id="${pack.id}"]`);
  await row.getByRole("button", { name: `Preview ${pack.name}`, exact: true }).click();
  await page.getByRole("button", { name: "Apply theme", exact: true }).click();
  await expect(page.getByTestId("saved-selection")).toHaveText(pack.id);
  await row.getByRole("button", { name: `Remove ${pack.name}`, exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Remove theme pack", exact: true });
  await expect(dialog.getByRole("button", { name: "Cancel removal", exact: true })).toBeFocused();
  expect(writes).toHaveLength(1);
  await dialog.getByRole("button", { name: "Remove pack", exact: true }).click();
  await expect(page.getByTestId("saved-selection")).toHaveText("system");
  expect(writes[1]).toEqual({ settings: { theme: "system", uiThemePacks: {} }, expectedSettings: { theme: pack.id, uiThemePacks: { [pack.id]: pack } } });
  expect(settings().accent).toBe("mint");
  expect(settings().density).toBe("compact");
  expect(settings().foreign).toEqual({ keep: ["synthetic", 7] });
  await expect(row).toHaveCount(0);
  await expect(page.locator("html")).toHaveAttribute("data-theme", "porcelain");
  check();
});

test("a conflicting Apply rolls back, then the existing peer refresh takes over without a second write", async ({ page }) => {
  const { pack, writes, peer, check } = await fixture(page);
  await importPack(page, pack);
  peer({ theme: "porcelain" });
  await page.getByRole("button", { name: "Apply theme", exact: true }).click();
  await expect(page.getByText("Theme settings changed elsewhere. Preview was cancelled without retrying the write.", { exact: true })).toBeVisible();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "graphite");
  await page.evaluate(() => window.dispatchEvent(new Event("focus")));
  await expect(page.getByTestId("saved-selection")).toHaveText("porcelain");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "porcelain");
  expect(writes).toHaveLength(1);
  check(1);
});

test("management rows use the existing engineering tokens with wrapped actions", async ({ page }, testInfo) => {
  const { pack, check } = await fixture(page, true);
  const list = page.locator(".theme-manager-list");
  await expect(list).toHaveCSS("list-style-type", "none");
  const row = page.locator(`[data-theme-id="${pack.id}"]`);
  await expect(row).toHaveCSS("display", "grid");
  await expect(row.locator(".theme-manager-actions")).toHaveCSS("flex-wrap", "wrap");
  await row.scrollIntoViewIfNeeded();
  await testInfo.attach("Appearance desktop", { body: await page.screenshot(), contentType: "image/png" });
  check();
});

test("narrow Appearance remains scrollable and closing Settings restores its external opener", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  const { pack, writes, check } = await fixture(page);
  await importPack(page, pack);
  const apply = page.getByRole("button", { name: "Apply theme", exact: true });
  await apply.scrollIntoViewIfNeeded();
  await expect(apply).toBeInViewport();
  expect(await page.locator("html").evaluate((root) => root.scrollWidth <= window.innerWidth)).toBe(true);
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog", { name: "Settings", exact: true })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Open settings", exact: true })).toBeFocused();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "graphite");
  expect(writes).toEqual([]);
  check();
});
