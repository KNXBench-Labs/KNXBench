/** Linux Chromium proof through the actual diagnostics parent; all API requests are intercepted. */
import { expect, test, type Page } from "@playwright/test";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";

function history(sequence = 1, incarnation = "synthetic-old-server") {
  return { format: 2, coverage: "partial", durability: "persistent", hasMore: false, nextCursor: sequence,
    untracked: ["deviceIdentify", "groupWrite", "serialAddress", "deviceDownload", "addressProgramming", "busMonitor", "lineScan"],
    entries: [{ sequence, serverIncarnation: incarnation, interrupted: true, id: 1, kind: "deviceDownload",
      address: "1.1.67", state: "finished", startedAt: "2026-10-01T12:00:00.000Z", finishedAt: "2026-10-01T12:01:00.000Z",
      writeEvidence: { backupRecorded: true, sendPossible: true },
      downloadEvidence: { sessionId: 7, written: "yes", restart: "unconfirmed", cleanup: "unknown" } }] };
}
async function openHistory(page: Page, requests: string[], language: "en" | "de" = "en") {
  await page.goto(`/e2e/activity-history-fixture.html?lang=${language}`);
  // The real parent initially mounts the monitor. Account for its existing
  // intercepted discovery/poll explicitly, rather than attributing them to history.
  await expect.poll(() => requests).toEqual([
    "POST /api/bus/discover", "GET /api/bus/monitor/telegrams?since=0",
  ]);
  const messages = language === "de" ? de : en;
  await page.getByRole("button", { name: messages["activityHistory.title"], exact: true }).click();
}
function expectHistoryRequests(requests: string[], queries = ["?after=0&limit=50"]) {
  expect(requests).toEqual([
    "POST /api/bus/discover", "GET /api/bus/monitor/telegrams?since=0",
    ...queries.map((query) => `GET /api/bus/history${query}`),
  ]);
}
for (const language of ["en", "de"] as const) {
  for (const width of [360, 1440]) {
    test(`${language} ${width}px: history preserves outcomes and uses only intercepted metadata GETs`, async ({ page }) => {
      const messages = language === "de" ? de : en;
      await page.setViewportSize({ width, height: 850 });
      const requests: string[] = [];
      await page.route("**/api/**", (route) => {
        const request = route.request(); const url = new URL(request.url()); requests.push(`${request.method()} ${url.pathname}${url.search}`);
        return route.fulfill({ status: url.pathname === "/api/bus/history" ? 200 : 404,
          contentType: "application/json", body: JSON.stringify(url.pathname === "/api/bus/history" ? history() : { error: "intercepted fixture only" }) });
      });
      await openHistory(page, requests, language);
      const view = page.locator(".activity-history-panel");
      await expect(view).toContainText(messages["activityHistory.partial"]);
      await expect(view).toContainText(messages["activityHistory.written.yes"]);
      await expect(view).toContainText(messages["activityHistory.restart.unconfirmed"]);
      await expect(view).toContainText(messages["activityHistory.cleanup.unknown"]);
      await expect(view).toContainText(messages["activityHistory.validationBoundary"]);
      await expect(page.locator("html")).toHaveAttribute("lang", language);
      expectHistoryRequests(requests);
      const geometry = await page.evaluate(() => ({ viewport: innerWidth, document: document.documentElement.scrollWidth }));
      expect(geometry.document, JSON.stringify(geometry)).toBeLessThanOrEqual(geometry.viewport);
    });
  }
}

for (const refusal of ["unsupported", "malformed", "unavailable"] as const) {
  test(`${refusal} clears previous success rather than displaying partial/empty history or private errors`, async ({ page }) => {
    let calls = 0;
    const requests: string[] = [];
    await page.route("**/api/**", (route) => {
      const request = route.request(); const url = new URL(request.url());
      requests.push(`${request.method()} ${url.pathname}${url.search}`);
      if (new URL(route.request().url()).pathname !== "/api/bus/history") return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"intercepted fixture only"}' });
      calls += 1; const body = history();
      if (calls > 1 && refusal === "unsupported") body.format = 3;
      if (calls > 1 && refusal === "malformed") Object.assign(body.entries[0], { payload: [1, 2] });
      return route.fulfill({ status: calls > 1 && refusal === "unavailable" ? 503 : 200,
        contentType: "application/json", body: JSON.stringify(calls > 1 && refusal === "unavailable" ? { error: "synthetic-private-error" } : body) });
    });
    await openHistory(page, requests);
    await expect(page.locator(".activity-history-entry")).toHaveCount(1);
    await page.getByRole("button", { name: en["activityHistory.refresh"], exact: true }).click();
    await expect(page.locator(".activity-history-panel [role=alert]")).toHaveText(en[`activityHistory.${refusal}`]);
    await expect(page.locator(".activity-history-entry")).toHaveCount(0);
    await expect(page.locator(".activity-history-panel")).not.toContainText(en["activityHistory.empty"]);
    await expect(page.locator(".activity-history-panel")).not.toContainText("synthetic-private-error");
    expect(calls).toBe(2);
    expectHistoryRequests(requests, ["?after=0&limit=50", "?after=0&limit=50"]);
  });
}

test("sequence paging and refresh do not confuse identical operation ids across server lifetimes", async ({ page }) => {
  const queries: string[] = [];
  const requests: string[] = [];
  await page.route("**/api/**", (route) => {
    const url = new URL(route.request().url());
    requests.push(`${route.request().method()} ${url.pathname}${url.search}`);
    if (url.pathname !== "/api/bus/history") return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"intercepted fixture only"}' });
    queries.push(url.search); const body = url.searchParams.get("after") === "1" ? history(2, "synthetic-new-server") : history();
    body.hasMore = queries.length === 1;
    return route.fulfill({ contentType: "application/json", body: JSON.stringify(body) });
  });
  await openHistory(page, requests);
  await expect(page.locator(".activity-history-entry")).toHaveCount(1);
  await page.getByRole("button", { name: en["activityHistory.more"], exact: true }).click();
  await expect(page.locator(".activity-history-entry")).toHaveCount(2);
  await page.getByRole("button", { name: en["activityHistory.refresh"], exact: true }).click();
  await expect(page.locator(".activity-history-entry")).toHaveCount(1);
  expect(queries).toEqual(["?after=0&limit=50", "?after=1&limit=50", "?after=0&limit=50"]);
  expectHistoryRequests(requests, queries);
});
