/** Built app: GA members, gateway choice, device → download hand-off, step guide. */
import { expect, test, type Page } from "@playwright/test";
import { join } from "node:path";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import type { GroupAddressNode } from "../src/bindings/GroupAddressNode";
import type { DeviceNode } from "../src/bindings/DeviceNode";
import type { ComObjectNode } from "../src/bindings/ComObjectNode";
import type { DeviceDetail } from "../src/bindings/DeviceDetail";
import type { BusDiscoveredInterface, DeviceDownloadStatusResponse } from "../src/api";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";

type Language = "en" | "de";
type Catalogue = { readonly [K in keyof typeof en]: string };
const CAPTURE = process.env.KNX_GA_MEMBERS_CAPTURE_DIR;

// Fictional project: installation 1 "House" with a push button, an actuator
// and an unaddressed spare; installation 2 "Garage" with its own actuator.
interface FixtureObject { id: number; number: number; name: string; dpt: string; deviceId: number }
const DEVICES: Record<number, DeviceNode> = {
  10: { id: 10, name: "Push button", address: "1.1.10", description: null, com_object_count: 2 },
  11: { id: 11, name: "Actuator", address: "1.1.11", description: null, com_object_count: 2 },
  12: { id: 12, name: "Spare sensor", address: null, description: null, com_object_count: 1 },
  20: { id: 20, name: "Garage actuator", address: "2.1.1", description: null, com_object_count: 1 },
};
const OBJECTS: FixtureObject[] = [
  { id: 100, number: 0, name: "Switch", dpt: "DPST-1-1", deviceId: 10 },
  { id: 101, number: 1, name: "Status", dpt: "DPST-1-1", deviceId: 10 },
  { id: 110, number: 0, name: "Channel A switch", dpt: "DPST-1-1", deviceId: 11 },
  { id: 111, number: 1, name: "Channel A feedback", dpt: "DPST-1-1", deviceId: 11 },
  { id: 120, number: 0, name: "Temperature", dpt: "DPST-9-1", deviceId: 12 },
  { id: 200, number: 0, name: "Door", dpt: "DPST-1-1", deviceId: 20 },
];
type Link = { objectId: number; gaId: number; direction: "Send" | "Receive" };
const GAS: Record<number, { name: string; address: string; installation: number }> = {
  500: { name: "Kitchen light", address: "1/0/1", installation: 1 },
  501: { name: "Kitchen light status", address: "1/0/2", installation: 1 },
  600: { name: "Garage door", address: "2/0/1", installation: 2 },
};

function buildTree(links: Link[], revision: number, history: number): ProjectTree {
  const ga = (id: number): GroupAddressNode => ({
    id, name: GAS[id].name, address: GAS[id].address, range: null, dpts: ["DPST-1-1"],
    links: links.filter((link) => link.gaId === id).map((link) => {
      const object = OBJECTS.find((o) => o.id === link.objectId)!;
      const device = DEVICES[object.deviceId];
      return { device_id: device.id, device_name: device.name, device_address: device.address, com_object_id: object.id,
        com_object_number: object.number, com_object_name: object.name, direction: link.direction };
    }),
  });
  return {
    schema_version: 13, errors: 0, warnings: 0, can_undo: history > 0, can_redo: false, is_modified: history > 0,
    group_address_style: "ThreeLevel", server_incarnation: "ga-members-browser", project_incarnation: 1, snapshot_revision: revision,
    installations: [
      { id: 1, name: "House", buildings: [], group_ranges: [], group_addresses: [ga(500), ga(501)], unassigned: [DEVICES[12]],
        topology: [{ id: 1, name: "Area", address: 1, lines: [{ id: 2, name: "Ground floor", address: 1, devices: [DEVICES[10], DEVICES[11]] }] }] },
      { id: 2, name: "Garage", buildings: [], group_ranges: [], group_addresses: [ga(600)], unassigned: [],
        topology: [{ id: 3, name: "Area", address: 2, lines: [{ id: 4, name: "Garage line", address: 1, devices: [DEVICES[20]] }] }] },
    ],
  };
}

