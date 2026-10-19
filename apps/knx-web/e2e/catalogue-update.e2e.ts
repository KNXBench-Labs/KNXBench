/** Built catalogue view: previewed install, backup, ask-first restore and delete, fixture API. */
import { expect, test, type Page } from "@playwright/test";
import { join } from "node:path";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import type { CatalogCleanupItem, CatalogItem, CatalogueUpdatePreview } from "../src/api";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";

type Language = "en" | "de";
type Catalogue = { readonly [K in keyof typeof en]: string };
const CAPTURE = process.env.KNX_CATALOGUE_CAPTURE_DIR;
const BACKUP = "products.sqlite.backup-1760270400-ab12ab12ab12.sqlite";

const preview: CatalogueUpdatePreview = {
  sourceName: "fictional-dimmer-v2.knxprod",
  packageSha256: "ab12".repeat(16),
  packageAlreadyInstalled: false,
  programs: [
    { id: "M-00FA_A-0001-20-0001", manufacturerId: "M-00FA", name: "Fictional dimmer", applicationNumber: "1", applicationVersion: "2.0",
      status: "newVersion", installedVersions: [{ id: "M-00FA_A-0001-10-0001", applicationVersion: "1.0" }] },
    { id: "M-00FA_A-0002-10-0001", manufacturerId: "M-00FA", name: "Fictional scene module", applicationNumber: "2", applicationVersion: "1.0",
      status: "new", installedVersions: [] },
  ],
  productsAdded: 2, hardware2programsAdded: 2, idConflicts: 0, catalogueFingerprint: "c".repeat(64), token: "fixture-token",
};

/** Two placements of one fictional product + application, and a newer application version. */
function catalogItem(id: string, binding: string, version: string, source: string): CatalogItem {
  return { id, manufacturerId: "M-00FA", name: "Fictional dimmer", number: "DIM-1", visibleDescription: null,
    productRefId: "M-00FA_H-1_P-1", hardware2programRefId: binding, nameLanguage: null, visibleDescriptionLanguage: null,
    sourceLanguage: null, details: { applicationProgramId: `M-00FA_A-0001-${version}-0001`, applicationName: "Fictional dimmer",
      applicationVersion: version, catalogSectionId: "M-00FA_CS-1", sourceNames: [source] } };
}
const ITEMS: CatalogItem[] = [
  catalogItem("M-00FA_CI-1", "M-00FA_H-1_HP-0001-10-0001", "10", "fictional-dimmer-v1.knxprod"),
  catalogItem("M-00FA_CI-2", "M-00FA_H-1_HP-0001-10-0001", "10", "fictional-dimmer-v1-copy.knxprod"),
  catalogItem("M-00FA_CI-3", "M-00FA_H-1_HP-0001-20-0001", "20", "fictional-dimmer-v2.knxprod"),
];
const CLEANUP_BACKUP = "products.sqlite.backup-1760270500-ef56ef56ef56.sqlite";

interface Fixture {
  unexpected: string[];
  items: CatalogItem[];
  cleanupPreviews: string[][];
  cleanupApplies: { itemIds: string[]; token: string; confirmed: boolean }[];
  previews: number;
  applies: { token: boolean }[];
  restores: string[];
  deletes: { name: string; identity: string; confirmed: boolean }[];
  directInstalls: number;
  backups: { name: string; bytes: number; modifiedUnixSeconds: number; identity: string }[];
  applyMode: "ok" | "stale";
  restoreMode: "ok" | "invalid";
}

const tree: ProjectTree = {
  schema_version: 13, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
  server_incarnation: "catalogue-fixture", project_incarnation: 1, snapshot_revision: 1,
  group_address_style: "ThreeLevel",
  installations: [{ id: 0, name: "Fictional house", topology: [], buildings: [], unassigned: [], group_ranges: [], group_addresses: [] }],
};
const report = { sha256: preview.packageSha256, scheme: 20, skipped: false, members: [], unknown: 0, conflicts: 0,
  translations: { program: 0, catalog: 0, hardware: 0, master: 0 }, droppedDatapointTypes: 0, facts: null };

