/** About links the GitHub Hall of Fame in both languages and widths without fetching it. */
import { expect, test, type Page } from "@playwright/test";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { fulfillNoProjectOpen } from "./server-project-fixture";
import { openFileSubmenu } from "./file-menu";

const boardUrl = "https://github.com/KNXBench-Labs/KNXBench/blob/main/docs/community/HALL_OF_FAME.md";

async function fixture(page: Page, language: "en" | "de", width: number, theme: string) {
  await page.setViewportSize({ width, height: 900 });
  const settings = { uiLanguage: language, theme, motionLevel: "off", achievementsEnabled: false, releaseChecksEnabled: false,
    onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.11" } };
  await page.addInitScript((value) => localStorage.setItem("knx-desktop:settings-cache", JSON.stringify({ schemaVersion: 1, settings: value })), settings);
  const github: string[] = [], unexpected: string[] = [];
  // Abort even the deliberate link activation: this fixture never contacts GitHub.
  await page.context().route(/^https:\/\/([a-z0-9-]+\.)?github(usercontent)?\.com\//, (route) => {
    github.push(route.request().url()); return route.abort();
  });
  await page.route("**/api/**", (route) => {
    const path = new URL(route.request().url()).pathname, method = route.request().method();
    const ok = (body: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
    if (path === "/api/auth/status") return ok({ required: false, authenticated: true });
    const achievement = achievementsFixtureAnswer(method, path); if (achievement !== undefined) return ok(achievement);
    if (path === "/api/settings") return ok({ conditionalPatchVersion: 1, schemaVersion: 1, status: "ok", settings });
    if (path === "/api/version") return ok({ version: "0.1.0-alpha.11+g1234567" });
    if (path === "/api/bus/discover") return ok({ interfaces: [] });
    if (path === "/api/project" && method === "GET") return fulfillNoProjectOpen(route);
    if (path === "/api/legacy-password") return ok({ available: true, remembered: false, problem: null });
    if (["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items", "/api/log"].includes(path)) return ok([]);
    unexpected.push(`${method} ${path}`); return route.fulfill({ status: 404, body: "unexpected request" });
  });
  return { github, unexpected };
}

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  const theme = (language === "en") === (width === 1440) ? "graphite" : "porcelain";
  test(`${language}-${theme}-${width}: About links the Hall of Fame document only on activation`, async ({ page }) => {
    const m = language === "de" ? de : en;
    const errors: string[] = []; page.on("pageerror", (error) => errors.push(error.message));
    const f = await fixture(page, language, width, theme);
    await page.goto("/");
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    await page.locator(".file-menu > summary").click();
    await openFileSubmenu(page, "help");
    await page.getByRole("menuitem", { name: m["toolbar.about"], exact: true }).click();
    const dialog = page.getByRole("dialog", { name: m["about.title"], exact: true });
    const section = dialog.locator("[data-hall-of-fame]");
    const link = section.getByRole("link", { name: m["about.hallOfFame.board"], exact: true });
    await expect(link).toBeVisible();
    await expect(link).toHaveAttribute("href", boardUrl);
    await expect(link).toHaveAttribute("target", "_blank");
    await expect(link).toHaveAttribute("rel", "noopener noreferrer");
    await expect(section.locator("summary, time, ol, [data-hall-of-fame-entry]")).toHaveCount(0);
    await link.scrollIntoViewIfNeeded();
    const box = (await link.boundingBox())!;
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(width);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    if (process.env.KNX_ABOUT_CAPTURE_DIR) await page.screenshot({ path: `${process.env.KNX_ABOUT_CAPTURE_DIR}/about-hall-of-fame-${language}-${width}.png` });
    expect(f.github).toEqual([]);
    expect(f.unexpected).toEqual([]);
    expect(errors).toEqual([]);
    // Real keyboard activation opens the canonical link, not a background fetch.
    await link.focus();
    const [request] = await Promise.all([
      page.context().waitForEvent("request", (request) => request.url() === boardUrl),
      link.press("Enter"),
    ]);
    expect(request.isNavigationRequest()).toBe(true);
    await expect.poll(() => f.github).toEqual([boardUrl]);
    await expect(dialog).toBeVisible();
  });
}
