/** Built workbench release notices with synthetic API and release requests. */
import { expect, test, type Page } from "@playwright/test";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { fulfillNoProjectOpen } from "./server-project-fixture";
import { GITHUB_RELEASES_API } from "../src/githubReleases";
import { openFileSubmenu } from "./file-menu";
const UPDATE_CHECK_INTERVAL_MS = 60 * 60 * 1000;

test.use({ timezoneId: "UTC" });
async function fixture(page: Page, language: "en" | "de", width: number, enabled = true) {
  await page.setViewportSize({ width, height: 900 });
  await page.clock.install({ time: new Date("2026-10-11T12:00:00Z") });
  const version = "0.1.0-alpha.9+g1234567";
  const settings = { uiLanguage: language, theme: width === 400 ? "porcelain" : "graphite", motionLevel: "off", achievementsEnabled: false,
    releaseChecksEnabled: enabled, onboardingGuide: { seenStage: "alpha", version } };
  await page.addInitScript(settings => localStorage.setItem("knx-desktop:settings-cache", JSON.stringify({ schemaVersion: 1, settings })), settings);
  const release = (tag: string) => ({ tag_name: tag, draft: false, prerelease: true, published_at: "2026-10-11T10:00:00Z" });
  const feed = { rows: [release("v0.1.0-alpha.10")], calls: 0, status: 200 };
  const unexpected: string[] = [], apiCalls: string[] = [];
  await page.context().route("https://github.com/KNXBench-Labs/KNXBench/releases/**", route => route.fulfill({ contentType: "text/html", body: "<!doctype html><title>Synthetic release details</title>" }));
  await page.route("https://api.github.com/**", route => {
    if (route.request().url() !== GITHUB_RELEASES_API) unexpected.push(route.request().url());
    feed.calls++;
    return route.fulfill({ status: feed.status, contentType: "application/json", headers: { "Access-Control-Allow-Origin": "*" }, body: JSON.stringify(feed.rows) });
  });
  await page.route("**/api/**", route => {
    const path = new URL(route.request().url()).pathname, method = route.request().method(); apiCalls.push(`${method} ${path}`);
    const ok = (body: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
    if (path === "/api/auth/status") return ok({ required: false, authenticated: true });
    const achievement = achievementsFixtureAnswer(method, path); if (achievement !== undefined) return ok(achievement);
    if (path === "/api/settings" && method === "GET") return ok({ conditionalPatchVersion: 1, schemaVersion: 1, status: "ok", settings });
    if (path === "/api/settings" && method === "PUT") {
      Object.assign(settings, route.request().postDataJSON().settings);
      return ok({ conditionalPatchVersion: 1, schemaVersion: 1, status: "ok", settings });
    }
    if (path === "/api/version") return ok({ version });
    if (path === "/api/bus/discover") return ok({ interfaces: [] });
    if (path === "/api/project" && method === "GET") return fulfillNoProjectOpen(route);
    if (path === "/api/legacy-password") return ok({ available: true, remembered: false, problem: null });
    if (["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items", "/api/log"].includes(path)) return ok([]);
    unexpected.push(`${method} ${path}`); return route.fulfill({ status: 404, body: "unexpected request" });
  });
  return { feed, unexpected, apiCalls, release };
}
async function about(page: Page, language: "en" | "de") {
  const m = language === "de" ? de : en;
  await page.locator(".file-menu > summary").click();
  await openFileSubmenu(page, "help");
  await page.getByRole("menuitem", { name: m["toolbar.about"], exact: true }).click();
  return page.getByRole("dialog", { name: m["about.title"], exact: true });
}
for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language}-${width}: startup notice, accessible official link, durable About and hourly newer release`, async ({ page }) => {
    const f = await fixture(page, language, width); await page.goto("/");
    const m = language === "de" ? de : en;
    const toast = page.locator(".toast--update");
    await expect(toast).toBeVisible(); await expect(toast).toHaveAttribute("role", "status");
    await expect(page.locator("html")).toHaveAttribute("lang", language);
    await expect(page.locator("html")).toHaveAttribute("data-theme", width === 400 ? "porcelain" : "graphite");
    await expect(toast).toContainText("0.1.0-alpha.10");
    const action = toast.getByRole("link", { name: m["updates.releaseDetails"], exact: true });
    await expect(action).toHaveAttribute("href", "https://github.com/KNXBench-Labs/KNXBench/releases/tag/v0.1.0-alpha.10");
    await action.focus(); await expect(action).toBeFocused();
    expect(await action.evaluate(node => { const r = node.getBoundingClientRect(); return document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2) === node; })).toBe(true);
    // Resting geometry: a toast slides in from the right.
    await toast.evaluate((el) => Promise.all(el.getAnimations().map((animation) => animation.finished)));
    const box = await toast.boundingBox(); expect(box!.x).toBeGreaterThanOrEqual(0); expect(box!.x + box!.width).toBeLessThanOrEqual(width);
    if (process.env.UPDATE_SCREENSHOTS) await page.screenshot({ path: `${process.env.UPDATE_SCREENSHOTS}/toast-${language}-${width}.png` });
    const opening = page.waitForEvent("popup"); await action.click(); const releasePage = await opening;
    await expect(releasePage).toHaveURL("https://github.com/KNXBench-Labs/KNXBench/releases/tag/v0.1.0-alpha.10");
    await releasePage.close();
    await toast.getByRole("button", { name: m["toast.dismiss"], exact: true }).click(); await page.clock.runFor(1000); await expect(toast).toHaveCount(0);
    const dialog = await about(page, language);
    await expect(dialog.locator(".release-update-panel")).toContainText("0.1.0-alpha.10");
    if (process.env.UPDATE_SCREENSHOTS) await page.screenshot({ path: `${process.env.UPDATE_SCREENSHOTS}/about-${language}-${width}.png` });
    const manualCheck = dialog.getByRole("button", { name: m["updates.check"], exact: true });
    await manualCheck.click();
    await expect.poll(() => f.feed.calls).toBe(2);
    // The intercepted handler increments before fetch and hook cleanup finish.
    // Do not jump an hour while that request can still be active/aborted.
    await expect(manualCheck).toBeEnabled();
    await expect(dialog.locator(".release-update-panel [role='status']")).toContainText("0.1.0-alpha.10");
    await expect(toast).toHaveCount(0);
    await dialog.getByRole("button", { name: m["about.close"], exact: true }).click();
    f.feed.rows = [f.release("v0.1.0-alpha.11")];
    await page.clock.fastForward(UPDATE_CHECK_INTERVAL_MS); await expect(toast).toContainText("0.1.0-alpha.11");
    expect(f.unexpected).toEqual([]);
    expect(f.apiCalls.filter(call => call.startsWith("POST ") || call.startsWith("PUT "))).toEqual(["POST /api/bus/discover"]);
  });
}
test("disabled auto check makes no external request; explicit About check remains available", async ({ page }) => {
  const f = await fixture(page, "en", 400, false); await page.goto("/");
  const dialog = await about(page, "en"); expect(f.feed.calls).toBe(0);
  await dialog.getByRole("button", { name: en["updates.check"], exact: true }).click(); await expect.poll(() => f.feed.calls).toBe(1);
  await expect(dialog.locator(".release-update-panel")).toContainText("0.1.0-alpha.10"); expect(f.unexpected).toEqual([]);
});
test("GitHub unavailable never blocks startup or claims the running release is current", async ({ page }) => {
  const f = await fixture(page, "de", 400); f.feed.status = 429; await page.goto("/");
  const dialog = await about(page, "de"); await expect(dialog.locator(".release-update-panel")).toContainText(de["updates.unavailable"]);
  await expect(page.locator(".toast--update")).toHaveCount(0); expect(f.unexpected).toEqual([]);
});

test("Settings opt-out persists, cancels automatic checks and survives a fresh app start", async ({ page }) => {
  const f = await fixture(page, "de", 400); await page.goto("/"); await expect(page.locator(".toast--update")).toBeVisible();
  await page.getByRole("button", { name: de["toolbar.commands"], exact: true }).click();
  await page.getByRole("combobox").fill(de["toolbar.settings"]);
  await page.getByRole("option").filter({ hasText: de["toolbar.settings"] }).click();
  const checkbox = page.getByRole("checkbox", { name: de["updates.enabled"], exact: true });
  await expect(checkbox).toBeChecked(); await checkbox.uncheck();
  await expect.poll(() => f.apiCalls.filter(call => call === "PUT /api/settings").length).toBe(1);
  await page.clock.fastForward(UPDATE_CHECK_INTERVAL_MS); expect(f.feed.calls).toBe(1);
  await page.reload(); await expect(page.getByRole("button", { name: de["toolbar.commands"], exact: true })).toBeVisible();
  await page.clock.fastForward(UPDATE_CHECK_INTERVAL_MS); expect(f.feed.calls).toBe(1); expect(f.unexpected).toEqual([]);
});
