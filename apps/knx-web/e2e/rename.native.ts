/** Explicit isolated-server browser proof; excluded from intercepted default fixtures. */
import { expect, test, type Page } from "@playwright/test";
import { copyFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";

const data = process.env.KNX_RENAME_NATIVE_DATA!;
const demo = process.env.KNX_RENAME_NATIVE_DEMO!;
if (!data || !demo || process.env.KNX_RENAME_ISOLATED !== "1") throw new Error("Run only through the isolated rename verification runner.");
async function palette(page: Page, name: string) {
  await page.keyboard.press("Control+Shift+P"); await page.getByRole("combobox").fill(name);
  await page.getByRole("combobox").press("Enter");
}
async function openCopy(page: Page, m: typeof en | typeof de) {
  await page.getByRole("button", { name: m["workbench.openNativeTitle"], exact: true }).click();
  await page.getByRole("dialog", { name: m["fsPicker.open"], exact: true }).getByRole("button", { name: "rename-demo.knxdb", exact: true }).click();
  await expect(page.locator(".welcome-card")).toHaveCount(0);
}
for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} built app/native server naming, conflict and reopen at ${width}px`, async ({ page, request }) => {
    test.setTimeout(120_000);
    const m = language === "en" ? en : de;
    const reset = await request.post("/api/project/new", { data: { name: "Isolated reset", discardChanges: true } }); expect(reset.ok()).toBe(true);
    mkdirSync(data, { recursive: true }); copyFileSync(demo, join(data, "rename-demo.knxdb"));
    const settings = await request.put("/api/settings", { data: { settings: {
      uiLanguage: language, theme: language === "en" ? "graphite" : "porcelain", motionLevel: "off", uiScale: 1,
      autosaveEnabled: false, achievementsEnabled: false, onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.6" },
    } } }); expect(settings.ok()).toBe(true);
    const errors: string[] = []; const writes: string[] = []; const successes: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    page.on("request", (r) => { if (!["GET", "HEAD"].includes(r.method())) writes.push(`${r.method()} ${new URL(r.url()).pathname}`); });
    page.on("response", (r) => { if (r.request().method() === "PATCH" && r.url().endsWith("/name") && r.ok()) successes.push(r.url()); });
    await page.setViewportSize({ width, height: 900 }); await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto("/"); await openCopy(page, m);
    await palette(page, m["workbench.devices"]);
    await expect(page.locator(".devices-table tbody tr")).toHaveCount(32);
    const row = page.locator(".devices-table tbody tr").first();
    const id = Number(await row.getAttribute("data-rename-id"));
    const original = await row.locator("td:nth-child(3) button").innerText();
    await row.locator("td:nth-child(3) button").focus(); await row.locator("td:nth-child(3) button").press("F2");
    let dialog = page.getByRole("dialog", { name: m["rename.title"], exact: true });
    await expect(dialog).toBeVisible(); await dialog.locator("input").fill("  Device renamed 🛠  ");
    await dialog.locator("input").press("Enter"); await expect(dialog).toHaveCount(0);
    await expect(page.locator(`.devices-table [data-rename-id="${id}"] td:nth-child(3)`)).toContainText("Device renamed 🛠");
    await page.locator(`.devices-table [data-rename-id="${id}"] td:nth-child(3) button`).click({ button: "right" });
    await page.getByRole("menuitem", { name: new RegExp(m["rename.title"]) }).click();
    await dialog.locator("input").fill("Device via list menu"); await dialog.locator("input").press("Enter");
    await expect(dialog).toHaveCount(0);
    await page.locator(`.devices-table [data-rename-id="${id}"] td:nth-child(3) button`).click();
    let field = page.locator(".device-workspace input[data-rename-name]");
    await expect(field).toHaveValue("Device via list menu");
    await field.fill("Device via editor"); await field.press("Enter"); await expect(page.locator(".device-workspace h2")).toHaveText("Device via editor");
    // A second client races this exact name draft; the real API must refuse it.
    await field.fill("Retained conflict draft");
    const snapshot = await (await request.get("/api/project")).json();
    const external = await request.patch(`/api/devices/${id}/name`, { data: { name: "External name", expectedName: "Device via editor",
      serverIncarnation: snapshot.server_incarnation, snapshotRevision: snapshot.snapshot_revision, projectIncarnation: snapshot.project_incarnation } });
    expect(external.ok()).toBe(true);
    await field.press("Enter"); await expect(field).toHaveValue("Retained conflict draft");
    await expect(page.locator(".device-workspace .rename-name-field [role=alert]")).toBeVisible();
    await page.locator(".device-workspace .rename-name-field").getByRole("button", { name: m["rename.refresh"], exact: true }).click();
    await expect(field).toHaveValue("Retained conflict draft"); await field.press("Enter");
    await expect(page.locator(".device-workspace h2")).toHaveText("Retained conflict draft");
    if (width === 1440) {
      const inspector = page.locator(".workbench-pane-right input[data-rename-name]");
      await expect(inspector).toHaveValue("Retained conflict draft");
      await inspector.fill("Device via properties"); await inspector.press("Tab");
      await expect(page.locator(".device-workspace h2")).toHaveText("Device via properties");
      const explorerDevice = page.locator(`.workbench-pane-left [data-rename-kind="device"][data-rename-id="${id}"]`).first();
      await explorerDevice.click({ button: "right" });
      await page.getByRole("menuitem", { name: new RegExp(m["rename.title"]) }).click();
      await dialog.locator("input").fill("Device via explorer"); await dialog.locator("input").press("Enter");
      await expect(dialog).toHaveCount(0); await expect(page.locator(".device-workspace h2")).toHaveText("Device via explorer");
    }
    const navigation = page.getByRole("navigation", { name: m["workbench.navigation"], exact: true });
    if (!await navigation.isVisible()) await page.locator(".workbench-panel-controls").getByRole("button", { name: m["workbench.navigation"], exact: true }).click();
    await navigation.getByRole("button", { name: m["workbench.addresses"], exact: true }).click();
    await expect(page.locator(".address-table tbody tr")).toHaveCount(105);
    const gaRow = page.locator(".address-table tbody tr").first();
    const gaId = Number(await gaRow.getAttribute("data-rename-id"));
    const gaOriginal = await gaRow.locator("td:nth-child(3)").innerText();
    await gaRow.locator("td:nth-child(3)").click({ button: "right" });
    await page.getByRole("menuitem", { name: new RegExp(m["rename.title"]) }).click();
    dialog = page.getByRole("dialog", { name: m["rename.title"], exact: true });
    await dialog.locator("input").fill("<script> Literal GA 🛠"); await dialog.locator("input").press("Enter"); await expect(dialog).toHaveCount(0);
    await expect(page.locator(`.address-table [data-rename-id="${gaId}"] td:nth-child(3)`)).toHaveText("<script> Literal GA 🛠");
    expect(await page.locator(".address-table script").count()).toBe(0);
    await page.locator(`.address-table [data-rename-id="${gaId}"] .table-select`).focus();
    await page.locator(`.address-table [data-rename-id="${gaId}"] .table-select`).press("F2");
    await dialog.locator("input").fill("Do not save"); await dialog.locator("input").press("Escape"); await expect(dialog).toHaveCount(0);
    await expect(page.locator(`.address-table [data-rename-id="${gaId}"] td:nth-child(3)`)).toHaveText("<script> Literal GA 🛠");
    await page.locator(`.address-table [data-rename-id="${gaId}"] .table-select`).focus();
    await page.locator(`.address-table [data-rename-id="${gaId}"] .table-select`).press("F2");
    await dialog.locator("input").fill(" "); await dialog.locator("input").press("Enter");
    await expect(dialog.locator("[role=alert]")).toHaveText(m["rename.blank"]);
    const box = await dialog.boundingBox(); expect(box!.x).toBeGreaterThanOrEqual(0); expect(box!.x + box!.width).toBeLessThanOrEqual(width); expect(box!.height).toBeLessThan(400);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    if (process.env.KNX_RENAME_CAPTURE) await page.screenshot({ path: join(process.env.KNX_RENAME_CAPTURE, `rename-${language}-${width}.png`) });
    await dialog.getByRole("button", { name: m["rename.cancel"], exact: true }).click(); await expect(dialog).toHaveCount(0);
    let gaUndoName = gaOriginal;
    if (width === 1440) {
      await page.locator(`.address-table [data-rename-id="${gaId}"] .table-select`).click();
      const gaProperties = page.locator(".workbench-pane-right input[data-rename-name]");
      await expect(gaProperties).toHaveValue("<script> Literal GA 🛠");
      await gaProperties.fill("GA via properties"); await gaProperties.press("Enter");
      await expect(page.locator(`.address-table [data-rename-id="${gaId}"] td:nth-child(3)`)).toHaveText("GA via properties");
      const explorerGa = page.locator(`.workbench-pane-left [data-rename-kind="group_address"][data-rename-id="${gaId}"]`).first();
      await explorerGa.focus(); await explorerGa.press("F2");
      await dialog.locator("input").fill("GA via explorer"); await dialog.locator("input").press("Enter");
      await expect(dialog).toHaveCount(0);
      await expect(page.locator(`.address-table [data-rename-id="${gaId}"] td:nth-child(3)`)).toHaveText("GA via explorer");
      gaUndoName = "GA via properties";
    }
    await page.locator('[data-crt-surface="save"]').click();
    await expect.poll(async () => (await (await request.get("/api/project")).json()).is_modified).toBe(false);
    const beforeReopen = await (await request.get("/api/project/history")).json();
    expect(beforeReopen.undoSteps).toBe(successes.length + 1); // plus the independent external client
    await page.reload(); await openCopy(page, m);
    const afterReopen = await (await request.get("/api/project/history")).json();
    expect(afterReopen.undoSteps).toBe(beforeReopen.undoSteps);
    expect(afterReopen.project.installations).toEqual(beforeReopen.project.installations);
    await page.getByRole("button", { name: m["toolbar.undo"], exact: true }).click();
    await expect.poll(async () => (await (await request.get("/api/project/history")).json()).redoSteps).toBe(1);
    const undone = await (await request.get("/api/project")).json();
    expect(undone.installations.flatMap((i: { group_addresses: { id: number; name: string }[] }) => i.group_addresses).find((g: { id: number }) => g.id === gaId).name).toBe(gaUndoName);
    await page.getByRole("button", { name: m["toolbar.redo"], exact: true }).click();
    await expect.poll(async () => (await (await request.get("/api/project/history")).json()).redoSteps).toBe(0);
    expect(writes.filter((w) => /\/api\/(?:bus\/(?:write|monitor\/start|monitor\/stop)|device-download\/(?:start|plan)|device-address\/start)/.test(w))).toEqual([]);
    expect(errors).toEqual([]); expect(original.length).toBeGreaterThan(0);
  });
}
