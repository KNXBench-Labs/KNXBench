/** Browser proof of plug-in data adoption: palette, preview, token-only confirm. */
import { expect, test, type Page } from "@playwright/test";
import { join } from "node:path";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";

// Strictly fictional: devices, digests and the archive name are made up here.
const tree = (revision: number) => ({
  schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
  group_address_style: "ThreeLevel", server_incarnation: "plugin-fixture", project_incarnation: 3, snapshot_revision: revision,
  installations: [{ id: 1, name: "Fixture house", topology: [], buildings: [], unassigned: [], group_addresses: [], group_ranges: [] }],
});
const device = (id: number, name: string, address: string) => ({ id, name, etsId: `P-0001-0_DI-${id}`, address });
const listing = {
  sourceHash: "5".repeat(64), persistence: "native",
  candidates: [
    { sourceDevice: device(1, "Room sensor", "1.1.7"), targetDevice: device(10, "Room sensor", "1.1.7"), refusal: null, refusalDetail: null, replacements: 2 },
    { sourceDevice: device(2, "Presence detector", "1.1.8"), targetDevice: device(20, "Presence detector", "1.1.8"),
      refusal: "links differ", refusalDetail: { code: "groupLinksDiffer", message: "links differ" }, replacements: 0 },
  ],
};
const preview = {
  sourceHash: listing.sourceHash, request: { sourceDevice: 1, targetDevice: 10 },
  sourceDevice: listing.candidates[0].sourceDevice, targetDevice: listing.candidates[0].targetDevice,
  unchangedPaths: ["P-0001/ExtraData/393257.rbg"],
  replacements: [
    { channel: "BinaryData", path: "P-0001/BinaryData/b113ce24-349e-45db-8ad5-c2a7003d0edf.dat", bytesBefore: 4096, bytesAfter: 4352, sha256Before: "a1".repeat(32), sha256After: "b2".repeat(32) },
    { channel: "ExtraData", path: "P-0001/ExtraData/393256.rbg", bytesBefore: 812, bytesAfter: 840, sha256Before: "c3".repeat(32), sha256After: "d4".repeat(32) },
  ],
  notes: ["Bytes are copied verbatim and never decoded; their meaning and ETS acceptance are not verified."],
};
const TOKEN = "e5".repeat(32);

async function intercept(page: Page, language: "en" | "de") {
  const state = { revision: 1, open: false, unexpected: [] as string[], writes: [] as { path: string; body: Record<string, unknown> }[] };
  await page.route("**/api/**", (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname; const method = request.method();
    let body: unknown; let status = 200;
    if (path === "/api/auth/status" && method === "GET") body = { required: false, authenticated: true };
    else if (path === "/api/settings" && method === "GET") body = { schemaVersion: 1, status: "ok", settings: { uiLanguage: language, theme: language === "en" ? "graphite" : "porcelain" } };
    else if (path === "/api/project" && method === "GET") { if (state.open) body = tree(state.revision); else { status = 400; body = { error: "no project open" }; } }
    else if (path === "/api/project/new" && method === "POST") { state.open = true; body = tree(state.revision); }
    else if (path === "/api/project/load-progress" && method === "GET") body = null;
    else if (path === "/api/bus/discover" && method === "POST") body = { interfaces: [] };
    else if (path === "/api/product-languages" && method === "GET") body = [];
    else if (path === "/api/fs/list" && method === "GET") body = [{ name: "edited-export.knxproj", is_dir: false }];
    else if (["/api/device-download/status", "/api/device-address/status"].includes(path) && method === "GET") { status = 404; body = { error: "no operation in this fixture" }; }
    else if (path.startsWith("/api/project/plugin-data/") && method === "POST") {
      const sent = request.postDataJSON() as Record<string, unknown>;
      state.writes.push({ path, body: sent });
      if (path.endsWith("/candidates")) body = listing;
      else if (path.endsWith("/preview")) {
        // Bound like the server: the tree the dialog was shown under.
        if (sent.serverIncarnation !== "plugin-fixture" || sent.projectIncarnation !== 3 || sent.snapshotRevision !== state.revision) {
          status = 409; body = { error: "stale", kind: "pluginDataStale" };
        } else body = { preview, confirmationToken: TOKEN };
      } else if (path.endsWith("/apply")) {
        if (sent.confirmationToken !== TOKEN || Object.keys(sent).length !== 1) { status = 409; body = { error: "stale", kind: "pluginDataStale" }; }
        else { state.revision += 1; body = { preview, project: tree(state.revision), rollbackVersion: { id: 4, label: "Before plug-in data adoption" } }; }
      } else if (path.endsWith("/cancel")) body = { cancelled: sent.confirmationToken === TOKEN };
      else { state.unexpected.push(`${method} ${path}`); status = 404; body = { error: "unknown plug-in route" }; }
    }
    else if (achievementsFixtureAnswer(method, path) !== undefined) body = achievementsFixtureAnswer(method, path);
    else { state.unexpected.push(`${method} ${path}`); status = 404; body = { error: "strict plug-in data fixture only" }; }
    return route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
  });
  return state;
}

