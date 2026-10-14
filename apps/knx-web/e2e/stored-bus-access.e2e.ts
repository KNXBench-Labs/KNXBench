/** Goal B3 in the browser: stored bus access is read-only, hidden until asked, never a gateway. */
import { expect, test, type Locator, type Page } from "@playwright/test";
import { join } from "node:path";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import type { StoredRecord } from "../src/api";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";

// RFC 5737 and a locally administered MAC: fictional on purpose.
const HOST = "198.51.100.23", DEVICE_IP = "192.0.2.77", MAC = "02:00:5e:10:00:01";
const CAPTURE = process.env.KNX_B3_CAPTURE_DIR;
type Catalogue = { readonly [K in keyof typeof en]: string };

const tree: ProjectTree = { schema_version: 21, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
  group_address_style: "ThreeLevel", server_incarnation: "b3-browser", snapshot_revision: 1,
  installations: [{ id: 1, name: "House", topology: [{ id: 1, name: "Area", address: 1,
    lines: [{ id: 2, name: "Ground floor", address: 1, devices: [] }] }], unassigned: [], buildings: [],
    group_ranges: [], group_addresses: [] }] };

const access = (revealed: boolean): StoredRecord => ({
  recordId: "1:aaaaaaaaaaaaaaaa", element: "BusAccess", status: "read", revealed, unread: [],
  owner: { kind: "line", id: 2, address: "1.1", name: "Ground floor", reason: null },
  source: { sourcePath: "P-0001/0.xml",
    xpath: "/KNX/Project/Installations/Installation/Topology/Area/Line[@Id='P-0001-0_L-2']/Segment/BusAccess",
    size: 142, sha256: "3f".repeat(32) },
  fields: [
    { name: "Name", part: "attribute", known: true, value: revealed ? "Synthetic interface" : null, hidden: revealed ? null : { class: "text", length: 19 } },
    { name: "Type", part: "parameter", known: true, value: "KnxIpTunneling", hidden: null },
    { name: "HostAddress", part: "parameter", known: true, value: revealed ? HOST : null, hidden: revealed ? null : { class: "ipv4Address", length: 13 } },
    { name: "IpPort", part: "parameter", known: true, value: revealed ? "3671" : null, hidden: revealed ? null : { class: "number", length: 4 } },
  ],
});
const ipConfig: StoredRecord = {
  recordId: "0:bbbbbbbbbbbbbbbb", element: "IPConfig", status: "read", revealed: false, unread: [],
  owner: { kind: "unattributed", id: null, address: null, name: null, reason: "notKeyed" },
  source: { sourcePath: "P-0001/0.xml", xpath: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/IPConfig",
    size: 64, sha256: "7a".repeat(32) },
  fields: [
    { name: "IPAddress", part: "attribute", known: true, value: null, hidden: { class: "ipv4Address", length: 10 } },
    { name: "MACAddress", part: "attribute", known: true, value: null, hidden: { class: "macAddress", length: 17 } },
  ],
};

async function intercept(page: Page, language: "en" | "de") {
  const state = { unexpected: [] as string[], requests: [] as string[] };
  // Like the real server: no project until the test creates one (ADR-0127).
  let projectOpen = false;
  await page.route("**/api/**", (route) => {
    const url = new URL(route.request().url()); const path = url.pathname; const method = route.request().method();
    state.requests.push(`${method} ${path}`);
    let body: unknown; let status = 200;
    if (path === "/api/auth/status" && method === "GET") body = { required: false, authenticated: true };
    else if (path === "/api/settings" && method === "GET") body = { schemaVersion: 1, status: "ok", settings: { uiLanguage: language, theme: "porcelain" } };
    else if (path === "/api/project" && method === "GET") { if (projectOpen) body = tree; else { status = 400; body = { error: "no project open" }; } }
    else if (path === "/api/project/new" && method === "POST") { projectOpen = true; body = tree; }
    else if (path === "/api/project/load-progress" && method === "GET") body = null;
    else if (path === "/api/product-languages" && method === "GET") body = [];
    // The workbench's own start-up interface discovery, answered empty: not this view.
    else if (path === "/api/bus/discover" && method === "POST") body = { interfaces: [] };
    else if (["/api/device-download/status", "/api/device-address/status"].includes(path) && method === "GET") { status = 404; body = { error: "no operation" }; }
    else if (path === "/api/project/stored-bus-access" && method === "GET") body = { records: [ipConfig, access(false)] };
    else if (path === "/api/project/stored-bus-access/reveal" && method === "POST") {
      const { recordId } = route.request().postDataJSON();
      if (recordId === "1:aaaaaaaaaaaaaaaa") body = access(true);
      else { status = 404; body = { error: "gone", kind: "storedBusAccessChanged" }; }
    }
    else if (path === "/api/bus/activity" && method === "GET") body = { serverIncarnation: "fixture", coverage: "partial", historyState: "disabled", sessions: [], oneShot: [], oneShotDropped: 0, busyLocks: [], untracked: [] };
    else if (achievementsFixtureAnswer(method, path) !== undefined) body = achievementsFixtureAnswer(method, path);
    else { state.unexpected.push(`${method} ${path}`); status = 404; body = { error: "strict B3 fixture only" }; }
    return route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
  });
  return state;
}

async function openProject(page: Page, m: Catalogue) {
  await page.goto("/");
  await page.getByRole("button", { name: m["toolbar.newProject"], exact: true }).first().click();
  await page.getByRole("dialog", { name: m["newProject.title"], exact: true }).getByRole("button", { name: m["newProject.create"], exact: true }).click();
  await page.getByRole("dialog", { name: m["projectWizard.done.title"], exact: true }).getByRole("button", { name: m["projectWizard.done.close"], exact: true }).click();
}

