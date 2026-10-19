/** Verifies achievements in Chromium: popup, overview through the palette, and off means off. */

import { expect, test, type Page } from "@playwright/test";
import { ACHIEVEMENTS } from "../src/achievementCatalog";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { fulfillNoProjectOpen } from "./server-project-fixture";
import { openFileSubmenu } from "./file-menu";

interface Fixture {
  settings: Record<string, unknown>;
  record: { unlocked: Record<string, string>; progress: Record<string, number> };
  posts: unknown[];
  /** KL-164: request ids of applied increment batches, like the real server. */
  applied: string[];
  unexpected: string[];
}

const VERSION = "0.1.0-alpha.2+gfixture";

// Every request is answered locally: no server, no gateway, no network.
async function serve(page: Page, fixture: Fixture) {
  await page.route("**/api/**", async (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    const method = request.method();
    const record = () => ({ schemaVersion: 1, status: "ok", ...fixture.record });
    let body: unknown;
    if (path === "/api/auth/status" && method === "GET") {
      body = { required: false, authenticated: true };
    } else if (path === "/api/settings" && method === "GET") {
      body = { schemaVersion: 1, conditionalPatchVersion: 1, status: "ok", settings: fixture.settings };
    } else if (path === "/api/settings" && method === "PUT") {
      const patch = (request.postDataJSON() as { settings: Record<string, unknown> }).settings;
      fixture.settings = { ...fixture.settings, ...patch };
      body = { schemaVersion: 1, conditionalPatchVersion: 1, status: "ok", settings: fixture.settings };
    } else if (path === "/api/version" && method === "GET") {
      body = { version: VERSION };
    } else if (path === "/api/achievements" && method === "GET") {
      body = record();
    } else if (path === "/api/achievements/record" && method === "POST") {
      const delta = request.postDataJSON() as Fixture["record"] & { increments?: Record<string, number>; requestId?: string };
      fixture.posts.push(delta);
      for (const [id, at] of Object.entries(delta.unlocked)) fixture.record.unlocked[id] ??= at;
      for (const [id, n] of Object.entries(delta.progress)) {
        fixture.record.progress[id] = Math.max(fixture.record.progress[id] ?? 0, n);
      }
      // KL-164: "one more" steps add up, once per request id.
      if (delta.requestId !== undefined && !fixture.applied.includes(delta.requestId)) {
        for (const [id, n] of Object.entries(delta.increments ?? {})) {
          fixture.record.progress[id] = (fixture.record.progress[id] ?? 0) + n;
        }
        fixture.applied.push(delta.requestId);
      }
      body = record();
    } else if (path === "/api/bus/discover" && method === "POST") {
      body = { interfaces: [] };
    } else if (path === "/api/project" && method === "GET") {
      await fulfillNoProjectOpen(route);
      return;
    } else if (method === "GET" && ["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items"].includes(path)) {
      body = [];
    } else {
      fixture.unexpected.push(`${method} ${path}`);
      await route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"local fixture only"}' });
      return;
    }
    await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
  });
}

function fresh(settings: Record<string, unknown> = {}): Fixture {
  // The first-run guide is marked seen so it does not cover the workbench.
  return {
    settings: { onboardingGuide: { seenStage: "alpha", version: VERSION }, ...settings },
    record: { unlocked: {}, progress: {} },
    posts: [],
    applied: [],
    unexpected: [],
  };
}

const KONAMI = ["ArrowUp", "ArrowUp", "ArrowDown", "ArrowDown", "ArrowLeft", "ArrowRight", "ArrowLeft", "ArrowRight", "b", "a"];

async function konami(page: Page) {
  await page.locator(".workbench-panel-controls").click({ position: { x: 2, y: 2 } });
  for (const key of KONAMI) await page.keyboard.press(key);
}

