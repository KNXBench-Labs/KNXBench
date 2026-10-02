/** Verifies browser paint/rejection and System/built-in restoration without any API path. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { expect, test, type Page } from "@playwright/test";
import { themePackFixture } from "../src/themePackFixtures";

async function fixture(page: Page, kind: "valid" | "unsafe" | "future" = "valid") {
  const pack = themePackFixture(); pack.name = '<img src="https://example.invalid/name" onerror="alert(1)">';
  const stored = kind === "future" ? { ...pack, tokenVersion: 2 }
    : kind === "unsafe" ? { ...pack, tokens: { ...pack.tokens, "--knx-backdrop-image": "url(https://example.invalid/asset)" } } : pack;
  const unexpected: string[] = [], errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.route("**/*", async (route) => {
    const url = new URL(route.request().url());
    if (url.origin !== "http://127.0.0.1:4173" || url.pathname.startsWith("/api/")) {
      unexpected.push(url.pathname); await route.abort();
    } else await route.continue();
  });
  await page.addInitScript(({ id, stored }) => {
    localStorage.setItem("knx-desktop:settings-cache", JSON.stringify({ schemaVersion: 1, settings: {
      theme: id, uiThemePacks: { [id]: stored }, accent: "violet", density: "compact", motionLevel: "off", motionStyle: "apple",
    } }));
  }, { id: pack.id, stored });
  await page.goto("/e2e/theme-pack-fixture.html");
  await expect(page.getByRole("heading", { name: "Offline theme runtime" })).toBeVisible();
  return { pack, stored, check: () => { expect(unexpected).toEqual([]); expect(errors).toEqual([]); } };
}

test("a validated cached palette paints, metadata stays text, and other preferences remain independent", async ({ page }) => {
  const { pack, check } = await fixture(page);
  await expect(page.locator("html")).toHaveAttribute("data-theme", pack.id);
  await expect(page.locator("html")).toHaveCSS("--knx-bg", pack.tokens["--knx-bg"]);
  await expect(page.getByRole("button", { name: pack.name, exact: true })).toBeVisible();
  await expect(page.locator("img")).toHaveCount(0);
  await expect(page.locator("html")).toHaveAttribute("data-motion-level", "off");
  await expect(page.locator("html")).toHaveAttribute("data-density", "compact");
  await page.emulateMedia({ colorScheme: "dark" });
  await expect(page.locator("html")).toHaveAttribute("data-theme", pack.id);
  check();
});
for (const id of ["porcelain", "graphite", "cupertino", "neon-grid", "bitcoin-defi", "system"]) {
  test(`switching imported → ${id} clears the bounded overrides`, async ({ page }) => {
    const { pack, check } = await fixture(page);
    await expect(page.locator("html")).toHaveAttribute("data-theme", pack.id);
    await page.locator(`[data-choice="${id}"]`).click();
    await expect(page.getByTestId("selected")).toHaveText(id);
    expect(await page.locator("html").evaluate((element, keys) => keys.every((key) => (element as HTMLElement).style.getPropertyValue(key) === ""), Object.keys(pack.tokens))).toBe(true);
    await expect(page.locator("html")).toHaveAttribute("data-density", "compact");
    await expect(page.locator("html")).toHaveAttribute("data-motion-level", "off");
    if (id === "system") {
      await page.emulateMedia({ colorScheme: "dark" }); await expect(page.locator("html")).toHaveAttribute("data-theme", "graphite");
      await page.emulateMedia({ colorScheme: "light" }); await expect(page.locator("html")).toHaveAttribute("data-theme", "porcelain");
    } else await expect(page.locator("html")).toHaveAttribute("data-theme", id);
    check();
  });
}
for (const kind of ["unsafe", "future"] as const) {
  test(`cached ${kind} data is retained, diagnosed and never painted or requested`, async ({ page }) => {
    const { pack, stored, check } = await fixture(page, kind);
    await expect(page.getByTestId("selected")).toHaveText("system");
    await expect(page.getByTestId("diagnostics")).toContainText(kind === "unsafe" ? "invalidValue" : "unsupportedVersion");
    const cache = await page.evaluate(() => JSON.parse(localStorage.getItem("knx-desktop:settings-cache")!).settings);
    expect(cache.theme).toBe(pack.id); expect(cache.uiThemePacks[pack.id]).toEqual(stored);
    expect(await page.locator("html").evaluate((element) => (element as HTMLElement).style.getPropertyValue("--knx-backdrop-image"))).toBe("");
    await page.emulateMedia({ colorScheme: "dark" }); await expect(page.locator("html")).toHaveAttribute("data-theme", "graphite");
    check();
  });
}
