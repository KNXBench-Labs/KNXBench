/** Last-download marks of parameters (ADR-0173) and objects (ADR-0174) on fictional data. */
import { expect, test, type Page } from "@playwright/test";
import type { LastDownload, ParameterField, ParameterPanel } from "../src/api";
import type { ComObjectNode } from "../src/bindings/ComObjectNode";
import type { DownloadObjectDifference } from "../src/bindings/DownloadObjectDifference";
import type { ObjectLastDownload } from "../src/bindings/ObjectLastDownload";
import type { DeviceNode } from "../src/bindings/DeviceNode";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import { messages as de } from "../src/messages/de";
import { messages as en } from "../src/messages/en";
import { formatLogTime } from "../src/logRows";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";

// The expected time is formatted in this Node process by the app's own helper;
// the browser runs in the same zone, so both sides format the same instant alike.
const zone = Intl.DateTimeFormat().resolvedOptions().timeZone;
test.use({ timezoneId: zone });

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
  // Manufacturer-restricted: only the Manufacturer fields tab shows (and marks) it.
  field("P-Factory", "Factory calibration", { access: "Read", editable: false, writeEtsId: null }),
];
const at = "2026-11-12T09:30:00.000Z";
const usable: LastDownload = { state: "usable", at, differing: [
  { refId: "P-Delay", downloaded: "10", current: "30" },
  { refId: "P-Mode", downloaded: "0", current: "1" },
  // Inactive under the current settings: counted, not shown.
  { refId: "P-Inactive", downloaded: "5", current: null },
  { refId: "P-Factory", downloaded: "12", current: "30" },
] };

// ADR-0174: one row with link changes, one whose flags differ, one object the server could not select.
const kitchen = { key: "c-kitchen", kind: "Channel" as const, text: "Kitchen", name: null, number: null, order: 0 };
const comObject = (id: number, number: number, name: string): ComObjectNode => ({
  saved_instance_priority: { status: "unavailable", reason: "NotCaptured" }, effective_priority: { status: "unresolved", reason: "InstanceUnavailable", source: null, raw: null },
  id, number, name, function_text: null, description: null, description_layer: null, dpt: null, dpt_layer: null, program_dpt: null, dpt_text: null,
  is_active: true, activation: "Active", channel: kitchen, read: false, write: true, transmit: false, update: false, communication: true, read_on_init: false, links: [] });
const comObjects = [comObject(21, 1, "Switch with a long fictional object name that has to wrap on narrow screens"), comObject(22, 2, "Status"), comObject(23, 3, "Unchanged")];
// 1/2/3 = 0x0A03, 1/2/4 = 0x0A04.
const linkChange: DownloadObjectDifference = { number: 1, instance_id: 21, flags: null, links: [
  { group_address: 0x0a03, downloaded: null, current: true }, { group_address: 0x0a04, downloaded: false, current: null }] };
const flagChange: DownloadObjectDifference = { number: 2, instance_id: 22, links: [], flags: {
  downloaded: { read: true, write: true, transmit: false, update: false, communication: true, read_on_init: null },
  current: { read: false, write: true, transmit: false, update: false, communication: true, read_on_init: null } } };
const objectsUsable: ObjectLastDownload = { links: { state: "usable", at }, flags: { state: "usable", at },
  differing: [linkChange, flagChange, { number: 9, instance_id: null, links: [], flags: { downloaded: flagChange.flags!.current, current: null } }],
  instances: { 21: linkChange, 22: flagChange }, differing_objects: 3, no_selected_row: 1 };