test("the Konami code unlocks a popup, the record is saved, and the palette opens the overview", { tag: "@exclusive" }, async ({ page }) => {
  const fixture = fresh();
  await serve(page, fixture);
  await page.goto("/");
  await expect(page.getByRole("button", { name: "New project…" }).first()).toBeVisible();

  await konami(page);
  const popup = page.locator(".toast--achievement");
  await expect(popup).toContainText("Achievement unlocked");
  await expect(popup).toContainText("↑↑↓↓←→←→BA");
  await expect(popup.locator(".achievement-badge--legendary")).toBeVisible();
  await expect.poll(() => Object.keys(fixture.record.unlocked)).toEqual(["konami"]);

  // Enter in the palette must not also press the dialog's first button.
  await page.keyboard.press("Control+Shift+P");
  await page.keyboard.type("Achievements");
  await page.keyboard.press("Enter");
  const dialog = page.getByRole("dialog", { name: "Achievements" });
  await expect(dialog).toBeVisible();
  await page.waitForTimeout(300);
  await expect(dialog).toBeVisible();
  await expect(dialog).toContainText(`1 of ${ACHIEVEMENTS.length} unlocked`);
  await expect(dialog.locator('[data-achievement="konami"]')).toContainText("Unlocked");
  await expect(dialog.locator('[data-achievement="night-shift"]')).toContainText("Hidden achievement");
  // The palette run counted towards its own achievement.
  await expect.poll(() => fixture.record.progress["palette-pro"]).toBe(1);
  // The list, not the dialog, scrolls: the unlocked entry stays in view.
  expect(await dialog.locator(".achievements-list").evaluate((el) => el.scrollTop)).toBe(0);

  await dialog.getByRole("button", { name: "Close" }).click();
  await expect(dialog).toBeHidden();
  expect(fixture.unexpected).toEqual([]);
});

test("switched off, nothing is counted, nothing pops up and the File menu has no entry", { tag: "@exclusive" }, async ({ page }) => {
  const fixture = fresh({ achievementsEnabled: "false" });
  await serve(page, fixture);
  await page.goto("/");
  await expect(page.getByRole("button", { name: "New project…" }).first()).toBeVisible();

  await konami(page);
  await page.waitForTimeout(400);
  await expect(page.locator(".toast--achievement")).toHaveCount(0);
  expect(fixture.posts).toEqual([]);

  await page.locator(".file-menu summary").click();
  await openFileSubmenu(page, "help");
  await expect(page.locator(".file-menu-content").getByRole("menuitem", { name: "Show introduction…" })).toBeVisible();
  await expect(page.locator(".file-menu-content").getByRole("menuitem", { name: "Achievements…" })).toHaveCount(0);
  expect(fixture.unexpected).toEqual([]);
});

/** Dismisses the popup and measures, in the page, how long it took from
 * `toast--leaving` to removal, and which animation was running. */
async function dismissAndMeasure(page: Page) {
  const popup = page.locator(".toast--achievement");
  await expect(popup).toBeVisible();
  const measured = popup.evaluate((el) => new Promise<{ animation: string; opacity: string; ms: number }>((done) => {
    let start = 0;
    let animation = "";
    let opacity = "";
    new MutationObserver((_, observer) => {
      if (!start && el.classList.contains("toast--leaving")) {
        start = performance.now();
        animation = getComputedStyle(el).animationName;
        opacity = getComputedStyle(el).opacity;
      }
      if (start && !el.isConnected) {
        observer.disconnect();
        done({ animation, opacity, ms: performance.now() - start });
      }
    }).observe(document.body, { subtree: true, childList: true, attributes: true, attributeFilter: ["class"] });
  }));
  await popup.getByRole("button", { name: "Dismiss" }).click();
  return measured;
}

test("a dismissed popup slides out and is removed when its exit animation ends", { tag: "@exclusive" }, async ({ page }) => {
  const fixture = fresh();
  await serve(page, fixture);
  await page.goto("/");
  await expect(page.getByRole("button", { name: "New project…" }).first()).toBeVisible();
  await konami(page);
  const result = await dismissAndMeasure(page);
  expect(result.animation).toBe("knx-achievement-out");
  // Removed by `animationend`, well before the one-second fallback.
  expect(result.ms).toBeLessThan(800);
  await expect(page.locator(".toast--achievement")).toHaveCount(0);
});

test("with reduced motion the popup vanishes at once and the fallback removes it", { tag: "@exclusive" }, async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  const fixture = fresh();
  await serve(page, fixture);
  await page.goto("/");
  await expect(page.getByRole("button", { name: "New project…" }).first()).toBeVisible();
  await konami(page);
  const result = await dismissAndMeasure(page);
  expect(result.animation).toBe("none");
  expect(result.opacity).toBe("0");
  expect(result.ms).toBeGreaterThanOrEqual(900);
});


