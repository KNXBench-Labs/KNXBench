/** Parameters-tab last-download marks and summary (ADR-0173) on intercepted fictional data. */
import { expect, test, type Page } from "@playwright/test";
import type { LastDownload, ParameterField, ParameterPanel } from "../src/api";
import type { DeviceNode } from "../src/bindings/DeviceNode";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import { messages as de } from "../src/messages/de";
import { messages as en } from "../src/messages/en";
import { formatLogTime } from "../src/logRows";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";

const device: DeviceNode = { id: 1, name: "Fictional dimmer", address: "1.1.9", description: "Fictional device", com_object_count: 0 };
const tree: ProjectTree = { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
  group_address_style: "ThreeLevel", server_incarnation: "baseline-browser", project_incarnation: 1, snapshot_revision: 1,
  installations: [{ id: 1, name: "House", topology: [{ id: 1, name: "Area", address: 1,
    lines: [{ id: 2, name: "Line", address: 1, devices: [device] }] }], unassigned: [], buildings: [], group_ranges: [], group_addresses: [] }] };

const field = (etsId: string, text: string, extra: Partial<ParameterField> = {}): ParameterField => ({
  etsId, writeEtsId: etsId, name: etsId, text, nameLanguage: null, textLanguage: null, kind: "Number", value: "30",
  valueSource: "Stored", editable: true, min: "0", max: "600", enumOptions: [], displayOrder: null, access: null,
  group: { key: "general", label: "General" }, controlsComObjects: false, cell: null, ...extra,
});
const fields: ParameterField[] = [
  field("P-Delay", "Switch-off delay with a long descriptive caption that has to wrap on narrow screens"),
  field("P-Mode", "Operating mode", { kind: "Restriction", value: "1", min: null, max: null,
    enumOptions: [{ value: "0", text: "Switching", language: null }, { value: "1", text: "Dimming", language: null }] }),
  field("P-Name", "Channel name", { kind: "Text", value: "Kitchen", min: null, max: null }),
];
const at = "2026-11-12T09:30:00.000Z";
const usable: LastDownload = { state: "usable", at, differing: [
  { refId: "P-Delay", downloaded: "10", current: "30" },
  { refId: "P-Mode", downloaded: "0", current: "1" },
  // Inactive under the current settings: counted, not shown.
  { refId: "P-Inactive", downloaded: "5", current: null },
] };

async function intercept(page: Page, language: "en" | "de", lastDownload: LastDownload) {
  const state = { unexpected: [] as string[], posts: 0, projectOpen: false };
  const panel = (): ParameterPanel => ({ programId: "A-FICTIONAL", sourceLanguage: "en", tree: null, stale: [], diagnostics: [],
    lastDownload, sections: [{ scope: null, fields }] });
  await page.route("**/api/**", (route) => {
    const url = new URL(route.request().url()); const path = url.pathname; const method = route.request().method();
    let body: unknown; let status = 200;
    if (path === "/api/auth/status" && method === "GET") body = { required: false, authenticated: true };
    else if (path === "/api/settings" && method === "GET") body = { schemaVersion: 1, status: "ok", settings: { uiLanguage: language, theme: "porcelain" } };
    else if (path === "/api/project" && method === "GET") { if (state.projectOpen) body = tree; else { status = 400; body = { error: "no project open" }; } }
    else if (path === "/api/project/new" && method === "POST") { state.projectOpen = true; body = tree; }
    else if (path === "/api/project/load-progress" && method === "GET") body = null;
    else if (path === "/api/project/status-metrics" && method === "GET") body = statusMetricsFixtureBody(tree);
    else if (path === "/api/log" && method === "GET") body = [];
    else if (path === "/api/legacy-password" && method === "GET") body = { available: true, remembered: false, problem: null };
    else if (path === "/api/bus/discover" && method === "POST") body = { interfaces: [] };
    else if (path === "/api/product-languages" && method === "GET") body = [];
    else if (["/api/device-download/status", "/api/device-download/batch/status", "/api/device-address/status"].includes(path) && method === "GET") { status = 404; body = { error: "no operation in this fixture" }; }
    else if (path === "/api/device-system2/status" && method === "GET") body = null;
    else if (path === "/api/devices" && method === "GET") body = { schemaVersion: 1, serverIncarnation: "baseline-browser", snapshotRevision: 1,
      devices: [{ id: 1, device, productRef: "Fictional-P-1", resolution: "Resolved", manufacturerId: "Fictional-M-1",
        manufacturerName: "Fictional Devices", productText: "Fictional dimmer", orderNumber: "FD-1", productTextLanguage: null, productSourceLanguage: null,
        system: { state: "family", mask: 0x07b0, family: "systemB" } }] };
    else if (path === "/api/device/1" && method === "GET") body = { ...device, com_objects: [], product: { resolution: "NoReference", product_ref: null, program_ref: null, catalog: null } };
    else if (path === "/api/device/1/parameters" && method === "GET") body = panel();
    else if (path === "/api/device/1/parameters" && method === "POST") { state.posts += 1; status = 500; body = { error: "no writes in this fixture" }; }
    else if (path === "/api/bus/monitor/telegrams" && method === "GET") body = { sessionId: 1, serverIncarnation: "baseline-browser", contextStatus: "current",
      projectOpen: true, status: "active", nextSince: 0, droppedBefore: 0, telegrams: [] };
    else if (path === "/api/device-readiness" && method === "GET") body = { devices: [], counts: {} };
    else if (path === "/api/device-address/availability" && method === "GET") body = { startAvailable: false, reason: "Not in this fixture" };
    else if (path === "/api/bus/activity" && method === "GET") body = { serverIncarnation: "fixture", coverage: "partial", historyState: "disabled", sessions: [], oneShot: [], oneShotDropped: 0, busyLocks: [], untracked: [] };
    else if (achievementsFixtureAnswer(method, path) !== undefined) body = achievementsFixtureAnswer(method, path);
    else { state.unexpected.push(`${method} ${path}`); status = 404; body = { error: "strict last-download fixture only" }; }
    return route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
  });
  return state;
}

