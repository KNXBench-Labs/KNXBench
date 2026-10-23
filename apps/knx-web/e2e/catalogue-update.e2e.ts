/** Built catalogue view: previewed install, backup, ask-first restore and delete, fixture API. */
import { expect, test, type Page } from "@playwright/test";
import { join } from "node:path";
import { createHash } from "node:crypto";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import type { CatalogCleanupItem, CatalogInstallReport, CatalogItem, CatalogueUpdatePreview } from "../src/api";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";
import { catalogueJobsFixture } from "./catalogue-jobs-fixture";
import type { Artifact, PublicRecord } from "../src/catalogueJobsTypes";

type Language = "en" | "de";
type Catalogue = { readonly [K in keyof typeof en]: string };
const CAPTURE = process.env.KNX_CATALOGUE_CAPTURE_DIR;
const BACKUP = "products.sqlite.backup-1760270400-ab12ab12ab12.sqlite";
const PACKAGE_BYTES = Buffer.from("PK\u0003\u0004fixture");

const preview: CatalogueUpdatePreview = {
  sourceName: "fictional-dimmer-v2.knxprod",
  packageSha256: createHash("sha256").update(PACKAGE_BYTES).digest("hex"),
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
  report: CatalogInstallReport;
  unexpected: string[];
  items: CatalogItem[];
  cleanupPreviews: string[][];
  cleanupApplies: { itemIds: string[]; token: string; confirmed: boolean }[];
  jobs: ReturnType<typeof catalogueJobsFixture>;
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

/** Minimal complete synthetic projection of this fixture's exact reports (no real install). */
function reportRecords(report: CatalogInstallReport, kind: Artifact["kind"]): PublicRecord[] {
  const rows: PublicRecord[] = [];
  const withheld = (field: "name" | "detail" | "alias", value: string) =>
    [{ field, reason: "untrustedFreeText" as const, rawUtf8Bytes: String(Buffer.byteLength(value)) }];
  const summary = { packageAlreadyInstalled: preview.packageAlreadyInstalled, wrapper: false, packages: "1",
    programs: String(preview.programs.length), productsAdded: String(preview.productsAdded),
    hardware2ProgramsAdded: String(preview.hardware2programsAdded), idConflicts: String(preview.idConflicts), growthCliffPossible: true as const };
  if (kind === "preview") rows.push({ ordinal: "0", kind: "previewSummary", ...summary, withheld: [] });
  else rows.push({ ordinal: "0", kind: "resultSummary", packages: "1", cataloguePageBytes: "3145728", nextJobInputLimitExceeded: false, withheld: [] });
  for (const [index, program] of preview.programs.entries()) {
    rows.push({ ordinal: String(rows.length), kind: "program", programKey: String(index), packageKey: "0",
      id: program.id, manufacturerId: program.manufacturerId, name: null,
      applicationNumber: program.applicationNumber, applicationVersion: program.applicationVersion,
      status: program.status, withheld: program.name === null ? [] : withheld("name", program.name) });
    for (const version of program.installedVersions) rows.push({ ordinal: String(rows.length), kind: "installedVersion",
      programKey: String(index), id: version.id, applicationVersion: version.applicationVersion, withheld: [] });
  }
  // This deliberately narrow fixture must not silently omit newly added report content.
  expect(report.members).toEqual([]);
  expect(report.conflicts).toBe(0);
  expect(report.facts?.counts ?? []).toEqual([]);
  expect(report.facts?.unknownConstructs ?? []).toEqual([]);
  rows.push({ ordinal: String(rows.length), kind: "packageReport", packageKey: "0", path: null,
    sha256: report.sha256, scheme: report.scheme, skipped: report.skipped, unknown: String(report.unknown),
    conflicts: String(report.conflicts), translations: { program: String(report.translations.program), catalog: String(report.translations.catalog),
      hardware: String(report.translations.hardware), master: String(report.translations.master) },
    droppedDatapointTypes: String(report.droppedDatapointTypes), facts: report.facts ? "available" : "unavailable",
    unknownOccurrences: report.facts ? String(report.facts.unknownOccurrences) : null, baggage: "unavailable", withheld: [] });
  if (report.facts) for (const diagnostic of report.facts.diagnostics) rows.push({ ordinal: String(rows.length), kind: "diagnostic",
    packageKey: "0", diagnosticKind: diagnostic.kind, archivePath: diagnostic.archivePath, xmlPath: diagnostic.xmlPath,
    detail: null, occurrences: String(diagnostic.occurrences), withheld: withheld("detail", diagnostic.detail) });
  else rows.push({ ordinal: String(rows.length), kind: "historyUnavailable", packageKey: "0", evidenceKind: "installFacts", reason: "notRecorded", withheld: [] });
  rows.push({ ordinal: String(rows.length), kind: "provenance", packageKey: "0", sourceKey: "0", sha256: report.sha256,
    alias: null, withheld: withheld("alias", preview.sourceName) });
  rows.push({ ordinal: String(rows.length), kind: "baggageSummary", packageKey: "0", status: "unavailable", declarations: null, payloads: null, withheld: [] });
  rows.push({ ordinal: String(rows.length), kind: "historyUnavailable", packageKey: "0", evidenceKind: "baggage", reason: "unavailable", withheld: [] });
  return rows;
}

async function fixture(page: Page, language: Language, theme: string, width: number): Promise<Fixture> {
  const jobs = catalogueJobsFixture({ records: kind => reportRecords(f.report, kind), backup: BACKUP,
    stale: () => f.applyMode === "stale",
    onApplied: () => f.backups.unshift({ name: BACKUP, bytes: 3 * 1024 * 1024, modifiedUnixSeconds: 1760270400, identity: "id-newest" }) });
  const f: Fixture = { report: { ...report }, jobs, unexpected: [], items: [], cleanupPreviews: [], cleanupApplies: [], restores: [], deletes: [], directInstalls: 0, applyMode: "ok", restoreMode: "ok",
    backups: [{ name: "products.sqlite.backup-1750000000-cd34cd34cd34.sqlite", bytes: 3 * 1024 * 1024, modifiedUnixSeconds: 1750000000, identity: "id-older" }] };
  await page.setViewportSize({ width, height: 900 });
  await page.route("https://api.github.com/**", (route) => { f.unexpected.push(route.request().url()); return route.abort(); });
  await page.route("**/api/**", (route) => {
    const path = new URL(route.request().url()).pathname;
    const method = route.request().method();
    const ok = (body: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
    const refuse = (status: number, kind: string, error: string) => route.fulfill({ status, contentType: "application/json", body: JSON.stringify({ error, kind }) });
    const jobReply = jobs.answer(route);
    if (jobReply) return jobReply;
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
async function choosePackage(catalogue: ReturnType<Page["locator"]>, m: Catalogue, f: Fixture) {
  const before = f.jobs.creates.length;
  await expect(catalogue.locator('input[type="file"]')).toHaveCount(1);
  await catalogue.locator('input[type="file"]').setInputFiles({
    name: "fictional-dimmer-v2.knxprod", mimeType: "application/zip", buffer: PACKAGE_BYTES,
  });
  const selection = catalogue.locator("[data-catalogue-job-selection]");
  await expect(selection.getByRole("combobox", { name: m["catalogJob.profile"] })).toHaveValue("standard");
  expect(f.jobs.creates).toHaveLength(before); // Picking a file is not upload consent.
  await selection.getByRole("button", { name: m["catalogJob.start"], exact: true }).click();
  const shown = catalogue.getByRole("region", { name: m["catalogJob.title"], exact: true });
  await expect(shown).toContainText(m["catalogUpdate.nothingYet"]);
  expect(f.jobs.creates).toHaveLength(before + 1);
  expect(f.jobs.creates.at(-1)).toMatchObject({ sourceName: preview.sourceName, profile: "standard", allowLargePackage: false });
  expect(f.jobs.uploads).toHaveLength(before + 1);
  expect(f.jobs.status()?.source.sha256).toBe(preview.packageSha256);
  return shown;
}
async function installPreview(shown: ReturnType<Page["locator"]>, m: Catalogue) {
  const apply = shown.getByRole("button", { name: m["catalogUpdate.apply"], exact: true });
  await expect(apply).toBeDisabled();
  await shown.getByRole("checkbox", { name: m["catalogJob.confirm"], exact: true }).check();
  await expect(apply).toBeEnabled();
  await apply.click();
}
async function readReport(shown: ReturnType<Page["locator"]>, m: Catalogue) {
  await shown.getByRole("button", { name: m["catalogJob.details"], exact: true }).click();
  const report = shown.locator(".catalogue-job-report");
  await expect(report).toContainText(m["catalogJob.lastPage"]);
  return report;
}
async function noHorizontalOverflow(page: Page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
}
async function capture(page: Page, name: string) {
  if (CAPTURE) await page.screenshot({ path: join(CAPTURE, `${name}.png`), fullPage: false });
}

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} ${width}: master provenance reports retain structured details and explicit redaction`, async ({ page }) => {
    const m: Catalogue = language === "de" ? de : en;
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    const f = await fixture(page, language, "graphite", width);
    const source = "a".repeat(64);
    const winner = "b".repeat(64);
    const detail = `function_type FT-1; candidate ${source}#4; winner ${winner}#1`;
    const malformed = "FunctionPoint at declaration 8: missing parent; <source-text>";
    // Preserve real producer addresses: terminal generated ordinal, one occurrence
    // per declaration. Raw free text remains a synthetic privacy sentinel.
    f.report.facts = { counts: [], unknownConstructs: [], unknownOccurrences: 0, diagnostics: [
      { kind: "master-identity-collision", archivePath: "knx_master.xml", xmlPath: "/KNX/MasterData/FunctionTypes/FunctionType[@master-ordinal=4]", detail, occurrences: 1 },
      { kind: "malformed-master-declaration", archivePath: "knx_master.xml", xmlPath: "/KNX/MasterData/FunctionTypes/FunctionPoint[@master-ordinal=8]", detail: malformed, occurrences: 1 },
    ] };
    const catalogue = await openCatalogue(page, m);
    const job = await choosePackage(catalogue, m, f);
    await installPreview(job, m);
    await expect(job).toContainText(m["catalogJob.state.succeeded"]);
    const shown = await readReport(job, m);
    const diagnosticRows = shown.locator("article").filter({ has: page.getByRole("heading", { name: m["catalogJob.record.diagnostic"], exact: true }) });
    await expect(diagnosticRows).toHaveCount(2);
    for (const diagnostic of f.report.facts.diagnostics) {
      const row = diagnosticRows.filter({ hasText: diagnostic.kind });
      await expect(row).toContainText(diagnostic.archivePath);
      await expect(row).toContainText(diagnostic.xmlPath);
      await expect(row.locator("dl > div").filter({ has: page.getByText(m["catalogJob.field.occurrences"], { exact: true }) }).locator("dd"))
        .toHaveText(String(diagnostic.occurrences));
      const detailField = row.locator(":scope > dl > div").filter({ has: page.getByText(m["catalogJob.field.detail"], { exact: true }) });
      await expect(detailField.locator("dd")).toHaveText(m["catalogJob.notAvailable"]);
      await expect(row).toContainText("untrustedFreeText");
      await expect(row.getByText(m["catalogJob.field.rawUtf8Bytes"], { exact: true }).locator("..").locator("dd"))
        .toHaveText(String(Buffer.byteLength(diagnostic.detail)));
    }
    // The production public projection withholds free text by contract. The raw-detail
    // disclosures remain covered unchanged in CatalogBrowser.test.tsx's legacy consumer.
    await expect(shown).not.toContainText(detail);
    await expect(shown).not.toContainText(malformed);
    await expect(shown.locator("source-text")).toHaveCount(0);
    await shown.scrollIntoViewIfNeeded();
    const bounds = (await shown.boundingBox())!;
    expect(bounds.x).toBeGreaterThanOrEqual(0);
    expect(bounds.x + bounds.width).toBeLessThanOrEqual(width);
    await noHorizontalOverflow(page);
    expect(await shown.evaluate((element) => element.scrollWidth - element.clientWidth)).toBeLessThanOrEqual(1);
    if (CAPTURE) await shown.screenshot({ path: join(CAPTURE, `${language}-${width}-master-provenance.png`) });
    expect(f.jobs.applies).toHaveLength(1);
    expect(f.jobs.status()?.commit.outcome).toBe("committed");
    expect(f.unexpected).toEqual([]);
    expect(errors).toEqual([]);
  });
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
    const shown = await choosePackage(catalogue, m, f);
    await expect(shown).toContainText(m["catalogUpdate.nothingYet"]);
    const previewReport = await readReport(shown, m);
    for (const program of preview.programs) {
      const row = previewReport.locator("article").filter({ hasText: program.id });
      await expect(row.locator(":scope > dl > div").filter({ has: page.getByText(m["catalogJob.field.status"], { exact: true }) }).locator("dd"))
        .toHaveText(program.status); // Keep exact new/newVersion statuses, not substring matches.
      await expect(row).toContainText(program.applicationVersion!);
    }
    const installedVersion = previewReport.locator("article").filter({ hasText: "M-00FA_A-0001-10-0001" });
    await expect(installedVersion).toContainText("1.0");
    expect(f.jobs.applies).toEqual([]);
    await shown.scrollIntoViewIfNeeded();
    await noHorizontalOverflow(page);
    await capture(page, `${tag}-1-preview`);

    await installPreview(shown, m);
    await expect(shown).toContainText(m["catalogJob.state.succeeded"]);
    await expect(shown).toContainText(m["catalogJob.commit.committed"]);
    await expect(shown).toContainText(BACKUP);
    expect(f.jobs.applies).toHaveLength(1);
    expect(f.jobs.status()?.backup).toEqual({ state: "retained", name: BACKUP, identity: "2".repeat(32), publication: "fileAndDirectorySyncAcknowledged" });
    await expect(shown.getByRole("alert")).toHaveCount(0);
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

    await dialog.getByRole("button", { name: m["catalog.cleanup.confirm.other"].replace("{count}", "2"), exact: true }).click();
    await expect(dialog).toContainText(CLEANUP_BACKUP);
    expect(f.cleanupApplies).toEqual([{ itemIds: ["M-00FA_CI-1", "M-00FA_CI-2"], token: "token-M-00FA_CI-1+M-00FA_CI-2", confirmed: true }]);
    await capture(page, `${tag}-4-done`);
    await dialog.getByRole("button", { name: m["catalog.cleanup.close"], exact: true }).click();
    await expect(dialog).toHaveCount(0);
    await expect(rows).toHaveCount(1);
    // The result stays in the catalogue after the dialog is gone.
    await expect(catalogue.locator(".catalog-cleanup-notice")).toContainText(CLEANUP_BACKUP);
    expect(f.unexpected).toEqual([]);
    expect(errors).toEqual([]);
  });
}

