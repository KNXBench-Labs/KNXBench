/** Built device editor: instructions tab, identity, apply refresh, refusals. */
import { openExplorerBranches } from "./disclosure-fixture";
import { expect, test, type Page } from "@playwright/test";
import { join } from "node:path";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import type { Document } from "../src/features/rtf/document";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";
import { controlPaint } from "./control-roles";

type Language = "en" | "de";
type Catalogue = { readonly [K in keyof typeof en]: string };
const CAPTURE = process.env.KNX_INSTRUCTIONS_CAPTURE_DIR;
type ApplyMode = "ok" | "stale" | "persistence";
const plain = { bold: false, italic: false, underline: false };
const DEVICE = { id: 41, name: "Switch actuator kitchen", address: "1.1.4", description: null, com_object_count: 0 };

interface Fixture {
  unexpected: string[];
  revision: number;
  override: Document | null;
  applyMode: ApplyMode;
  applied: number;
  projectReads: number;
  identity: boolean;
  native: boolean;
}

function projectTree(f: Fixture): ProjectTree {
  return {
    schema_version: 13, errors: 0, warnings: 0, can_undo: f.applied > 0, can_redo: false, is_modified: f.applied > 0,
    ...(f.identity ? { server_incarnation: "instructions-fixture", project_incarnation: 2, snapshot_revision: f.revision } : {}),
    group_address_style: "ThreeLevel",
    installations: [{ id: 0, name: "Fictional house",
      topology: [{ id: 21, name: "Main area", address: 1, lines: [{ id: 31, name: "Ground floor", address: 1, devices: [DEVICE] }] }],
      buildings: [], unassigned: [], group_ranges: [], group_addresses: [] }],
  };
}

const ORIGINAL: Document = { runs: [
  { text: "Mount the actuator ", style: plain },
  { text: "only", style: { bold: true, italic: false, underline: false } },
  { text: " on a DIN rail.\nKeep <script> away.", style: plain },
] };

function viewOf(f: Fixture) {
  return { serverIncarnation: "instructions-fixture", projectIncarnation: 2, snapshotRevision: f.revision, device: 41,
    ...(f.native
      ? { origin: "native", originalXmlSha256: null, originalPresent: false, originalBytes: 0, originalFormat: null }
      : { origin: "imported", originalXmlSha256: "d".repeat(64), originalPresent: true, originalBytes: 64, originalFormat: "rtf" }),
    unsupported: [], previewError: null, effectiveDocument: f.override ?? (f.native ? null : ORIGINAL), overrideDocument: f.override,
    persistence: "session-only" };
}

async function fixture(page: Page, language: Language, theme: string, width: number, identity = true, native = false): Promise<Fixture> {
  const f: Fixture = { unexpected: [], revision: 1, override: null, applyMode: "ok", applied: 0, projectReads: 0, identity, native };
  const plans = new Map<string, Document | null>();
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
    if (path === "/api/version") return ok({ version: "0.1.0-alpha.9+instructions-fixture" });
    if (path === "/api/project" && method === "GET") { f.projectReads += 1; return ok(projectTree(f)); }
    if (path === "/api/project/load-progress") return ok(null);
    if (path === "/api/project/status-metrics" && method === "GET") return ok(statusMetricsFixtureBody(projectTree(f)));
    if (path === "/api/log") return ok([]);
    if (path === "/api/bus/discover" && method === "POST") return ok({ interfaces: [] });
    if (["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items"].includes(path)) return ok([]);
    if (["/api/device-download/status", "/api/device-address/status"].includes(path)) return refuse(404, "none", "no fixture operation");
    if (path === "/api/device-system2/status" && method === "GET") return ok(null);
    if (path === "/api/devices" && method === "GET") return ok({ schemaVersion: 1, serverIncarnation: identity ? "instructions-fixture" : null,
      snapshotRevision: identity ? f.revision : null, devices: [] });
    if (path === "/api/device/41" && method === "GET") return ok({ ...DEVICE, com_objects: [], product: { resolution: "NoReference", product_ref: null, program_ref: null, catalog: null } });
    if (path === "/api/device/41/parameters" && method === "GET") return ok({ programId: null, sections: [], stale: [], diagnostics: [] });
    if (path === "/api/devices/41/instructions" && method === "GET") return ok(viewOf(f));
    if (path === "/api/devices/41/instructions/preview" && method === "POST") {
      const body = route.request().postDataJSON();
      if (body.serverIncarnation !== "instructions-fixture" || body.projectIncarnation !== 2 || body.snapshotRevision !== f.revision) {
        return refuse(409, "instructionStale", "Instruction preview no longer matches the open project; refresh and preview again");
      }
      const token = `plan-${plans.size + 1}`;
      plans.set(token, body.document);
      return ok({ token, device: 41, replacement: body.document, conversionRequired: false, noOp: false, expiresInSeconds: 300 });
    }
    if (path === "/api/devices/41/instructions/apply" && method === "POST") {
      const body = route.request().postDataJSON();
      if (!plans.has(body.token) || body.confirmed !== true) return refuse(409, "instructionStale", "Instruction preview no longer matches the open project; refresh and preview again");
      if (f.applyMode === "stale") return refuse(409, "instructionStale", "Instruction preview no longer matches the open project; refresh and preview again");
      if (f.applyMode === "persistence") return refuse(422, "instructionPersistenceFailed", "Instruction edit was not applied because native recovery persistence failed");
      f.override = plans.get(body.token) ?? null;
      plans.delete(body.token);
      f.applied += 1; f.revision += 1;
      return ok({ changed: true, nativeGeneration: null, snapshotRevision: f.revision, persistence: "session-only" });
    }
    if (path === "/api/devices/41/instructions/cancel" && method === "POST") { plans.delete(route.request().postDataJSON().token); return ok({ cancelled: true }); }
    f.unexpected.push(`${method} ${path}`);
    return refuse(404, "strict", "strict instructions fixture");
  });
  await page.goto("/?view=editor");
  await expect(page.locator("html")).toHaveAttribute("lang", language);
  await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
  return f;
}