function detail(deviceId: number, links: Link[]): DeviceDetail {
  const device = DEVICES[deviceId];
  const objects: ComObjectNode[] = OBJECTS.filter((o) => o.deviceId === deviceId).map((o) => ({
    id: o.id, number: o.number, name: o.name, dpt: o.dpt, dpt_layer: "Instance", description: null, description_layer: null,
    is_active: true, read: false, write: true, transmit: true, update: false, communication: true, read_on_init: false,
    links: links.filter((link) => link.objectId === o.id).map((link) => ({ ga_id: link.gaId, address: GAS[link.gaId].address, name: GAS[link.gaId].name, direction: link.direction })),
    activation: "NotEvaluated", channel: null, program_dpt: null, dpt_text: null, function_text: null,
  }));
  return { id: device.id, name: device.name, description: null, address: device.address, com_objects: objects,
    product: { product_ref: null, program_ref: null, catalog: null, resolution: "NoReference" } };
}

const PLAN = {
  planId: 9, address: "1.1.10", deviceId: 10, deviceName: "Push button", programId: "M-00FA_A-0001-01-0000",
  maskVersion: 0x07b0, manufacturer: 0x00fa, parameterValues: 2, groupLinks: 1,
  segments: [{ id: "M-00FA_A-0001-01-0000_RS-04-00000", address: 0x4000, size: 16, written: 16 }], dataOctets: 16,
  steps: ["connect", "check the mask", "write 4000h"], confirmationPhrase: "I confirm download to 1.1.10",
  support: { level: "untested", evidence: null, inferences: [] }, warnings: [], untestedAcknowledgement: "I accept an untested download to 1.1.10",
  partial: false, notWritten: [],
};
const RUNNING: DeviceDownloadStatusResponse = { downloadId: 4, address: "1.1.11", deviceName: "Actuator", steps: 3, dataOctets: 16,
  status: { state: "running" }, nextSince: 1, events: [{ kind: "stepStarted", number: 1, of: 3, step: "connect" }], backupFile: null };

const gateway = (endpoint: string, name: string, address: string): BusDiscoveredInterface =>
  ({ controlEndpoint: endpoint, friendlyName: name, individualAddress: address, supportsTunnelling: true });

interface Options { language: Language; theme: string; width: number; gateways?: BusDiscoveredInterface[]; running?: boolean;
  /** #18: the status read answers only once this settles, like a slow server. */
  statusGate?: Promise<void> }
interface Fixture { unexpected: string[]; writes: string[]; linkBodies: unknown[]; reads: string[]; undo: number; discover: number }

