/** Fictional own-backend catalogue fixtures with explicit actions and blocked external requests. */
import { expect, test, type Page } from "@playwright/test";
import { createHash } from "node:crypto";
import { onlineEntry, onlineStatus } from "../src/onlineCatalogueFixtures";
import { ONLINE_BASE, type OnlineEntry } from "../src/onlineCatalogueApi";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import type { PublicRecord } from "../src/catalogueJobsTypes";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { busActivityFixtureAnswer } from "./bus-activity-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";
import { catalogueJobsFixture } from "./catalogue-jobs-fixture";

export const PACKAGE = Buffer.from("fictional verified package");
const SHA = createHash("sha256").update(PACKAGE).digest("hex");
export const MODERN = onlineEntry({ size: PACKAGE.length, sha256: SHA });
export const LEGACY = onlineEntry({ id: "legacy-4", name: "Fictional legacy", fileName: "fictional.vd4", fileType: "vd4", downloaded: true });
export const UNSUPPORTED = onlineEntry({ id: "legacy-1", name: "Fictional unsupported", fileName: "fictional.vd1", fileType: "vd1", importable: false });
export const BACKUP = "products.sqlite.backup-1760270400-ab12ab12ab12.sqlite";
export const LEGACY_REPORT = {
  payloadSha256: "a".repeat(64), originalSha256: "b".repeat(64), namespace: "LX1A2B3C4D", skipped: false,
  programs: ["M-1092_A-LX1A2B3C4D-300"], catalogItems: 1, parameters: 10, parameterRefs: 12, comObjectRefs: 3,
  translations: 8, diagnostics: [], password: "given", remembered: true, rememberProblem: null,
};

