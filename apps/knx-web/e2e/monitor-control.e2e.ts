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
          body: JSON.stringify({ sessionId: 1, serverIncarnation: "fixture", status: "active", nextSince: 5, droppedBefore: 0, telegrams }),
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