async function noHorizontalOverflow(page: Page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
}

async function openInstructions(page: Page, m: Catalogue) {
  await openExplorerBranches(page);
  await page.locator(".project-explorer").getByText(DEVICE.name, { exact: false }).first().click();
  const tab = page.getByRole("tab", { name: m["deviceInstructions.tab"], exact: true });
  await tab.click();
  const panel = page.locator(".device-instructions");
  await expect(panel).toBeVisible();
  return panel;
}

async function capture(page: Page, name: string) {
  if (CAPTURE) await page.screenshot({ path: join(CAPTURE, `${name}.png`), fullPage: false });
}

for (const language of ["en", "de"] as const) for (const theme of ["porcelain", "graphite"]) for (const width of [1440, 400]) {
  test(`${language} ${theme} ${width}: instructions tab previews, applies and refreshes`, async ({ page }) => {
    const m: Catalogue = language === "de" ? de : en;
    const tag = `${language}-${theme}-${width}`;
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    const f = await fixture(page, language, theme, width);
    const panel = await openInstructions(page, m);
    const text = panel.locator("textarea");
    await expect(text).toHaveValue("Mount the actuator only on a DIN rail.\nKeep <script> away.");
    await expect(panel.locator("script")).toHaveCount(0);
    await expect(panel.locator("span", { hasText: "only" }).first()).toHaveCSS("font-weight", "700");
    await panel.scrollIntoViewIfNeeded();
    await noHorizontalOverflow(page);
    await capture(page, `${tag}-1-open`);

    await text.fill("Mount the actuator only on a DIN rail.\nKeep <script> away.\nLabel the cable.");
    const readsBefore = f.projectReads;
    await panel.getByRole("button", { name: language === "de" ? "Änderung prüfen" : "Preview change", exact: true }).click();
    const change = panel.getByRole("region", { name: language === "de" ? "Änderungsvorschau" : "Change preview" });
    await expect(change).toContainText("Label the cable.");
    expect(f.applied).toBe(0);
    await change.scrollIntoViewIfNeeded();
    await noHorizontalOverflow(page);
    await capture(page, `${tag}-2-preview`);
    await change.getByRole("button", { name: language === "de" ? "Änderung anwenden" : "Apply change", exact: true }).click();
    await expect.poll(() => f.projectReads).toBeGreaterThan(readsBefore);
    await expect(panel.getByText(language === "de" ? "Native Bearbeitung" : "Native override", { exact: true })).toBeVisible();
    expect(f.applied).toBe(1);
    expect(f.override?.runs.map((run) => run.text).join("")).toBe("Mount the actuator only on a DIN rail.\nKeep <script> away.\nLabel the cable.");
    await expect(page.getByRole("button", { name: m["toolbar.undo"], exact: true })).toBeEnabled();
    await noHorizontalOverflow(page);
    await capture(page, `${tag}-3-applied`);
    expect(f.unexpected).toEqual([]);
    expect(errors).toEqual([]);
  });
}