async function fixture(page: Page, options: Options): Promise<Fixture> {
  const f: Fixture = { unexpected: [], writes: [], linkBodies: [], reads: [], undo: 0, discover: 0 };
  let links: Link[] = [{ objectId: 100, gaId: 500, direction: "Send" }];
  const history: Link[][] = [];
  let revision = 1;
  const tree = () => buildTree(links, revision, history.length);
  await page.setViewportSize({ width: options.width, height: 900 });
  await page.route("https://api.github.com/**", (route) => { f.unexpected.push(route.request().url()); return route.abort(); });
  await page.route("**/api/**", (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    const method = request.method();
    const ok = (body: unknown, status = 200) => route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
    const achievements = achievementsFixtureAnswer(method, path);
    if (achievements !== undefined) return ok(achievements);
    if (path === "/api/auth/status") return ok({ required: false, authenticated: true });
    if (path === "/api/settings" && method === "GET") return ok({ conditionalPatchVersion: 1, schemaVersion: 1, status: "ok", settings: {
      uiLanguage: options.language, theme: options.theme, motionLevel: "off", releaseChecksEnabled: false,
      onboardingGuide: { seenStage: "alpha", version: "0.1.0-alpha.9" } } });
    if (path === "/api/settings" && method === "PUT") return ok({ conditionalPatchVersion: 1, schemaVersion: 1, status: "ok", settings: request.postDataJSON().settings });
    if (path === "/api/version") return ok({ version: "0.1.0-alpha.9+gfixture" });
    if (path === "/api/project" && method === "GET") return ok(tree());
    if (path === "/api/project/load-progress") return ok(null);
    if (path === "/api/product-languages") return ok([]);
    if (path === "/api/bus/discover" && method === "POST") { f.discover += 1; return ok({ interfaces: options.gateways ?? [] }); }
    if (path === "/api/devices" && method === "GET") return ok({ schemaVersion: 1, serverIncarnation: "ga-members-browser", snapshotRevision: revision,
      devices: Object.values(DEVICES).map((device) => ({ id: device.id, device, productRef: null, resolution: "NoReference", manufacturerId: null,
        manufacturerName: null, productText: null, orderNumber: null, productTextLanguage: null, productSourceLanguage: null })), problem: null });
    if (/^\/api\/device\/\d+$/.test(path) && method === "GET") { f.reads.push(path); return ok(detail(Number(path.split("/").at(-1)), links)); }
    if (/^\/api\/device\/\d+\/parameters$/.test(path) && method === "GET") return ok({ programId: null, sections: [], stale: [], diagnostics: [] });
    if (path === "/api/group-links" && (method === "POST" || method === "DELETE")) {
      const body = request.postDataJSON() as { comObjectId: number; gaId: number; direction: "Send" | "Receive" | "Both" };
      f.linkBodies.push({ method, ...body });
      const directions = body.direction === "Both" ? ["Send", "Receive"] as const : [body.direction] as const;
      history.push(links.map((link) => ({ ...link })));
      if (method === "POST") links = [...links, ...directions.map((direction) => ({ objectId: body.comObjectId, gaId: body.gaId, direction }))];
      else links = links.filter((link) => !(link.objectId === body.comObjectId && link.gaId === body.gaId && directions.includes(link.direction as never)));
      revision += 1;
      return ok(tree());
    }
    if (path === "/api/undo" && method === "POST") { f.undo += 1; links = history.pop() ?? links; revision += 1; return ok(tree()); }
    if (path === "/api/device-download/status" && method === "GET") {
      return (options.statusGate ?? Promise.resolve()).then(() => options.running ? ok(RUNNING) : ok({ error: "no download" }, 404));
    }
    if (path === "/api/device-address/status" && method === "GET") return ok({ error: "no operation" }, 404);
    // The monitor tab reads the server's telegram buffer when the bus
    // workspace mounts; without a session the real server answers 404.
    if (path === "/api/bus/monitor/telegrams" && method === "GET") return ok({ error: "no monitor session is active" }, 404);
    // Read-only hints of the download page (issue #10): nothing holds the gateway, nothing is marked.
    if (path === "/api/bus/activity" && method === "GET") return ok({ serverIncarnation: "fixture", coverage: "partial", historyState: "disabled", sessions: [], oneShot: [], oneShotDropped: 0, busyLocks: [], untracked: [] });
    if (path === "/api/device-readiness" && method === "GET") return ok({ devices: [], counts: {} });
    if (path === "/api/device-download/plan" && method === "POST") { f.writes.push(`${method} ${path}`); return ok(PLAN); }
    f.unexpected.push(`${method} ${path}`);
    return ok({ error: "strict GA members/download fixture" }, 404);
  });
  await page.goto("/?view=editor");
  await expect(page.locator("html")).toHaveAttribute("lang", options.language);
  await expect(page.locator("html")).toHaveAttribute("data-theme", options.theme);
  await expect(page.getByRole("navigation", { name: (options.language === "de" ? de : en)["workbench.navigation"], exact: true })).toBeVisible();
  return f;
}

async function noHorizontalOverflow(page: Page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
}

async function shot(page: Page, name: string) {
  if (CAPTURE) await page.screenshot({ path: join(CAPTURE, `${name}.png`), fullPage: false });
}

async function inViewport(page: Page, selector: string) {
  const box = await page.locator(selector).first().boundingBox();
  expect(box, selector).not.toBeNull();
  expect(box!.x).toBeGreaterThanOrEqual(-1);
  expect(box!.x + box!.width).toBeLessThanOrEqual(page.viewportSize()!.width + 1);
}

async function openAddresses(page: Page, m: Catalogue) {
  await page.getByRole("navigation", { name: m["workbench.navigation"], exact: true }).getByRole("button", { name: m["workbench.addresses"], exact: true }).click();
  await expect(page.locator(".ga-workspace").first()).toBeVisible();
  // Issue #11: address lists start closed; this case works on rows, so open them like a user.
  // Issue #11: every installation's tree opens compact; open each one.
  await expect(page.locator("[data-ga-expand-all]").first()).toBeVisible();
  for (const button of await page.locator("[data-ga-expand-all]").all()) await button.click();
}

