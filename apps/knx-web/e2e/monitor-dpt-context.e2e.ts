/** Offline monitor reasons stay distinct and historical across themes, widths and a late project. */
import { test, expect } from "@playwright/test";
import type { BusMonitorTelegramsResponse, BusTelegramRow } from "../src/api";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";

const scenarios = (["en", "de"] as const).flatMap(language =>
  ["porcelain", "graphite", "lcars", "user-modern-retro-green-crt"].flatMap(theme =>
    [360, 1024, 1440].map(width => ({ language, theme, width, motion: "off" }))),
);
scenarios.push({ language: "en", theme: "porcelain", width: 1024, motion: "standard" });
for (const { language, theme, width, motion } of scenarios) {
  test(`${language} captured DPT reasons in ${theme} at ${width}px motion ${motion}`, async ({ page }, info) => {
    const messages = language === "en" ? en : de;
    const unexpected: string[] = [];
    let stale = false;
    const outcomes: BusTelegramRow["decoded"][] = [
      { kind: "unresolved", reason: "noProject", text: "no project open" },
      { kind: "unresolved", reason: "noDpt", text: "no DPT resolved for this group address" },
      { kind: "unresolved", reason: "notInProject", text: "address not in project" },
      { kind: "unresolved", reason: "programDefaultCleared", programDefaultDpts: ["9.001", "9.004"], text: "no DPT assigned: the project clears the program default (DPT 9.001, 9.004)" },
      { kind: "conflict", text: "conflicting DPTs: DPST-1-1, DPST-9-4" },
      { kind: "error", dpt: "DPST-40-1", reason: "unsupportedDpt", text: "unsupported", error: "unsupported" },
      { kind: "error", dpt: "DPST-9-4", reason: "decodeFailed", text: "wrong payload length", error: "wrong payload length" },
      { kind: "unresolved", text: "legacy unresolved result" },
    ];
    const telegrams: BusTelegramRow[] = outcomes.map((decoded, seq) => ({
      seq, timestamp: "2026-10-18T12:00:00Z", source: "1.2.3", destination: "2/3/4",
      destinationName: "Measurement", service: "GroupValueWrite", rawPayload: "[0c, 80]", decoded,
    }));
    await page.route("**/*", route => {
      const url = new URL(route.request().url());
      if (url.hostname !== "127.0.0.1") {
        unexpected.push(url.origin); return route.abort();
      }
      if (url.pathname === "/api/bus/discover") return route.fulfill({ json: { interfaces: [] } });
      if (url.pathname === "/api/bus/monitor/telegrams") {
        const since = Number(url.searchParams.get("since") ?? 0);
        const body: BusMonitorTelegramsResponse = {
          sessionId: 1, serverIncarnation: "fixture", contextStatus: stale ? "stale" : "current",
          projectOpen: true, status: "active", nextSince: 8, droppedBefore: 0,
          telegrams: since === 0 ? telegrams : [],
        };
        return route.fulfill({ json: body });
      }
      if (url.pathname.startsWith("/api/")) {
        unexpected.push(`${route.request().method()} ${url.pathname}`);
        return route.fulfill({ status: 404, json: { error: "offline fixture only" } });
      }
      return route.continue();
    });
    await page.setViewportSize({ width, height: 1000 });
    if (motion === "off") await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto(`/e2e/monitor-control-fixture.html?lang=${language}&theme=${theme}${motion === "off" ? "&motion=off" : ""}`);
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    await expect(page.locator("html")).toHaveAttribute("data-motion-level", motion);
    const rows = page.locator(".bus-monitor-table tbody tr");
    await expect(rows).toHaveCount(8);
    await expect(page.locator(".bus-monitor-decode-state")).toHaveText([
      messages["busMonitor.decode.noProject"], messages["busMonitor.decode.unresolved"],
      messages["busMonitor.decode.notInProject"],
      messages["busMonitor.decode.programDefaultCleared"].replace("{dpts}", "9.001, 9.004"),
      messages["busMonitor.decode.conflict"], messages["busMonitor.decode.unsupported"],
      messages["busMonitor.decode.failed"], messages["busMonitor.decode.unresolvedUnknown"],
    ]);
    await rows.first().focus();
    await page.keyboard.press("Enter");
    const details = page.locator(".telegram-details");
    await expect(details).toContainText(messages["busMonitor.decode.noProject"]);
    await expect(details).toContainText("[0c, 80]");
    stale = true;
    await expect(page.locator(".bus-monitor-stale-lock")).toBeVisible();
    await expect(page.locator(".bus-compose-value")).toBeDisabled();
    await expect(details).toContainText(messages["busMonitor.decode.noProject"]);
    await expect(rows).toHaveCount(8);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    await page.screenshot({ path: info.outputPath("context.png"), fullPage: true });
    expect(unexpected).toEqual([]);
  });
}