async function openParameters(page: Page, language: "en" | "de") {
  const m = language === "en" ? en : de;
  await page.goto("/");
  await page.getByRole("button", { name: m["toolbar.newProject"], exact: true }).first().click();
  await page.getByRole("dialog", { name: m["newProject.title"], exact: true }).getByRole("button", { name: m["newProject.create"], exact: true }).click();
  await page.getByRole("dialog", { name: m["projectWizard.done.title"], exact: true }).getByRole("button", { name: m["projectWizard.done.close"], exact: true }).click();
  await page.keyboard.press("Control+Shift+P");
  await page.getByRole("combobox").fill(m["workbench.devices"]);
  await page.getByRole("combobox").press("Enter");
  await page.locator(".devices-table tbody td:nth-child(3) button").filter({ hasText: /^Fictional dimmer$/ }).click();
  await page.getByRole("tab", { name: m["workbench.parameters"], exact: true }).click();
}

const noOverflow = (page: Page) => page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} marks values that differ from the last download at ${width}px`, async ({ page }) => {
    const m = language === "de" ? de : en;
    await page.setViewportSize({ width, height: 1000 });
    const state = await intercept(page, language, usable);
    await openParameters(page, language);
    const summary = page.locator(".parameter-last-download");
    await expect(summary).toHaveAttribute("data-state", "usable");
    // The time in the browser's zone, formatted by the app's own log-time helper.
    const time = await page.evaluate((iso) => Date.parse(iso), at).then((ms) => formatLogTime(ms, language, new Date()));
    await expect(summary).toHaveText(`${m["parameters.lastDownload.differs.other"].replace("{count}", "3").replace("{time}", time!)} ${
      m["parameters.lastDownload.hidden.one"].replace("{hidden}", "1")}`);
    const workspace = page.locator(".parameter-workspace");
    await workspace.locator(".parameter-workspace-nav button").filter({ hasText: "General" }).click();
    const value = (shown: string) => m["parameters.lastDownload.value"].replace("{value}", shown);
    // The name stays the field's caption; the mark is the control's description.
    const delay = workspace.getByRole("spinbutton", { name: fields[0].text!, exact: true });
    await expect(delay).toHaveAccessibleDescription(value("10"));
    const mode = workspace.getByRole("combobox", { name: "Operating mode", exact: true });
    await expect(mode).toHaveAccessibleDescription(value("Switching"));
    const name = workspace.getByRole("textbox", { name: "Channel name", exact: true });
    await expect(name).toHaveAccessibleDescription("");
    const marks = workspace.locator(".parameter-download-mark");
    await expect(marks).toHaveCount(2);
    for (const mark of await marks.all()) {
      await expect(mark).toBeVisible();
      // A 7 px dot inside a transparent 4 px ring (the tooltip target): the dot itself must paint.
      await expect(mark).toHaveCSS("width", "7px");
      const box = (await mark.boundingBox())!;
      expect(box.width).toBeGreaterThanOrEqual(15);
      expect(box.x).toBeGreaterThanOrEqual(0);
      expect(box.x + box.width).toBeLessThanOrEqual(width);
    }
    await expect(workspace.locator('[data-ets-id="P-Mode"] .parameter-download-mark')).toHaveAttribute("title", value("Switching"));
    const summaryBox = (await summary.boundingBox())!;
    expect(summaryBox.x).toBeGreaterThanOrEqual(0);
    expect(summaryBox.x + summaryBox.width).toBeLessThanOrEqual(width + 1);
    expect(await noOverflow(page)).toBeLessThanOrEqual(1);
    expect(state.posts).toBe(0);
    expect(state.unexpected).toEqual([]);
  });
}

for (const language of ["en", "de"] as const) {
  test(`${language} says plainly when values cannot be compared, without marks`, async ({ page }) => {
    const m = language === "de" ? de : en;
    await page.setViewportSize({ width: 400, height: 1000 });
    const state = await intercept(page, language, { state: "otherProgram" });
    await openParameters(page, language);
    const summary = page.locator(".parameter-last-download");
    await expect(summary).toHaveText(m["parameters.lastDownload.otherProgram"]);
    await page.locator(".parameter-workspace-nav button").filter({ hasText: "General" }).click();
    await expect(page.locator(".parameter-download-mark")).toHaveCount(0);
    await expect(page.getByRole("spinbutton", { name: fields[0].text!, exact: true })).toHaveAccessibleDescription("");
    expect(await noOverflow(page)).toBeLessThanOrEqual(1);
    expect(state.unexpected).toEqual([]);
  });
}