async function openDeviceEditor(page: Page, m: Catalogue, name: string) {
  await page.keyboard.press("Control+Shift+P");
  await page.getByRole("combobox").fill(m["workbench.devices"]);
  await page.getByRole("combobox").press("Enter");
  await page.locator(".devices-workspace").getByRole("button", { name, exact: true }).click();
  await expect(page.locator(".device-workspace h2")).toHaveText(name);
}

for (const language of ["en", "de"] as const) for (const theme of ["porcelain", "graphite"]) for (const width of [1440, 400]) {
  const tag = `${language}-${theme}-${width}`;
  const m: Catalogue = language === "de" ? de : en;

  test(`${tag}: add, refuse and undo group-address members`, async ({ page }) => {
    const errors: string[] = []; page.on("pageerror", (error) => errors.push(error.message));
    const f = await fixture(page, { language, theme, width });
    await openAddresses(page, m);
    await page.locator('button[data-ga-select="501"]').click();
    const panel = page.locator(".ga-workspace .address-links-panel");
    await expect(panel).toContainText(m["addressTable.noLinksYet"]);
    await panel.getByRole("button", { name: m["addressTable.members.add"], exact: true }).click();
    const form = panel.locator(".ga-member-form");
    await expect(form).toContainText(m["addressTable.members.installationRule"]);
    // The link rule: the garage actuator belongs to another installation.
    await expect(form.locator("[data-ga-member-device] option")).toHaveText([
      m["addressTable.members.chooseDevice"], "1.1.10 — Push button", "1.1.11 — Actuator", `${m["addressTable.members.noAddress"]} — Spare sensor`,
    ]);
    expect(f.reads).toEqual([]);
    await form.locator("[data-ga-member-device]").selectOption("11");
    await expect(form.locator("[data-ga-member-object] option")).toHaveCount(3);
    expect(f.reads).toEqual(["/api/device/11"]);
    await form.locator("[data-ga-member-object]").selectOption("110");
    await form.locator("[data-ga-member-direction]").selectOption("Both");
    await form.getByRole("button", { name: m["addressTable.members.submit"], exact: true }).scrollIntoViewIfNeeded();
    await inViewport(page, ".ga-workspace .ga-member-form");
    await noHorizontalOverflow(page);
    await shot(page, `members-form-${tag}`);
    await form.getByRole("button", { name: m["addressTable.members.submit"], exact: true }).click();
    await expect(panel.locator("tbody tr")).toHaveCount(2);
    await expect(panel.locator("tbody tr").first()).toContainText("Channel A switch");
    expect(f.linkBodies).toEqual([{ method: "POST", comObjectId: 110, gaId: 501, direction: "Both" }]);
    // The Inspector shows the same members as a compact list.
    if (width === 1440) await expect(page.locator(".inspector .ga-members .ga-member")).toHaveCount(2);
    await noHorizontalOverflow(page);
    await shot(page, `members-added-${tag}`);

    // One undo step takes both directions of the one action back.
    // Ctrl+Z belongs to a focused field first; with none focused it undoes the project.
    await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
    await page.keyboard.press("Control+z");
    await expect(panel.locator("tbody tr")).toHaveCount(0);
    expect(f.undo).toBe(1);

    // A duplicate is refused before any request.
    await page.locator('button[data-ga-select="500"]').click();
    await panel.getByRole("button", { name: m["addressTable.members.add"], exact: true }).click();
    await form.locator("[data-ga-member-device]").selectOption("10");
    await form.locator("[data-ga-member-object]").selectOption("100");
    await expect(form).toContainText(m["addressTable.members.duplicate"]);
    await expect(form.getByRole("button", { name: m["addressTable.members.submit"], exact: true })).toBeDisabled();
    await form.getByRole("button", { name: m["addressTable.members.submit"], exact: true }).scrollIntoViewIfNeeded();
    await shot(page, `members-duplicate-${tag}`);
    // Still only the one add: the refused duplicate sent nothing.
    expect(f.linkBodies).toHaveLength(1);
    expect(f.writes).toEqual([]);
    expect(f.unexpected).toEqual([]);
    expect(errors).toEqual([]);
  });

  test(`${tag}: device editor opens the download tab with the device and walks the steps`, async ({ page }) => {
    const errors: string[] = []; page.on("pageerror", (error) => errors.push(error.message));
    const f = await fixture(page, { language, theme, width, gateways: [
      gateway("198.51.100.10:3671", "KNX IP router", "1.1.250"), gateway("198.51.100.11:3671", "KNX IP router", "1.1.251")] });
    await openDeviceEditor(page, m, "Push button");
    await inViewport(page, ".device-workspace .device-download-entry button");
    await page.locator(".device-workspace").getByRole("button", { name: m["devices.downloadToDevice"], exact: true }).click();
    const panel = page.locator(".device-download-panel");
    await expect(panel).toBeVisible();
    await expect(page.locator(".bus-diagnostics-tabs button[aria-current=page]")).toHaveText(m["deviceDownload.title"]);
    await expect(panel.locator("select[data-download-device]")).toHaveValue("10");
    await expect(panel.locator("[data-handoff=selected]")).toContainText("1.1.10");
    // Navigation alone: no plan, no start, no further search.
    expect(f.writes).toEqual([]);
    const searchesAfterStartup = f.discover;
    // Two gateways with the same name stay distinguishable; neither is chosen for the user.
    await expect(panel.locator("select[data-download-gateway-found] option")).toHaveText([
      m["deviceDownload.gatewayChooseFound"], "KNX IP router — 198.51.100.10:3671 · 1.1.250", "KNX IP router — 198.51.100.11:3671 · 1.1.251",
    ]);
    await expect(panel.locator("input[data-download-gateway]")).toHaveValue("");
    await expect(panel.locator('[data-guide-step="device"]')).toHaveAttribute("data-guide-state", "done");
    await expect(panel.locator('[aria-current="step"]')).toHaveAttribute("data-guide-step", "gateway");
    await inViewport(page, ".device-download-guide");
    await noHorizontalOverflow(page);
    await shot(page, `download-handoff-${tag}`);

    await panel.locator("select[data-download-gateway-found]").selectOption("198.51.100.11:3671");
    await expect(panel.locator("input[data-download-gateway]")).toHaveValue("198.51.100.11:3671");
    await panel.getByRole("button", { name: m["deviceDownload.preparePlan"], exact: true }).click();
    await expect(panel.locator(".device-download-plan")).toBeVisible();
    await expect(panel.locator('[data-guide-step="plan"]')).toHaveAttribute("data-guide-state", "done");
    await expect(panel.locator('[data-guide-step="confirm"]')).toContainText(m["deviceDownload.guide.missing.acknowledgement"]);
    await panel.locator(".device-download-guide").scrollIntoViewIfNeeded();
    await noHorizontalOverflow(page);
    await shot(page, `download-plan-${tag}`);
    expect(f.writes).toEqual(["POST /api/device-download/plan"]);
    expect(f.discover).toBe(searchesAfterStartup);
    expect(f.unexpected).toEqual([]);
    expect(errors).toEqual([]);
  });
}

