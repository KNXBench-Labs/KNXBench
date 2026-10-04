/** MODEL-03: device number 0 goes to the server, which alone decides about couplers. */
import { test, expect } from "@playwright/test";

const refusal = "address 1.1.0 ends in 0, reserved for couplers; device 42 cannot be assigned "
  + "a new coupler address without device classification";

for (const language of ["en", "de"] as const) {
  test(`${language} device editor submits device number 0 and keeps an accepted coupler address`, async ({ page }) => {
    const writes: unknown[] = [];
    const unexpected: string[] = [];
    await page.route("**/api/**", (route) => {
      const path = new URL(route.request().url()).pathname;
      if (path === "/api/individual-address") {
        writes.push(route.request().postDataJSON());
        return route.fulfill({ contentType: "application/json", body: "{}" });
      }
      if (path.includes("/parameters")) {
        return route.fulfill({
          contentType: "application/json",
          body: JSON.stringify({ programId: null, sections: [], stale: [], diagnostics: [] }),
        });
      }
      unexpected.push(`${route.request().method()} ${path}`);
      return route.fulfill({ status: 404, contentType: "application/json", body: JSON.stringify({ error: "fixture only" }) });
    });
    await page.goto(`/e2e/device-editor-fixture.html?lang=${language}`);
    const input = page.locator(".individual-address-field input");
    await expect(input).toHaveValue("12");
    await input.fill("0");
    await input.press("Tab");
    await expect.poll(() => writes.length).toBe(1);
    expect(writes).toEqual([{ deviceId: 42, address: "1.1.0" }]);
    await expect(page.locator(".individual-address-field .field-error")).toHaveCount(0);
    expect(unexpected).toEqual([]);
  });

  test(`${language} device editor shows the server's refusal for device number 0 and restores the field`, async ({ page }) => {
    const writes: unknown[] = [];
    const unexpected: string[] = [];
    await page.route("**/api/**", (route) => {
      const path = new URL(route.request().url()).pathname;
      if (path === "/api/individual-address") {
        writes.push(route.request().postDataJSON());
        return route.fulfill({ status: 400, contentType: "application/json", body: JSON.stringify({ error: refusal }) });
      }
      if (path.includes("/parameters")) {
        return route.fulfill({
          contentType: "application/json",
          body: JSON.stringify({ programId: null, sections: [], stale: [], diagnostics: [] }),
        });
      }
      unexpected.push(`${route.request().method()} ${path}`);
      return route.fulfill({ status: 404, contentType: "application/json", body: JSON.stringify({ error: "fixture only" }) });
    });
    await page.goto(`/e2e/device-editor-fixture.html?lang=${language}`);
    const input = page.locator(".individual-address-field input");
    await expect(input).toHaveValue("12");
    await input.fill("0");
    await input.press("Tab");
    await expect.poll(() => writes.length).toBe(1);
    expect(writes).toEqual([{ deviceId: 42, address: "1.1.0" }]);
    await expect(page.locator(".individual-address-field .field-error")).toHaveText(refusal);
    await expect(input).toHaveValue("12");
    expect(unexpected).toEqual([]);
  });
}