/** Nothing that could connect, write or pick a gateway was requested. */
function readOnly(requests: string[]) {
  return requests.filter((r) => r.startsWith("POST ") || r.startsWith("PUT ") || r.startsWith("DELETE "))
    .filter((r) => r !== "POST /api/project/new" && r !== "POST /api/project/stored-bus-access/reveal")
    // The app-wide achievement counter (palette use): not the project, not a gateway.
    .filter((r) => r !== "POST /api/achievements/record");
}

async function noOverflow(page: Page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
}

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  test(`${language} ${width}px: stored bus access is listed hidden, shown on request, never a gateway`, async ({ page }) => {
    const m: Catalogue = language === "en" ? en : de;
    await page.setViewportSize({ width, height: 900 });
    const state = await intercept(page, language);
    await openProject(page, m);
    // Count from here: the app's own start-up (interface discovery) is not this view.
    const before = state.requests.length;
    await page.keyboard.press("Control+Shift+P");
    await page.getByRole("combobox").fill(m["storedBusAccess.command"]);
    await page.getByRole("option").filter({ hasText: m["storedBusAccess.command"] }).first().click();
    const dialog = page.getByRole("dialog", { name: m["storedBusAccess.title"], exact: true });
    await expect(dialog).toBeVisible();
    await expect(dialog).toContainText(m["storedBusAccess.intro"]);
    await expect(dialog.locator("[data-record-id]")).toHaveCount(2);
    await expect(dialog).toContainText(m["storedBusAccess.owner.notKeyed"]);
    await expect(dialog).toContainText("KnxIpTunneling");
    for (const secret of [HOST, DEVICE_IP, MAC]) await expect(dialog).not.toContainText(secret);
    await noOverflow(page);
    if (CAPTURE) await page.screenshot({ path: join(CAPTURE, `b3-dialog-${language}-${width}-hidden.png`) });

    const record = dialog.locator('[data-record-id="1:aaaaaaaaaaaaaaaa"]');
    await record.getByRole("button", { name: m["storedBusAccess.show"], exact: true }).click();
    await expect(record).toContainText(HOST);
    await expect(dialog.locator('[data-record-id="0:bbbbbbbbbbbbbbbb"]')).not.toContainText(DEVICE_IP);
    await record.locator("summary").click();
    await expect(record).toContainText("Line[@Id='P-0001-0_L-2']");
    await noOverflow(page);
    if (CAPTURE) await page.screenshot({ path: join(CAPTURE, `b3-dialog-${language}-${width}-shown.png`) });
    await record.getByRole("button", { name: m["storedBusAccess.hide"], exact: true }).click();
    await expect(record).not.toContainText(HOST);
    await dialog.getByRole("button", { name: m["storedBusAccess.close"], exact: true }).click();
    await expect(dialog).toHaveCount(0);

    const since = state.requests.slice(before);
    expect(since.filter((r) => r === "POST /api/project/stored-bus-access/reveal")).toHaveLength(1);
    expect(readOnly(since), "nothing connects, writes or picks a gateway").toEqual([]);
    expect(since.some((r) => r.includes("/api/bus/") && !r.endsWith("/api/bus/activity"))).toBe(false);
    expect(state.unexpected).toEqual([]);
  });
}

test("en 1440px: the line inspector shows its own stored bus access", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  const state = await intercept(page, "en");
  await openProject(page, en);
  const before = state.requests.length;
  await page.locator("button.tree-label", { hasText: /^Line 1: Ground floor$/ }).click();
  // The properties panel and the structure workspace both show the line.
  await expect(page.locator(".stored-bus-access-line")).toHaveCount(2);
  const ids = await page.locator(".stored-bus-access-line").evaluateAll((sections) =>
    sections.map((section) => section.getAttribute("aria-labelledby") ?? ""));
  expect(new Set(ids).size, "no duplicate heading ids").toBe(2);
  for (const id of ids) expect(await page.locator(`[id="${id}"]`).count(), `exactly one #${id}`).toBe(1);
  const section = page.getByRole("complementary").locator(".stored-bus-access-line");
  // The properties panel is narrow at any window width: its fields stack, label over
  // value, while the wider structure workspace keeps them side by side.
  const placement = (scope: Locator) => scope.locator(".stored-bus-access-fields > div").filter({ hasText: "KnxIpTunneling" })
    .evaluate((row) => { const [dt, dd] = [row.querySelector("dt")!, row.querySelector("dd")!].map((e) => e.getBoundingClientRect());
      return { stacked: dd.top >= dt.bottom - 1, valueLines: Math.round(dd.height / parseFloat(getComputedStyle(row.querySelector("dd")!).lineHeight)) }; });
  expect(await placement(page.getByRole("complementary"))).toEqual({ stacked: true, valueLines: 1 });
  expect((await placement(page.getByRole("region", { name: en["structure.editSelection"] }))).stacked).toBe(false);
  await expect(section.getByRole("heading", { name: en["storedBusAccess.lineTitle"] })).toBeVisible();
  await expect(section.locator("[data-record-id]")).toHaveCount(1);
  await expect(section).not.toContainText(HOST);
  if (CAPTURE) await page.screenshot({ path: join(CAPTURE, "b3-line-inspector-en-1440.png") });
  expect(readOnly(state.requests.slice(before))).toEqual([]);
  expect(state.unexpected).toEqual([]);
});