for (const [mode, language, width] of [["stale", "en", 1440], ["persistence", "de", 400]] as const) {
  test(`${language} ${width}: a ${mode} refusal on Apply leaves the instructions unchanged`, async ({ page }) => {
    const m: Catalogue = language === "de" ? de : en;
    const f = await fixture(page, language, "graphite", width);
    const panel = await openInstructions(page, m);
    f.applyMode = mode;
    await panel.locator("textarea").fill("A rewrite that must not land");
    await panel.getByRole("button", { name: language === "de" ? "Änderung prüfen" : "Preview change", exact: true }).click();
    await panel.getByRole("button", { name: language === "de" ? "Änderung anwenden" : "Apply change", exact: true }).click();
    const alert = panel.getByRole("alert");
    await expect(alert).toContainText(mode === "stale" ? "no longer matches the open project" : "native recovery persistence failed");
    await expect(alert).toBeInViewport();
    expect(f.override).toBeNull();
    expect(f.applied).toBe(0);
    await expect(panel.locator("textarea")).toHaveValue("A rewrite that must not land");
    await capture(page, `refusal-${mode}-${language}-${width}`);
    expect(f.unexpected).toEqual([]);
  });
}

test("en 1440: without a complete project identity the instructions are read-only", async ({ page }) => {
  const f = await fixture(page, "en", "porcelain", 1440, false);
  const panel = await openInstructions(page, en);
  await expect(panel.locator("textarea")).toHaveValue("Mount the actuator only on a DIN rail.\nKeep <script> away.");
  await expect(panel.locator("textarea")).toHaveJSProperty("readOnly", true);
  await expect(panel.getByRole("button", { name: "Preview change", exact: true })).toHaveCount(0);
  await expect(panel.getByRole("note")).toContainText("Read-only");
  await capture(page, "read-only-en-1440");
  expect(f.applied).toBe(0);
  expect(f.unexpected).toEqual([]);
});

// Goal B4: a device made in KNXBench has no imported original, and none is invented.
for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} ${width}: a device made in KNXBench gets its first instructions, then loses them again`, async ({ page }) => {
    const m: Catalogue = language === "de" ? de : en;
    const f = await fixture(page, language, "porcelain", width, true, true);
    const panel = await openInstructions(page, m);
    const madeHere = language === "de" ? "In KNXBench angelegt: kein Importoriginal" : "Made in KNXBench: no imported original";
    const remove = panel.getByRole("button", { name: language === "de" ? "Hinweise entfernen" : "Remove instructions", exact: true });
    await expect(panel.getByText(madeHere, { exact: true })).toBeVisible();
    await expect(panel.locator("textarea")).toHaveValue("");
    await expect(remove).toBeDisabled();
    await noHorizontalOverflow(page);
    await capture(page, `native-${language}-${width}-1-empty`);

    await panel.locator("textarea").fill("Mount above the kitchen door.");
    await panel.getByRole("button", { name: language === "de" ? "Änderung prüfen" : "Preview change", exact: true }).click();
    // An editing apply is an ordinary answer; the removing one below is destructive (UI-02 review).
    const writeApply = panel.getByRole("button", { name: language === "de" ? "Änderung anwenden" : "Apply change", exact: true });
    await page.mouse.move(1, 1);
    expect((await controlPaint(writeApply)).role).toBe("secondary");
    await writeApply.click();
    await expect(remove).toBeEnabled();
    expect(f.override?.runs.map((run) => run.text).join("")).toBe("Mount above the kitchen door.");
    await expect(panel.getByText(madeHere, { exact: true })).toBeVisible();
    await capture(page, `native-${language}-${width}-2-written`);

    await remove.click();
    const change = panel.getByRole("region", { name: language === "de" ? "Änderungsvorschau" : "Change preview" });
    await expect(change).toContainText(language === "de" ? "Das Gerät hat dann wieder keine" : "The device then has none again");
    await expect(change).not.toContainText(language === "de" ? "Importoriginal" : "imported original");
    await change.scrollIntoViewIfNeeded();
    await noHorizontalOverflow(page);
    await capture(page, `native-${language}-${width}-3-remove`);
    const removeApply = change.getByRole("button", { name: language === "de" ? "Änderung anwenden" : "Apply change", exact: true });
    await page.mouse.move(1, 1);
    const paint = await controlPaint(removeApply);
    expect(paint.role, JSON.stringify(paint)).toBe("danger");
    await removeApply.click();
    await expect(remove).toBeDisabled();
    expect(f.override).toBeNull();
    expect(f.applied).toBe(2);
    expect(f.unexpected).toEqual([]);
  });
}
