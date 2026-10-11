/** Checks banner layout in the built workbench using intercepted import phases. */
import { expect, test } from "@playwright/test";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";

for (const language of ["en", "de"] as const) {
  for (const width of [1440, 400]) {
    test(`${language} ${width}: uncounted import keeps its current step readable`, async ({ page }) => {
      const messages = language === "de" ? de : en;
      const unexpected: string[] = [];
      let token: string | null = null;
      let phase = "ingestManufacturerData";
      let finishImport: (() => Promise<void>) | undefined;
      const theme = width === 400 ? "porcelain" : "graphite";
      await page.setViewportSize({ width, height: 900 });
      await page.route("**/api/**", async (route) => {
        const request = route.request();
        const path = new URL(request.url()).pathname;
        const method = request.method();
        const ok = (body: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
        const achievements = achievementsFixtureAnswer(method, path);
        if (achievements !== undefined) return ok(achievements);
        if (path === "/api/auth/status") return ok({ required: false, authenticated: true });
        if (path === "/api/settings" && method === "GET") return ok({ schemaVersion: 1, status: "ok", settings: {
          uiLanguage: language, theme, onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.8" },
        } });
        if (path === "/api/version") return ok({ version: "0.1.0-alpha.8" });
        if (path === "/api/project" && method === "GET") return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"no project open"}' });
        if (path === "/api/bus/discover" && method === "POST") return ok({ interfaces: [] });
        if (path === "/api/project/load-progress") return ok(token === null ? null : {
          operationId: 1, kind: "import", source: "fictional-progress.knxproj", phase,
          completed: phase === "ingestManufacturerData" ? 13 : null,
          total: phase === "ingestManufacturerData" ? 251 : null,
          status: "running", error: null, clientToken: token,
        });
        if (path === "/api/fs/list") return ok([{ name: "fictional-progress.knxproj", is_dir: false }]);
        if (method === "GET" && ["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items", "/api/session-log"].includes(path)) return ok([]);
        if (path === "/api/project/import" && method === "POST") {
          token = request.postDataJSON().clientToken;
          expect(token).toBeTruthy();
          // Hold only the fixture's response, not a real import or hardware operation.
          finishImport = () => route.fulfill({ status: 422, contentType: "application/json", body: '{"error":"fictional failure","kind":"projectNotImportable"}' });
          return;
        }
        unexpected.push(`${method} ${path}`);
        return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"unexpected fixture request"}' });
      });
      await page.goto("/");
      await expect(page.locator("html")).toHaveAttribute("lang", language);
      await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
      await page.keyboard.press("Control+Shift+P");
      await page.getByRole("combobox").fill(messages["toolbar.openProject"]);
      await page.getByRole("option").filter({ hasText: messages["toolbar.openProject"] }).click();
      await page.getByRole("button", { name: "fictional-progress.knxproj", exact: true }).click();
      const banner = page.locator(".load-progress");
      const bar = banner.getByRole("progressbar");
      const label = banner.locator(".load-progress-phase");
      await expect(banner.locator(".load-progress-count")).toHaveText(language === "de" ? "13 von 251" : "13 of 251");
      await expect(bar).toHaveAttribute("aria-valuenow", "5");
      for (const next of ["ingestMasterData", "enrichFromProductDatabase", "persistOpaque", "buildProjectTree"] as const) {
        phase = next;
        const step = messages["loadProgress.currentStep"].replace("{phase}", messages[`loadProgress.phase.${next}`]);
        await expect(label).toHaveText(step);
        await expect(label).toBeInViewport();
        await expect(bar).not.toHaveAttribute("aria-valuenow");
        await expect(bar).toHaveAttribute("aria-valuetext", step);
        await expect(banner.locator(".load-progress-count")).toHaveCount(0);
        await expect(banner.locator(".load-progress-unmeasured")).toHaveText(messages["loadProgress.unmeasured"]);
      }
      const geometry = await banner.evaluate((element) => {
        const track = element.querySelector(".load-progress-track")!;
        const phaseLabel = element.querySelector(".load-progress-phase")!;
        const flavour = element.querySelector(".load-progress-flavour")!;
        return {
          trackBottom: track.getBoundingClientRect().bottom,
          phaseTop: phaseLabel.getBoundingClientRect().top,
          phaseFont: parseFloat(getComputedStyle(phaseLabel).fontSize),
          flavourFont: parseFloat(getComputedStyle(flavour).fontSize),
          phaseColour: getComputedStyle(phaseLabel).color,
          foreground: getComputedStyle(element.querySelector("strong")!).color,
        };
      });
      expect(geometry.phaseTop).toBeGreaterThanOrEqual(geometry.trackBottom);
      expect(geometry.phaseFont).toBeCloseTo(12 + 8 / 3, 1);
      expect(geometry.flavourFont).toBeCloseTo(11 + 8 / 3, 1);
      expect(geometry.phaseColour).toBe(geometry.foreground);
      expect(await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth)).toBeLessThanOrEqual(1);
      if (process.env.IMPORT_STATUS_SCREENSHOTS) await page.screenshot({ path: `${process.env.IMPORT_STATUS_SCREENSHOTS}/${language}-${width}.png` });
      await finishImport!();
      await expect(banner).toHaveAttribute("data-failed", "true");
      await expect(bar).toHaveCount(0);
      await expect(banner.locator(".load-progress-flavour")).toHaveCount(0);
      await expect(label).toContainText(messages["loadProgress.phase.buildProjectTree"]);
      expect(unexpected).toEqual([]);
    });
  }
}
