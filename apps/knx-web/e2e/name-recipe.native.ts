/** The naming recipe against the real server: preview, stale refusal, confirm, undo, save and reopen. */
import { expect, test, type Page } from "@playwright/test";
import { copyFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";

const data = process.env.KNX_RENAME_NATIVE_DATA!;
const demo = process.env.KNX_RENAME_NATIVE_DEMO!;
if (!data || !demo || process.env.KNX_RENAME_ISOLATED !== "1") throw new Error("Run only through the isolated verification runner.");
// Three addresses of the fictional single-family home.
const COMMAND = "Living room / Switch command";
const STATUS = "Living room / Switch status";
const OTHER = "Living room / Level command";

async function openCopy(page: Page, m: typeof en | typeof de) {
  await page.getByRole("button", { name: m["workbench.openNativeTitle"], exact: true }).click();
  await page.getByRole("dialog", { name: m["fsPicker.open"], exact: true }).getByRole("button", { name: "name-recipe-demo.knxdb", exact: true }).click();
  await expect(page.locator(".welcome-card")).toHaveCount(0);
}
async function addresses(page: Page, m: typeof en | typeof de) {
  const nav = page.getByRole("navigation", { name: m["workbench.navigation"], exact: true });
  if (!await nav.isVisible()) await page.locator(".workbench-panel-controls").getByRole("button", { name: m["workbench.navigation"], exact: true }).click();
  await nav.getByRole("button", { name: m["workbench.addresses"], exact: true }).click();
  // Issue #11: every installation's tree opens compact; open each one.
  await expect(page.locator("[data-ga-expand-all]").first()).toBeVisible();
  for (const button of await page.locator("[data-ga-expand-all]").all()) await button.click();
}
async function names(request: import("@playwright/test").APIRequestContext): Promise<string[]> {
  const tree = await (await request.get("/api/project")).json();
  return tree.installations.flatMap((i: { group_addresses: { name: string }[] }) => i.group_addresses.map((ga) => ga.name));
}

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} built app/native server naming recipe at ${width}px`, async ({ page, request }) => {
    test.setTimeout(150_000);
    const m = language === "en" ? en : de;
    const reset = await request.post("/api/project/new", { data: { name: "Isolated reset", discardChanges: true } }); expect(reset.ok()).toBe(true);
    mkdirSync(data, { recursive: true }); copyFileSync(demo, join(data, "name-recipe-demo.knxdb"));
    const settings = await request.put("/api/settings", { data: { settings: {
      uiLanguage: language, theme: language === "en" ? "graphite" : "porcelain", motionLevel: "off", uiScale: 1,
      autosaveEnabled: false, achievementsEnabled: false, onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.6" },
    } } }); expect(settings.ok()).toBe(true);
    const errors: string[] = []; const writes: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    page.on("request", (r) => { if (!["GET", "HEAD"].includes(r.method())) writes.push(`${r.method()} ${new URL(r.url()).pathname}`); });
    await page.setViewportSize({ width, height: 900 }); await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto("/"); await openCopy(page, m); await addresses(page, m);

    // An explicit selection, in the order it is ticked.
    await page.getByRole("checkbox", { name: `0/0/2 ${STATUS}`, exact: true }).check();
    await page.getByRole("checkbox", { name: `0/0/1 ${COMMAND}`, exact: true }).check();
    await page.locator(".bulk-action-toolbar").getByRole("button", { name: m["bulkAction.nameRecipe"], exact: true }).click();
    const dialog = page.getByRole("dialog", { name: m["nameRecipe.title.other"].replace("{count}", "2"), exact: true });
    await dialog.getByLabel(m["nameRecipe.prefix"], { exact: true }).fill("EG ");
    await dialog.getByRole("button", { name: m["nameRecipe.preview"], exact: true }).click();
    const rows = dialog.locator("[data-recipe-row]");
    await expect(rows).toHaveCount(2);
    await expect(rows.nth(0)).toContainText(`EG ${STATUS}`);
    await expect(rows.nth(1)).toContainText(`EG ${COMMAND}`);
    expect(await names(request)).toContain(COMMAND);

    // Another client renames something else while the preview is shown: confirming must be refused.
    const tree = await (await request.get("/api/project")).json();
    const other = tree.installations[0].group_addresses.find((ga: { name: string }) => ga.name === OTHER);
    const external = await request.patch(`/api/group-addresses/${other.id}/name`, { data: { name: `${OTHER} (B)`, expectedName: OTHER,
      serverIncarnation: tree.server_incarnation, projectIncarnation: tree.project_incarnation, snapshotRevision: tree.snapshot_revision } });
    expect(external.ok()).toBe(true);
    await dialog.getByRole("button", { name: m["nameRecipe.confirm.other"].replace("{count}", "2"), exact: true }).click();
    await expect(dialog.getByRole("alert")).toContainText(m["nameRecipe.stale"]);
    await expect(rows).toHaveCount(0);
    expect(await names(request)).toContain(COMMAND);

    // A new preview binds to the current project and is the one confirmed.
    await dialog.getByRole("button", { name: m["nameRecipe.preview"], exact: true }).click();
    await expect(rows).toHaveCount(2);
    await expect(dialog.locator(".name-recipe-table-wrap")).toBeVisible();
    const box = (await dialog.boundingBox())!;
    expect(box.x).toBeGreaterThanOrEqual(0); expect(box.x + box.width).toBeLessThanOrEqual(width);
    if (process.env.KNX_RENAME_CAPTURE) await page.screenshot({ path: join(process.env.KNX_RENAME_CAPTURE, `name-recipe-${language}-${width}.png`) });
    await dialog.getByRole("button", { name: m["nameRecipe.confirm.other"].replace("{count}", "2"), exact: true }).click();
    await expect(dialog).toHaveCount(0);
    await expect(page.locator(".ga-workspace")).toContainText(`EG ${COMMAND}`);
    expect(await names(request)).toEqual(expect.arrayContaining([`EG ${COMMAND}`, `EG ${STATUS}`, `${OTHER} (B)`]));

    // One step: Undo brings both names back, Redo both again.
    const toolbar = page.locator(".workbench-toolbar");
    await toolbar.getByRole("button", { name: m["toolbar.undo"], exact: true }).click();
    await expect.poll(() => names(request)).toEqual(expect.arrayContaining([COMMAND, STATUS, `${OTHER} (B)`]));
    await toolbar.getByRole("button", { name: m["toolbar.redo"], exact: true }).click();
    await expect.poll(() => names(request)).toEqual(expect.arrayContaining([`EG ${COMMAND}`, `EG ${STATUS}`]));

    await page.locator('[data-crt-surface="save"]').click();
    await expect.poll(async () => (await (await request.get("/api/project")).json()).is_modified).toBe(false);
    await page.reload(); await openCopy(page, m);
    expect(await names(request)).toEqual(expect.arrayContaining([`EG ${COMMAND}`, `EG ${STATUS}`, `${OTHER} (B)`]));
    await addresses(page, m);
    await expect(page.locator(".ga-workspace")).toContainText(`EG ${STATUS}`);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    // Same rule as rename.native.ts: no bus write, download or address write may start.
    expect(writes.filter((w) => /\/api\/(?:bus\/(?:write|monitor\/start|monitor\/stop)|device-download\/(?:start|plan)|device-address\/start)/.test(w))).toEqual([]);
    expect(writes.filter((w) => w.startsWith("POST /api/name-recipe/apply"))).toHaveLength(2);
    expect(errors).toEqual([]);
  });
}
