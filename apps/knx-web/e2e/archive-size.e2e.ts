/** Verifies localized size-limit toasts through the real app's intercepted import flow. */
import { expect, test } from "@playwright/test";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";

for (const language of ["en", "de"] as const) {
  for (const width of [1440, 400]) {
    test(`${language} ${width}: oversized import names its expanded size and limit plainly`, async ({ page }) => {
      const messages = language === "de" ? de : en;
      const requests: string[] = [];
      const unexpected: string[] = [];
      const raw = "the archive declares 1342177280 uncompressed bytes, more than the 1073741824-byte limit";
      await page.setViewportSize({ width, height: 900 });
      await page.route("**/api/**", (route) => {
        const request = route.request();
        const path = new URL(request.url()).pathname;
        const method = request.method();
        requests.push(`${method} ${path}`);
        const ok = (body: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
        const achievements = achievementsFixtureAnswer(method, path);
        if (achievements !== undefined) return ok(achievements);
        if (path === "/api/auth/status") return ok({ required: false, authenticated: true });
        if (path === "/api/settings" && method === "GET") return ok({ schemaVersion: 1, status: "ok", settings: {
          uiLanguage: language, theme: "graphite", onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.7" },
        } });
        if (path === "/api/version") return ok({ version: "0.1.0-alpha.7" });
        if (path === "/api/project" && method === "GET") return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"no project open"}' });
        if (path === "/api/bus/discover" && method === "POST") return ok({ interfaces: [] });
        if (path === "/api/project/load-progress") return ok(null);
        if (path === "/api/fs/list") return ok([{ name: "fictional-large.knxproj", is_dir: false }]);
        if (method === "GET" && ["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items", "/api/session-log"].includes(path)) return ok([]);
        if (path === "/api/project/import" && method === "POST") {
          expect(request.postDataJSON().path).toBe("fictional-large.knxproj");
          return route.fulfill({ status: 422, contentType: "application/json", body: JSON.stringify({ error: raw, kind: "projectNotImportable" }) });
        }
        unexpected.push(`${method} ${path}`);
        return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"unexpected fixture request"}' });
      });
      await page.goto("/");
      await expect(page.locator("html")).toHaveAttribute("lang", language);
      await expect(page.locator("html")).toHaveAttribute("data-theme", "graphite");
      await page.keyboard.press("Control+Shift+P");
      await page.getByRole("combobox").fill(messages["toolbar.openProject"]);
      await page.getByRole("option").filter({ hasText: messages["toolbar.openProject"] }).click();
      await page.getByRole("button", { name: "fictional-large.knxproj", exact: true }).click();
      const toast = page.locator(".toast--error");
      await expect(toast).toHaveAttribute("role", "alert");
      await expect(toast.locator(".toast-title")).toHaveText(language === "de"
        ? "Die Datei ist entpackt mit 1280 MiB zu groß. Das Limit liegt bei 1024 MiB."
        : "The file is too large when unpacked: 1280 MiB. The limit is 1024 MiB.");
      await expect(page.locator(".load-progress-error")).toHaveText(language === "de"
        ? "Die Datei ist entpackt mit 1280 MiB zu groß. Das Limit liegt bei 1024 MiB."
        : "The file is too large when unpacked: 1280 MiB. The limit is 1024 MiB.");
      await expect(toast.locator(".toast-hint")).toHaveCount(0);
      await expect(toast.getByRole("button")).toBeInViewport();
      const box = await toast.boundingBox();
      expect(box!.x).toBeGreaterThanOrEqual(0);
      expect(box!.x + box!.width).toBeLessThanOrEqual(width);
      expect(await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth)).toBeLessThanOrEqual(1);
      expect(requests.filter((request) => request === "POST /api/project/import")).toHaveLength(1);
      expect(unexpected).toEqual([]);
      await toast.evaluate(async (element) => {
        await Promise.all(element.getAnimations().map((animation) => animation.finished));
      });
      if (process.env.ARCHIVE_TOAST_SCREENSHOTS) await page.screenshot({ path: `${process.env.ARCHIVE_TOAST_SCREENSHOTS}/${language}-${width}.png` });
    });
  }
}
