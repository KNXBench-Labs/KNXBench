/** Built workbench notes: identity, targets, apply refresh, refusals, draft guard. */
import { openNavigationSections } from "./disclosure-fixture";
import { expect, test, type Dialog, type Page } from "@playwright/test";
import { join } from "node:path";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import type { NoteAction, NoteView } from "../src/features/notes/client";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";

type Language = "en" | "de";
type Catalogue = { readonly [K in keyof typeof en]: string };
const CAPTURE = process.env.KNX_NOTES_CAPTURE_DIR;
type ApplyMode = "ok" | "stale" | "persistence";

interface Fixture {
  unexpected: string[];
  notes: NoteView[];
  revision: number;
  projectReads: number;
  applyMode: ApplyMode;
  applied: number;
  targetReads: number;
}

/** A fictional project with identity; its notes live in this in-memory service. */
function projectTree(revision: number): ProjectTree {
  return {
    schema_version: 13, errors: 0, warnings: 0, can_undo: revision > 1, can_redo: false, is_modified: revision > 1,
    server_incarnation: "notes-fixture", project_incarnation: 4, snapshot_revision: revision,
    group_address_style: "ThreeLevel",
    installations: [{
      id: 0, name: "Fictional house",
      topology: [{ id: 21, name: "Main area", address: 1, lines: [{ id: 31, name: "Ground floor", address: 1, devices: [
        { id: 41, name: "Switch actuator kitchen", address: "1.1.4", description: null, com_object_count: 4 },
      ] }] }],
      buildings: [{ id: 51, name: "House", kind: "Building", devices: [], children: [
        { id: 52, name: "Kitchen", kind: "Room", children: [], devices: [] },
      ] }],
      unassigned: [],
      group_ranges: [{ id: 61, name: "Lighting", start: "1/0/0", end: "1/7/255", parent: null }],
      group_addresses: [{ id: 71, name: "Kitchen ceiling", address: "1/0/1", range: 61, dpts: [], links: [] }],
    }],
  };
}

/** What `knx_app::note_targets` answers for `projectTree` (one object added). */
function targetsOf(f: Fixture) {
  return { serverIncarnation: "notes-fixture", projectIncarnation: 4, snapshotRevision: f.revision, targets: [
    { target: { kind: "installation", id: 0 }, owner: [], parts: ["Fictional house"] },
    { target: { kind: "area", id: 21 }, owner: [], parts: ["1", "Main area"] },
    { target: { kind: "line", id: 31 }, owner: [], parts: ["1.1", "Ground floor"] },
    { target: { kind: "buildingPart", id: 51 }, owner: [], parts: ["House"] },
    { target: { kind: "buildingPart", id: 52 }, owner: [], parts: ["Kitchen"] },
    { target: { kind: "device", id: 41 }, owner: [], parts: ["1.1.4", "Switch actuator kitchen"] },
    { target: { kind: "comObject", id: 401 }, owner: ["1.1.4", "Switch actuator kitchen"], parts: ["0", "Switching"] },
    { target: { kind: "groupRange", id: 61 }, owner: [], parts: ["1/0/0–1/7/255", "Lighting"] },
    { target: { kind: "groupAddress", id: 71 }, owner: [], parts: ["1/0/1", "Kitchen ceiling"] },
  ] };
}

function viewOf(f: Fixture) {
  return { serverIncarnation: "notes-fixture", projectIncarnation: 4, snapshotRevision: f.revision,
    persistence: "session-only", notes: f.notes };
}

function applyAction(f: Fixture, action: NoteAction) {
  const notes = f.notes.map((note) => ({ ...note }));
  if (action.kind === "create") notes.splice(action.position, 0, { ...action.draft, id: 100 + f.applied, position: 0 });
  else if (action.kind === "replace") Object.assign(notes.find((note) => note.id === action.id)!, action.draft);
  else if (action.kind === "remove") notes.splice(notes.findIndex((note) => note.id === action.id), 1);
  else { const index = notes.findIndex((note) => note.id === action.id); const [moved] = notes.splice(index, 1); notes.splice(action.position, 0, moved); }
  f.notes = notes.map((note, position) => ({ ...note, position }));
}

