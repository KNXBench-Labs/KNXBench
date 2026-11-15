/** Explicit-TCP device diagnostics: real App gestures against replayed or fictional HTTP bodies. */
import { expect, test, type Page, type Route } from "@playwright/test";
import { join } from "node:path";
import { readFileSync } from "node:fs";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import type { DeviceNode } from "../src/bindings/DeviceNode";
import { IDENTITY_MEMORY_REASON, type DiagnosticView } from "../src/diagnosticApi";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { busActivityFixtureAnswer } from "./bus-activity-fixture";
import { fulfillNoProjectOpen } from "./server-project-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";
import { openFileSubmenu } from "./file-menu";

const gateway = "192.0.2.10:3671";
const UNOPENED = { requestedTransport: "tcp", actualTransport: null, assignedAddress: null, skippedFramesBeforeDisconnect: null };
type Mode = "ok" | "eof" | "unavailable" | "legacy" | "missing" | "udp";

function tree(): ProjectTree {
  const devices: DeviceNode[] = [{ id: 1, name: "Fictional device 1", address: "1.1.25", description: null, com_object_count: 0 }];
  return { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
    group_address_style: "ThreeLevel", server_incarnation: "diagnostics-browser", project_incarnation: 3, snapshot_revision: 1,
    installations: [{ id: 1, name: "House", topology: [{ id: 1, name: "Area", address: 1, lines: [{ id: 1, name: "Ground floor", address: 1, devices }] }],
      unassigned: [], buildings: [], group_ranges: [], group_addresses: [] }] };
}
const items = (read: boolean) => ["descriptor-type-0", "manufacturer-id", "hardware-type"].map((query, i) => ({
  order: i + 1, query, objectIndex: i === 0 ? null : 0, propertyId: i === 0 ? null : i === 1 ? 12 : 78, startIndex: i === 0 ? null : 1,
  requestedElements: i === 0 ? null : 1, expectedBytes: i === 2 ? 6 : 2, status: read ? "read" : "not-read", receivedBytes: read ? (i === 2 ? 6 : 2) : 0,
  returnedElements: read && i > 0 ? 1 : null, bytes: read ? (i === 2 ? [0, 1, 0, 0, 0, 91] : i === 1 ? [0, 1] : [0, 37]) : [],
  attemptedAt: read ? "2026-11-14T06:00:00Z" : null, completedAt: read ? "2026-11-14T06:00:01Z" : null, error: null }));
