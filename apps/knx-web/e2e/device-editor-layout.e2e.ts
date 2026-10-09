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
      await expect(page.locator(".com-object-channel-summary")).toContainText(
        language === "de" ? "Ohne ausgewerteten Kanal" : "Without evaluated channel",
      );
      await page.locator(".com-object-channel-summary").click();
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
              return !element.closest(".com-table-scroll") && rect.width > 0 && rect.right > innerWidth + 1;
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

for (const language of ["en", "de"] as const) {
  for (const width of [360, 1440]) {
    test(`${language} evaluated channel evidence stays keyboard-accessible at ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 800 });
      await page.route("**/api/**", (route) => route.fulfill({
        contentType: "application/json",
        body: JSON.stringify({ programId: "A-MOCK", sections: [], stale: [], diagnostics: [] }),
      }));
      await page.goto(`/e2e/device-editor-fixture.html?lang=${language}&channels=1`);
      const groups = page.locator(".com-object-channel");
      await expect(groups).toHaveCount(3, { timeout: 2_000 });
      await expect(groups.nth(0).locator(".com-object-channel-summary > strong")).toHaveText("Raw manufacturer name");
      await expect(groups.nth(0).locator(".com-object-channel-number")).toHaveText(language === "de" ? "Nummer: A-5" : "Number: A-5");
      await expect(groups.nth(1).locator(".com-object-channel-summary > strong")).toHaveText("Hall outputs");
      await expect(groups.nth(1).locator(".com-object-channel-name")).toHaveText("Name: Manufacturer output");
      await expect(groups.nth(1).locator(".com-object-channel-number")).toHaveText(language === "de" ? "Nummer: 19" : "Number: 19");
      await expect(groups.nth(2).locator(".com-object-channel-summary")).toContainText(
        language === "de" ? "Ohne ausgewerteten Kanal" : "Without evaluated channel",
      );
      expect(await groups.locator("button").evaluateAll(nodes => nodes.every(node => node.getAttribute("aria-expanded") === "false"))).toBe(true);

      await groups.nth(0).locator(".com-object-channel-summary").focus();
      await page.keyboard.press("Enter");
      await expect(groups.nth(0).locator("button")).toHaveAttribute("aria-expanded", "true");
      const programObject = page.locator('[data-object-id="8"]');
      await programObject.locator("summary").click();
      await expect(programObject.locator(".com-object-effective-dpt")).toContainText("DPST-9-1");
      await expect(programObject.locator(".com-object-dpt-origin")).toContainText(
        language === "de" ? "Programm-Standardwert" : "Program default",
      );
      await expect(programObject.locator(".com-object-effective-dpt")).toContainText("Temperature");
      const dptLineCounts = await programObject.locator(
        ".com-object-effective-dpt > .mono, .com-object-effective-dpt > small:not(.com-object-dpt-origin)",
      ).evaluateAll((elements) => elements.map((element) => {
        const text = document.createRange();
        text.selectNodeContents(element);
        return text.getClientRects().length;
      }));
      expect(dptLineCounts, "DPT code and description must stay readable without character-level wrapping").toEqual([1, 1]);

      await groups.nth(1).locator(".com-object-channel-summary").click();
      await expect(page.locator('[data-object-id="7"]')).toContainText("Switching lights");
      await expect(page.locator('[data-object-id="7"] .com-object-effective-dpt')).toContainText("DPST-1-1");
      await groups.nth(2).locator(".com-object-channel-summary").click();
      await expect(page.locator("tr[data-object-id][data-activation='Inactive']:visible")).toHaveCount(1);
      await expect(page.locator("tr[data-object-id][data-activation='Undetermined']:visible")).toHaveCount(1);
      await expect(page.locator("tr[data-object-id][data-activation='NotEvaluated']:visible")).toHaveCount(1);
      const widthState = await page.evaluate(() => ({ viewport: innerWidth, document: document.documentElement.scrollWidth }));
      expect(widthState.document, JSON.stringify(widthState)).toBeLessThanOrEqual(widthState.viewport);
    });
  }
}
