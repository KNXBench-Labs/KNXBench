/** Verifies achievements in Chromium: popup, overview through the palette, and off means off. */

import { expect, test, type Page } from "@playwright/test";

interface Fixture {
  settings: Record<string, unknown>;
  record: { unlocked: Record<string, string>; progress: Record<string, number> };
  posts: unknown[];
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
      const delta = request.postDataJSON() as Fixture["record"];
      fixture.posts.push(delta);
      for (const [id, at] of Object.entries(delta.unlocked)) fixture.record.unlocked[id] ??= at;
      for (const [id, n] of Object.entries(delta.progress)) {
        fixture.record.progress[id] = Math.max(fixture.record.progress[id] ?? 0, n);
      }
      body = record();
    } else if (path === "/api/bus/discover" && method === "POST") {
      body = { interfaces: [] };
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
    unexpected: [],
  };
}

const KONAMI = ["ArrowUp", "ArrowUp", "ArrowDown", "ArrowDown", "ArrowLeft", "ArrowRight", "ArrowLeft", "ArrowRight", "b", "a"];

async function konami(page: Page) {
  await page.locator(".workbench-panel-controls").click({ position: { x: 2, y: 2 } });
  for (const key of KONAMI) await page.keyboard.press(key);
}

test("the Konami code unlocks a popup, the record is saved, and the palette opens the overview", async ({ page }) => {
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
  await expect(dialog).toContainText("1 of 11 unlocked");
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

test("switched off, nothing is counted, nothing pops up and the File menu has no entry", async ({ page }) => {
  const fixture = fresh({ achievementsEnabled: "false" });
  await serve(page, fixture);
  await page.goto("/");
  await expect(page.getByRole("button", { name: "New project…" }).first()).toBeVisible();

  await konami(page);
  await page.waitForTimeout(400);
  await expect(page.locator(".toast--achievement")).toHaveCount(0);
  expect(fixture.posts).toEqual([]);

  await page.locator(".file-menu summary").click();
  await expect(page.locator(".file-menu-content").getByRole("button", { name: "Show introduction…" })).toBeVisible();
  await expect(page.locator(".file-menu-content").getByRole("button", { name: "Achievements…" })).toHaveCount(0);
  expect(fixture.unexpected).toEqual([]);
});
