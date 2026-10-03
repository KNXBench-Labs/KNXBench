/** Offline Chromium proof of actual App/ProjectExplorer/GroupAddressTable CRT interactions. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { chromium } from "../apps/knx-web/node_modules/playwright/index.mjs";
import { initialTree } from "../apps/knx-web/e2e/site-fixture-data.ts";

const root = fileURLToPath(new URL("../", import.meta.url));
const origin = process.env.CRT_FIXTURE_ORIGIN ?? "http://127.0.0.1:4173";
assert.equal(new URL(origin).hostname, "127.0.0.1", "Only the local no-proxy fixture server is allowed");
const pack = JSON.parse(await readFile(root + "apps/knx-web/themes/modern-retro-green-crt.knx-theme.json", "utf8"));
const tree = structuredClone(initialTree);
tree.is_modified = true;
tree.installations[0].group_addresses = Array.from({ length: 12 }, (_, i) => ({
  id: 21 + i, address: `1/0/${i + 1}`, name: `Synthetic lighting ${String(i + 1).padStart(2, "0")}`,
  range: null, dpts: [], links: [],
}));
const checks = [], errors = [], unexpected = [], saves = [];
const browser = await chromium.launch({ executablePath: "/usr/bin/chromium", headless: true });

async function workbench({ screenshot = false } = {}) {
  const page = await browser.newPage({ viewport: { width: 1440, height: 960 }, reducedMotion: "no-preference" });
  if (screenshot) await page.clock.install(); // Before navigation or any application timer.
  let settings = { theme: pack.id, uiThemePacks: { [pack.id]: pack }, motionStyle: "crt", motionLevel: "standard",
    accent: "mint", density: "compact", uiLanguage: "en", autosaveEnabled: false, uiScale: 1 };
  page.on("pageerror", error => errors.push(error.message));
  await page.addInitScript(settings => {
    localStorage.setItem("knx-desktop:settings-cache", JSON.stringify({ schemaVersion: 1, settings }));
    localStorage.setItem("knx-desktop:settings-adopted", "1");
  }, settings);
  await page.route("**/*", async route => {
    const request = route.request(), url = new URL(request.url()), method = request.method();
    if (url.origin !== origin) { unexpected.push(`${method} ${url.origin}`); await route.abort(); return; }
    if (!url.pathname.startsWith("/api/")) {
      if (method !== "GET") { unexpected.push(`${method} ${url.pathname}`); await route.abort(); return; }
      await route.continue(); return;
    }
    if (url.pathname === "/api/settings" && ["GET", "PUT"].includes(method)) {
      if (method === "PUT") {
        const patch = request.postDataJSON();
        if (patch.expectedSettings) {
          for (const [key, value] of Object.entries(patch.expectedSettings)) assert.deepEqual(settings[key] ?? null, value);
        }
        settings = { ...settings, ...patch.settings };
      }
      await route.fulfill({ json: { schemaVersion: 1, conditionalPatchVersion: 1, status: "ok", settings } });
    } else if (url.pathname === "/api/auth/status" && method === "GET") {
      await route.fulfill({ json: { required: false, authenticated: false } });
    } else if (url.pathname === "/api/project" && method === "GET") {
      await route.fulfill({ json: { ...tree, has_store_path: true } });
    } else if (url.pathname === "/api/fs/list" && method === "GET") {
      await route.fulfill({ json: [{ name: "Synthetic.knxdb", is_dir: false }] });
    } else if (url.pathname === "/api/project/open" && method === "POST") {
      assert.equal(request.postDataJSON().path, "Synthetic.knxdb");
      await route.fulfill({ json: tree }); // No file read: synthetic projection via the real native-open UI.
    } else if (url.pathname === "/api/product-languages" && method === "GET") {
      await route.fulfill({ json: [] });
    } else if (url.pathname === "/api/bus/discover" && method === "POST") {
      await route.fulfill({ json: [] }); // Intercept startup discovery; never contact a bus.
    } else if (url.pathname === "/api/project/load-progress" && method === "GET") {
      await route.fulfill({ json: null });
    } else if (url.pathname === "/api/project/save" && method === "POST") {
      saves.push("synthetic refusal");
      await route.fulfill({ status: 500, json: { error: "CRT synthetic Save refusal — no file written" } });
    } else { unexpected.push(`${method} ${url.pathname}`); await route.abort(); }
  });
  await page.goto(origin + "/");
  await page.locator(".file-menu > summary").click();
  await page.locator(".file-menu-content > button").nth(2).click();
  await page.locator(".fs-picker-list").getByRole("button", { name: "Synthetic.knxdb", exact: true }).click();
  await page.locator('.workbench-navigation').getByRole("button", { name: /group addresses/i }).click();
  await page.waitForFunction(() => document.querySelectorAll(".address-table tbody tr").length === 12);
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(650);
  return page;
}

