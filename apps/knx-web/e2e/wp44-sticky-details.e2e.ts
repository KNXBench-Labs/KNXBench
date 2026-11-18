/** WP-44.1: the details aside scrolls independently of the telegram table. */
import { expect, test } from "@playwright/test";
import type { BusTelegramRow, BusMonitorTelegramsResponse } from "../src/api";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { dismissToasts, intercept, openProject } from "./communication-fixture";

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} details pane stays a separate scroll area at ${width}px`, async ({ page }) => {
    const m = language === "en" ? en : de;
    const foreign: string[] = [];
    await page.route("**/*", route => {
      const url = new URL(route.request().url());
      if (url.hostname === "127.0.0.1") return route.continue();
      foreign.push(url.origin); return route.abort();
    });
    const state = await intercept(page, language, 8, width === 1440 ? "graphite" : "porcelain");
    await page.route("**/api/version", route => route.fulfill({ json: { version: "0.1.0-alpha.9+gfixture" } }));
    const ga = state.tree.installations[0].group_addresses[0];
    ga.dpts = ["DPST-1-1"];
    const rows: BusTelegramRow[] = Array.from({ length: 120 }, (_, index) => ({
      seq: index + 1, timestamp: "2026-10-12T06:00:00Z", source: "1.1.2", destination: ga.address,
      destinationName: ga.name, service: "GroupValueWrite", rawPayload: "0x01", control: null,
      decoded: { kind: "value", dpt: "DPST-1-1", text: String(index + 1) },
    }));
    await page.route("**/api/bus/monitor/telegrams?**", route => {
      const since = Number(new URL(route.request().url()).searchParams.get("since") ?? 0);
      const body: BusMonitorTelegramsResponse = { sessionId: 1, serverIncarnation: "devices-browser",
        contextStatus: "current", projectOpen: true, status: "active", nextSince: 121, droppedBefore: 0,
        telegrams: since === 0 ? rows : [] };
      return route.fulfill({ json: body });
    });
    await page.route("**/api/settings", route => route.fulfill({ json: { schemaVersion: 1, conditionalPatchVersion: 1,
      status: "ok", settings: { uiLanguage: language, theme: width === 1440 ? "graphite" : "porcelain",
        releaseChecksEnabled: false, onboardingGuide: { seenStage: "alpha" }, motionLevel: "off" } } }));
    await page.setViewportSize({ width, height: 800 });
    await openProject(page, language);
    await dismissToasts(page, language);
    await page.keyboard.press("Control+Shift+P");
    await page.getByRole("combobox").fill(m["toolbar.busMonitor"]);
    await page.getByRole("combobox").press("Enter");
    const data = page.locator(".monitor-data");
    const scroll = page.locator(".monitor-table-scroll");
    const details = page.locator(".telegram-details");
    await expect(page.locator(".bus-monitor-table tbody tr")).toHaveCount(120);
    // The aside is a direct grid child, the table's scroll viewport sits below it.
    await expect(details).toBeVisible();
    expect(await details.evaluate(node => node.parentElement?.className)).toBe("monitor-data");
    expect(await scroll.evaluate(node => node.parentElement?.parentElement?.className)).toBe("monitor-data");
    // Wide layout only: both panes have their own bounded scroll; at 400px the
    // single-column fallback removes the max-heights and the page scrolls.
    const metrics = await page.evaluate(() => {
      const s = document.querySelector<HTMLElement>(".monitor-table-scroll")!;
      const d = document.querySelector<HTMLElement>(".telegram-details")!;
      const wide = matchMedia("(min-width: 1351px)").matches;
      return { wide, vh: innerHeight * 0.75, fullVh: innerHeight, tableMax: getComputedStyle(s).maxHeight,
        detailsMax: getComputedStyle(d).maxHeight, detailsOverflow: getComputedStyle(d).overflowY };
    });
    if (width === 1440) {
      expect(metrics.wide).toBe(true);
      // 75dvh resolves against the 800px viewport height; the sticky aside
      // uses calc(100dvh - 24px).
      expect(metrics.tableMax).toBe(`${Math.round(metrics.vh)}px`);
      expect(metrics.detailsMax).toBe(`${Math.round(metrics.fullVh - 24)}px`);
      expect(metrics.detailsOverflow).toBe("auto");
    } else {
      expect(metrics.wide).toBe(false);
      expect(metrics.tableMax).toBe("none");
      expect(metrics.detailsMax).toBe("none");
    }
    // Selecting the first row fills the details; scrolling the table must not
    // move the aside (independent scroll areas).
    await page.locator(".bus-monitor-table tbody tr").first().click();
    await expect(details).toContainText(ga.address);
    const before = await details.evaluate(node => node.getBoundingClientRect().top);
    await scroll.evaluate(node => { node.scrollTop = 4000; });
    const after = await details.evaluate(node => node.getBoundingClientRect().top);
    if (width === 1440) expect(after).toBe(before);
    expect(foreign).toEqual([]);
  });
}