test("en 1440: a stale preview installs nothing and explicitly previews the same file again", async ({ page }) => {
  const f = await fixture(page, "en", "graphite", 1440);
  const catalogue = await openCatalogue(page, en);
  f.applyMode = "stale";
  const shown = await choosePackage(catalogue, en, f);
  const original = f.jobs.creates[0];
  const backupListBefore = [...f.backups];
  await installPreview(shown, en);
  await expect(shown.getByRole("alert")).toContainText(en["catalogJob.error.catalogueUpdateStale"]);
  expect(f.jobs.status()?.state).toBe("stale");
  expect(f.jobs.status()?.commit).toEqual({ outcome: "notCommitted", evidence: "noLiveMutation" });
  expect(f.jobs.status()?.backup.state).toBe("none");
  expect(f.backups).toEqual(backupListBefore);
  await capture(page, "stale-en-1440");
  // A spent job cannot reapply. Dispose settled history, then explicitly reselect the same file.
  await shown.getByRole("button", { name: en["catalogJob.dispose"], exact: true }).click();
  await expect(catalogue.locator('input[type="file"]')).toBeEnabled();
  expect(f.jobs.disposes).toHaveLength(1);
  f.applyMode = "ok";
  await choosePackage(catalogue, en, f);
  expect(f.jobs.creates).toHaveLength(2);
  expect(f.jobs.creates[1]).toMatchObject({ sourceName: original.sourceName, declaredBytes: original.declaredBytes, profile: original.profile });
  expect(f.jobs.creates[1].admissionSerial).not.toBe(original.admissionSerial);
  expect(f.jobs.uploads).toHaveLength(2);
  expect(f.jobs.uploads[1]).toEqual(f.jobs.uploads[0]);
  expect(f.jobs.applies).toHaveLength(1);
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