const hasEffects = page => page.evaluate(() => document.getAnimations().some(animation =>
  ["knx-crt-sweep", "knx-crt-flash"].includes(animation.animationName) || animation.transitionProperty === "background-size"));
const mode = (page, value) => page.evaluate(async level => {
  const { setSetting } = await import("/src/settingsStore.ts"); setSetting("motionLevel", level);
}, value);
const style = (page, value) => page.evaluate(async id => {
  const { setSetting } = await import("/src/settingsStore.ts"); setSetting("motionStyle", id);
}, value);

try {
  const page = await workbench();
  const rows = page.locator(".address-table tbody tr"), buttons = page.locator(".address-table .table-select");
  const selectedRows = page.locator('.address-table tbody tr[aria-selected="true"]');
  const flashingRows = page.locator('.address-table tbody tr[data-crt-flash="true"]');
  assert.equal(await page.locator("html").getAttribute("data-theme"), pack.id);
  assert.equal(await page.locator("html").getAttribute("data-motion-style"), "crt");
  assert.equal(await rows.first().evaluate(row => getComputedStyle(row).transitionDuration), "0.25s");
  checks.push("real workbench loads the cached v1 CRT palette and persisted CRT style; 250ms fill");

  await rows.nth(2).hover();
  await page.waitForFunction(() => document.getAnimations().some(animation => animation.animationName === "knx-crt-sweep"));
  assert.equal(await page.locator(".crt-interaction-light").getAttribute("aria-hidden"), "true");
  assert.equal(await page.locator(".crt-interaction-light").getAttribute("inert"), "");
  assert.equal(await page.locator("table .crt-interaction-light").count(), 0);
  await page.waitForTimeout(300);
  assert.equal(await page.locator(".crt-interaction-light").count(), 0);
  checks.push("one actual inert light sweeps outside native table markup and expires normally");

  await page.waitForTimeout(650);
  await buttons.nth(0).focus();
  await page.keyboard.press("ArrowDown");
  assert.ok(await buttons.nth(1).evaluate(button => button === document.activeElement));
  assert.equal(await selectedRows.count(), 0);
  await page.keyboard.press("Enter");
  assert.equal(await rows.nth(1).getAttribute("aria-selected"), "true");
  assert.equal(await flashingRows.count(), 1);
  await page.waitForTimeout(160);
  assert.equal(await flashingRows.count(), 0);
  checks.push("native arrow focus preserves selection; Enter selects and flashes once, then clears");

  await rows.nth(3).locator('input[type="checkbox"]').check();
  assert.equal(await rows.nth(1).getAttribute("aria-selected"), "true");
  assert.ok((await rows.nth(3).getAttribute("class")).includes("row-multi-selected"));
  assert.match(await rows.nth(3).evaluate(row => getComputedStyle(row).boxShadow), /inset/);
  assert.equal(await flashingRows.count(), 0);
  checks.push("checkbox multiselection stays independent, including the existing bulk-selection rail");

  const treeButtons = page.locator(".project-explorer .tree-label");
  await treeButtons.first().focus();
  await page.keyboard.press("ArrowDown");
  assert.ok(await treeButtons.nth(1).evaluate(button => button === document.activeElement));
  await page.keyboard.press("Home");
  assert.ok(await treeButtons.first().evaluate(button => button === document.activeElement));
  await treeButtons.nth(1).focus();
  const installationToggle = treeButtons.nth(1).locator("..").locator(".tree-toggle");
  await page.keyboard.press("ArrowLeft");
  assert.equal(await installationToggle.getAttribute("aria-expanded"), "false");
  await page.keyboard.press("ArrowRight");
  assert.equal(await installationToggle.getAttribute("aria-expanded"), "true");
  checks.push("existing explorer arrow/Home and Left/Right expansion survive CRT without duplicate key handlers");

  await page.waitForTimeout(650);
  await rows.nth(6).hover();
  assert.ok(await hasEffects(page));
  await mode(page, "off");
  await page.waitForTimeout(30);
  assert.equal(await hasEffects(page), false);
  assert.equal(await page.locator(".crt-interaction-light").count(), 0);
  await buttons.nth(5).click();
  assert.equal(await rows.nth(5).getAttribute("aria-selected"), "true");
  assert.equal(await hasEffects(page), false);
  checks.push("Motion Off cancels live effects and allows immediate native selection without new effects");

  await mode(page, "subtle");
  await page.waitForTimeout(650);
  await rows.nth(7).hover();
  assert.equal(await rows.nth(7).evaluate(row => getComputedStyle(row).transitionDuration), "0.12s");
  assert.equal(await page.locator(".crt-interaction-light").count(), 0);
  await buttons.nth(7).click();
  assert.equal(await flashingRows.count(), 0);
  checks.push("Subtle uses 120ms fill only; no transient flash or light");

  await mode(page, "standard");
  await page.waitForTimeout(650);
  await rows.nth(8).hover();
  assert.ok(await hasEffects(page));
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.waitForTimeout(30);
  assert.equal(await hasEffects(page), false);
  assert.equal(await page.locator(".crt-interaction-light").count(), 0);
  await buttons.nth(8).click();
  assert.equal(await rows.nth(8).getAttribute("aria-selected"), "true");
  assert.equal(await hasEffects(page), false);
  checks.push("OS reduced motion cancels ongoing effects and suppresses subsequent click effects");
  await page.emulateMedia({ reducedMotion: "no-preference" });

  await page.waitForTimeout(650);
  await page.locator('[data-crt-surface="save"]').click();
  assert.equal(saves.length, 1);
  assert.equal(await page.locator('[data-crt-surface="save"]').getAttribute("data-crt-flash"), "true");
  await page.waitForTimeout(160);
  assert.equal(await page.locator('[data-crt-surface="save"]').getAttribute("data-crt-flash"), null);
  assert.ok(await page.getByText("CRT synthetic Save refusal — no file written", { exact: false }).count());
  await page.locator('[data-crt-surface="save"]').focus();
  await page.keyboard.press("Enter");
  assert.equal(saves.length, 2);
  checks.push("actual manual Save path glows but preserves errors; Enter issues exactly one mocked request");

  await page.evaluate(async () => { const { setSetting } = await import("/src/settingsStore.ts"); setSetting("uiScale", 1.5); });
  await rows.nth(10).scrollIntoViewIfNeeded();
  await page.waitForTimeout(650);
  await rows.nth(10).hover();
  const alignment = await page.evaluate(() => {
    const beam = document.querySelector(".crt-interaction-light").getBoundingClientRect();
    const wrap = document.querySelector(".address-table-view .workspace-table-wrap").getBoundingClientRect();
    const row = document.querySelectorAll(".address-table tbody tr")[10].getBoundingClientRect();
    return { left: beam.left, right: beam.right, top: beam.top, rowTop: row.top, wrapLeft: wrap.left, wrapRight: wrap.right };
  });
  assert.ok(alignment.left >= alignment.wrapLeft - 2 && alignment.right <= alignment.wrapRight + 2);
  assert.ok(Math.abs(alignment.top - alignment.rowTop) <= 2);
  await page.evaluate(() => window.dispatchEvent(new Event("resize")));
  assert.equal(await page.locator(".crt-interaction-light").count(), 0);
  checks.push("root zoom 1.5 aligns/clips the portal to the visible table; resize cancels it");

  await style(page, "apple");
  await page.waitForTimeout(650);
  await buttons.nth(4).click();
  assert.equal(await flashingRows.count(), 0);
  assert.equal(await rows.first().evaluate(row => getComputedStyle(row).backgroundImage), "none");
  checks.push("Smooth remains free of CRT paint/flash/light when switching the motion style");
  await page.close();

  const shot = await workbench({ screenshot: true });
  while (await shot.locator(".toast button").count()) await shot.locator(".toast button").first().click();
  await shot.locator(".address-table .table-select").first().click();
  await shot.waitForTimeout(650);
  await shot.clock.pauseAt(await shot.evaluate(() => Date.now()) + 1000);
  await shot.locator(".address-table tbody tr").nth(8).hover();
  await shot.evaluate(async () => {
    const animations = document.getAnimations().filter(animation => animation.animationName === "knx-crt-sweep" || animation.transitionProperty === "background-size");
    for (const animation of animations) animation.pause();
    await Promise.all(animations.map(animation => animation.ready));
    for (const animation of animations) animation.currentTime = 125;
  });
  await shot.screenshot({ path: root + "design/retro-green-crt.production.png", fullPage: false });
  assert.equal(await shot.locator(".crt-interaction-light").count(), 1);
  checks.push("fixed-viewport production screenshot samples a real native 125ms frame; timing tests run separately");
  await shot.close();
  assert.deepEqual(errors, []); assert.deepEqual(unexpected, []);
  assert.equal(new Set(checks).size, checks.length);
  await writeFile(root + "design/retro-green-crt.production.receipt.json", JSON.stringify({
    componentScope: ["App", "ProjectExplorer", "GroupAddressTable"], paletteVersion: pack.version,
    motionStyle: "crt", verifiedAt: new Date().toISOString(), checks: checks.length, passed: checks,
    mockedSaveRequests: saves.length, realBackendRequests: 0, pageErrors: errors.length, unexpectedRequests: unexpected.length,
    screenshotFrame: "native animation at 125ms with Playwright Clock paused; lifecycle tested separately with real timers",
  }, null, 2) + "\n");
  console.log(JSON.stringify({ checks: checks.length, passed: checks, mockedSaveRequests: saves.length, pageErrors: errors, unexpectedRequests: unexpected }, null, 2));
} finally { await browser.close(); }
