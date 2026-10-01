/** EN/DE local browser gate: K6 refusal is visible before any consent or write. */
import { expect, test } from "@playwright/test";

for (const language of ["en", "de"] as const) {
  for (const width of [360, 1440]) {
    test(`${language} at ${width}px shows recovery refusal without consent or tunnel`, async ({ page }) => {
      await page.setViewportSize({ width, height: 840 });
      const unmocked: string[] = [];
      const calls: string[] = [];
      await page.route("**/api/**", (route) => {
        const url = new URL(route.request().url());
        const signature = `${route.request().method()} ${url.pathname}`;
        calls.push(signature);
        if (signature === "GET /api/device-address/availability") {
          return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify({
            startAvailable: false, reason: "no verified durable pre-write backup",
          }) });
        }
        if (signature === "GET /api/device-address/status") {
          return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"no session"}' });
        }
        unmocked.push(signature);
        return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"local fixture only"}' });
      });
      await page.goto(`/e2e/address-unavailable-fixture.html?lang=${language}`);
      await expect(page.locator("html")).toHaveAttribute("lang", language);
      const warning = page.locator('[role="status"]').filter({ hasText: "no verified durable pre-write backup" });
      await expect(warning).toBeVisible();
      await expect(warning).toContainText(language === "de" ? "gesperrt" : "unavailable");
      await page.locator('input[name="address"]').fill("1.1.30");
      await page.locator('input[name="gateway"]').fill("192.0.2.10:3671");
      await expect(page.locator(".address-programming-plan .primary-action")).toBeDisabled();
      await expect(page.locator(".programming-consent-dialog")).toHaveCount(0);
      expect(calls.sort()).toEqual(["GET /api/device-address/availability", "GET /api/device-address/status"]);
      expect(unmocked).toEqual([]);
      const layout = await page.evaluate(() => ({ viewport: innerWidth, document: document.documentElement.scrollWidth }));
      expect(layout.document, JSON.stringify(layout)).toBeLessThanOrEqual(layout.viewport);
    });
  }
}
