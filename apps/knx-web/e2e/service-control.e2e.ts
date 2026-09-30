/** EN/DE responsive Debug gate: all API calls intercepted locally, no KNX device. */
import { expect, test } from "@playwright/test";

const address = "1.1.67";
const gateway = "192.0.2.10:3671";
const phrase = "I confirm individual-address write enable to 1.1.67";
const reading = { address, raw: "0000", mask: "0701", individualAddressWriteEnabled: false };

for (const language of ["en", "de"] as const) {
  for (const width of [360, 1440]) {
    test(`${language} service control at ${width}px needs saved opt-in, a read and a typed write phrase`, async ({ page }) => {
      await page.setViewportSize({ width, height: 840 });
      const settings: Record<string, unknown> = { uiLanguage: language, debugIndividualAddressWriteEnable: false };
      const reads: string[] = [];
      const writes: unknown[] = [];
      const unmocked: string[] = [];
      await page.route("**/api/**", (route) => {
        const url = new URL(route.request().url());
        const method = route.request().method();
        const json = (body: unknown, status = 200) => route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
        if (url.pathname === "/api/settings" && method === "GET") {
          return json({ schemaVersion: 1, settings, status: "ok" });
        }
        if (url.pathname === "/api/settings" && method === "PUT") {
          Object.assign(settings, route.request().postDataJSON().settings);
          return json({ schemaVersion: 1, settings, status: "ok" });
        }
        if (url.pathname === "/api/device/service-control" && method === "GET") {
          reads.push(url.search);
          if (settings.debugIndividualAddressWriteEnable !== true) return json({ error: "debug action is off" }, 403);
          return json(reading);
        }
        if (url.pathname === "/api/device/service-control" && method === "POST") {
          const body = route.request().postDataJSON();
          writes.push(body);
          if (settings.debugIndividualAddressWriteEnable !== true || body.confirmation !== phrase) return json({ error: "not authorised" }, 403);
          return json({ before: reading, after: "0004", individualAddressWriteEnabled: true, written: true, backupPath: "[LOCAL MOCK BACKUP]" });
        }
        unmocked.push(`${method} ${url.pathname}`);
        return json({ error: "local fixture only" }, 404);
      });
      await page.goto(`/e2e/service-control-fixture.html?lang=${language}`);
      await expect(page.locator("html")).toHaveAttribute("lang", language);
      const optIn = page.locator('.settings-section-debug input[type="checkbox"]');
      await expect(optIn).toBeEnabled();
      await expect(optIn).not.toBeChecked();
      expect(reads).toHaveLength(0);
      expect(writes).toHaveLength(0);
      await page.locator('input[name="address"]').fill(address);
      await page.locator('input[name="gateway"]').fill(gateway);
      await page.locator(".service-control-read").click();
      await expect(page.locator('[role="alert"]')).toContainText("debug action is off");
      expect(reads).toHaveLength(1);
      expect(writes).toHaveLength(0);
      await optIn.click();
      await expect(optIn).toBeChecked();
      await page.locator(".service-control-read").click();
      await expect(page.locator(".service-control-observation")).toContainText("0000");
      expect(writes).toHaveLength(0);
      await page.locator(".service-control-review").click();
      expect(writes).toHaveLength(0);
      await page.locator('input[name="confirmation"]').fill("I confirm download to 1.1.67");
      await expect(page.locator(".service-control-write")).toBeDisabled();
      await page.locator('input[name="confirmation"]').fill(phrase);
      await page.locator(".service-control-write").click();
      await expect(page.locator(".service-control-result")).toContainText("[LOCAL MOCK BACKUP]");
      expect(writes).toEqual([{ address, gateway, confirmation: phrase, enable: true }]);
      expect(unmocked).toEqual([]);
      const layout = await page.evaluate(() => ({ viewport: innerWidth, document: document.documentElement.scrollWidth }));
      expect(layout.document, JSON.stringify(layout)).toBeLessThanOrEqual(layout.viewport);
    });
  }
}