async function openProject(page: Page, m: Record<keyof typeof en, string>) {
  await page.goto("/");
  await page.getByRole("button", { name: m["toolbar.newProject"], exact: true }).first().click();
  await page.getByRole("dialog", { name: m["newProject.title"], exact: true }).getByRole("button", { name: m["newProject.create"], exact: true }).click();
  await page.getByRole("dialog", { name: m["projectWizard.done.title"], exact: true }).getByRole("button", { name: m["projectWizard.done.close"], exact: true }).click();
}

async function expectInViewport(page: Page, width: number) {
  const panel = page.getByRole("dialog", { name: en["pluginData.title"] }).or(page.getByRole("dialog", { name: de["pluginData.title"] }));
  const box = await panel.boundingBox();
  expect(box).not.toBeNull();
  expect(box!.x).toBeGreaterThanOrEqual(0);
  expect(box!.x + box!.width).toBeLessThanOrEqual(width);
  expect(await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth)).toBeLessThanOrEqual(1);
  expect(await panel.evaluate((node) => node.scrollWidth - node.clientWidth)).toBeLessThanOrEqual(1);
  // Inner scrollers hide overflow from the checks above: none may scroll sideways.
  const inner = await panel.evaluate((node) => [...node.querySelectorAll<HTMLElement>(".plugin-data-pairs, .plugin-data-replacements > li")]
    .map((element) => element.scrollWidth - element.clientWidth));
  for (const overflow of inner) expect(overflow).toBeLessThanOrEqual(1);
}

/** The pair's names and verdict are on screen inside the list, next to a normal-sized radio. */
async function expectPairReadable(page: Page, label: string) {
  const list = page.locator(".plugin-data-pairs");
  const text = list.getByText(label, { exact: true });
  await expect(text).toBeVisible();
  const [listBox, textBox, radioBox] = await Promise.all([list.boundingBox(), text.boundingBox(), list.getByRole("radio").boundingBox()]);
  expect(textBox!.x).toBeGreaterThanOrEqual(listBox!.x);
  expect(textBox!.x + textBox!.width).toBeLessThanOrEqual(listBox!.x + listBox!.width + 1);
  expect(radioBox!.width).toBeLessThan(40);
}

