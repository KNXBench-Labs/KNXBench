/** Responsive EN/DE checks with a local mock API; compare is never sent to hardware. */
import { expect, test } from "@playwright/test";

const readiness = {
  counts: { verified: 1, unsupported: 1, "no-address": 1 },
  devices: [
    { address: "1.1.67", name: "Push button", programRef: "M-0083_A-0027", readiness: "verified", category: null, detail: "One tested device; 2026-09-29", steps: 11, octets: 20 },
    { address: "1.1.68", name: "No program", programRef: "", readiness: "unsupported", category: "configuration", detail: "cannot prepare plan", steps: null, octets: null },
    { address: null, name: "Not addressed", programRef: "M-X", readiness: "no-address", category: null, detail: null, steps: null, octets: null },
  ],
};
const comparison = {
  address: "1.1.67", deviceName: "Push button", programId: "M-0083_A-0027", written: false, partial: false,
  mask: 0x0701, manufacturer: 0x0083, loadStates: [{ machine: "GroupAddressTable", state: "Loaded" }],
  octets: 20, differingOctets: 2, same: false,
  changes: [{ address: 0x4000, segment: "seg-4", device: [0, 255], project: [1, 2] }],
};

for (const language of ["en", "de"] as const) {
  for (const width of [360, 1440]) {
    test(`${language} device checks at ${width}px remain offline until a confirmed mocked read`, async ({ page }) => {
      await page.setViewportSize({ width, height: 800 });
      const comparisons: unknown[] = [];
      const unmocked: string[] = [];
      await page.route("**/api/**", (route) => {
        const path = new URL(route.request().url()).pathname;
        const method = route.request().method();
        if (path === "/api/device-readiness" && method === "GET") return route.fulfill({
          contentType: "application/json", body: JSON.stringify(readiness),
        });
        if (path === "/api/device-compare" && method === "POST") {
          comparisons.push(route.request().postDataJSON());
          return route.fulfill({ contentType: "application/json", body: JSON.stringify(comparison) });
        }
        unmocked.push(`${method} ${path}`);
        return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"local fixture only"}' });
      });
      await page.goto(`/e2e/device-checks-fixture.html?lang=${language}`);
      await expect(page.locator("html")).toHaveAttribute("lang", language);
      await expect(page.locator(".device-checks-readiness tbody tr")).toHaveCount(3);
      await expect(page.locator(".device-checks-readiness")).toContainText("cannot prepare plan");
      expect(comparisons).toHaveLength(0);
      await page.locator(".device-checks-target").selectOption("1.1.67");
      await page.locator(".device-checks-gateway").fill("192.0.2.10:3671");
      await page.getByRole("button", { name: language === "de" ? "Schreibgeschützten Vergleich prüfen" : "Review read-only comparison" }).click();
      expect(comparisons).toHaveLength(0);
      await expect(page.locator(".device-checks-confirm")).toContainText("1.1.67");
      await page.getByRole("button", { name: language === "de" ? "Gerät jetzt lesen" : "Read device now" }).click();
      await expect(page.locator(".device-checks-result")).toContainText("00 FF");
      await expect(page.locator(".device-checks-result")).toContainText("01 02");
      await expect(page.locator(".device-checks-result")).toContainText("GroupAddressTable");
      expect(comparisons).toEqual([{ address: "1.1.67", gateway: "192.0.2.10:3671" }]);
      expect(unmocked).toEqual([]);
      const layout = await page.evaluate(() => ({ viewport: innerWidth, document: document.documentElement.scrollWidth }));
      expect(layout.document, JSON.stringify(layout)).toBeLessThanOrEqual(layout.viewport);
    });
  }
}
