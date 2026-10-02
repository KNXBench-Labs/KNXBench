/** Checks monitor control evidence at narrow/wide widths against a local mocked API only. */
import { test, expect } from "@playwright/test";

const frames = [
  { priority: "urgent", repeated: true, hopCount: 0 },
  { priority: "system", repeated: false, hopCount: 7 },
  { priority: "normal", repeated: null, hopCount: 6 },
  null,
];
const telegrams = frames.map((control, index) => ({
  seq: index + 1,
  timestamp: "2026-09-30T12:00:00Z",
  source: "1.1.5",
  destination: "1/2/3",
  destinationName: null,
  service: control ? "GroupValueWrite" : "SessionClosed",
  rawPayload: null,
  decoded: null,
  control,
}));

for (const language of ["en", "de"] as const) {
  for (const width of [360, 1440]) {
    test(`${language} monitor shows control evidence at ${width}px without contacting KNX`, async ({ page }) => {
      await page.setViewportSize({ width, height: 800 });
      const unmocked: string[] = [];
      await page.route("**/api/**", (route) => {
        const path = new URL(route.request().url()).pathname;
        if (path === "/api/bus/monitor/telegrams") return route.fulfill({
          contentType: "application/json",
          body: JSON.stringify({ sessionId: 1, serverIncarnation: "fixture", contextStatus: "current", projectOpen: false, status: "active", nextSince: 5, droppedBefore: 0, telegrams }),
        });
        if (path === "/api/bus/discover") return route.fulfill({
          contentType: "application/json", body: JSON.stringify({ interfaces: [] }),
        });
        unmocked.push(`${route.request().method()} ${path}`);
        return route.fulfill({ status: 404, contentType: "application/json", body: JSON.stringify({ error: "local fixture only" }) });
      });
      await page.goto(`/e2e/monitor-control-fixture.html?lang=${language}`);
      await expect(page.locator("html")).toHaveAttribute("lang", language);
      const rows = page.locator(".bus-monitor-table tbody tr");
      await expect(rows).toHaveCount(4);
      await expect(page.locator(".bus-monitor-table thead th").filter({ hasText: language === "de" ? "Steuerfeld" : "Control" })).toHaveCount(1);
      const first = rows.nth(0).locator(".bus-monitor-control");
      await expect(first).toContainText(language === "de" ? "Dringend" : "Urgent");
      await expect(first).toContainText(language === "de" ? "Hop-Zähler: 0" : "Hop count: 0");
      await expect(first).toContainText(language === "de" ? "Wiederholt" : "Repeated");
      await expect(rows.nth(1).locator(".bus-monitor-control-repeat")).toHaveText(language === "de" ? "Nicht wiederholt" : "Not repeated");
      await expect(rows.nth(2).locator(".bus-monitor-control-repeat")).toHaveCount(0);
      await expect(rows.nth(3).locator(".bus-monitor-control")).toHaveText("—");
      await rows.nth(2).focus();
      await page.keyboard.press("Enter");
      await expect(page.locator(".telegram-details .bus-monitor-control")).toContainText(language === "de" ? "Hop-Zähler: 6" : "Hop count: 6");
      await expect(page.locator(".telegram-details .bus-monitor-control-repeat")).toHaveCount(0);
      const layout = await page.evaluate(() => ({ viewport: innerWidth, document: document.documentElement.scrollWidth }));
      expect(layout.document, JSON.stringify(layout)).toBeLessThanOrEqual(layout.viewport);
      expect(unmocked).toEqual([]);
    });
  }
}

for (const language of ["en", "de"] as const) {
  test(`${language} monitor checks authoritative context while paused with no browser records`, async ({page}) => {
    let contextStatus = "current";
    let contextChecks = 0;
    const unexpected: string[] = [];
    await page.route("**/api/**", route => {
      const url = new URL(route.request().url());
      if (url.pathname === "/api/bus/monitor/telegrams") {
        const contextOnly = url.searchParams.get("contextOnly") === "true";
        if (contextOnly) contextChecks++;
        return route.fulfill({contentType: "application/json", body: JSON.stringify({
          sessionId: 17, serverIncarnation: "fixture-other-client", contextStatus,
          projectOpen: false, status: "active", nextSince: contextOnly ? Number(url.searchParams.get("since")) : 5,
          droppedBefore: 0, telegrams: contextOnly ? [] : telegrams,
        })});
      }
      if (url.pathname === "/api/bus/discover") return route.fulfill({contentType: "application/json", body: JSON.stringify({interfaces: []})});
      unexpected.push(`${route.request().method()} ${url.pathname}`);
      return route.fulfill({status: 404, contentType: "application/json", body: JSON.stringify({error: "fixture only"})});
    });
    await page.goto(`/e2e/monitor-control-fixture.html?lang=${language}`);
    await expect(page.locator(".bus-monitor-table tbody tr")).toHaveCount(4);
    await expect(page.locator(".bus-compose-value")).toBeEnabled();
    await expect(page.locator(".bus-monitor-unverified-lock")).toHaveCount(0);
    await page.locator(".bus-monitor-pause").click();
    contextStatus = "stale";
    await expect(page.locator(".bus-monitor-stale-lock")).toBeVisible();
    await expect(page.locator(".bus-compose-value")).toBeDisabled();
    expect(contextChecks).toBeGreaterThan(0);
    await expect(page.locator(".bus-monitor-table tbody tr")).toHaveCount(4);
    contextStatus = "unavailable";
    await expect(page.locator(".bus-monitor-unverified-lock")).toBeVisible();
    await expect(page.locator(".bus-compose-value")).toBeDisabled();
    contextStatus = "current";
    await expect(page.locator(".bus-compose-value")).toBeEnabled();
    await expect(page.locator(".bus-monitor-unverified-lock")).toHaveCount(0);
    expect(unexpected).toEqual([]);
  });
}