function records(kind: "preview" | "result"): PublicRecord[] {
  return kind === "preview" ? [{ ordinal: "0", kind: "previewSummary", packageAlreadyInstalled: false,
    wrapper: false, packages: "1", programs: "1", productsAdded: "1", hardware2ProgramsAdded: "1",
    idConflicts: "0", growthCliffPossible: true, withheld: [] }] : [{ ordinal: "0", kind: "resultSummary",
    packages: "1", cataloguePageBytes: "3145728", nextJobInputLimitExceeded: false, withheld: [] }];
}
export type CaptureBodies = { status: Buffer; search: Buffer };
export async function onlineFixture(page: Page, language: "en" | "de", width: number, captured?: CaptureBodies) {
  const m = language === "de" ? de : en;
  const f = {
    status: onlineStatus(), entries: [MODERN, LEGACY, UNSUPPORTED] as OnlineEntry[],
    calls: [] as { method: string; path: string; body: unknown }[], unexpected: [] as string[],
    downloadMode: "done" as "done" | "running" | "selectionChanged" | "dailyLimit" | "forbidden" | "notFound" | "serviceUnavailable",
    cancelTerminal: false, legacy: [] as unknown[], settingsWrites: [] as unknown[],
    jobs: catalogueJobsFixture({ records, backup: BACKUP, onlineFeed: { entryId: MODERN.id, sha256: SHA, bytes: PACKAGE } }),
  };
  let projectOpen = false;
  const tree: ProjectTree = { schema_version: 13, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
    server_incarnation: "online-fixture", project_incarnation: 1, snapshot_revision: 1, group_address_style: "ThreeLevel",
    installations: [{ id: 0, name: "Fictional installation", unassigned: [], group_addresses: [], group_ranges: [], topology: [], buildings: [] }] };
  await page.setViewportSize({ width, height: 900 });
  // No external provider, release service, LAN discovery or hardware may be reached.
  const previewOrigin = new URL(test.info().project.use.baseURL!).origin;
  await page.route("**/*", route => {
    const url = new URL(route.request().url());
    if (url.origin === previewOrigin) return route.continue();
    f.unexpected.push(`external ${url.origin}${url.pathname}`); return route.abort();
  });
  await page.route("**/api/**", async route => {
    const request = route.request(); const url = new URL(request.url()); const path = url.pathname; const method = request.method();
    const body = request.postData() && request.headers()["content-type"]?.includes("json") ? request.postDataJSON() : null;
    f.calls.push({ method, path: path + url.search, body });
    const ok = (value: unknown, status = 200) => route.fulfill({ status, headers: { "Content-Type": "application/json", "Cache-Control": "no-store" }, body: JSON.stringify(value) });
    const raw = (bytes: Buffer) => route.fulfill({ status: 200, headers: { "Content-Type": "application/json", "Cache-Control": "no-store" }, body: bytes });
    const fail = (kind: string, status = 409) => ok({ kind, error: "Fictional refusal",
      ...(["dailyLimit", "rateLimited"].includes(kind) ? { resetAt: "2026-01-04T00:00:00Z", retryAfterSecs: 30 } : {}) }, status);
    if (captured && method === "GET" && path === ONLINE_BASE) return raw(captured.status);
    if (captured && method === "GET" && path === `${ONLINE_BASE}/search`) return raw(captured.search);
    const job = f.jobs.answer(route); if (job) return job;
    const achievement = achievementsFixtureAnswer(method, path); if (achievement !== undefined) return ok(achievement);
    const bus = busActivityFixtureAnswer(method, path); if (bus !== undefined) return ok(bus);
    if (path === "/api/auth/status" && method === "GET") return ok({ required: false, authenticated: true });
    if (path === "/api/settings" && method === "GET") return ok({ conditionalPatchVersion: 1, schemaVersion: 1, status: "ok", settings: {
      uiLanguage: language, theme: "graphite", motionLevel: "off", releaseChecksEnabled: false,
      onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.22" } } });
    if (path === "/api/settings" && method === "PUT") { f.settingsWrites.push(body); return ok({ conditionalPatchVersion: 1, schemaVersion: 1, status: "ok", settings: body.settings }); }
    if (path === "/api/version" && method === "GET") return ok({ version: "0.1.0-alpha.22+online-fixture" });
    if (path === "/api/project" && method === "GET") return projectOpen ? ok(tree) : fail("noProject", 400);
    if (path === "/api/project/new" && method === "POST") { projectOpen = true; return ok(tree); }
    if (path === "/api/project/status-metrics" && method === "GET") return ok(statusMetricsFixtureBody(tree));
    if (path === "/api/project/load-progress" && method === "GET") return ok(null);
    if (path === "/api/legacy-password" && method === "GET") return ok({ available: true, remembered: false, problem: null });
    if (path === "/api/log" && method === "GET") return ok([]);
    if (path === "/api/bus/discover" && method === "POST") return ok({ interfaces: [] });
    if (method === "GET" && ["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items"].includes(path)) return ok([]);
    if (method === "GET" && path === "/api/catalog/backups") return ok({ backups: [] });
    if (method === "GET" && ["/api/device-download/status", "/api/device-download/batch/status", "/api/device-address/status"].includes(path)) return fail("none", 404);
    if (method === "GET" && path === "/api/device-system2/status") return ok(null);
    if (method === "GET" && path === "/api/devices") return ok({ schemaVersion: 1, serverIncarnation: "online-fixture", snapshotRevision: 1, devices: [] });
    if (path === "/api/online-catalogue" && method === "GET") return ok({ available: true, sources: [{ id: "reko", label: "Reko", defaultBaseUrl: "https://catalogue.example" }] });
    if (path === ONLINE_BASE && method === "GET") return ok(f.status);
    if (path === `${ONLINE_BASE}/sync` && method === "POST") {
      f.status = { ...f.status, index: { ...onlineStatus().index!, lastCheckedAt: "2026-01-03T00:00:00Z" } };
      return ok({ outcome: "notModified", status: f.status });
    }
    if (path === `${ONLINE_BASE}/search` && method === "GET") {
      expect(url.searchParams.get("limit")).toBe("25");
      const offset = Number(url.searchParams.get("offset"));
      return ok({ total: 30, offset, limit: 25, entries: offset ? [onlineEntry({ id: "second", name: "Fictional page two" })] : f.entries });
    }
    if (path === `${ONLINE_BASE}/credentials` && ["PUT", "DELETE"].includes(method)) {
      f.status = { ...f.status, credentials: method === "DELETE" ? { state: "empty", mode: null, user: null, problem: null }
        : { state: "stored", mode: body.mode, user: body.user ?? null, problem: null } };
      return ok(f.status);
    }
    if (path === `${ONLINE_BASE}/config` && method === "PUT") {
      f.status = { ...f.status, baseUrl: body.baseUrl ?? f.status.defaultBaseUrl, baseUrlIsDefault: body.baseUrl === null, index: null,
        credentials: { state: "problem", mode: null, user: null, problem: "vaultOriginMismatch" } };
      return ok(f.status);
    }
    const entry = f.entries.find(value => path === `${ONLINE_BASE}/entries/${value.id}/download` || path === `${ONLINE_BASE}/entries/${value.id}/install-legacy`);
    if (entry && method === "POST") {
      expect([...url.searchParams]).toEqual([["sha256", entry.sha256]]);
      if (path.endsWith("/install-legacy")) {
        f.legacy.push(body);
        return body.password ? ok(LEGACY_REPORT) : fail("legacyPasswordRequired", 422);
      }
      if (!["done", "running"].includes(f.downloadMode)) return fail(f.downloadMode, { selectionChanged: 409, dailyLimit: 429, forbidden: 403, notFound: 404, serviceUnavailable: 503 }[f.downloadMode as "selectionChanged"]);
      f.status = { ...f.status, download: { state: f.downloadMode as "done" | "running", entryId: entry.id,
        receivedBytes: f.downloadMode === "done" ? entry.size : 0, totalBytes: entry.size, error: null } };
      if (f.downloadMode === "done") f.entries = f.entries.map(value => value.id === entry.id ? { ...value, downloaded: true } : value);
      return ok(f.status.download, f.downloadMode === "running" ? 202 : 200);
    }
    if (path === `${ONLINE_BASE}/download` && method === "DELETE") {
      if (f.cancelTerminal) f.status = { ...f.status, download: { ...f.status.download, state: "failed", error: { kind: "cancelled" } } };
      return ok(f.status.download);
    }
    f.unexpected.push(`${method} ${path}`); return fail("strict", 404);
  });
  await page.goto("/"); await expect(page.locator("html")).toHaveAttribute("lang", language);
  return { ...f, m, state: f };
}

/** Dismiss only unrelated startup/project-created notices before exercising catalogue actions. */
export async function dismissOnlineStartupToasts(page: Page, language: "en" | "de") {
  const m = language === "de" ? de : en;
  await expect(page.locator("html")).toHaveAttribute("lang", language);
  const buttons = page.locator(".toast:not(.toast--leaving)").getByRole("button", { name: m["toast.dismiss"], exact: true });
  for (let round = 0; round < 20 && await buttons.count() > 0; round++) await buttons.first().click();
  await expect(page.locator(".toast")).toHaveCount(0);
}
export async function openOnline(page: Page, language: "en" | "de") {
  const m = language === "de" ? de : en;
  await page.getByRole("button", { name: language === "de" ? "Neues Projekt…" : "New project…", exact: true }).first().click();
  await page.getByRole("dialog", { name: language === "de" ? "Neues Projekt" : "New project", exact: true })
    .getByRole("button", { name: language === "de" ? "Projekt anlegen" : "Create project", exact: true }).click();
  await page.getByRole("dialog", { name: language === "de" ? "Projekt angelegt" : "Project created", exact: true })
    .getByRole("button", { name: language === "de" ? "Fertig" : "Done", exact: true }).click();
  await dismissOnlineStartupToasts(page, language);
  await page.keyboard.press("Control+Shift+P");
  await page.getByRole("option", { name: new RegExp(`^${m["workbench.catalog"]}`) }).click();
  const catalogue = page.getByRole("region", { name: m["catalog.title"] });
  await expect(catalogue).toBeVisible();
  await catalogue.getByRole("combobox", { name: m["online.source"], exact: true }).selectOption("reko");
  const online = catalogue.getByRole("region", { name: m["online.title"], exact: true });
  await expect(online.getByRole("button", { name: m["online.sync"], exact: true })).toBeEnabled();
  return { catalogue, online };
}