async function intercept(page: Page, language: "en" | "de", lastDownload: LastDownload, objectLastDownload?: ObjectLastDownload) {
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
    else if (path === "/api/device/1" && method === "GET") body = { ...device, com_objects: objectLastDownload ? comObjects : [],
      product: { resolution: "NoReference", product_ref: null, program_ref: null, catalog: null }, last_download: objectLastDownload };
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

async function openDevice(page: Page, language: "en" | "de") {
  const m = language === "en" ? en : de;
  await page.goto("/");
  await page.getByRole("button", { name: m["toolbar.newProject"], exact: true }).first().click();
  await page.getByRole("dialog", { name: m["newProject.title"], exact: true }).getByRole("button", { name: m["newProject.create"], exact: true }).click();
  await page.getByRole("dialog", { name: m["projectWizard.done.title"], exact: true }).getByRole("button", { name: m["projectWizard.done.close"], exact: true }).click();
  await page.keyboard.press("Control+Shift+P");
  await page.getByRole("combobox").fill(m["workbench.devices"]);
  await page.getByRole("combobox").press("Enter");
  await page.locator(".devices-table tbody td:nth-child(3) button").filter({ hasText: /^Fictional dimmer$/ }).click();
}

async function openParameters(page: Page, language: "en" | "de") {
  await openDevice(page, language);
  await page.getByRole("tab", { name: (language === "en" ? en : de)["workbench.parameters"], exact: true }).click();
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
    const time = formatLogTime(Date.parse(at), language, new Date());
    await expect(summary).toHaveText(`${m["parameters.lastDownload.differs.other"].replace("{count}", "4").replace("{time}", time!)} ${
      m["parameters.lastDownload.hidden.one"].replace("{hidden}", "1")} ${
      m["parameters.lastDownload.manufacturer.one"].replace("{count}", "1").replace("{tab}", m["parameters.restrictedTab"])}`);
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
    // Keyboard and touch: the value is a caption under each differing row.
    const notes = workspace.locator(".parameter-download-note");
    await expect(notes).toHaveCount(2);
    const delayNote = workspace.locator('[data-ets-id="P-Delay"] + .parameter-download-note');
    await expect(delayNote).toBeVisible();
    await expect(delayNote).toHaveText(value("10"));
    await expect(workspace.locator('[data-ets-id="P-Mode"] + .parameter-download-note')).toHaveText(value("Switching"));
    for (const note of await notes.all()) {
      const noteBox = (await note.boundingBox())!;
      expect(noteBox.x).toBeGreaterThanOrEqual(0);
      expect(noteBox.x + noteBox.width).toBeLessThanOrEqual(width);
    }
    // Focus moves nothing within the row (the caption is always there), so a
    // pointer press that moves focus from the input to the checked-editor
    // button lands where it started.
    const checkedOpen = workspace.locator('[data-checked-for="P-Delay"]');
    const offset = async () => (await checkedOpen.boundingBox())!.y - (await delay.boundingBox())!.y;
    const unfocused = await offset();
    await delay.focus();
    expect(await offset()).toBe(unfocused);
    await checkedOpen.focus();
    expect(await offset()).toBe(unfocused);
    expect(await noOverflow(page)).toBeLessThanOrEqual(1);
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

test("keeps the row and summary dots painted under forced colours", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.emulateMedia({ forcedColors: "active" });
  const state = await intercept(page, "en", usable);
  await openParameters(page, "en");
  await page.locator(".parameter-workspace-nav button").filter({ hasText: "General" }).click();
  const canvasText = await page.evaluate(() => {
    const probe = document.createElement("span");
    probe.style.color = "CanvasText";
    document.body.append(probe);
    const color = getComputedStyle(probe).color;
    probe.remove();
    return color;
  });
  const dots = [page.locator(".parameter-last-download-dot"), ...await page.locator(".parameter-workspace .parameter-download-mark").all()];
  expect(dots).toHaveLength(3);
  for (const dot of dots) {
    await expect(dot).toBeVisible();
    await expect(dot).toHaveCSS("forced-color-adjust", "none");
    await expect(dot).toHaveCSS("background-color", canvasText);
  }
  expect(state.unexpected).toEqual([]);
});

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} marks objects whose links or flags differ from the last download at ${width}px`, async ({ page }) => {
    const m = language === "de" ? de : en;
    await page.setViewportSize({ width, height: 1000 });
    const state = await intercept(page, language, usable, objectsUsable);
    await openDevice(page, language);
    const panel = page.locator(".com-table-panel");
    const summary = panel.locator(".com-last-download");
    const time = formatLogTime(Date.parse(at), language, new Date())!;
    // The grouped view starts collapsed: both selected rows are hidden, and the group says it holds them.
    await expect(summary).toHaveText(`${m["comTable.lastDownload.differs.other"].replace("{count}", "3")} ${
      m["comTable.lastDownload.hidden.other"].replace("{count}", "2")} ${m["comTable.lastDownload.noRow.one"].replace("{count}", "1")} ${
      m["comTable.lastDownload.links.usable"].replace("{time}", time)} ${m["comTable.lastDownload.flags.usable"].replace("{time}", time)}`);
    const group = panel.locator(".com-object-channel-summary");
    await expect(group).toHaveAccessibleDescription(m["comTable.lastDownload.group"]);
    await expect(group.locator(".com-object-download-mark")).toBeVisible();
    await group.click();
    await expect(summary).not.toContainText(m["comTable.lastDownload.hidden.other"].replace("{count}", "2"));
    const toggle = (id: number) => panel.locator(`tr[data-object-id="${id}"] summary`);
    await expect(toggle(21)).toHaveAccessibleName(m["comTable.edit"].replace("{number}", "1"));
    await expect(toggle(21)).toHaveAccessibleDescription(m["comTable.lastDownload.row.links"]);
    await expect(toggle(22)).toHaveAccessibleDescription(m["comTable.lastDownload.row.flags"]);
    await expect(toggle(23)).toHaveAccessibleDescription("");
    const marks = panel.locator("tbody .com-object-download-mark");
    await expect(marks).toHaveCount(3);
    for (const mark of await marks.all()) {
      await expect(mark).toBeVisible();
      await expect(mark).toHaveCSS("width", "7px");
    }
    await toggle(21).click();
    const diff = panel.locator('tr[data-editor-id="21"] .com-object-download-diff');
    await expect(diff.locator('[data-link-change="added"]')).toHaveText(`${m["comTable.lastDownload.linkAdded"]} 1/2/3 (${m["inspector.direction.send"]})`);
    const removed = diff.locator('[data-link-change="removed"] del');
    await expect(removed).toHaveText("1/2/4");
    await expect(removed).toHaveCSS("text-decoration-line", "line-through");
    await toggle(22).click();
    await expect(panel.locator('tr[data-editor-id="22"] [data-flag-change="changed"]')).toContainText(`${m["comTable.lastDownload.flagsAt"]} R W C`);
    for (const box of [await summary.boundingBox(), ...await Promise.all((await panel.locator(".com-object-download-note, .com-object-download-diff").all()).map(n => n.boundingBox()))]) {
      expect(box!.x).toBeGreaterThanOrEqual(0);
      expect(box!.x + box!.width).toBeLessThanOrEqual(width + 1);
    }
    expect(await noOverflow(page)).toBeLessThanOrEqual(1);
    expect(state.posts).toBe(0);
    expect(state.unexpected).toEqual([]);
  });
}

test("keeps the object dots painted under forced colours", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.emulateMedia({ forcedColors: "active" });
  const state = await intercept(page, "en", usable, objectsUsable);
  await openDevice(page, "en");
  await page.locator(".com-object-channel-summary").click();
  const canvasText = await page.evaluate(() => {
    const probe = document.createElement("span");
    probe.style.color = "CanvasText";
    document.body.append(probe);
    const color = getComputedStyle(probe).color;
    probe.remove();
    return color;
  });
  const dots = [page.locator(".com-last-download-dot"), ...await page.locator(".com-table-panel .com-object-download-mark").all()];
  expect(dots).toHaveLength(4);
  for (const dot of dots) {
    await expect(dot).toBeVisible();
    await expect(dot).toHaveCSS("forced-color-adjust", "none");
    await expect(dot).toHaveCSS("background-color", canvasText);
  }
  expect(state.unexpected).toEqual([]);
});