/** A refused pair reads as name, then reason underneath: never two squeezed columns. */
async function expectRefusedPairStacked(page: Page) {
  const row = page.locator(".plugin-data-pair-refused");
  const [name, reason] = await Promise.all([row.locator("strong").boundingBox(), row.locator(".plugin-data-reason").boundingBox()]);
  expect(reason!.y).toBeGreaterThanOrEqual(name!.y + name!.height - 1);
}

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  const m = language === "en" ? en : de;
  test(`${language} lists, previews and adopts plug-in data at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    const state = await intercept(page, language);
    await openProject(page, m);
    await page.keyboard.press("Control+Shift+P");
    await page.getByRole("combobox").fill(m["pluginData.palette"]);
    await page.getByRole("combobox").press("Enter");
    const dialog = page.getByRole("dialog", { name: m["pluginData.title"], exact: true });
    await expect(dialog).toBeVisible();

    await dialog.getByRole("button", { name: m["pluginData.choose"], exact: true }).click();
    await page.getByRole("button", { name: "edited-export.knxproj" }).click();
    await expect(dialog).toContainText(m["pluginData.source"].replace("{name}", "edited-export.knxproj"));
    await expect(dialog).toContainText(m["pluginData.refusal.groupLinksDiffer"]);
    await expectPairReadable(page, m["pluginData.pair.label"].replace("{source}", "Room sensor (1.1.7)").replace("{target}", "Room sensor (1.1.7)"));
    await expectRefusedPairStacked(page);
    await expectInViewport(page, width);
    if (process.env.KNX_PLUGIN_DATA_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_PLUGIN_DATA_CAPTURE_DIR, `plugin-data-pairs-${language}-${width}.png`) });
    // A refused pair offers no choice at all.
    await expect(dialog.getByRole("radio")).toHaveCount(1);
    const preview_ = dialog.getByRole("button", { name: m["pluginData.preview"], exact: true });
    await expect(preview_).toBeDisabled();
    // Keyboard reachable: focus the pair and choose it with Space.
    await dialog.getByRole("radio").focus();
    await page.keyboard.press("Space");
    await expect(dialog.getByRole("radio")).toBeChecked();
    await preview_.click();

    const section = dialog.locator("[data-plugin-preview]");
    await expect(section).toContainText(m["pluginData.previewHeading"]);
    for (const r of preview.replacements) {
      await expect(section).toContainText(r.path);
      await expect(section).toContainText(r.sha256Before);
      await expect(section).toContainText(r.sha256After);
    }
    await expect(section).toContainText(m["pluginData.note.noUndo"].replace("{label}", "Before plug-in data adoption"));
    await expectInViewport(page, width);
    if (process.env.KNX_PLUGIN_DATA_CAPTURE_DIR) {
      await section.scrollIntoViewIfNeeded();
      await page.screenshot({ path: join(process.env.KNX_PLUGIN_DATA_CAPTURE_DIR, `plugin-data-preview-${language}-${width}.png`) });
    }
    const apply = dialog.getByRole("button", { name: m["pluginData.apply"], exact: true });
    await expect(apply).toBeDisabled();
    await dialog.getByRole("checkbox", { name: m["pluginData.consent"] }).check();
    await apply.click();

    const done = dialog.locator("[data-plugin-done]");
    await expect(done).toHaveText(m["pluginData.done.other"].replace("{count}", "2").replace("{label}", "Before plug-in data adoption"));
    await expect(dialog.getByRole("button", { name: m["pluginData.openHistory"], exact: true })).toBeVisible();
    await expectInViewport(page, width);
    if (process.env.KNX_PLUGIN_DATA_CAPTURE_DIR) await page.screenshot({ path: join(process.env.KNX_PLUGIN_DATA_CAPTURE_DIR, `plugin-data-done-${language}-${width}.png`) });

    const sent = state.writes.map(w => w.path.split("/").pop());
    expect(sent).toEqual(["candidates", "preview", "apply"]);
    expect(state.writes[0].body).toEqual({ path: expect.stringMatching(/edited-export\.knxproj$/) });
    expect(state.writes[1].body).toMatchObject({ sourceHash: listing.sourceHash, request: { sourceDevice: 1, targetDevice: 10 },
      serverIncarnation: "plugin-fixture", projectIncarnation: 3, snapshotRevision: 1 });
    expect(state.writes[2].body).toEqual({ confirmationToken: TOKEN });
    await dialog.getByRole("button", { name: m["pluginData.close"], exact: true }).click();
    await expect(dialog).toHaveCount(0);
    expect(state.writes).toHaveLength(3);
    expect(state.unexpected).toEqual([]);
  });
}

test("closing with a shown preview releases it on the server", async ({ page }) => {
  const state = await intercept(page, "en");
  await openProject(page, en);
  await page.keyboard.press("Control+Shift+P");
  await page.getByRole("combobox").fill(en["pluginData.palette"]);
  await page.getByRole("combobox").press("Enter");
  const dialog = page.getByRole("dialog", { name: en["pluginData.title"], exact: true });
  await dialog.getByRole("button", { name: en["pluginData.choose"], exact: true }).click();
  await page.getByRole("button", { name: "edited-export.knxproj" }).click();
  await dialog.getByRole("radio").check();
  await dialog.getByRole("button", { name: en["pluginData.preview"], exact: true }).click();
  await expect(dialog.locator("[data-plugin-preview]")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
  await expect.poll(() => state.writes.map(w => w.path.split("/").pop())).toEqual(["candidates", "preview", "cancel"]);
  expect(state.writes[2].body).toEqual({ confirmationToken: TOKEN });
  expect(state.unexpected).toEqual([]);
});