async function fixture(page: Page, language: Language, theme: string, width: number): Promise<Fixture> {
  const f: Fixture = { unexpected: [], items: [], cleanupPreviews: [], cleanupApplies: [], previews: 0, applies: [], restores: [], deletes: [], directInstalls: 0, applyMode: "ok", restoreMode: "ok",
    backups: [{ name: "products.sqlite.backup-1750000000-cd34cd34cd34.sqlite", bytes: 3 * 1024 * 1024, modifiedUnixSeconds: 1750000000, identity: "id-older" }] };
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
    if (path === "/api/version") return ok({ version: "0.1.0-alpha.9+catalogue-fixture" });
    if (path === "/api/project" && method === "GET") return ok(tree);
    if (path === "/api/project/load-progress") return ok(null);
    if (path === "/api/project/status-metrics" && method === "GET") return ok(statusMetricsFixtureBody(tree));
    if (path === "/api/log") return ok([]);
    if (path === "/api/bus/discover" && method === "POST") return ok({ interfaces: [] });
    if (["/api/product-languages", "/api/catalog/manufacturers"].includes(path)) return ok([]);
    if (path === "/api/catalog/items" && method === "GET") return ok(f.items);
    if (path === "/api/catalog/cleanup/preview" && method === "POST") {
      const ids = (route.request().postDataJSON() as { itemIds: string[] }).itemIds;
      f.cleanupPreviews.push(ids);
      const items: CatalogCleanupItem[] = f.items.filter((item) => ids.includes(item.id)).map((item) => ({ ...item,
        impact: { lastPlacementForBinding: !f.items.some((other) => !ids.includes(other.id)
          && other.hardware2programRefId === item.hardware2programRefId), openProjectDevices: item.id === "M-00FA_CI-3" ? 0 : 2 } }));
      return ok({ token: `token-${ids.join("+")}`, items });
    }
    if (path === "/api/catalog/cleanup/apply" && method === "POST") {
      const body = route.request().postDataJSON() as { itemIds: string[]; token: string; confirmed: boolean };
      f.cleanupApplies.push(body);
      f.items = f.items.filter((item) => !body.itemIds.includes(item.id));
      return ok({ removedItemIds: body.itemIds, backup: CLEANUP_BACKUP });
    }
    if (["/api/device-download/status", "/api/device-address/status"].includes(path)) return refuse(404, "none", "no fixture operation");
    if (path === "/api/devices" && method === "GET") return ok({ schemaVersion: 1, serverIncarnation: "catalogue-fixture", snapshotRevision: 1, devices: [] });
    if (path === "/api/catalog/update/preview" && method === "POST") { f.previews += 1; return ok({ applied: false, preview }); }
    if (path === "/api/catalog/update/apply" && method === "POST") {
      const form = route.request().postDataBuffer()?.toString("latin1") ?? "";
      f.applies.push({ token: form.includes("fixture-token") });
      if (f.applyMode === "stale") return refuse(409, "catalogueUpdateStale", "the product database or the package changed since the preview; preview again");
      f.backups.unshift({ name: BACKUP, bytes: 3 * 1024 * 1024, modifiedUnixSeconds: 1760270400, identity: "id-newest" });
      return ok({ applied: true, preview, backup: BACKUP, packages: [{ path: null, report }] });
    }
    if (path === "/api/catalog/backups" && method === "GET") return ok({ backups: f.backups });
    if (path === "/api/catalog/backups/delete" && method === "POST") {
      const body = route.request().postDataJSON() as { name: string; identity: string; confirmed: boolean };
      f.deletes.push(body);
      f.backups = f.backups.filter((backup) => !(backup.name === body.name && backup.identity === body.identity));
      return ok({ deleted: body.name });
    }
    // Goal B5: the web never installs directly any more.
    if (path === "/api/catalog/install") { f.directInstalls += 1; return refuse(410, "strict", "direct install is not used by the web"); }
    if (path === "/api/catalog/restore" && method === "POST") {
      const name = route.request().postDataJSON().name as string;
      if (f.restoreMode === "invalid") return refuse(422, "invalidBackup", "not a usable product database backup: integrity check: page 7 is never used");
      f.restores.push(name);
      return ok({ restored: name });
    }
    f.unexpected.push(`${method} ${path}`);
    return refuse(404, "strict", "strict catalogue fixture");
  });
  await page.goto("/");
  await expect(page.locator("html")).toHaveAttribute("lang", language);
  await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
  return f;
}

async function openCatalogue(page: Page, m: Catalogue) {
  await page.keyboard.press("Control+Shift+P");
  await page.getByRole("option", { name: new RegExp(`^${m["workbench.catalog"]}`) }).click();
  const catalogue = page.getByRole("region", { name: m["catalog.title"] });
  await expect(catalogue).toBeVisible();
  return catalogue;
}
async function choosePackage(catalogue: ReturnType<Page["locator"]>) {
  // Since Goal B5 the catalogue has one product picker, and it previews packages.
  await expect(catalogue.locator('input[type="file"]')).toHaveCount(1);
  await catalogue.locator('input[type="file"]').setInputFiles({
    name: "fictional-dimmer-v2.knxprod", mimeType: "application/zip", buffer: Buffer.from("PK\u0003\u0004fixture"),
  });
}
async function noHorizontalOverflow(page: Page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
}
async function capture(page: Page, name: string) {
  if (CAPTURE) await page.screenshot({ path: join(CAPTURE, `${name}.png`), fullPage: false });
}

