/** Reproducible offline Chromium proof of the CRT 1.1 palette and reference study. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { chromium } from "../apps/knx-web/node_modules/playwright/index.mjs";

const root = fileURLToPath(new URL("../", import.meta.url));
const packText = await readFile(root + "apps/knx-web/themes/modern-retro-green-crt.knx-theme.json", "utf8");
const pack = JSON.parse(packText);
const checks = [];
const browser = await chromium.launch({ executablePath: "/usr/bin/chromium", headless: true });
try {
  const demo = await browser.newPage({ viewport: { width: 1440, height: 960 }, reducedMotion: "no-preference" });
  const pageErrors = [], externalRequests = [];
  demo.on("pageerror", error => pageErrors.push(error.message));
  demo.on("request", request => { if (/^https?:/.test(request.url())) externalRequests.push(request.url()); });
  await demo.goto("file://" + root + "design/modern-retro-green-crt.preview.html");
  await demo.evaluate(() => document.fonts.ready);
  await demo.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  assert.equal(await demo.locator("tbody tr").count(), 12);
  assert.equal(await demo.locator("body").evaluate(element => getComputedStyle(element).color), "rgb(190, 219, 187)");
  assert.equal(await demo.locator("#save").evaluate(element => getComputedStyle(element).backgroundColor), "rgb(107, 33, 168)");
  assert.ok(await demo.evaluate(() => document.fonts.check('12px "JetBrains Mono"')));
  checks.push("reference palette: phosphor ink, purple Save, embedded Mono, twelve native rows");
  assert.ok(await demo.evaluate(() => document.documentElement.scrollHeight <= innerHeight));
  checks.push("desktop chrome and footer fit the viewport; only the table scrolls");

  await demo.locator(".table-select").nth(0).focus();
  await demo.keyboard.press("ArrowDown");
  assert.ok(await demo.locator(".table-select").nth(1).evaluate(element => element === document.activeElement));
  assert.equal(await demo.locator('tbody tr[aria-selected="true"]').count(), 0);
  await demo.keyboard.press("Enter");
  assert.equal(await demo.locator('tbody tr[aria-selected="true"]').count(), 1);
  assert.equal(await demo.locator("#name").inputValue(), "Küche Licht");
  assert.ok(await demo.evaluate(() => document.getAnimations().some(animation => animation.animationName === "confirm-glow")));
  checks.push("native arrow focus does not select; Enter activates once and paints actual flash");

  await demo.locator('tbody tr input[type="checkbox"]').nth(3).check();
  assert.equal(await demo.locator("#marked").textContent(), "1 markiert");
  assert.equal(await demo.locator("#name").inputValue(), "Küche Licht");
  checks.push("checkbox multiselection is independent of primary row selection");

  await demo.locator("#filter").focus();
  await demo.locator("tbody tr").nth(8).hover();
  assert.equal(await demo.locator("tbody tr").nth(8).evaluate(element => getComputedStyle(element).transitionDuration), "0.25s");
  assert.ok(await demo.evaluate(() => document.getAnimations().some(animation => animation.animationName === "row-sweep")));
  const alignment = await demo.evaluate(() => {
    const row = document.querySelectorAll("tbody tr")[8].getBoundingClientRect();
    const beam = document.querySelector("#row-light").getBoundingClientRect();
    return { width: Math.abs(row.width - beam.width), top: Math.abs(row.top - beam.top) };
  });
  assert.ok(alignment.width < 1 && alignment.top < 1);
  checks.push("real 250ms native-row fill and transient overlay align without table pseudo-cells");
  await demo.waitForFunction(() => !document.querySelector("#row-light").hasAttribute("data-visible"));
  checks.push("beam retires after its bounded pulse; no persistent white flare over data");

  await demo.locator("#filter").fill("Küche");
  assert.equal(await demo.locator("tbody tr:visible").count(), 2);
  await demo.locator(".table-select").nth(1).focus();
  await demo.keyboard.press("ArrowDown");
  assert.ok(await demo.locator(".table-select").nth(9).evaluate(element => element === document.activeElement));
  checks.push("filter preserves rows; keyboard skips hidden native rows");
  await demo.locator("#filter").fill("");

  await demo.locator("tbody tr").nth(7).hover();
  await demo.locator("#motion").click();
  assert.equal(await demo.locator("tbody tr").nth(7).evaluate(element => getComputedStyle(element).transitionDuration), "0s");
  assert.equal(await demo.evaluate(() => document.getAnimations().length), 0);
  assert.equal(await demo.locator("#row-light").isVisible(), false);
  checks.push("Off cancels live fill, feedback and beam instead of waiting for old transitions");

  await demo.locator("#motion").click();
  await demo.locator("tbody tr").nth(6).hover();
  await demo.emulateMedia({ reducedMotion: "reduce" });
  await demo.locator(".table-select").nth(6).click();
  assert.equal(await demo.evaluate(() => document.getAnimations().length), 0);
  assert.equal(await demo.locator("#row-light").isVisible(), false);
  assert.equal(await demo.locator("tbody tr").nth(6).getAttribute("aria-selected"), "true");
  checks.push("OS reduced motion cancels running effects but retains static selection");

  await demo.emulateMedia({ reducedMotion: "no-preference" });
  await demo.waitForFunction(() => performance.now() - lastPulse >= 600);
  await demo.locator("#save").click();
  assert.ok(await demo.evaluate(() => document.getAnimations().some(animation => animation.animationName === "save-confirm")));
  assert.match(await demo.locator("#status").textContent(), /keine Datei geschrieben/);
  await demo.locator("#motion").click();
  assert.equal(await demo.evaluate(() => document.getAnimations().length), 0);
  checks.push("Save uses a real bounded violet flash and never claims a real file write");

  await demo.setViewportSize({ width: 390, height: 844 });
  assert.ok(await demo.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
  assert.ok(await demo.locator(".table-scroll").evaluate(element => element.scrollWidth > element.clientWidth));
  assert.equal(await demo.locator("tbody tr").count(), 12);
  checks.push("390px layout keeps the native table intact in local horizontal scrolling");
  assert.deepEqual(pageErrors, []); assert.deepEqual(externalRequests, []);

  await demo.setViewportSize({ width: 1440, height: 960 });
  await demo.reload();
  await demo.evaluate(() => document.fonts.ready);
  await demo.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  await demo.locator("tbody tr").nth(8).hover();
  await demo.waitForFunction(() => document.getAnimations().some(animation => animation.animationName === "row-sweep"));
  // Sample an actual browser-rendered animation frame; lifetime/motion tests above run normally.
  await demo.evaluate(async () => {
    clearTimeout(beamTimer);
    const animations = document.getAnimations();
    for (const animation of animations) animation.pause();
    await Promise.all(animations.map(animation => animation.ready));
    for (const animation of animations) animation.currentTime = 125;
    await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  });
  await demo.screenshot({ path: root + "design/modern-retro-green-crt.preview.png", fullPage: false });
  assert.equal(await demo.locator("#row-light").getAttribute("data-visible"), "true");
  await demo.close();

  const page = await browser.newPage({ viewport: { width: 1440, height: 960 } });
  const oldPack = { ...pack, version: "1.0.0", tokens: { ...pack.tokens, "--knx-foreground": "#39ff14", "--knx-border": "#39ff14", "--knx-surface": "#050505" } };
  let settings = { theme: "graphite", uiThemePacks: { [pack.id]: oldPack }, accent: "mint", density: "compact", motionLevel: "off", motionStyle: "apple", uiLanguage: "en", foreign: { keep: "synthetic" } };
  const original = structuredClone(settings), writes = [], unexpected = [], runtimeErrors = [];
  page.on("pageerror", error => runtimeErrors.push(error.message));
  await page.route("**/*", async route => {
    const request = route.request(), url = new URL(request.url());
    if (url.origin !== "http://127.0.0.1:4173") { unexpected.push(url.origin); await route.abort(); return; }
    if (url.pathname === "/api/settings") {
      if (request.method() === "PUT") {
        const patch = request.postDataJSON();
        assert.ok(patch.expectedSettings);
        for (const [key, value] of Object.entries(patch.expectedSettings)) assert.deepEqual(settings[key] ?? null, value);
        writes.push(patch); settings = { ...settings, ...patch.settings };
      } else assert.equal(request.method(), "GET");
      await route.fulfill({ json: { schemaVersion: 1, conditionalPatchVersion: 1, status: "ok", settings } });
    } else if (url.pathname.startsWith("/api/")) { unexpected.push(url.pathname); await route.abort(); }
    else await route.continue();
  });
  await page.goto("http://127.0.0.1:4173/e2e/theme-manager-fixture.html?representative=1");
  await page.getByRole("button", { name: "Open settings", exact: true }).click();
  const importPack = () => page.locator("#theme-pack-import").setInputFiles({ name: "modern-retro-green-crt.knx-theme.json", mimeType: "application/json", buffer: Buffer.from(packText) });
  await importPack();
  await page.waitForFunction(id => document.documentElement.dataset.theme === id, pack.id);
  assert.equal(writes.length, 0); assert.deepEqual(settings, original);
  checks.push("actual manager: reference palette previews without mutating installed pack");
  await page.getByRole("button", { name: "Apply theme", exact: true }).click();
  const confirmation = page.getByRole("dialog", { name: "Replace theme pack", exact: true });
  await confirmation.getByRole("button", { name: "Cancel replacement", exact: true }).click();
  assert.equal(writes.length, 0); assert.deepEqual(settings, original);
  checks.push("actual manager: refusing same-ID replacement preserves old contents and selection");

  await importPack();
  await page.getByRole("button", { name: "Apply theme", exact: true }).click();
  await confirmation.getByRole("button", { name: "Replace and apply", exact: true }).click();
  await page.waitForFunction(id => document.querySelector('[data-testid="saved-selection"]').textContent === id, pack.id);
  assert.equal(writes.length, 1); assert.deepEqual(settings.uiThemePacks[pack.id], pack);
  assert.deepEqual(settings.foreign, original.foreign); assert.equal(settings.motionLevel, "off");
  checks.push("actual manager: explicit 1.1 replacement acknowledges one guarded mock write");
  await page.reload();
  await page.waitForFunction(id => document.documentElement.dataset.theme === id, pack.id);
  const tokens = await page.locator("html").evaluate((element, keys) => Object.fromEntries(keys.map(key => [key, element.style.getPropertyValue(key)])), Object.keys(pack.tokens));
  assert.deepEqual(tokens, pack.tokens);
  checks.push("actual manager: cold reload restores every exact v1.1 palette token");
  await page.getByRole("button", { name: "Open settings", exact: true }).click();
  const downloading = page.waitForEvent("download");
  await page.locator(`[data-theme-id="${pack.id}"]`).getByRole("button", { name: /Export/ }).click();
  const download = await downloading;
  assert.deepEqual(JSON.parse(await readFile(await download.path(), "utf8")), pack);
  checks.push("actual manager: exported 1.1 values and metadata roundtrip without loss");
  assert.deepEqual(unexpected, []); assert.deepEqual(runtimeErrors, []);
  await page.close();
  const receipt = { paletteVersion: pack.version, baseline: "e7f9db8e", checks: checks.length, passed: checks, screenshotFrame: "paused native animation at 125ms; lifetime tests use real timers", mockSettingsWrites: writes.length, pageErrors: pageErrors.length + runtimeErrors.length, unexpectedRequests: externalRequests.length + unexpected.length };
  await writeFile(root + "design/retro-green-crt-reference.receipt.json", JSON.stringify(receipt, null, 2) + "\n");
  console.log(JSON.stringify(receipt));
} finally { await browser.close(); }
