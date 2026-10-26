/** Project-history browser contracts with strictly fictional, intercepted APIs. */
import { expect, test, type Page } from "@playwright/test";
import { join } from "node:path";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { HISTORY_FIXTURE } from "../src/projectHistory.fixture";
import { admitProjectHistory } from "../src/projectHistory";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";
import { controlPaint } from "./control-roles";

async function intercept(page: Page, language: "en" | "de", session = false, many = false) {
  const history = admitProjectHistory({...structuredClone(HISTORY_FIXTURE), limits: {...HISTORY_FIXTURE.limits, maxContextBytes: 268435456, maxDecodedImageBytes: 536870912, maxDecodedHistoryBytes: 536870912}});
  if (session) { history.persistence = "session"; history.generation = 0; history.versions = []; history.totalBytes = 0; }
  if (many) {
    const base = HISTORY_FIXTURE.versions[0];
    history.totalBytes = 600000; history.undoSteps = 3; history.redoSteps = 2; history.project.can_redo = true;
    history.versions = [
      { ...base, id: 6, createdAt: "2026-10-09T08:00:00.000Z", reason: "save", label: "Saved six", imageHash: "d".repeat(64) },
      { ...base, id: 5, createdAt: "2026-10-09T08:00:00.000Z", reason: "named", label: "Review milestone with a deliberately long name", imageHash: "e".repeat(64) },
      { ...base, id: 4, createdAt: "2026-10-09T07:30:00.000Z", reason: "pre_restore", label: "Before restore", imageHash: "f".repeat(64) },
      { ...base, id: 3, createdAt: "2026-10-09T07:15:00.000Z", reason: "replaced_workspace", label: "Recovered", imageHash: "1".repeat(64) },
      { ...base, id: 2, createdAt: "2026-10-09T07:00:00.000Z", reason: "named", label: "Before redesign" },
      { ...base, id: 1, createdAt: "2026-10-09T06:00:00.000Z", reason: "save", label: "Saved one", imageHash: "2".repeat(64) },
    ];
  }
  const state = { history, unexpected: [] as string[], writes: [] as { path: string; body: Record<string, unknown> }[], reads: 0 };
  // Like the real server: no project until the test creates one (ADR-0127).
  let projectOpen = false;
  await page.route("**/api/**", (route) => {
    const path = new URL(route.request().url()).pathname; const method = route.request().method();
    let body: unknown; let status = 200;
    if (path === "/api/auth/status" && method === "GET") body = { required: false, authenticated: true };
    else if (path === "/api/settings" && method === "GET") body = { schemaVersion: 1, status: "ok", settings: { uiLanguage: language, theme: language === "en" ? "graphite" : "porcelain" } };
    else if (path === "/api/project" && method === "GET") { if (projectOpen) body = history.project; else { status = 400; body = { error: "no project open" }; } }
    else if (path === "/api/project/new" && method === "POST") { projectOpen = true; body = history.project; }
    else if (path === "/api/project/load-progress" && method === "GET") body = null;
    else if (path === "/api/project/status-metrics" && method === "GET") body = statusMetricsFixtureBody(history.project);
    else if (path === "/api/bus/discover" && method === "POST") body = { interfaces: [] };
    else if (path === "/api/product-languages" && method === "GET") body = [];
    else if (path === "/api/project/history" && method === "GET") { state.reads++; body = history; }
    else if (path.startsWith("/api/project/history/") && method === "POST") {
      const request = route.request().postDataJSON() as Record<string, unknown>;
      state.writes.push({ path, body: request });
      if (request.serverIncarnation !== history.serverIncarnation || request.snapshotRevision !== history.snapshotRevision || request.generation !== history.generation) {
        status = 409; body = { error: "Stale fictional history request" };
      } else {
        history.generation++; history.snapshotRevision++; history.project.snapshot_revision = history.snapshotRevision;
        if (path === "/api/project/history/versions") history.versions.unshift({ id: 3, createdAt: "2026-10-09T09:00:00.000Z", reason: "named", label: String(request.label), bytes: 100000, imageHash: "b".repeat(64) });
        else if (path === "/api/project/history/versions/2/restore" && request.confirmed === true) {
          history.undoSteps = 0; history.redoSteps = 0; history.project.can_undo = false; history.project.can_redo = false;
          history.versions.unshift({ id: 4, createdAt: "2026-10-09T09:10:00.000Z", reason: "pre_restore", label: "Before restore", bytes: 100000, imageHash: "c".repeat(64) });
        } else { state.unexpected.push(`${method} ${path}`); status = 404; }
        body = history;
      }
    } else if (["/api/device-download/status", "/api/device-address/status"].includes(path) && method === "GET") { status = 404; body = { error: "no operation in this fixture" }; }
    else if (achievementsFixtureAnswer(method, path) !== undefined) body = achievementsFixtureAnswer(method, path);
    else { state.unexpected.push(`${method} ${path}`); status = 404; body = { error: "strict history fixture only" }; }
    return route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
  });
  return state;
}
async function openProject(page: Page, language: "en" | "de") {
  const m = language === "en" ? en : de;
  await page.goto("/");
  await page.getByRole("button", { name: m["toolbar.newProject"], exact: true }).first().click();
  await page.getByRole("dialog", { name: m["newProject.title"], exact: true }).getByRole("button", { name: m["newProject.create"], exact: true }).click();
  await page.getByRole("dialog", { name: m["projectWizard.done.title"], exact: true }).getByRole("button", { name: m["projectWizard.done.close"], exact: true }).click();
}
// The plain list is the alternate route: collapsed by default, opened only by tests of that route.
async function openAlternateList(panel: ReturnType<Page["locator"]>) {
  await panel.locator("details.project-history-alternate > summary").click();
  await expect(panel.locator(".project-history-versions").first()).toBeVisible();
}
async function openHistory(page: Page, language: "en" | "de") {
  const m = language === "en" ? en : de;
  await page.keyboard.press("Control+Shift+P");
  await page.getByRole("combobox").fill(m["projectHistory.title"]);
  await page.getByRole("combobox").press("Enter");
  const panel = page.getByRole("dialog", { name: m["projectHistory.title"], exact: true });
  await expect(panel).toBeVisible();
  return panel;
}
for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  const m = language === "en" ? en : de;
  test(`${language} named versions, cancellation and confirmed restore at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 850 });
    const state = await intercept(page, language); await openProject(page, language);
    let panel = await openHistory(page, language);
    await expect(panel).toContainText(m["projectHistory.native"]);
    await expect(panel.locator('[data-history-limit="context"]')).toHaveText(m["projectHistory.contextLimit"].replace("{context}","256"));
    await expect(panel.locator('[data-history-limit="decoded"]')).toHaveText(m["projectHistory.decodedLimit"].replace("{decoded}","512"));
    await expect(panel.locator('[data-history-limit="total-decoded"]')).toHaveText(m["projectHistory.decodedHistoryLimit"].replace("{decoded}","512"));
    await expect(panel).toContainText(m["projectHistory.backupCaveat"]);
    await expect(panel.locator("[data-history-stat=undo] dd")).toHaveText("1");
    await panel.getByLabel(m["projectHistory.label"], { exact: true }).fill("Checkpoint for review");
    await panel.getByRole("button", { name: m["projectHistory.create"], exact: true }).click();
    await openAlternateList(panel);
    await expect(panel.locator(".project-history-versions strong").first()).toHaveText("Checkpoint for review");
    expect(state.writes).toHaveLength(1);
    expect(state.writes[0].body).toMatchObject({ serverIncarnation: "history-fixture", snapshotRevision: 5, generation: 3, label: "Checkpoint for review", confirmed: false });
    // Selection (version 2) survives the refresh by id; a search that hides it gates restore and delete.
    await panel.getByRole("searchbox").fill("Checkpoint for review");
    await expect(panel.locator(".project-history-versions li")).toHaveCount(1);
    await expect(panel.locator("[data-timeline-hidden-selection]")).toBeVisible();
    await expect(panel.getByRole("button", { name: m["projectHistory.restore"], exact: true })).toBeDisabled();
    await expect(panel.getByRole("button", { name: m["projectHistory.delete"], exact: true })).toBeDisabled();
    await panel.getByRole("searchbox").fill("Before redesign");
    await expect(panel.locator(".project-history-versions li")).toHaveCount(1);
    await panel.locator(".project-history-versions button").click();
    await panel.getByRole("button", { name: m["projectHistory.restore"], exact: true }).click();
    await expect(panel.getByRole("button", { name: m["projectHistory.cancel"], exact: true })).toBeFocused();
    const cancelInk = await panel.getByRole("button", { name: m["projectHistory.cancel"], exact: true }).evaluate((node) => getComputedStyle(node).color);
    expect(cancelInk).toBe(await panel.evaluate((node) => getComputedStyle(node).color));
    expect(state.writes).toHaveLength(1);
    await panel.getByRole("button", { name: m["projectHistory.cancel"], exact: true }).click();
    expect(state.writes).toHaveLength(1);
    await panel.getByRole("button", { name: m["projectHistory.restore"], exact: true }).click();
    const box = await panel.boundingBox(); expect(box).not.toBeNull(); expect(box!.x).toBeGreaterThanOrEqual(0); expect(box!.x + box!.width).toBeLessThanOrEqual(width);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    if (process.env.KNX_HISTORY_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_HISTORY_CAPTURE_DIR, `history-confirm-${language}-${width}.png`) });
    await panel.getByRole("button", { name: m["projectHistory.confirmRestore"], exact: true }).click();
    await expect(panel).toHaveCount(0);
    expect(state.writes).toHaveLength(2);
    expect(state.writes[1]).toEqual({ path: "/api/project/history/versions/2/restore", body: { serverIncarnation: "history-fixture", snapshotRevision: 6, generation: 4, confirmed: true } });
    panel = await openHistory(page, language);
    await expect(panel.locator("[data-history-stat=undo] dd")).toHaveText("0");
    await openAlternateList(panel);
    await expect(panel.locator(".project-history-versions")).toContainText(m["projectHistory.reason.pre_restore"]);
    if (process.env.KNX_HISTORY_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_HISTORY_CAPTURE_DIR, `history-versions-${language}-${width}.png`) });
    expect(state.unexpected).toEqual([]);
  });
}
for (const language of ["en", "de"] as const) test(`${language} session-only history cannot create native versions`, async ({ page }) => {
  const m = language === "en" ? en : de;
  const state = await intercept(page, language, true); await openProject(page, language);
  const panel = await openHistory(page, language);
  await expect(panel).toContainText(m["projectHistory.session"]);
  await expect(panel.getByRole("button", { name: m["projectHistory.create"], exact: true })).toBeDisabled();
  expect(state.writes).toEqual([]); expect(state.unexpected).toEqual([]);
});
// Review UI02-001: every history confirmation destroys or replaces state, so it
// paints as the shared destructive variant and its Cancel stays secondary.
for (const theme of ["porcelain", "graphite"] as const) test(`restore, delete and clear confirmations render as danger (${theme})`, async ({ page }) => {
  const m = en;
  const state = await intercept(page, "en");
  await page.route("**/api/settings", (route) => route.request().method() === "GET"
    ? route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify({ schemaVersion: 1, status: "ok", settings: { uiLanguage: "en", theme } }) })
    : route.fallback());
  await page.setViewportSize({ width: 1440, height: 850 });
  await openProject(page, "en");
  await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
  const panel = await openHistory(page, "en");
  await panel.locator("ol.project-history-timeline li[data-version-id] button").first().click();
  for (const [open, confirm] of [["projectHistory.restore", "projectHistory.confirmRestore"], ["projectHistory.delete", "projectHistory.confirmDelete"], ["projectHistory.clearUndo", "projectHistory.confirmClear"]] as const) {
    await panel.getByRole("button", { name: m[open], exact: true }).click();
    const confirmation = panel.getByRole("button", { name: m[confirm], exact: true });
    await expect(confirmation).toBeEnabled();
    await page.mouse.move(1, 1);
    const paint = await controlPaint(confirmation);
    expect(paint.role, `${m[confirm]}: ${JSON.stringify(paint)}`).toBe("danger");
    const cancel = panel.getByRole("button", { name: m["projectHistory.cancel"], exact: true });
    expect((await controlPaint(cancel)).role, `${m[confirm]} cancel`).toBe("secondary");
    if (process.env.KNX_HISTORY_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_HISTORY_CAPTURE_DIR, `history-${open.split(".")[1]}-${theme}.png`) });
    await cancel.click();
  }
  expect(state.writes).toEqual([]);
  expect(state.unexpected).toEqual([]);
});

// WP-22: the chronological timeline selects only; restore keeps its own confirmation.
for (const language of ["en", "de"] as const) for (const width of [1440, 400]) test(`${language} version timeline orders, selects without writing and stays contained at ${width}px`, async ({ page }) => {
  const m = language === "en" ? en : de;
  await page.setViewportSize({ width, height: 850 });
  const state = await intercept(page, language, false, true); await openProject(page, language);
  const panel = await openHistory(page, language);
  const timeline = panel.locator("ol.project-history-timeline");
  await expect(timeline).toBeVisible();
  await expect(timeline.locator("li[data-version-id]")).toHaveCount(6);
  expect(await timeline.locator("li[data-version-id]").evaluateAll((nodes) => nodes.map((n) => (n as HTMLElement).dataset.versionId))).toEqual(["1", "2", "3", "4", "5", "6"]);
  await expect(timeline.locator("li").last()).toContainText(m["projectHistory.timelineCurrent"]);
  await expect(timeline.locator("li").last().locator("button")).toHaveCount(0);
  await expect(panel.locator(".project-history-timeline-note")).toHaveText(m["projectHistory.timelineNote"]);
  await expect(timeline.locator("li[data-timeline-current]")).toHaveAttribute("aria-current", "step");
  // Timeline is primary: the collapsed alternate list adds no tab stop per version.
  const alternate = panel.locator("details.project-history-alternate");
  await expect(alternate).not.toHaveAttribute("open", "");
  await expect(alternate.locator("summary")).toHaveText(m["projectHistory.versions"]);
  await expect(alternate.locator(".project-history-versions button").first()).toBeHidden();
  const target = timeline.locator('li[data-version-id="3"] button');
  await target.scrollIntoViewIfNeeded(); await target.click();
  await expect(target).toHaveAttribute("aria-pressed", "true");
  expect(state.writes).toEqual([]);
  await target.focus(); await page.keyboard.press("Tab");
  await expect(timeline.locator('li[data-version-id="4"] button')).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(timeline.locator('li[data-version-id="4"] button')).toHaveAttribute("aria-pressed", "true");
  expect(state.writes).toEqual([]);
  const scroll = panel.locator(".project-history-timeline-scroll");
  const metrics = await scroll.evaluate((node) => ({ client: node.clientWidth, scroll: node.scrollWidth }));
  if (width === 400) expect(metrics.scroll).toBeGreaterThan(metrics.client);
  const box = await panel.boundingBox(); expect(box!.x).toBeGreaterThanOrEqual(0); expect(box!.x + box!.width).toBeLessThanOrEqual(width);
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
  if (process.env.KNX_HISTORY_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_HISTORY_CAPTURE_DIR, `history-timeline-${language}-${width}.png`) });
  await panel.getByRole("searchbox").fill("Recovered");
  await expect(panel.locator("[data-timeline-hidden-selection]")).toBeVisible();
  await expect(panel.getByRole("button", { name: m["projectHistory.restore"], exact: true })).toBeDisabled();
  await expect(panel.getByRole("button", { name: m["projectHistory.delete"], exact: true })).toBeDisabled();
  await panel.getByRole("searchbox").fill("");
  await panel.getByRole("button", { name: m["projectHistory.restore"], exact: true }).click();
  expect(state.writes).toEqual([]);
  await expect(panel.getByRole("button", { name: m["projectHistory.confirmRestore"], exact: true })).toBeVisible();
  await panel.getByRole("button", { name: m["projectHistory.cancel"], exact: true }).click();
  expect(state.writes).toEqual([]); expect(state.unexpected).toEqual([]);
});
test("session-only timeline shows just the current working state", async ({ page }) => {
  const state = await intercept(page, "en", true); await openProject(page, "en");
  const panel = await openHistory(page, "en");
  await expect(panel.locator("ol.project-history-timeline li[data-version-id]")).toHaveCount(0);
  await expect(panel.locator("ol.project-history-timeline li[data-timeline-current]")).toContainText(en["projectHistory.timelineCurrent"]);
  expect(state.writes).toEqual([]); expect(state.unexpected).toEqual([]);
});

// Cancelling a confirmation remounts the timeline; the selected older checkpoint must stay in view.
for (const language of ["en", "de"] as const) for (const width of [1440, 400]) test(`${language} cancelled confirmation keeps the selected older checkpoint visible at ${width}px`, async ({ page }) => {
  const m = language === "en" ? en : de;
  await page.setViewportSize({ width, height: 850 });
  const state = await intercept(page, language, false, true); await openProject(page, language);
  const panel = await openHistory(page, language);
  const scroll = panel.locator(".project-history-timeline-scroll");
  const oldest = panel.locator('ol.project-history-timeline li[data-version-id="1"] button');
  const geometry = async () => ({ ...(await oldest.evaluate((node) => {
    const b = node.getBoundingClientRect(); const c = node.closest(".project-history-timeline-scroll")!.getBoundingClientRect();
    return { left: b.left - c.left, right: c.right - b.right };
  })), ...(await scroll.evaluate((node) => ({ scrollLeft: node.scrollLeft, max: node.scrollWidth - node.clientWidth }))) });
  expect((await geometry()).max, "timeline must overflow for this test to mean anything").toBeGreaterThan(100);
  await oldest.scrollIntoViewIfNeeded(); await oldest.click();
  const reads = state.reads;
  await panel.getByRole("button", { name: m["projectHistory.restore"], exact: true }).click();
  await panel.getByRole("button", { name: m["projectHistory.cancel"], exact: true }).click();
  await expect(oldest).toHaveAttribute("aria-pressed", "true");
  const after = await geometry();
  expect(after.left, JSON.stringify(after)).toBeGreaterThanOrEqual(-1);
  expect(after.right, JSON.stringify(after)).toBeGreaterThanOrEqual(-1);
  expect(after.scrollLeft, JSON.stringify(after)).toBeLessThan(after.max);
  expect(state.reads).toBe(reads); expect(state.writes).toEqual([]); expect(state.unexpected).toEqual([]);
});