async function fixture(page: Page, language: Language, theme: string, width: number): Promise<Fixture> {
  const f: Fixture = { unexpected: [], revision: 1, projectReads: 0, applyMode: "ok", applied: 0, targetReads: 0, notes: [
    { id: 1, position: 0, target: { kind: "device", id: 41 }, title: "Check the kitchen actuator", body: "Relay clicks twice.\nRecheck after commissioning.", includeInDocumentation: true },
  ] };
  const plans = new Map<string, NoteAction>();
  await page.setViewportSize({ width, height: 900 });
  await page.route("https://api.github.com/**", (route) => { f.unexpected.push(route.request().url()); return route.abort(); });
  await page.route("**/api/**", (route) => {
    const path = new URL(route.request().url()).pathname;
    const method = route.request().method();
    const ok = (body: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
    const refuse = (status: number, kind: string, error: string) => route.fulfill({ status, contentType: "application/json", body: JSON.stringify({ error, kind }) });
    const achievements = achievementsFixtureAnswer(method, path);
    if (achievements !== undefined) return ok(achievements);
    if (path === "/api/auth/status") return ok({ required: false, authenticated: true });
    if (path === "/api/settings" && method === "GET") return ok({ conditionalPatchVersion: 1, schemaVersion: 1, status: "ok", settings: {
      uiLanguage: language, theme, motionLevel: "off", releaseChecksEnabled: false,
      onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.9" } } });
    if (path === "/api/settings" && method === "PUT") return ok({ conditionalPatchVersion: 1, schemaVersion: 1, status: "ok", settings: route.request().postDataJSON().settings });
    if (path === "/api/version") return ok({ version: "0.1.0-alpha.9+notes-fixture" });
    if (path === "/api/project" && method === "GET") { f.projectReads += 1; return ok(projectTree(f.revision)); }
    if (path === "/api/project/load-progress") return ok(null);
    if (path === "/api/project/status-metrics" && method === "GET") return ok(statusMetricsFixtureBody(projectTree(f.revision)));
    if (path === "/api/log") return ok([]);
    if (path === "/api/legacy-password" && method === "GET") return ok({ available: true, remembered: false, problem: null });
    if (path === "/api/fs/list") return ok([{ name: "other-fictional.knxproj", is_dir: false }]);
    if (path === "/api/bus/discover" && method === "POST") return ok({ interfaces: [] });
    if (["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items"].includes(path)) return ok([]);
    if (["/api/device-download/status", "/api/device-address/status"].includes(path)) return refuse(404, "none", "no fixture operation");
    if (path === "/api/project/notes" && method === "GET") return ok(viewOf(f));
    if (path === "/api/project/notes/targets" && method === "GET") { f.targetReads += 1; return ok(targetsOf(f)); }
    if (path === "/api/project/notes/preview" && method === "POST") {
      const body = route.request().postDataJSON();
      if (body.serverIncarnation !== "notes-fixture" || body.projectIncarnation !== 4 || body.snapshotRevision !== f.revision) return refuse(409, "noteStale", "stale note context");
      const action = body.action as NoteAction;
      const token = `plan-${plans.size + 1}`;
      plans.set(token, action);
      const previous = action.kind === "create" ? null : f.notes.find((note) => note.id === action.id) ?? null;
      return ok({ token, action, previous, createdId: action.kind === "create" ? 100 + f.applied : null, noOp: false, expiresInSeconds: 300 });
    }
    if (path === "/api/project/notes/apply" && method === "POST") {
      const body = route.request().postDataJSON();
      const action = plans.get(body.token);
      if (!action || body.confirmed !== true) return refuse(409, "noteStale", "unknown plan");
      if (f.applyMode === "stale") return refuse(409, "noteStale", "The project changed since this preview.");
      if (f.applyMode === "persistence") return refuse(422, "notePersistenceFailed", "The note could not be persisted.");
      plans.delete(body.token);
      applyAction(f, action);
      f.applied += 1; f.revision += 1;
      return ok({ changed: true, nativeGeneration: null, snapshotRevision: f.revision, persistence: "session-only" });
    }
    if (path === "/api/project/notes/cancel" && method === "POST") { plans.delete(route.request().postDataJSON().token); return ok({ cancelled: true }); }
    f.unexpected.push(`${method} ${path}`);
    return refuse(404, "strict", "strict notes fixture");
  });
  await page.goto("/?view=editor");
  await expect(page.locator("html")).toHaveAttribute("lang", language);
  await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
  return f;
}

async function noHorizontalOverflow(page: Page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
}

async function openNotes(page: Page, m: Catalogue, width: number) {
  await openNavigationSections(page);
  if (width >= 1000) {
    await openNavigationSections(page);
    await page.locator(".workbench-navigation").getByRole("button", { name: m["workbench.notes"], exact: true }).click();
  } else {
    await page.keyboard.press("Control+Shift+P");
    await page.getByRole("combobox").fill(m["workbench.notes"]);
    await page.getByRole("option").filter({ hasText: m["workbench.notes"] }).first().click();
  }
  const workspace = page.locator(".notes-workspace");
  await expect(workspace).toBeVisible();
  return workspace;
}

async function capture(page: Page, name: string) {
  if (CAPTURE) await page.screenshot({ path: join(CAPTURE, `${name}.png`), fullPage: false });
}

for (const language of ["en", "de"] as const) for (const theme of ["porcelain", "graphite"]) for (const width of [1440, 400]) {
  test(`${language} ${theme} ${width}: notes workspace creates through review and refresh`, async ({ page }) => {
    const m: Catalogue = language === "de" ? de : en;
    const tag = `${language}-${theme}-${width}`;
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    const f = await fixture(page, language, theme, width);
    const workspace = await openNotes(page, m, width);
    const editor = workspace.locator("section").first();
    await expect(editor.getByRole("heading", { level: 3 })).toHaveText(language === "de" ? "Projektnotizen" : "Project notes");
    const target = editor.locator("select");
    await expect(target).toHaveValue("device:41");
    await expect(target.locator("option")).toHaveText([
      language === "de" ? "Projekt" : "Project",
      `${m["notes.target.installation"]}: Fictional house`,
      `${m["notes.target.area"]}: 1 Main area`,
      `${m["notes.target.line"]}: 1.1 Ground floor`,
      `${m["notes.target.buildingPart"]}: House`,
      `${m["notes.target.buildingPart"]}: Kitchen`,
      `${m["notes.target.device"]}: 1.1.4 Switch actuator kitchen`,
      `${m["notes.target.comObject"]}: 0 Switching · 1.1.4 Switch actuator kitchen`,
      `${m["notes.target.groupRange"]}: 1/0/0–1/7/255 Lighting`,
      `${m["notes.target.groupAddress"]}: 1/0/1 Kitchen ceiling`,
    ]);
    // The opt-in checkbox sits on its label's first text line: the global
    // control min-height must not stretch it into a taller, lower box.
    const box = await editor.locator('input[type="checkbox"]').evaluate((input) => {
      const own = input.getBoundingClientRect(); const label = input.parentElement!.getBoundingClientRect();
      return { height: own.height, offset: own.top - label.top };
    });
    expect(box.height).toBeLessThanOrEqual(20);
    expect(box.offset).toBeLessThanOrEqual(6);
    await noHorizontalOverflow(page);
    await capture(page, `${tag}-1-open`);

    const newNote = language === "de" ? "Neue Notiz" : "New note";
    await editor.getByRole("button", { name: newNote, exact: true }).click();
    await editor.getByLabel(language === "de" ? "Titel" : "Title", { exact: true }).fill("Label the ceiling circuit");
    await editor.getByLabel(language === "de" ? "Notiztext" : "Note text", { exact: true }).fill("Use <b>tape</b>, not HTML.\nTwo lines.");
    await target.selectOption("comObject:401");
    await editor.locator("textarea").first().evaluate((node) => node.setAttribute("data-settings-retained", "notes"));
    await page.keyboard.press("Control+Shift+P");
    await page.getByRole("combobox").fill(m["toolbar.settings"]);
    await page.getByRole("combobox").press("Enter");
    await expect(workspace).toBeHidden();
    await page.getByRole("button", { name: m["settings.back"], exact: true }).click();
    await expect(workspace).toBeVisible();
    await expect(editor.locator("[data-settings-retained=notes]")).toHaveCount(1);
    await expect(editor.getByLabel(language === "de" ? "Titel" : "Title", { exact: true })).toHaveValue("Label the ceiling circuit");
    await expect(editor.getByLabel(language === "de" ? "Notiztext" : "Note text", { exact: true })).toHaveValue("Use <b>tape</b>, not HTML.\nTwo lines.");
    await expect(target).toHaveValue("comObject:401");
    expect(f.applied).toBe(0);
    const readsBefore = f.projectReads;
    await editor.getByRole("button", { name: language === "de" ? "Änderung prüfen" : "Preview change", exact: true }).click();
    const review = editor.getByRole("region", { name: language === "de" ? "Änderung prüfen" : "Review change" });
    await expect(review).toContainText("Use <b>tape</b>, not HTML.");
    await expect(review).toContainText(language === "de" ? "Aktion: Notiz anlegen" : "Action: Create note");
    await expect(review).toContainText(language === "de" ? "In die Projektdokumentation aufnehmen: Nein" : "Include in project documentation: No");
    await expect(review).toContainText(`${m["notes.target.comObject"]}: 0 Switching · 1.1.4 Switch actuator kitchen`);
    expect(f.applied).toBe(0);
    await noHorizontalOverflow(page);
    await capture(page, `${tag}-2-review`);
    await review.getByRole("button", { name: language === "de" ? "Änderung bestätigen" : "Confirm change", exact: true }).click();
    await expect(editor.locator("nav li button")).toHaveText(["Check the kitchen actuator", "Label the ceiling circuit"]);
    expect(f.applied).toBe(1);
    expect(f.notes[1]).toMatchObject({ title: "Label the ceiling circuit", body: "Use <b>tape</b>, not HTML.\nTwo lines.", target: { kind: "comObject", id: 401 }, includeInDocumentation: false });
    // The confirmed change bumps the revision; the host reloads the catalogue for it.
    await expect.poll(() => f.targetReads).toBeGreaterThan(1);
    expect(f.projectReads).toBeGreaterThan(readsBefore);
    await expect(editor.locator("b")).toHaveCount(0);
    await noHorizontalOverflow(page);
    await capture(page, `${tag}-3-applied`);
    expect(f.unexpected).toEqual([]);
    expect(errors).toEqual([]);
  });
}

for (const [mode, language, width] of [["stale", "en", 1440], ["persistence", "de", 400]] as const) {
  test(`${language} ${width}: a ${mode} refusal on Apply leaves the notes unchanged`, async ({ page }) => {
    const m: Catalogue = language === "de" ? de : en;
    const f = await fixture(page, language, "graphite", width);
    const editor = (await openNotes(page, m, width)).locator("section").first();
    f.applyMode = mode;
    const before = structuredClone(f.notes);
    const revision = f.revision;
    await editor.getByLabel(language === "de" ? "Titel" : "Title", { exact: true }).fill("A rewrite that must not land");
    await editor.getByRole("button", { name: language === "de" ? "Änderung prüfen" : "Preview change", exact: true }).click();
    await editor.getByRole("button", { name: language === "de" ? "Änderung bestätigen" : "Confirm change", exact: true }).click();
    await expect(editor.getByRole("alert")).toContainText(mode === "stale" ? "The project changed since this preview." : "The note could not be persisted.");
    // The refusal must be seen where the user just clicked, even on a narrow screen.
    await expect(editor.getByRole("alert")).toBeInViewport();
    expect(f.notes).toEqual(before);
    expect(f.revision).toBe(revision);
    await expect(editor.locator("nav li button")).toHaveText(["Check the kitchen actuator"]);
    await capture(page, `refusal-${mode}-${language}-${width}`);
    expect(f.unexpected).toEqual([]);
  });
}

test("en 1440: opening another project over a note draft asks first; keeping it changes nothing", async ({ page }) => {
  const m: Catalogue = en;
  const f = await fixture(page, "en", "porcelain", 1440);
  const editor = (await openNotes(page, m, 1440)).locator("section").first();
  await editor.getByLabel("Title", { exact: true }).fill("Unapplied thought");
  await page.keyboard.press("Control+Shift+P");
  await page.getByRole("combobox").fill(m["toolbar.openProject"]);
  await page.getByRole("option").filter({ hasText: m["toolbar.openProject"] }).click();
  await page.getByRole("button", { name: "other-fictional.knxproj", exact: true }).click();
  const guard = page.getByRole("dialog", { name: m["draftGuard.title"], exact: true });
  await expect(guard).toBeVisible();
  await expect(guard.getByRole("button", { name: m["draftGuard.keep"], exact: true })).toBeFocused();
  await capture(page, "draft-guard-en-1440");
  await guard.getByRole("button", { name: m["draftGuard.keep"], exact: true }).click();
  await expect(guard).toHaveCount(0);
  await expect(editor.getByLabel("Title", { exact: true })).toHaveValue("Unapplied thought");
  expect(f.applied).toBe(0);
  expect(f.unexpected).toEqual([]);
});

test("en 1440: reloading or closing the tab over a note draft asks the browser first", async ({ page }) => {
  const m: Catalogue = en;
  const f = await fixture(page, "en", "porcelain", 1440);
  // Without a draft the page navigates away (here: back to itself) without asking.
  const quiet: string[] = [];
  const listen = (dialog: Dialog) => { quiet.push(dialog.type()); void dialog.dismiss(); };
  page.on("dialog", listen);
  await openNotes(page, m, 1440);
  await page.goto("/?view=editor");
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  expect(quiet).toEqual([]);
  page.off("dialog", listen);

  const reopened = (await openNotes(page, m, 1440)).locator("section").first();
  await reopened.getByLabel("Title", { exact: true }).fill("Unapplied thought");
  const asked = page.waitForEvent("dialog");
  void page.close({ runBeforeUnload: true });
  const dialog = await asked;
  expect(dialog.type()).toBe("beforeunload");
  await dialog.dismiss();
  // Staying keeps the draft and changes nothing.
  await expect(reopened.getByLabel("Title", { exact: true })).toHaveValue("Unapplied thought");
  expect(f.applied).toBe(0);
  expect(f.unexpected).toEqual([]);
});

// Goal B2: a reload is what the desktop webview can do (WebKitGTK's context
// menu offers Reload); the guard is the same listener in every engine. Typed
// with the keyboard: Firefox only asks after a real user interaction.
test("en 1440: a reload over a note draft asks first; staying keeps it, leaving reloads", async ({ page }) => {
  const m: Catalogue = en;
  const f = await fixture(page, "en", "porcelain", 1440);
  const quiet: string[] = [];
  const listen = (dialog: Dialog) => { quiet.push(dialog.type()); void dialog.dismiss(); };
  page.on("dialog", listen);
  // This fixture reloads to the start page; the editor is one navigation away.
  const backToNotes = async () => {
    await page.goto("/?view=editor");
    return openNotes(page, m, 1440);
  };
  await openNotes(page, m, 1440);
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  expect(quiet, "no draft, no question").toEqual([]);
  page.off("dialog", listen);

  const note = (await backToNotes()).locator("section").first();
  const title = note.getByLabel("Title", { exact: true });
  await title.click();
  await page.keyboard.press("ControlOrMeta+A");
  await page.keyboard.type("Unapplied thought");
  const stay = page.waitForEvent("dialog");
  void page.reload().catch(() => undefined);
  const first = await stay;
  expect(first.type()).toBe("beforeunload");
  await first.dismiss();
  await expect(title).toHaveValue("Unapplied thought");

  const leave = page.waitForEvent("dialog");
  const reloaded = page.waitForEvent("load");
  void page.reload().catch(() => undefined);
  const second = await leave;
  expect(second.type()).toBe("beforeunload");
  await second.accept();
  await reloaded;
  // Leaving was the user's choice: back in Notes, the stored title is shown.
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  await expect((await backToNotes()).locator("section").first().getByLabel("Title", { exact: true }))
    .toHaveValue("Check the kitchen actuator");
  expect(f.applied).toBe(0);
  expect(f.unexpected).toEqual([]);
});
