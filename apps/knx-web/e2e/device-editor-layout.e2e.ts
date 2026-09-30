/** Checks the real device editor at narrow and wide widths using only a local mocked API. */
import { test, expect } from "@playwright/test";

const translations = {
  en: ["Read", "Write", "Transmit", "Update", "Communication", "Read on init"],
  de: ["Lesen", "Schreiben", "Übertragen", "Aktualisieren", "Kommunikation", "Lesen bei Initialisierung"],
} as const;

for (const language of ["en", "de"] as const) {
  for (const width of [360, 640, 1440]) {
    test(`${language} device editor keeps long labels and addresses visible at ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 800 });
      const writes: unknown[] = [];
      await page.route("**/api/**", (route) => route.fulfill({
        contentType: "application/json",
        body: JSON.stringify({ programId: null, sections: [], stale: [], diagnostics: [] }),
      }));
      await page.route("**/api/individual-address", (route) => {
        writes.push(route.request().postDataJSON());
        return route.fulfill({ contentType: "application/json", body: "{}" });
      });
      await page.goto(`/e2e/device-editor-fixture.html?lang=${language}`);
      await expect(page.locator(".individual-address-field input")).toHaveValue("12");
      await page.locator(".com-object-detail summary").click();
      await expect(page.locator(".com-object-flags .flag-name")).toHaveText([...translations[language]]);
      await expect(page.locator(".com-object-flags .flag-code")).toHaveText(["R", "W", "T", "U", "C", "I"]);
      await expect(page.locator(".group-link-row")).toHaveCount(2);
      const layout = await page.evaluate(() => {
        const flags = document.querySelector<HTMLElement>(".com-object-flags")!;
        const clipped = Array.from(document.querySelectorAll<HTMLElement>(
          ".device-tabs button, .com-object-flags label, .com-object-summary strong, .group-link-row, .individual-address-field",
        )).filter((element) => element.scrollWidth > element.clientWidth + 1)
          .map((element) => `${element.className || element.tagName}: ${element.scrollWidth}/${element.clientWidth}`);
        return {
          documentWidth: document.documentElement.scrollWidth,
          viewportWidth: innerWidth,
          flagColumns: getComputedStyle(flags).gridTemplateColumns.split(" ").length,
          clipped,
          offscreen: Array.from(document.querySelectorAll<HTMLElement>("body *"))
            .filter((element) => {
              const rect = element.getBoundingClientRect();
              return rect.width > 0 && rect.right > innerWidth + 1;
            })
            .slice(0, 12)
            .map((element) => ({ name: `${element.tagName}.${element.className}`, parent: element.parentElement?.className, text: element.textContent?.trim().slice(0, 45), right: Math.ceil(element.getBoundingClientRect().right) })),
        };
      });
      expect(layout.documentWidth, JSON.stringify(layout)).toBeLessThanOrEqual(layout.viewportWidth);
      expect(layout.flagColumns).toBe(2);
      expect(layout.clipped).toEqual([]);
      expect(layout.offscreen).toEqual([]);

      await page.locator(".individual-address-field input").fill("21");
      await page.locator(".individual-address-field input").press("Tab");
      await expect.poll(() => writes.length).toBe(1);
      expect(writes).toEqual([{ deviceId: 42, address: "1.1.21" }]);
    });
  }
}
