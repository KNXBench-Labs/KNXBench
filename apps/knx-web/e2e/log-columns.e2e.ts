/** #13 in real Chromium: a Log with a time column, even rows and one small Export… button. */
import { test, expect } from "@playwright/test";
import { readFile } from "node:fs/promises";
import type { LogEntry } from "../src/api";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";

// Fixed "now", in the browser's local time: 13 October 2026, 18:00.
const NOW = new Date(2026, 9, 13, 18, 0, 0);
const at = (day: number, hours: number, minutes: number) => new Date(2026, 9, day, hours, minutes, 0).toISOString();
const ROWS: LogEntry[] = [
  { timestamp: at(12, 23, 1), severity: "info", source: "open", message: "opened project Fictional house", location: null, detail: null },
  { timestamp: at(13, 9, 15), severity: "warning", source: "import:unsupported", message: "1400 retained files: manufacturer payload (png)",
    location: null, detail: Array.from({ length: 1400 }, (_, n) => `M-0001/Baggages/${n}.png`).join("\n") },
  { timestamp: at(13, 9, 16), severity: "info", source: "LinkComObject",
    message: "Linked object 3 “Switch” of “Kitchen actuator” (1.1.5) to 1/2/3 “Kitchen light” (send)", location: null,
    detail: "LinkComObject { com_object: ComObjectInstanceId(153), ga: GroupAddressId(7), direction: Send }" },
  { timestamp: at(13, 9, 17), severity: "error", source: "save", message: "Saving failed: ".concat("a very long reason ".repeat(20)),
    location: "fictional-house.knxdb", detail: null },
  { timestamp: at(13, 9, 18), severity: "info", source: "RenameGroupAddress", message: "RenameGroupAddress", location: null,
    detail: "first line\nsecond line" },
];

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} ${width}: rows are even, timed, and export is one small button`, async ({ page }) => {
    const m = language === "de" ? de : en;
    await page.clock.setFixedTime(NOW);
    await page.setViewportSize({ width, height: 900 });
    const unexpected: string[] = [];
    await page.route("**/api/**", route => {
      const request = route.request(), path = new URL(request.url()).pathname;
      if (request.method() === "GET" && path === "/api/log") return route.fulfill({ contentType: "application/json", body: JSON.stringify(ROWS) });
      unexpected.push(`${request.method()} ${path}`);
      return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"unexpected fixture request"}' });
    });
    await page.goto(`/e2e/raw-log-fixture.html?lang=${language}`);
    const rows = page.locator(".log-entry");
    await expect(rows).toHaveCount(ROWS.length);

    // Every row is one line high (two in a narrow list), the same for all.
    const heights = await rows.evaluateAll(items => items.map(item => item.getBoundingClientRect().height));
    expect(Math.max(...heights) - Math.min(...heights), JSON.stringify(heights)).toBeLessThanOrEqual(1);
    const line = await rows.first().evaluate(item => parseFloat(getComputedStyle(item).lineHeight));
    expect(heights[0]).toBeLessThan((width >= 1000 ? 2 : 3) * line + 16);

    // Newest first; today's time only, yesterday with its date.
    await expect(page.locator(".log-entry-time")).toHaveText(
      language === "de" ? ["09:18:00", "09:17:00", "09:16:00", "09:15:00", "12. Okt., 23:01:00"]
        : ["09:18:00", "09:17:00", "09:16:00", "09:15:00", "Oct 12, 23:01:00"]);
    await expect(page.locator(".log-entry-time").first()).toHaveAttribute("title", ROWS[4].timestamp);
    await expect(page.locator(".log-panel-columns")).toBeVisible({ visible: width >= 1000 });
    await expect(page.locator(".log-entry-message").nth(2)).toHaveText(ROWS[2].message);
    await expect(page.locator(".log-entry-detail").first()).toContainText("first line");
    await expect(page.locator(".log-entry-detail").first()).not.toContainText("second line");

    // Nothing pushes the page sideways.
    expect(await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth)).toBeLessThanOrEqual(1);
    for (const row of await rows.all()) expect(await row.evaluate(item => item.scrollWidth - item.clientWidth)).toBeLessThanOrEqual(1);

    // One small Export… button; the scope lives in its dialog.
    const exportButton = page.locator(".log-panel-export");
    await expect(exportButton).toHaveText(m["logPanel.export"]);
    expect((await exportButton.boundingBox())!.width).toBeLessThan(160);
    await expect(page.locator("select")).toHaveCount(0);
    if (process.env.KNX_LOG_CAPTURE_DIR) await page.screenshot({ path: `${process.env.KNX_LOG_CAPTURE_DIR}/log-${language}-${width}.png`, fullPage: true });
    await exportButton.click();
    const dialog = page.locator(".log-export-panel");
    await expect(dialog.getByRole("heading")).toHaveText(m["logPanel.exportTitle"]);
    await expect(dialog.getByRole("radio", { name: m["logPanel.exportFiltered"].replace("{count}", "5") })).toBeChecked();
    // A radio sits right before its words, and the dialog is as tall as what it says.
    const radio = dialog.getByRole("radio", { name: m["logPanel.exportAll"].replace("{count}", "5") });
    const [radioBox, labelBox, panelBox] = [await radio.boundingBox(), await radio.locator("xpath=..").boundingBox(), await dialog.boundingBox()];
    expect(radioBox!.width).toBeLessThanOrEqual(24);
    expect(radioBox!.x - labelBox!.x).toBeLessThanOrEqual(4);
    expect(panelBox!.height).toBeLessThan(600);
    await radio.check();
    const box = await dialog.boundingBox();
    expect(box!.x).toBeGreaterThanOrEqual(0); expect(box!.x + box!.width).toBeLessThanOrEqual(width);
    if (process.env.KNX_LOG_CAPTURE_DIR) await page.screenshot({ path: `${process.env.KNX_LOG_CAPTURE_DIR}/export-${language}-${width}.png` });
    const downloading = page.waitForEvent("download");
    await dialog.getByRole("button", { name: m["logPanel.exportConfirm"], exact: true }).click();
    const download = await downloading;
    const exported = JSON.parse(await readFile((await download.path())!, "utf8"));
    expect(exported.scope).toBe("all");
    expect(exported.entries).toEqual(ROWS);
    await expect(dialog).toHaveCount(0);
    expect(unexpected).toEqual([]);
  });
}
