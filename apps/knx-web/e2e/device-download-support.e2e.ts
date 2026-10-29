/** Exercises pre-plan refusal help offline: instructions link and report shortcut, no upload. */
import { expect, test } from "@playwright/test";
import { join } from "node:path";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { intercept, openProject, dismissToasts } from "./communication-fixture";
import { answerBundleRoute, bundleLog } from "./report-problem-fixture";

for (const language of ["en", "de"] as const) {
  for (const width of [1440, 400]) {
    test(`${language} ${width}: refusal help opens instructions and the report wizard before planning`, async ({ page }) => {
      const m = language === "en" ? en : de;
      const theme = width === 400 ? "graphite" : "porcelain";
      await page.setViewportSize({ width, height: 1000 });
      const state = await intercept(page, language, 8, theme);
      const calls: string[] = [];
      const bundle = bundleLog();
      await page.route("**/api/diagnostics/**", async (route) => { if (!(await answerBundleRoute(route, bundle))) await route.fallback(); });
      await page.route("**/api/device-readiness", (route) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify({
        devices: [{ address: "1.1.2", name: "PRIVATE customer", programRef: "M-0001_A-0001-01-0000", readiness: "unsupported", category: "byte-order", detail: "CUSTOMER_SECRET 1.1.2 /home/private token=secret", steps: null, octets: null }], counts: { unsupported: 1 },
      }) }));
      await page.route("**/api/device-download/plan", (route) => {
        calls.push("plan"); return route.fulfill({ status: 422, contentType: "application/json", body: JSON.stringify({ error: "PRIVATE configuration detail", kind: "downloadPreparationRefused", category: "modules" }) });
      });
      await page.context().route("https://github.com/**", (route) => route.fulfill({ contentType: "text/html", body: "<title>Offline link target</title><p>No real submission.</p>" }));
      await openProject(page, language); await dismissToasts(page, language);
      await page.keyboard.press("Control+Shift+P");
      await page.getByRole("combobox").fill(m["toolbar.busMonitor"]);
      await page.getByRole("combobox").press("Enter");
      await page.getByRole("button", { name: m["deviceDownload.title"], exact: true }).click();
      const panel = page.locator(".device-download-panel");
      await panel.locator("[data-download-device]").selectOption("1");
      const help = panel.locator("[data-support-guidance]");
      await expect(help).toContainText(m["deviceDownload.readiness.reason.byteOrder"]);
      await expect(help.locator("[data-next-action=requestSupport]")).toHaveText(m["deviceDownload.help.action.requestSupport"]);
      await expect(panel.locator(".device-download-plan,.device-download-verify")).toHaveCount(0);
      expect(calls).toEqual([]);
      await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
      for (const selector of ["[data-support-instructions]"]) {
        const link = help.locator(selector);
        const href = await link.getAttribute("href");
        expect(href).not.toContain("CUSTOMER_SECRET"); expect(href).not.toContain("1.1.2"); expect(href).not.toContain("private");
        const popupPromise = page.waitForEvent("popup"); await link.click();
        const popup = await popupPromise; await popup.waitForLoadState();
        expect(popup.url()).toBe(href);
        expect(popup.url()).toContain(language === "de" ? "DEUTSCH.md" : "README.md");
        await popup.close();
      }
      // DIAG-03: the issue and mail links became one shortcut into the report wizard, this device preselected.
      await expect(help.locator("a")).toHaveCount(1);
      await expect(panel.locator("[data-report-problem-open]")).toHaveCount(1);
      await help.locator("[data-report-problem-open=support-notice]").click();
      const wizard = page.getByRole("dialog", { name: m["reportProblem.title"], exact: true });
      await expect(wizard.locator("[aria-current=step]")).toContainText(m["reportProblem.step.contents"]);
      await expect.poll(() => bundle.previews.length).toBe(1);
      expect(bundle.previews[0]).toMatchObject({ problemType: "deviceProgramming", deviceId: 1 });
      expect(bundle.previews[0].operation).toBeUndefined();
      expect(JSON.stringify(bundle.previews[0])).not.toContain("CUSTOMER_SECRET");
      expect(bundle.exports).toEqual([]);
      await page.keyboard.press("Escape");
      await expect(wizard).toHaveCount(0);
      await help.locator("summary").click();
      await expect(help.locator("textarea")).toHaveValue("byte-order\nCUSTOMER_SECRET 1.1.2 /home/private token=secret");
      await help.scrollIntoViewIfNeeded();
      const geometry = await help.evaluate((element) => {
        const box = element.getBoundingClientRect();
        const form = element.closest("form")!;
        const style = getComputedStyle(form);
        return { left: box.left, right: box.right, width: box.width,
          available: form.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight),
          overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth };
      });
      expect(geometry.left).toBeGreaterThanOrEqual(0); expect(geometry.right).toBeLessThanOrEqual(width); expect(geometry.overflow).toBeLessThanOrEqual(1);
      expect(geometry.width, "help spans the configuration instead of a narrow grid cell").toBeGreaterThanOrEqual(geometry.available - 2);
      await expect(help.locator("[data-copy-technical]")).toBeVisible();
      if (process.env.KNX_SUPPORT_CAPTURE_DIR) await help.screenshot({ path: join(process.env.KNX_SUPPORT_CAPTURE_DIR, `${language}-${width}-${theme}.png`) });
      await panel.getByRole("button", { name: m["deviceDownload.preparePlan"], exact: true }).click();
      await expect(help).toContainText(m["deviceDownload.readiness.reason.modules"]);
      await expect(help.locator("textarea")).toHaveValue("downloadPreparationRefused\nmodules\nPRIVATE configuration detail");
      await expect(panel.locator(".device-download-plan,.device-download-verify")).toHaveCount(0);
      expect(calls).toEqual(["plan"]);
      expect(state.requests.filter((r) => r.includes("/device-download/start") || r.includes("/device-download/report"))).toEqual([]);
      expect(state.unexpected).toEqual([]);
    });
  }
}
