/** Checks read-only instance evidence in the real Inspector using an intercepted local API. */
import { test, expect } from "@playwright/test";

for (const language of ["en", "de"] as const) {
  for (const width of [360, 1440]) {
    test(`${language} stored instance evidence is readable and never editable at ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 800 });
      const methods: string[] = [];
      const instanceValues = [
        { etsId: "A-TEST_MD-71_M-93_MI-2_P-83_R-61", raw: "17" },
        { etsId: "A-TEST_MD-71_M-93_MI-4_P-83_R-61", raw: "23 <b>raw evidence</b>" },
      ];
      await page.route("**/api/**", (route) => {
        methods.push(route.request().method());
        return route.fulfill({ contentType: "application/json", body: JSON.stringify({
          programId: "A-TEST", sourceLanguage: null, sections: [], stale: [], diagnostics: [], tree: null, instanceValues,
        }) });
      });
      await page.goto(`/e2e/device-editor-fixture.html?lang=${language}&channels=1`);
      await page.getByRole("tab", { name: language === "de" ? "Diagnose" : "Diagnostics", exact: true }).click();
      const evidence = page.locator(".parameter-instance-values");
      await expect(evidence).toBeVisible();
      await expect(evidence).toContainText(language === "de" ? "nicht ausgewertet" : "not evaluated");
      await expect(evidence.locator("li")).toHaveText(instanceValues.map(v => `${v.etsId}: ${v.raw}`));
      await expect(evidence.locator("input, select, textarea, button, b")).toHaveCount(0);
      const layout = await evidence.evaluate(element => ({
        right: element.getBoundingClientRect().right, viewport: innerWidth,
        clipped: [...element.querySelectorAll("li")].some(e => e.scrollWidth > e.clientWidth + 1),
      }));
      expect(layout.right).toBeLessThanOrEqual(layout.viewport + 1);
      expect(layout.clipped).toBe(false);
      expect(methods.length).toBeGreaterThan(0);
      expect(methods.every(method => method === "GET")).toBe(true);
      if (process.env.KNXBENCH_INSTANCE_SCREENSHOTS) {
        await evidence.screenshot({ path: `${process.env.KNXBENCH_INSTANCE_SCREENSHOTS}/${language}-${width}.png` });
      }
    });
  }
}
