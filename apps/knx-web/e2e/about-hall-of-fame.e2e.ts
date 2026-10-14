/** About lists the bundled Hall of Fame in both languages and widths without asking GitHub. */
import { readFileSync } from "node:fs";
import { expect, test, type Page } from "@playwright/test";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { fulfillNoProjectOpen } from "./server-project-fixture";

const bundled = JSON.parse(readFileSync(new URL("../../../docs/community/hall-of-fame.json", import.meta.url), "utf8")) as {
  generated_at: string; contributors: Array<{ login: string; xp: number; rank: { title: string }; medals: string[] }> };

async function fixture(page: Page, language: "en" | "de", width: number, theme: string) {
  await page.setViewportSize({ width, height: 900 });
  const settings = { uiLanguage: language, theme, motionLevel: "off", achievementsEnabled: false, releaseChecksEnabled: false,
    onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.11" } };
  await page.addInitScript((value) => localStorage.setItem("knx-desktop:settings-cache", JSON.stringify({ schemaVersion: 1, settings: value })), settings);
  const github: string[] = [], unexpected: string[] = [];
  // Any request to GitHub would be a background request About must never make.
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
  test(`${language}-${theme}-${width}: About lists every participant of the bundled Hall of Fame`, async ({ page }) => {
    const m = language === "de" ? de : en;
    const errors: string[] = []; page.on("pageerror", (error) => errors.push(error.message));
    const f = await fixture(page, language, width, theme);
    await page.goto("/");
    await page.locator(".file-menu > summary").click();
    await page.getByRole("button", { name: m["toolbar.about"], exact: true }).click();
    const dialog = page.getByRole("dialog", { name: m["about.title"], exact: true });
    const section = dialog.locator("[data-hall-of-fame]");
    // Closed at first, so About stays compact; one click shows everyone.
    const summary = section.locator("summary");
    await expect(summary).toHaveText(m["about.hallOfFame.titleCount"].replace("{count}", String(bundled.contributors.length)));
    await expect(section).not.toHaveAttribute("open", "");
    await summary.click();
    await expect(section).toHaveAttribute("open", "");
    const rows = section.locator("[data-hall-of-fame-entry]");
    await expect(rows).toHaveCount(bundled.contributors.length);
    for (const [index, person] of bundled.contributors.entries()) {
      const row = rows.nth(index);
      await expect(row).toHaveAttribute("data-hall-of-fame-entry", person.login);
      await expect(row).toContainText(person.rank.title);
      await expect(row).toContainText(`${person.xp} XP`);
      for (const medal of person.medals) await expect(row).toContainText(medal);
    }
    await expect(section.locator("time")).toHaveAttribute("datetime", bundled.generated_at);
    await expect(section).toContainText(m["about.hallOfFame.notLive"]);
    await expect(section.locator("[data-hall-of-fame-board]")).toHaveAttribute("href",
      "https://github.com/KNXBench-Labs/KNXBench/blob/main/docs/community/HALL_OF_FAME.md");
    await rows.last().scrollIntoViewIfNeeded();
    const box = (await section.boundingBox())!;
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(width);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    if (process.env.KNX_ABOUT_CAPTURE_DIR) await page.screenshot({ path: `${process.env.KNX_ABOUT_CAPTURE_DIR}/about-hall-of-fame-${language}-${width}.png` });
    expect(f.github).toEqual([]);
    expect(f.unexpected).toEqual([]);
    expect(errors).toEqual([]);
  });
}