for (const language of ["en", "de"] as const) for (const theme of ["porcelain", "graphite"]) for (const width of [1440, 400]) {
  test(`${language} ${theme} ${width}: preview, back up and install, then restore after asking`, async ({ page }) => {
    const m: Catalogue = language === "de" ? de : en;
    const tag = `${language}-${theme}-${width}`;
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    const f = await fixture(page, language, theme, width);
    const catalogue = await openCatalogue(page, m);
    await expect(catalogue.getByText(m["catalog.installLabel"], { exact: true })).toBeVisible();
    await choosePackage(catalogue);
    const shown = catalogue.getByRole("region", { name: m["catalogUpdate.heading"].replace("{name}", preview.sourceName) });
    await expect(shown).toContainText(m["catalogUpdate.nothingYet"]);
    await expect(shown).toContainText(m["catalogUpdate.status.newVersion"]);
    await expect(shown).toContainText(m["catalogUpdate.installedVersions"].replace("{versions}", "1.0"));
    await expect(shown).toContainText(m["catalogUpdate.status.new"]);
    expect(f.applies).toEqual([]);
    await shown.scrollIntoViewIfNeeded();
    await noHorizontalOverflow(page);
    await capture(page, `${tag}-1-preview`);

    await shown.getByRole("button", { name: m["catalogUpdate.apply"], exact: true }).click();
    await expect(catalogue).toContainText(m["catalogUpdate.applied"].replace("{backup}", BACKUP));
    expect(f.applies).toEqual([{ token: true }]);
    await noHorizontalOverflow(page);
    await capture(page, `${tag}-2-installed`);

    await catalogue.getByText(m["catalogUpdate.backups.headingUnread"], { exact: true }).click();
    const names = catalogue.locator(".catalog-backup-name");
    await expect(names).toHaveText([BACKUP, "products.sqlite.backup-1750000000-cd34cd34cd34.sqlite"]);
    await names.first().scrollIntoViewIfNeeded();
    await noHorizontalOverflow(page);
    await capture(page, `${tag}-3-backups`);

    await catalogue.getByRole("button", { name: m["catalogUpdate.backups.restore"], exact: true }).first().click();
    const dialog = page.getByRole("dialog", { name: m["catalogUpdate.restore.title"] });
    await expect(dialog).toContainText(BACKUP);
    await expect(dialog.getByRole("button", { name: m["catalogUpdate.restore.keep"], exact: true })).toBeFocused();
    const box = (await dialog.boundingBox())!;
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(width);
    await capture(page, `${tag}-4-restore-ask`);
    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
    expect(f.restores).toEqual([]);

    await catalogue.getByRole("button", { name: m["catalogUpdate.backups.restore"], exact: true }).first().click();
    await dialog.getByRole("button", { name: m["catalogUpdate.restore.confirm"], exact: true }).click();
    await expect(catalogue).toContainText(m["catalogUpdate.restore.done"].replace("{name}", BACKUP));
    expect(f.restores).toEqual([BACKUP]);

    // Goal B5: delete the older backup, after asking, with exactly the listed identity.
    const older = catalogue.locator(".catalog-backup-list li").filter({ hasText: "products.sqlite.backup-1750000000-cd34cd34cd34.sqlite" });
    await older.getByRole("button", { name: m["catalogUpdate.backups.delete"], exact: true }).click();
    const ask = page.getByRole("dialog", { name: m["catalogUpdate.delete.title"] });
    await expect(ask).toContainText("products.sqlite.backup-1750000000-cd34cd34cd34.sqlite");
    await expect(ask.getByRole("button", { name: m["catalogUpdate.delete.keep"], exact: true })).toBeFocused();
    const askBox = (await ask.boundingBox())!;
    expect(askBox.x).toBeGreaterThanOrEqual(0);
    expect(askBox.x + askBox.width).toBeLessThanOrEqual(width);
    await capture(page, `${tag}-5-delete-ask`);
    await ask.getByRole("button", { name: m["catalogUpdate.delete.confirm"], exact: true }).click();
    await expect(catalogue).toContainText(m["catalogUpdate.delete.done"].replace("{name}", "products.sqlite.backup-1750000000-cd34cd34cd34.sqlite"));
    await expect(names).toHaveText([BACKUP]);
    expect(f.deletes).toEqual([{ name: "products.sqlite.backup-1750000000-cd34cd34cd34.sqlite", identity: "id-older", confirmed: true }]);
    await noHorizontalOverflow(page);
    await capture(page, `${tag}-6-deleted`);
    expect(f.directInstalls).toBe(0);
    expect(f.unexpected).toEqual([]);
    expect(errors).toEqual([]);
  });
}

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} ${width}: entries show their identity, cleanup previews before it backs up and removes`, async ({ page }) => {
    const m: Catalogue = language === "de" ? de : en;
    const tag = `cleanup-${language}-${width}`;
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    const f = await fixture(page, language, "porcelain", width);
    f.items = [...ITEMS];
    const catalogue = await openCatalogue(page, m);
    // Equal labels, told apart by application version and source.
    // The results listbox, not the manufacturer filter's own options.
    const rows = catalogue.getByRole("listbox").getByRole("option");
    await expect(rows).toHaveCount(3);
    await expect(rows.nth(2)).toContainText("fictional-dimmer-v2.knxprod");
    await expect(rows.nth(2)).toContainText("M-00FA_A-0001-20-0001");
    await noHorizontalOverflow(page);
    await capture(page, `${tag}-1-identity`);

    await catalogue.getByRole("button", { name: m["catalog.cleanup.open"], exact: true }).click();
    const dialog = page.getByRole("dialog", { name: m["catalog.cleanup.title"] });
    await expect(dialog.getByRole("checkbox")).toHaveCount(3);
    await expect(dialog.getByRole("checkbox", { checked: true })).toHaveCount(0);
    await dialog.getByRole("button", { name: m["catalog.cleanup.suggest"], exact: true }).click();
    await expect(dialog.getByRole("checkbox", { checked: true })).toHaveCount(1);
    await expect(dialog.getByRole("checkbox", { name: /fictional-dimmer-v1-copy/ })).toBeChecked();
    // The older version is chosen by hand; no version is ever preferred automatically.
    await dialog.getByRole("checkbox", { name: /M-00FA_CI-1/ }).check();
    const box = (await dialog.boundingBox())!;
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(width);
    await noHorizontalOverflow(page);
    await capture(page, `${tag}-2-select`);

    await dialog.getByRole("button", { name: m["catalog.cleanup.preview"], exact: true }).click();
    await expect(dialog).toContainText(m["catalog.cleanup.nothingYet"]);
    await expect(dialog.locator(".catalog-cleanup-warning")).toHaveCount(2);
    expect(f.cleanupPreviews).toEqual([["M-00FA_CI-1", "M-00FA_CI-2"]]);
    expect(f.cleanupApplies).toEqual([]);
    await noHorizontalOverflow(page);
    await capture(page, `${tag}-3-preview`);

    await dialog.getByRole("button", { name: m["catalog.cleanup.confirm"].replace("{count}", "2"), exact: true }).click();
    await expect(dialog).toContainText(CLEANUP_BACKUP);
    expect(f.cleanupApplies).toEqual([{ itemIds: ["M-00FA_CI-1", "M-00FA_CI-2"], token: "token-M-00FA_CI-1+M-00FA_CI-2", confirmed: true }]);
    await capture(page, `${tag}-4-done`);
    await dialog.getByRole("button", { name: m["catalog.cleanup.close"], exact: true }).click();
    await expect(dialog).toHaveCount(0);
    await expect(rows).toHaveCount(1);
    expect(f.unexpected).toEqual([]);
    expect(errors).toEqual([]);
  });
}

test("en 1440: a stale preview installs nothing and previews the same file again", async ({ page }) => {
  const f = await fixture(page, "en", "graphite", 1440);
  const catalogue = await openCatalogue(page, en);
  f.applyMode = "stale";
  await choosePackage(catalogue);
  await catalogue.getByRole("button", { name: en["catalogUpdate.apply"], exact: true }).click();
  await expect(catalogue.getByRole("alert")).toContainText(en["catalogUpdate.stale"]);
  await capture(page, "stale-en-1440");
  await catalogue.getByRole("button", { name: en["catalogUpdate.previewAgain"], exact: true }).click();
  await expect(catalogue).toContainText(en["catalogUpdate.nothingYet"]);
  expect(f.previews).toBe(2);
  expect(f.applies).toHaveLength(1);
  expect(f.unexpected).toEqual([]);
});

test("de 400: a refused restore says why and keeps the list", async ({ page }) => {
  const f = await fixture(page, "de", "porcelain", 400);
  const catalogue = await openCatalogue(page, de);
  f.restoreMode = "invalid";
  await catalogue.getByText(de["catalogUpdate.backups.headingUnread"], { exact: true }).click();
  await catalogue.getByRole("button", { name: de["catalogUpdate.backups.restore"], exact: true }).first().click();
  await page.getByRole("dialog", { name: de["catalogUpdate.restore.title"] })
    .getByRole("button", { name: de["catalogUpdate.restore.confirm"], exact: true }).click();
  await expect(catalogue.getByRole("alert")).toContainText("not a usable product database backup");
  await expect(catalogue.locator(".catalog-backup-name")).toHaveCount(1);
  await noHorizontalOverflow(page);
  await capture(page, "restore-refused-de-400");
  expect(f.restores).toEqual([]);
  expect(f.unexpected).toEqual([]);
});