for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} ${width}: additive achievement persists once and retains old unlocks`, async ({ page }) => {
    const m = language === "de" ? de : en;
    await page.setViewportSize({ width, height: 900 });
    await page.emulateMedia({ reducedMotion: width === 400 ? "reduce" : "no-preference" });
    const fixture = fresh({ uiLanguage: language, theme: width === 400 ? "porcelain" : "graphite",
      motionToasts: "off", motionAchievements: "subtle" });
    const oldAt = "2026-10-01T10:00:00Z";
    fixture.record.unlocked = { konami: oldAt, "palette-pro": oldAt, "bus-master": oldAt };
    fixture.record.progress = { "palette-pro": 50, "palette-resident": 99 };
    await serve(page, fixture); await page.goto("/");
    await expect(page.locator("html")).toHaveAttribute("lang", language);
    await expect(page.locator("html")).toHaveAttribute("data-theme", width === 400 ? "porcelain" : "graphite");
    async function overview() {
      await expect(page.locator(".workbench-toolbar")).toBeVisible();
      await expect(page.locator("html")).toHaveAttribute("lang", language);
      await expect(page.locator("html")).toHaveAttribute("data-theme", width === 400 ? "porcelain" : "graphite");
      await page.keyboard.press("Control+Shift+P");
      await page.getByRole("combobox").fill(m["achievements.command"]);
      await page.getByRole("option").filter({ hasText: m["achievements.command"] }).click();
    }
    await overview();
    const dialog = page.locator(".achievements-dialog");
    await expect(dialog).toBeVisible();
    const popup = page.locator(".toast--achievement");
    await expect(popup).toHaveCount(1);
    await expect(popup).toContainText(m["achievement.palette-resident.title"]);
    await expect.poll(() => fixture.record.progress["palette-resident"]).toBe(100);
    await expect.poll(() => fixture.record.unlocked["palette-resident"]).toBeTruthy();
    expect(fixture.record.unlocked.konami).toBe(oldAt);
    expect(fixture.record.unlocked["bus-master"]).toBe(oldAt);
    for (const id of ["save-point-regular", "palette-resident", "room-atlas", "address-librarian"] as const) {
      await expect(dialog.locator(`[data-achievement="${id}"]`)).toContainText(m[`achievement.${id}.description`]);
    }
    const resident = dialog.locator('[data-achievement="palette-resident"]');
    await resident.scrollIntoViewIfNeeded();
    if (width === 400) {
      const text = await resident.locator(".achievement-item-text").boundingBox();
      const meta = await resident.locator(".achievement-item-meta").boundingBox();
      expect.soft(text!.width, "narrow description retains a readable column").toBeGreaterThanOrEqual(180);
      expect.soft(meta!.y, "date/tier move below description on narrow screens").toBeGreaterThanOrEqual(text!.y + text!.height - 1);
    }
    const closeButton = dialog.getByRole("button", { name: m["achievements.close"], exact: true });
    const closeHit = await closeButton.evaluate(button => {
      const box = button.getBoundingClientRect();
      return button.contains(document.elementFromPoint(box.x + box.width / 2, box.y + box.height / 2));
    });
    expect.soft(closeHit, "achievement popup does not cover overview Close").toBe(true);
    const layers = await dialog.evaluate(panel => ({
      overview: Number(getComputedStyle(panel.parentElement!).zIndex),
      popup: Number(getComputedStyle(document.querySelector(".toast-stack")!).zIndex),
    }));
    expect.soft(layers.overview, "overview stays above its unlock popup").toBeGreaterThan(layers.popup);
    if (process.env.UX_FEEDBACK_SHOTS) {
      await dialog.locator('[data-achievement="palette-resident"]').scrollIntoViewIfNeeded();
      await page.screenshot({ path: `${process.env.UX_FEEDBACK_SHOTS}/achievements-${language}-${width}.png` });
    }
    await dialog.getByRole("button", { name: m["achievements.close"], exact: true }).click();
    await popup.getByRole("button", { name: m["toast.dismiss"], exact: true }).click();
    await expect(popup).toHaveCount(0);
    await page.reload(); await overview();
    await expect(dialog.locator('[data-achievement="palette-resident"]')).toHaveClass(/achievement-item--unlocked/);
    await expect(popup).toHaveCount(0);
    expect(fixture.record.progress["palette-resident"]).toBe(100);
    expect(fixture.record.unlocked.konami).toBe(oldAt);
    expect(fixture.unexpected).toEqual([]);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth)).toBeLessThanOrEqual(1);
  });
}