test("one found gateway is filled in; a repeated click selects the device again", async ({ page }) => {
  const f = await fixture(page, { language: "en", theme: "porcelain", width: 1440, gateways: [gateway("198.51.100.10:3671", "Line coupler", "1.1.250")] });
  await openDeviceEditor(page, en, "Push button");
  await page.locator(".device-workspace").getByRole("button", { name: en["devices.downloadToDevice"], exact: true }).click();
  const panel = page.locator(".device-download-panel");
  await expect(panel.locator("input[data-download-gateway]")).toHaveValue("198.51.100.10:3671");
  await expect(panel.locator("[data-gateway-note]")).toHaveText(en["deviceDownload.gatewayNote.auto"]);
  // The user picks another device, then asks for the push button again.
  await panel.locator("select[data-download-device]").selectOption("11");
  await page.getByRole("navigation", { name: en["workbench.navigation"], exact: true }).getByRole("button", { name: en["workbench.devices"], exact: true }).click();
  await page.locator(".devices-workspace").getByRole("button", { name: "Push button", exact: true }).click();
  await page.locator(".device-workspace").getByRole("button", { name: en["devices.downloadToDevice"], exact: true }).click();
  await expect(panel.locator("select[data-download-device]")).toHaveValue("10");
  expect(f.writes).toEqual([]);
  expect(f.unexpected).toEqual([]);
});

