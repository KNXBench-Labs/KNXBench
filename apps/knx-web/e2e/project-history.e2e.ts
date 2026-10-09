/** Project-history browser contracts with strictly fictional, intercepted APIs. */
import { expect, test, type Page } from "@playwright/test";
import { join } from "node:path";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { HISTORY_FIXTURE } from "../src/projectHistory.fixture";
import { admitProjectHistory } from "../src/projectHistory";
import { achievementsFixtureAnswer } from "./achievements-fixture";

async function intercept(page: Page, language: "en" | "de", session = false) {
  const history = admitProjectHistory(structuredClone(HISTORY_FIXTURE));
  if (session) { history.persistence = "session"; history.generation = 0; history.versions = []; history.totalBytes = 0; }
  const state = { history, unexpected: [] as string[], writes: [] as { path: string; body: Record<string, unknown> }[] };
  await page.route("**/api/**", (route) => {
    const path = new URL(route.request().url()).pathname; const method = route.request().method();
    let body: unknown; let status = 200;
    if (path === "/api/auth/status" && method === "GET") body = { required: false, authenticated: true };
    else if (path === "/api/settings" && method === "GET") body = { schemaVersion: 1, status: "ok", settings: { uiLanguage: language, theme: language === "en" ? "graphite" : "porcelain" } };
    else if (path === "/api/project" && method === "GET") body = history.project;
    else if (path === "/api/project/new" && method === "POST") body = history.project;
    else if (path === "/api/project/load-progress" && method === "GET") body = null;
    else if (path === "/api/bus/discover" && method === "POST") body = { interfaces: [] };
    else if (path === "/api/product-languages" && method === "GET") body = [];
    else if (path === "/api/project/history" && method === "GET") body = history;
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
    await expect(panel).toContainText(m["projectHistory.backupCaveat"]);
    await expect(panel.locator("[data-history-stat=undo] dd")).toHaveText("1");
    await panel.getByLabel(m["projectHistory.label"], { exact: true }).fill("Checkpoint for review");
    await panel.getByRole("button", { name: m["projectHistory.create"], exact: true }).click();
    await expect(panel.locator(".project-history-versions strong").first()).toHaveText("Checkpoint for review");
    expect(state.writes).toHaveLength(1);
    expect(state.writes[0].body).toMatchObject({ serverIncarnation: "history-fixture", snapshotRevision: 5, generation: 3, label: "Checkpoint for review", confirmed: false });
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
