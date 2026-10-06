/** UI-04 in Chromium: the live activity tab in the real diagnostics parent, HTTP intercepted. */
import { expect, test, type Page } from "@playwright/test";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";

function snapshot(state = "running", incarnation = "synthetic-server-a", historyState = "configured") {
  return {
    serverIncarnation: incarnation, coverage: "partial", historyState,
    sessions: [{ kind: "deviceDownload", id: 3, address: "1.1.67", state, completedSteps: state === "running" ? 2 : 5,
      totalSteps: 5, writtenOctets: state === "running" ? 40 : 120, totalOctets: 120 }],
    oneShot: [{ id: 9, kind: "deviceCompare", address: "1.1.5", state: "finished",
      startedAt: "2026-10-06T08:00:00Z", finishedAt: "2026-10-06T08:00:02Z" }],
    oneShotDropped: 0, busyLocks: ["lineScan"], untracked: ["groupWrite", "serialAddress"],
  };
}
async function openLive(page: Page, requests: string[], body: () => unknown, language: "en" | "de" = "en") {
  await page.route("**/api/**", (route) => {
    const request = route.request(); const url = new URL(request.url());
    requests.push(`${request.method()} ${url.pathname}${url.search}`);
    const live = url.pathname === "/api/bus/activity";
    return route.fulfill({ status: live ? 200 : 404, contentType: "application/json",
      body: JSON.stringify(live ? body() : { error: "intercepted fixture only" }) });
  });
  await page.goto(`/e2e/activity-history-fixture.html?lang=${language}`);
  await expect.poll(() => requests).toEqual(["POST /api/bus/discover", "GET /api/bus/monitor/telegrams?since=0"]);
  const messages = language === "de" ? de : en;
  await page.getByRole("button", { name: messages["activityLive.tab"], exact: true }).click();
}

for (const language of ["en", "de"] as const) {
  for (const width of [360, 1440]) {
    test(`${language} ${width}px: live activity states what it is and is not, using only intercepted GETs`, async ({ page }) => {
      const messages = language === "de" ? de : en;
      await page.setViewportSize({ width, height: 850 });
      const requests: string[] = [];
      await openLive(page, requests, () => snapshot(), language);
      const view = page.locator(".activity-live-panel");
      await expect(view).toContainText(messages["activityLive.partial"]);
      await expect(view).toContainText(messages["activityLive.volatile"]);
      await expect(view.locator(".activity-live-session")).toContainText(messages["activityLive.state.running"]);
      await expect(view).toContainText(`${messages["activityLive.busy"]}: ${messages["activityLive.lock.lineScan"]}`);
      await expect(view.locator(".activity-live-oneshot")).toContainText(messages["activityHistory.kind.deviceCompare"]);
      expect(requests.slice(2).every((request) => request === "GET /api/bus/activity")).toBe(true);
      const geometry = await page.evaluate(() => ({ viewport: innerWidth, document: document.documentElement.scrollWidth }));
      expect(geometry.document, JSON.stringify(geometry)).toBeLessThanOrEqual(geometry.viewport);
    });
  }
}

test("a running download finishes, the server restarts and history storage fails — each shown as such", async ({ page }) => {
  const requests: string[] = [];
  let calls = 0;
  await openLive(page, requests, () => {
    calls += 1;
    if (calls === 1) return snapshot();
    if (calls === 2) return snapshot("finished");
    return snapshot("finished", "synthetic-server-b", "unavailable");
  });
  const view = page.locator(".activity-live-panel");
  await expect(view.locator(".activity-live-session")).toContainText(en["activityLive.state.running"]);
  await expect(view.locator(".activity-live-session")).toContainText(en["activityLive.state.finished"], { timeout: 6_000 });
  await expect(view.locator(".activity-live-restart")).toHaveText(en["activityLive.restarted"], { timeout: 6_000 });
  await expect(view.locator(".activity-live-history-state[role=alert]")).toHaveText(en["activityLive.historyState.unavailable"]);
  expect(requests.slice(2).every((request) => request === "GET /api/bus/activity")).toBe(true);
});
