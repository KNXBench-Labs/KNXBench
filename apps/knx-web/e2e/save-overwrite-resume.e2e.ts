/** Owner feedback P1: a reload resumes the server project; Save As asks before replacing. */
import { openNavigationSections } from "./disclosure-fixture";
import { expect, test, type Page } from "@playwright/test";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { fulfillNoProjectOpen } from "./server-project-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";

type Catalogue = { readonly [K in keyof typeof en]: string };
const EXISTING = "fictional-villa.knxdb";

function tree(modified: boolean): ProjectTree & { has_store_path: boolean } {
  return {
    schema_version: 15, errors: 0, warnings: 0, can_undo: modified, can_redo: false, is_modified: modified,
    group_address_style: "ThreeLevel", server_incarnation: "p1-fixture", project_incarnation: 1, snapshot_revision: 1,
    installations: [{ id: 0, name: "Fictional flat", topology: [], buildings: [], unassigned: [], group_addresses: [], group_ranges: [] }],
    has_store_path: false,
  };
}

/** A strict fixture: `server` says whether the server already holds a project. */
async function serve(page: Page, language: "en" | "de", server: "empty" | "modified") {
  const state = {
    project: server === "modified" ? tree(true) : null as (ProjectTree & { has_store_path: boolean }) | null,
    saveAs: [] as unknown[], posts: [] as string[], unexpected: [] as string[],
  };
  await page.route("https://api.github.com/**", (route) => { state.unexpected.push(route.request().url()); return route.abort(); });
  await page.route("**/api/**", (route) => {
    const path = new URL(route.request().url()).pathname;
    const method = route.request().method();
    if (method === "POST") state.posts.push(path);
    const ok = (body: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
    const achievements = achievementsFixtureAnswer(method, path);
    if (achievements !== undefined) return ok(achievements);
    if (path === "/api/auth/status") return ok({ required: false, authenticated: true });
    if (path === "/api/settings" && method === "GET") return ok({ schemaVersion: 1, status: "ok", settings: {
      uiLanguage: language, theme: "porcelain", motionLevel: "standard", releaseChecksEnabled: false, achievementsEnabled: "false",
      onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.11" } } });
    if (path === "/api/settings" && method === "PUT") return ok({ schemaVersion: 1, status: "ok", settings: route.request().postDataJSON().settings });
    if (path === "/api/project" && method === "GET") return state.project ? ok(state.project) : fulfillNoProjectOpen(route);
    if (path === "/api/project/new" && method === "POST") { state.project = tree(true); return ok(state.project); }
    if (path === "/api/project/save-as" && method === "POST") {
      const body = route.request().postDataJSON();
      state.saveAs.push(body);
      // Like the real server: an existing file needs an explicit overwrite.
      if (body.path === EXISTING && body.overwrite !== true) {
        return route.fulfill({ status: 409, contentType: "application/json",
          body: JSON.stringify({ error: "a file with this name already exists; nothing was written", kind: "fileExists" }) });
      }
      // A save is a new snapshot, as on the real server; the app ignores stale revisions.
      state.project = { ...tree(false), has_store_path: true, snapshot_revision: 2, last_saved_at: "2026-10-14T10:00:00Z" };
      return route.fulfill({ status: 200, headers: { "content-length": "0" }, body: "" });
    }
    if (path === "/api/fs/list") return ok([{ name: EXISTING, is_dir: false }]);
    if (path === "/api/project/load-progress") return ok(null);
    if (path === "/api/project/status-metrics" && method === "GET") return ok(statusMetricsFixtureBody(state.project));
    if (path === "/api/bus/discover" && method === "POST") return ok({ interfaces: [] });
    if (method === "GET" && ["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items"].includes(path)) return ok([]);
    state.unexpected.push(`${method} ${path}`);
    return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"offline fixture only"}' });
  });
  return state;
}

async function settled(page: Page, selector: string) {
  await page.locator(selector).first().evaluate((el) => Promise.all(el.getAnimations().map((animation) => animation.finished)));
}

async function fitsViewport(page: Page, selector: string, width: number) {
  const box = await page.locator(selector).first().boundingBox();
  expect(box).not.toBeNull();
  expect(box!.x).toBeGreaterThanOrEqual(0);
  expect(box!.x + box!.width).toBeLessThanOrEqual(width);
  expect(await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth)).toBeLessThanOrEqual(1);
}

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} ${width}px: a reload resumes the server's project and says it is unsaved`, async ({ page }) => {
    const m: Catalogue = language === "de" ? de : en;
    const state = await serve(page, language, "modified");
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/");
    const notice = page.locator(".toast--fun").filter({ hasText: m["resume.noticeUnsaved"] });
    await expect(notice).toHaveCount(1);
    await expect(notice).toHaveAttribute("role", "status");
    await expect(page.locator(".welcome-workspace")).toHaveCount(0);
    await expect(page.locator(".workbench-status")).toContainText("Fictional flat");
    await settled(page, ".toast--fun");
    await fitsViewport(page, ".toast--fun", width);
    if (process.env.KNX_P1_CAPTURE_DIR) await page.screenshot({ path: `${process.env.KNX_P1_CAPTURE_DIR}/resume-${language}-${width}.png` });
    // A read, nothing else: no open, import or replacement was sent.
    expect(state.posts.filter((path) => path !== "/api/bus/discover")).toEqual([]);
    expect(state.unexpected).toEqual([]);
  });

  test(`${language} ${width}px: Save As over an existing file asks, and only Replace overwrites`, async ({ page }) => {
    const m: Catalogue = language === "de" ? de : en;
    const state = await serve(page, language, "empty");
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/");
    await page.getByRole("button", { name: m["toolbar.newProject"], exact: true }).click();
    await page.getByRole("dialog").getByRole("button", { name: m["newProject.create"], exact: true }).click();
    await page.getByRole("dialog").getByRole("button", { name: m["projectWizard.done.close"], exact: true }).click();
    await openNavigationSections(page);

    const saveAs = async () => {
      await page.locator(".file-menu summary").click();
      await page.locator(".file-menu-content").getByRole("menuitem", { name: m["toolbar.saveAs"], exact: true }).click();
      const picker = page.getByRole("dialog", { name: m["fsPicker.saveAs"], exact: true });
      await picker.getByRole("button", { name: EXISTING, exact: true }).click();
      await picker.getByRole("button", { name: m["fsPicker.save"], exact: true }).click();
      const question = page.getByRole("dialog", { name: m["overwrite.title"], exact: true });
      await expect(question).toBeVisible();
      return question;
    };

    // Cancel: one refused request, nothing replaced, still unsaved.
    let question = await saveAs();
    await expect(question).toContainText(`${EXISTING} ${language === "de" ? "gibt es schon" : "already exists"}`);
    await expect(question.getByRole("button", { name: m["overwrite.cancel"], exact: true })).toBeFocused();
    await fitsViewport(page, ".overwrite-confirm", width);
    if (process.env.KNX_P1_CAPTURE_DIR) await page.screenshot({ path: `${process.env.KNX_P1_CAPTURE_DIR}/overwrite-${language}-${width}.png` });
    await question.getByRole("button", { name: m["overwrite.cancel"], exact: true }).click();
    await expect(question).toHaveCount(0);
    expect(state.saveAs).toEqual([{ path: EXISTING }]);
    await expect(page.locator(".toast--error")).toHaveCount(0);

    // Replace: the same file, now with the explicit confirmation.
    question = await saveAs();
    await question.getByRole("button", { name: m["overwrite.replace"], exact: true }).click();
    await expect(question).toHaveCount(0);
    await expect.poll(() => state.saveAs).toEqual([{ path: EXISTING }, { path: EXISTING }, { path: EXISTING, overwrite: true }]);
    await expect(page.locator(".workbench-status-saved")).not.toContainText(m["statusBar.neverSaved"]);
    await expect(page.locator(".toast--error")).toHaveCount(0);
    expect(state.unexpected).toEqual([]);
  });
}