/** Hand-written stand-ins; `KNX_DIAGNOSTICS_HTTP_CAPTURE_DIR` swaps in the producer's own bodies. */
function body(kind: "preview" | "start" | "complete" | "eof"): DiagnosticView {
  if (process.env.KNX_DIAGNOSTICS_HTTP_CAPTURE_DIR) {
    return JSON.parse(readFileSync(join(process.env.KNX_DIAGNOSTICS_HTTP_CAPTURE_DIR, `${kind}.json`), "utf8")) as DiagnosticView;
  }
  const done = kind === "complete" || kind === "eof";
  const report = { kind: "knxbench-device-diagnostics", format: 1 as const, build: "fixture", collectedAt: "2026-11-14T06:00:00Z", device: "device-1",
    projectReference: "b".repeat(64), profile: "generic-identity-v1" as const, assertedMask: null, assertedManufacturer: null, simulated: false,
    written: false as const, complete: done, items: items(done) as DiagnosticView["report"]["items"], stopReason: kind === "eof" ? "connection-lost" : null,
    memoryStatus: "not-requested" as const, memoryReason: IDENTITY_MEMORY_REASON, disconnectAttemptCompleted: done };
  if (kind === "eof") report.items[2] = { ...report.items[2], status: "error", bytes: [], receivedBytes: 0, returnedElements: null, error: "connection-lost" };
  return { token: "c".repeat(64), target: "1.1.25", gateway, phase: kind === "preview" ? "preview" : done ? "complete" : "running", cancelRequested: false,
    report, memory: null, exportable: done, connection: kind === "preview" || kind === "start" ? UNOPENED :
      { requestedTransport: "tcp", actualTransport: "tcp", assignedAddress: "1.1.250", skippedFramesBeforeDisconnect: kind === "eof" ? 4 : 2 } } as DiagnosticView;
}
async function intercept(page: Page, language: "en" | "de", mode: Mode) {
  const state = { tree: tree(), unexpected: [] as string[], paths: [] as string[], bodies: [] as unknown[] };
  let opened = false;
  await page.route("**/api/**", async (route: Route) => {
    const path = new URL(route.request().url()).pathname; const method = route.request().method();
    let out: unknown; let status = 200;
    if (path === "/api/auth/status" && method === "GET") out = { required: false, authenticated: true };
    else if (path === "/api/settings" && method === "GET") out = { schemaVersion: 1, status: "ok", settings: { uiLanguage: language, theme: language === "en" ? "graphite" : "porcelain" } };
    else if (path === "/api/project" && method === "GET") { if (!opened) return fulfillNoProjectOpen(route); out = state.tree; }
    else if (path === "/api/project/new" && method === "POST") { opened = true; out = state.tree; }
    else if (path === "/api/project/load-progress" && method === "GET") out = null;
    else if (path === "/api/project/status-metrics" && method === "GET") out = statusMetricsFixtureBody(state.tree);
    else if (path === "/api/bus/discover" && method === "POST") out = { interfaces: [] };
    else if (path === "/api/legacy-password" && method === "GET") out = { available: true, remembered: false, problem: null };
    else if (path === "/api/product-languages" && method === "GET") out = [];
    else if (["/api/device-download/status", "/api/device-download/batch/status", "/api/device-address/status"].includes(path) && method === "GET") { status = 404; out = { error: "no active operation" }; }
    else if (path === "/api/device-system2/status" && method === "GET") out = null;
    else if (path === "/api/devices" && method === "GET") out = { schemaVersion: 1, serverIncarnation: "diagnostics-browser", snapshotRevision: 1, devices: [] };
    else if (path.startsWith("/api/device-diagnostics/")) {
      state.paths.push(`${method} ${path}`); if (method === "POST") state.bodies.push(route.request().postDataJSON());
      const final = mode === "eof" ? "eof" : "complete";
      const strip = (v: DiagnosticView) => { const { connection: _c, ...rest } = v; return rest; };
      if (path === "/api/device-diagnostics/preview/tcp") {
        if (mode === "legacy") { status = 404; out = { error: "unsupported" }; }
        else if (mode === "missing") out = strip(body("preview"));
        else out = body("preview");
      } else if (path === "/api/device-diagnostics/preview") out = strip(body("preview"));
      else if (path === "/api/device-diagnostics/start") { status = 202; out = mode === "udp" ? strip(body("start")) : body("start"); }
      else if (method === "GET") {
        const v = body(final);
        out = mode === "udp" ? strip(v) : mode === "unavailable" ? { ...v, connection: UNOPENED, report: { ...v.report, stopReason: "gateway-connect-failed" } } : v;
      } else { state.unexpected.push(`${method} ${path}`); status = 404; out = { error: "strict diagnostics fixture" }; }
    } else if (achievementsFixtureAnswer(method, path) !== undefined) out = achievementsFixtureAnswer(method, path);
    else if (busActivityFixtureAnswer(method, path) !== undefined) out = busActivityFixtureAnswer(method, path);
    else { state.unexpected.push(`${method} ${path}`); status = 404; out = { error: "strict diagnostics fixture" }; }
    return route.fulfill({ status, contentType: "application/json", body: JSON.stringify(out) });
  });
  return state;
}
async function openDialog(page: Page, language: "en" | "de", fresh: boolean) {
  const m = language === "en" ? en : de;
  if (fresh) {
    await page.goto("/");
    await page.getByRole("button", { name: m["toolbar.newProject"], exact: true }).first().click();
    await page.getByRole("dialog", { name: m["newProject.title"], exact: true }).getByRole("button", { name: m["newProject.create"], exact: true }).click();
    await page.getByRole("dialog", { name: m["projectWizard.done.title"], exact: true }).getByRole("button", { name: m["projectWizard.done.close"], exact: true }).click();
  }
  await page.locator(".file-menu > summary").click(); await openFileSubmenu(page, "help");
  await page.locator(".file-menu [data-diagnostics-open]").click();
  const dialog = page.getByRole("dialog", { name: m["diagnostics.title"], exact: true }); await expect(dialog).toBeVisible();
  return dialog;
}
async function capture(page: Page, name: string) {
  if (process.env.KNX_DIAGNOSTICS_CAPTURE_DIR) {
    const mode = test.info().config.configFile?.endsWith("playwright.devices.config.ts") ? "production" : "development";
    await page.screenshot({ path: join(process.env.KNX_DIAGNOSTICS_CAPTURE_DIR, mode, `${name}.png`), fullPage: true });
  }
}
const fits = async (page: Page, dialog: ReturnType<Page["locator"]>, width: number) => {
  const box = await dialog.boundingBox(); expect(box!.x).toBeGreaterThanOrEqual(0); expect(box!.x + box!.width).toBeLessThanOrEqual(width + 1);
  expect(await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth)).toBeLessThanOrEqual(1);
};
async function preview(dialog: ReturnType<Page["locator"]>, transport: "udp" | "tcp") {
  await dialog.locator("[data-diagnostics-device]").selectOption("1"); await dialog.locator("[data-diagnostics-gateway]").fill(gateway);
  await dialog.locator("[data-diagnostics-transport]").selectOption(transport); await dialog.locator("[data-diagnostics-preview]").click();
}

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  const m = language === "en" ? en : de;
  test(`${language} explicit TCP diagnostics freeze the transport and show actual facts at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    const state = await intercept(page, language, "ok"); const dialog = await openDialog(page, language, true);
    const select = dialog.locator("[data-diagnostics-transport]");
    await expect(select).toHaveValue("udp"); expect(state.paths).toEqual([]);
    await select.selectOption("tcp"); expect(state.paths).toEqual([]);
    await expect(select).toContainText(m["diagnostics.transport.tcp"]); await expect(dialog.locator("[data-diagnostics-transport-hint]")).toHaveText(m["diagnostics.transportHint"]);
    await preview(dialog, "tcp");
    await expect(dialog.locator("[data-diagnostics-frozen-transport]")).toHaveText(m["diagnostics.reviewedTransport"].replace("{transport}", "TCP"));
    expect(state.paths).toEqual(["POST /api/device-diagnostics/preview/tcp"]);
    expect(state.bodies[0]).toEqual({ deviceId: 1, gateway, profile: "generic-identity-v1", context: { serverIncarnation: "diagnostics-browser", projectIncarnation: 3, revision: 1 } });
    await capture(page, `tcp-preview-${language}-${width}`);
    await dialog.locator("[data-diagnostics-read-consent]").check(); await dialog.locator("[data-diagnostics-start]").click();
    await expect(dialog.locator("[data-diagnostics-result]")).toBeVisible();
    await expect(dialog.locator("[data-diagnostics-connection]")).toHaveText(m["diagnostics.actualConnection"].replace("{transport}", "TCP").replace("{address}", "1.1.250").replace("{skipped}", "2"));
    await expect(dialog.getByRole("alert")).toHaveCount(0);
    expect(state.paths.filter((p) => p.includes("/preview"))).toEqual(["POST /api/device-diagnostics/preview/tcp"]);
    expect(state.paths.filter((p) => p.includes("/start"))).toHaveLength(1); expect(state.unexpected).toEqual([]);
    await capture(page, `tcp-result-${language}-${width}`); await fits(page, dialog, width);
    // A reopened dialog is fresh: UDP again, nothing carried over.
    await dialog.getByRole("button", { name: m["support.close"], exact: true }).click();
    const again = await openDialog(page, language, false);
    await expect(again.locator("[data-diagnostics-transport]")).toHaveValue("udp");
  });
  test(`${language} explicit TCP peer EOF keeps the failure reason and facts at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    const state = await intercept(page, language, "eof"); const dialog = await openDialog(page, language, true);
    await preview(dialog, "tcp"); await dialog.locator("[data-diagnostics-read-consent]").check(); await dialog.locator("[data-diagnostics-start]").click();
    await expect(dialog.locator("[data-diagnostics-result]")).toBeVisible();
    await expect(dialog.locator("[data-diagnostics-stop-reason] code")).toHaveText("connection-lost");
    await expect(dialog.locator("[data-diagnostics-connection]")).toContainText("TCP");
    await expect(dialog.locator("[data-diagnostics-connection]")).toContainText("1.1.250");
    await capture(page, `tcp-eof-${language}-${width}`); await fits(page, dialog, width); expect(state.unexpected).toEqual([]);
  });
  test(`${language} explicit TCP unavailable facts are not zero at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    const state = await intercept(page, language, "unavailable"); const dialog = await openDialog(page, language, true);
    await preview(dialog, "tcp"); await dialog.locator("[data-diagnostics-read-consent]").check(); await dialog.locator("[data-diagnostics-start]").click();
    await expect(dialog.locator("[data-diagnostics-connection]")).toHaveText(m["diagnostics.connectionUnavailable"]);
    await capture(page, `tcp-unavailable-${language}-${width}`); await fits(page, dialog, width); expect(state.unexpected).toEqual([]);
  });
  for (const mode of ["legacy", "missing"] as const) {
    test(`${language} explicit TCP ${mode} server is refused without UDP fallback at ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 900 });
      const state = await intercept(page, language, mode); const dialog = await openDialog(page, language, true);
      await preview(dialog, "tcp");
      await expect(dialog.locator("[data-diagnostics-error]")).toBeVisible();
      if (mode === "legacy") await expect(dialog.locator("[data-diagnostics-error]")).toContainText(m["diagnostics.refusal.tcpUnsupported"]);
      await expect(dialog.locator("[data-diagnostics-review]")).toHaveCount(0); await expect(dialog.locator("[data-diagnostics-start]")).toHaveCount(0);
      expect(state.paths).toEqual(["POST /api/device-diagnostics/preview/tcp"]); expect(state.unexpected).toEqual([]);
      await capture(page, `tcp-${mode}-${language}-${width}`); await fits(page, dialog, width);
    });
  }
  test(`${language} UDP stays the default path with no connection facts at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    const state = await intercept(page, language, "udp"); const dialog = await openDialog(page, language, true);
    await preview(dialog, "udp"); await dialog.locator("[data-diagnostics-read-consent]").check(); await dialog.locator("[data-diagnostics-start]").click();
    await expect(dialog.locator("[data-diagnostics-result]")).toBeVisible();
    await expect(dialog.locator("[data-diagnostics-connection],[data-diagnostics-frozen-transport]")).toHaveCount(0);
    expect(state.paths[0]).toBe("POST /api/device-diagnostics/preview"); expect(state.paths.some((p) => p.includes("/tcp"))).toBe(false);
    expect(state.unexpected).toEqual([]);
  });
}