test("no gateway found, typed address kept, unaddressed device explained", async ({ page }) => {
  const f = await fixture(page, { language: "en", theme: "porcelain", width: 1440, gateways: [] });
  await openDeviceEditor(page, en, "Spare sensor");
  const entry = page.locator(".device-workspace .device-download-entry");
  await expect(entry.getByRole("button", { name: en["devices.downloadToDevice"], exact: true })).toBeDisabled();
  await expect(entry).toContainText(en["devices.downloadNeedsAddress"]);
  await openDeviceEditor(page, en, "Actuator");
  await page.locator(".device-workspace").getByRole("button", { name: en["devices.downloadToDevice"], exact: true }).click();
  const panel = page.locator(".device-download-panel");
  await expect(panel.locator(".device-download-gateway-status")).toHaveText(en["deviceDownload.gatewaySearch.none"]);
  await panel.locator("input[data-download-gateway]").fill("192.0.2.77:3671");
  await panel.getByRole("button", { name: en["deviceDownload.gatewaySearchAgain"], exact: true }).click();
  // Issue #10: with an address typed, an empty search says that address is used as-is.
  await expect(panel.locator(".device-download-gateway-status")).toHaveText(en["deviceDownload.gatewaySearch.noneUseTyped"]);
  await expect(panel.locator("input[data-download-gateway]")).toHaveValue("192.0.2.77:3671");
  expect(f.writes).toEqual([]);
  expect(f.unexpected).toEqual([]);
});

test("a running download is never taken over by a hand-off", async ({ page }) => {
  const f = await fixture(page, { language: "en", theme: "graphite", width: 1440, running: true });
  await openDeviceEditor(page, en, "Push button");
  await page.locator(".device-workspace").getByRole("button", { name: en["devices.downloadToDevice"], exact: true }).click();
  const panel = page.locator(".device-download-panel");
  await expect(panel.locator("[data-handoff=busy]")).toHaveText(en["deviceDownload.handoff.busy"]);
  await expect(panel.locator("select[data-download-device]")).toHaveValue("");
  await expect(panel.locator("select[data-download-device]")).toBeDisabled();
  await expect(panel.locator(".device-download-progress")).toContainText("1.1.11");
  await shot(page, "download-busy-en-graphite-1440");
  expect(f.writes).toEqual([]);
  expect(f.unexpected).toEqual([]);
});

// Issue #18: the requested device is named at once; it is selected only once the status is known.
for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  const theme = (language === "en") === (width === 1440) ? "graphite" : "porcelain";
  test(`#18 ${language}-${theme}-${width}: a slow status read still shows the requested device at once`, async ({ page }) => {
    const m: Catalogue = language === "de" ? de : en;
    const errors: string[] = []; page.on("pageerror", (error) => errors.push(error.message));
    let release!: () => void;
    const statusGate = new Promise<void>((done) => { release = done; });
    const f = await fixture(page, { language, theme, width, statusGate });
    await openDeviceEditor(page, m, "Push button");
    await page.locator(".device-workspace").getByRole("button", { name: m["devices.downloadToDevice"], exact: true }).click();
    const panel = page.locator(".device-download-panel");
    const pending = panel.locator('[data-handoff="pending"]');
    await expect(pending).toHaveText(m["deviceDownload.handoff.pending"].replace("{address}", "1.1.10").replace("{name}", "Push button"));
    // Context, not admission: nothing is selected or can be prepared yet.
    await expect(panel.locator("select[data-download-device]")).toHaveValue("");
    await expect(panel.getByRole("button", { name: m["deviceDownload.preparePlan"], exact: true })).toBeDisabled();
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    await pending.scrollIntoViewIfNeeded();
    await shot(page, `download-pending-${language}-${theme}-${width}`);
    release();
    await expect(panel.locator("[data-handoff=selected]")).toContainText("1.1.10");
    await expect(pending).toHaveCount(0);
    await expect(panel.locator("select[data-download-device]")).toHaveValue("10");
    expect(f.writes).toEqual([]);
    expect(f.unexpected).toEqual([]);
    expect(errors).toEqual([]);
  });
}
