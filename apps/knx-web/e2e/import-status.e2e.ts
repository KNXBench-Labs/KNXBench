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
      await page.emulateMedia({ reducedMotion: "no-preference", forcedColors: "none" });
      await page.route("https://api.github.com/**", route => route.abort());
      await page.route("**/api/**", async (route) => {
        const request = route.request();
        const path = new URL(request.url()).pathname;
        const method = request.method();
        const ok = (body: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
        const achievements = achievementsFixtureAnswer(method, path);
        if (achievements !== undefined) return ok(achievements);
        if (path === "/api/auth/status") return ok({ required: false, authenticated: true });
        if (path === "/api/settings" && method === "GET") return ok({ schemaVersion: 1, status: "ok", settings: {
          uiLanguage: language, theme, motionLevel: "standard", releaseChecksEnabled: false,
          onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.8" },
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
      const text = label.locator(".task-status-text");
      await expect(text).toHaveCount(1);
      await expect.poll(() => text.evaluate(element => getComputedStyle(element).animationName))
        .toBe("knx-task-text-shimmer");
      const cycle = await text.evaluate(element => {
        const animation = element.getAnimations()[0];
        animation.pause();
        const positions = [0, 1250, 2500, 3000, 3499, 3500, 4750].map(time => {
          animation.currentTime = time;
          return parseFloat(getComputedStyle(element).backgroundPositionX);
        });
        animation.currentTime = 1250;
        return { positions, duration: animation.effect!.getTiming().duration,
          easing: animation.effect!.getTiming().easing, text: element.textContent };
      });
      expect(cycle.duration).toBe(3500);
      expect(cycle.easing).toBe("linear");
      expect(cycle.positions[0]).toBeCloseTo(110);
      expect(cycle.positions[1]).toBeCloseTo(50);
      // Production CSS rounds the keyframe percentage; tolerate sub-pixel rounding
      // at the boundary, but require a fully stationary position inside the pause.
      expect(cycle.positions[2]).toBeCloseTo(-10, 3);
      expect(cycle.positions.slice(3, 5)).toEqual([-10, -10]);
      expect(cycle.positions[5]).toBeCloseTo(110);
      expect(cycle.positions[6]).toBeCloseTo(50);
      // The active address-programming message uses this warning/text shape too.
      await label.evaluate(element => {
        element.classList.add("form-warning");
        element.style.color = "var(--knx-warning-color)";
      });
      const warningInk = await text.evaluate(element => ({
        parent: getComputedStyle(element.parentElement!).color, text: getComputedStyle(element).color,
      }));
      expect(warningInk.text).toBe(warningInk.parent);
      await label.evaluate(element => {
        element.classList.remove("form-warning");
        element.style.removeProperty("color");
      });
      await page.emulateMedia({ reducedMotion: "reduce" });
      await expect.poll(() => text.evaluate(element => getComputedStyle(element).animationName)).toBe("none");
      await expect(text).toHaveText(cycle.text!);
      expect(await text.evaluate(element => getComputedStyle(element).webkitTextFillColor)).not.toBe("rgba(0, 0, 0, 0)");
      await page.emulateMedia({ reducedMotion: "no-preference" });
      await page.locator("html").evaluate(element => element.setAttribute("data-motion-level", "off"));
      await expect.poll(() => text.evaluate(element => getComputedStyle(element).animationName)).toBe("none");
      await page.locator("html").evaluate(element => {
        element.setAttribute("data-motion-level", "subtle");
        element.setAttribute("data-motion-style", "glitch");
      });
      await expect.poll(() => text.evaluate(element => getComputedStyle(element).animationName)).toBe("knx-task-text-shimmer");
      expect(await text.evaluate(element => getComputedStyle(element).animationTimingFunction)).toBe("linear");
      expect(await text.evaluate(element => Number(element.getAnimations()[0].effect!.getTiming().duration))).toBeCloseTo(3600, 3);
      await page.emulateMedia({ media: "print" });
      await expect.poll(() => text.evaluate(element => getComputedStyle(element).animationName)).toBe("none");
      expect(await text.evaluate(element => getComputedStyle(element).webkitTextFillColor)).not.toBe("rgba(0, 0, 0, 0)");
      await page.emulateMedia({ media: "screen" });
      await page.emulateMedia({ forcedColors: "active" });
      await expect.poll(() => text.evaluate(element => getComputedStyle(element).animationName)).toBe("none");
      expect(await text.evaluate(element => getComputedStyle(element).webkitTextFillColor)).not.toBe("rgba(0, 0, 0, 0)");
      await page.emulateMedia({ forcedColors: "none" });
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
      if (process.env.IMPORT_STATUS_SCREENSHOTS) {
        for (const [frame, fraction] of [["sweep", 5 / 14], ["pause", 6 / 7]] as const) {
          await text.evaluate((element, point) => {
            const animation = element.getAnimations()[0];
            animation.pause();
            animation.currentTime = Number(animation.effect!.getTiming().duration) * point;
          }, fraction);
          await page.screenshot({ path: `${process.env.IMPORT_STATUS_SCREENSHOTS}/${language}-${width}-${frame}.png` });
        }
      }
      await finishImport!();
      await expect(banner).toHaveAttribute("data-failed", "true");
      await expect(banner.locator(".task-status-text")).toHaveCount(0);
      await expect(bar).toHaveCount(0);
      await expect(banner.locator(".load-progress-flavour")).toHaveCount(0);
      await expect(label).toContainText(messages["loadProgress.phase.buildProjectTree"]);
      expect(unexpected).toEqual([]);
    });
  }
}
