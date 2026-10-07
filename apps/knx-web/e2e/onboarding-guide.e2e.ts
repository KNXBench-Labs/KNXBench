/** Verifies the first-run guide in Chromium: once per stage, remembered, and its buttons real. */

import { expect, test, type Page } from "@playwright/test";

interface Fixture {
  settings: Record<string, unknown>;
  puts: Record<string, unknown>[];
  versionRequests: number;
  unexpected: string[];
}

// Every request is answered locally: no server, no gateway, no network.
async function serve(page: Page, fixture: Fixture, version = "0.1.0-alpha.2+gfixture") {
  await page.route("**/api/**", async (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    const method = request.method();
    let body: unknown;
    if (path === "/api/auth/status" && method === "GET") {
      body = { required: false, authenticated: true };
    } else if (path === "/api/settings" && method === "GET") {
      body = { schemaVersion: 1, conditionalPatchVersion: 1, status: "ok", settings: fixture.settings };
    } else if (path === "/api/settings" && method === "PUT") {
      const patch = (request.postDataJSON() as { settings: Record<string, unknown> }).settings;
      fixture.puts.push(patch);
      fixture.settings = { ...fixture.settings, ...patch };
      body = { schemaVersion: 1, conditionalPatchVersion: 1, status: "ok", settings: fixture.settings };
    } else if (path === "/api/version" && method === "GET") {
      fixture.versionRequests += 1;
      body = { version };
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
  return { settings, puts: [], versionRequests: 0, unexpected: [] };
}

test("opens on a first alpha start, is remembered on Escape, and stays closed after a reload", async ({ page }) => {
  const fixture = fresh();
  await serve(page, fixture);
  await page.goto("/");

  const guide = page.getByRole("dialog", { name: "Welcome to KNXBench" });
  await expect(guide).toBeVisible();
  await expect(guide.getByText("Step 1 of 4")).toBeVisible();
  await expect(guide.locator(".onboarding-stage")).toHaveText("Alpha");
  await expect(guide.getByRole("button", { name: "Next" })).toBeFocused();

  // Both language buttons must be legible, the chosen one included: text and
  // background may not resolve to the same colour.
  for (const name of ["English", "Deutsch"]) {
    const colours = await guide.getByRole("button", { name }).evaluate((element) => {
      const style = getComputedStyle(element);
      return { text: style.color, background: style.backgroundColor, image: style.backgroundImage };
    });
    // A solid fill, so the comparison below sees the colour actually drawn
    // (the default button paints a gradient over a transparent colour).
    expect(colours.image, name).toBe("none");
    expect(colours.text, name).not.toBe(colours.background);
  }
  await expect(guide.getByRole("button", { name: "English" })).toHaveAttribute("aria-pressed", "true");
  // The card is as tall as its page, not stretched to the overlay's cap.
  const { height, cap } = await guide.evaluate((element) => ({
    height: element.getBoundingClientRect().height,
    cap: Number.parseFloat(getComputedStyle(element).maxHeight),
  }));
  expect(height).toBeLessThan(cap - 40);

  await page.keyboard.press("Escape");
  await expect(guide).toBeHidden();
  await expect.poll(() => fixture.puts).toContainEqual({
    onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.2+gfixture" },
  });

  await page.reload();
  await expect(page.getByRole("button", { name: "New project…" }).first()).toBeVisible();
  await expect.poll(() => fixture.versionRequests).toBe(2);
  // The decision has been taken by now; give a stray render a moment anyway.
  await page.waitForTimeout(300);
  await expect(page.getByRole("dialog", { name: "Welcome to KNXBench" })).toHaveCount(0);
  expect(fixture.unexpected).toEqual([]);
});

test("opens again when the build moves from alpha to beta", async ({ page }) => {
  const fixture = fresh({ onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.2" } });
  await serve(page, fixture, "0.2.0-beta.1");
  await page.goto("/");
  const guide = page.getByRole("dialog", { name: "Welcome to KNXBench" });
  await expect(guide).toBeVisible();
  await expect(guide.locator(".onboarding-stage")).toHaveText("Beta");
});

test("a task button closes the guide and opens the product catalog", async ({ page }) => {
  const fixture = fresh();
  await serve(page, fixture);
  await page.goto("/");
  const guide = page.getByRole("dialog", { name: "Welcome to KNXBench" });
  await guide.getByRole("button", { name: "Next" }).click();
  await expect(guide.getByRole("heading", { name: "What works, and what does not yet" })).toBeVisible();
  await guide.getByRole("button", { name: "Next" }).click();
  await guide.getByRole("button", { name: "Add product data", exact: true }).click();

  await expect(guide).toBeHidden();
  await expect(page.locator(".workbench-center .catalog-workspace")).toBeVisible();
  await expect.poll(() => fixture.puts.some((patch) => "onboardingGuide" in patch)).toBe(true);
});

test("switches to German from the first page and reopens from the File menu", async ({ page }) => {
  const fixture = fresh();
  await serve(page, fixture);
  await page.goto("/");
  const guide = page.getByRole("dialog", { name: "Welcome to KNXBench" });
  await guide.getByRole("button", { name: "Deutsch" }).click();
  const german = page.getByRole("dialog", { name: "Willkommen bei KNXBench" });
  await expect(german.getByText("Schritt 1 von 4")).toBeVisible();
  await expect.poll(() => fixture.puts).toContainEqual({ uiLanguage: "de" });

  await german.getByRole("button", { name: "Einführung überspringen" }).click();
  await expect(german).toBeHidden();
  await page.getByText("Datei").click();
  await page.getByRole("button", { name: "Einführung anzeigen…" }).click();
  await expect(page.getByRole("dialog", { name: "Willkommen bei KNXBench" })).toBeVisible();
});
