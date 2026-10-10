/** Raw/header candidate labels in real Chromium: literal, bounded and read-only. */
import { test, expect } from "@playwright/test";
import type { LogEntry } from "../src/api";
for (const language of ["en", "de"] as const) for (const width of [400, 1440]) {
  test(`${language} ${width}: RTF-like raw preview remains literal and fits`, async ({ page }) => {
    await page.setViewportSize({ width, height: 800 });
    const raw = String.raw`{\rtf1\ansi <b>literal</b> \u20320?}` + "界".repeat(300);
    const rows: LogEntry[] = [{ timestamp: "2026-09-10T12:00:00Z", source: "import:opaque:raw-text", severity: "info", message: "RTF-like raw text candidate; not validated or interpreted; original retained; preview truncated", location: "P-TEST/0.xml /synthetic", detail: raw }];
    const requests: string[] = [], unexpected: string[] = [];
    await page.route("**/api/**", route => {
      const request = route.request(), path = new URL(request.url()).pathname;
      requests.push(`${request.method()} ${path}`);
      if (request.method() === "GET" && path === "/api/log") return route.fulfill({ contentType: "application/json", body: JSON.stringify(rows) });
      unexpected.push(`${request.method()} ${path}`);
      return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"unexpected fixture request"}' });
    });
    await page.goto(`/e2e/raw-log-fixture.html?lang=${language}`);
    await expect(page.locator("html")).toHaveAttribute("data-theme", "porcelain");
    await expect(page.locator(".log-entry-raw-label")).toHaveText(language === "de" ? "RTF-ähnlicher Rohtext — nicht interpretiert" : "RTF-like raw text — not interpreted");
    const preview = page.locator(".log-entry-raw-text");
    await expect(preview).toHaveText(raw);
    await expect(preview.locator("b,input,textarea,button,img")).toHaveCount(0);
    expect(await preview.evaluate(e => e.scrollWidth - e.clientWidth)).toBeLessThanOrEqual(1);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth)).toBeLessThanOrEqual(1);
    expect(requests).toEqual(["GET /api/log"]);
    expect(unexpected).toEqual([]);
    if (process.env.KNXBENCH_RAW_LOG_SCREENSHOTS) await page.locator(".log-panel").screenshot({ path: `${process.env.KNXBENCH_RAW_LOG_SCREENSHOTS}/${language}-${width}.png` });
  });
}
