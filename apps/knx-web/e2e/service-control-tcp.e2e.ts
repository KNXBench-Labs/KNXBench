/** Explicit-TCP service control through the full app against a strict HTTP fixture. */
import { expect, test, type Page } from "@playwright/test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { dismissToasts, intercept, openProject } from "./communication-fixture";

const gateway = "192.0.2.10:3671";
type Mode = "ok" | "eof" | "unavailable" | "legacy" | "missing" | "wrong" | "udp";
interface Answer { status: number; body: Record<string, unknown> }
const facts = (skipped: number) => ({ requestedTransport: "tcp", actualTransport: "tcp", assignedAddress: "1.1.250", skippedFramesBeforeDisconnect: skipped });
const UNOPENED = { requestedTransport: "tcp", actualTransport: null, assignedAddress: null, skippedFramesBeforeDisconnect: null };
const device = { address: "1.1.25", raw: "0000", mask: "07B0", individualAddressWriteEnabled: false };
/** Hand-written stand-ins; `KNX_SERVICE_CONTROL_HTTP_CAPTURE_DIR` swaps in the Rust test's own bodies. */
function answer(name: "read" | "write" | "eof" | "refused-read"): Answer {
  const dir = process.env.KNX_SERVICE_CONTROL_HTTP_CAPTURE_DIR;
  if (dir) return JSON.parse(readFileSync(join(dir, `${name}.json`), "utf8")) as Answer;
  switch (name) {
    case "read": return { status: 200, body: { ...device, connection: facts(2) } };
    case "write": return { status: 200, body: { before: device, after: "0004", individualAddressWriteEnabled: true, written: true, backupPath: "[LOCAL MOCK BACKUP]", connection: facts(2) } };
    case "eof": return { status: 502, body: { error: "A_PropertyValue_Read PID_SERVICE_CONTROL: the connection was broken down during A_PropertyValue_Response", kind: "serviceControlFailed", connection: facts(4) } };
    default: return { status: 502, body: { error: "protocol error: synthetic connect refusal", kind: "gatewayConnectFailed", connection: UNOPENED } };
  }
}
async function serve(page: Page, language: "en" | "de", mode: Mode) {
  const state = await intercept(page, language);
  state.tree = { ...state.tree, project_incarnation: 3 };
  const settings = { uiLanguage: language, theme: "porcelain", preferredGateway: gateway, releaseChecksEnabled: false, debugIndividualAddressWriteEnable: true };
  const calls: { method: string; path: string; search: string; body: unknown }[] = [];
  await page.route("https://api.github.com/**", (route) => { state.unexpected.push("external release check"); return route.abort(); });
  await page.route("**/api/legacy-password", (route) => route.fulfill({ json: { available: true, remembered: false, problem: null } }));
  await page.route("**/api/settings", (route) => route.fulfill({ json: { schemaVersion: 1, status: "ok", settings } }));
  await page.route("**/api/bus/monitor/telegrams**", (route) => route.fulfill({ status: 404, json: { error: "no monitor" } }));
  await page.route("**/api/device/service-control**", (route) => {
    const request = route.request(); const url = new URL(request.url());
    calls.push({ method: request.method(), path: url.pathname, search: url.search, body: request.method() === "POST" ? request.postDataJSON() : null });
    const send = (a: Answer) => route.fulfill({ status: a.status, json: a.body });
    if (url.pathname === "/api/device/service-control") {
      // The unchanged legacy routes: the UDP path.
      return request.method() === "GET" ? send({ status: 200, body: device })
        : send({ status: 200, body: { before: device, after: "0004", individualAddressWriteEnabled: true, written: true, backupPath: "[LOCAL MOCK BACKUP]" } });
    }
    if (mode === "legacy") return send({ status: 404, body: { error: "unsupported" } });
    const read = url.pathname.endsWith("/read");
    if (mode === "eof") return send(read ? answer("eof") : answer("write"));
    if (mode === "unavailable") return send(answer("refused-read"));
    const ok = read ? answer("read") : answer("write");
    if (mode === "missing") { const { connection: _c, ...rest } = ok.body; return send({ status: 200, body: rest }); }
    // Claims success while no tunnel opened: facts without a tunnel contradict a completed read.
    if (mode === "wrong") return send({ status: 200, body: { ...ok.body, connection: UNOPENED } });
    return send(ok);
  });
  return { state, calls };
}
async function command(page: Page, label: string) {
  await page.keyboard.press("Control+Shift+P"); await page.getByRole("combobox").fill(label); await page.getByRole("combobox").press("Enter");
}
const noOverflow = async (page: Page) =>
  expect(await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth)).toBeLessThanOrEqual(1);
async function shot(page: Page, name: string) {
  if (process.env.KNX_SERVICE_CONTROL_CAPTURE_DIR) {
    const lane = test.info().config.configFile?.includes("communication") ? "production" : "development";
    await page.screenshot({ path: join(process.env.KNX_SERVICE_CONTROL_CAPTURE_DIR, lane, `${name}.png`), fullPage: true });
  }
}
async function openPanel(page: Page, language: "en" | "de") {
  const m = language === "en" ? en : de;
  await openProject(page, language);
  await command(page, m["toolbar.busMonitor"]);
  await dismissToasts(page, language);
  await page.getByRole("button", { name: m["serviceControl.tab"], exact: true }).click();
  const panel = page.locator(".service-control-panel"); await expect(panel).toBeVisible();
  await panel.locator('input[name="address"]').fill(String(device.address));
  await panel.locator('input[name="gateway"]').fill(gateway);
  return panel;
}

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  const m = language === "en" ? en : de;
  const phrase = `I confirm individual-address write enable to ${device.address}`;
  test(`${language} explicit TCP read and write freeze the transport and show actual facts at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    const { state, calls } = await serve(page, language, "ok");
    const panel = await openPanel(page, language);
    const select = panel.locator('select[name="transport"]');
    await expect(select).toHaveValue("udp"); expect(calls).toEqual([]);
    await select.selectOption("tcp"); expect(calls).toEqual([]); // choosing contacts nothing
    await expect(panel.locator(".service-control-transport-hint")).toHaveText(m["serviceControl.transportHint"]);
    await panel.locator(".service-control-read").click();
    await expect(panel.locator(".service-control-observation")).toBeVisible();
    expect(calls.map((c) => `${c.method} ${c.path}`)).toEqual(["POST /api/device/service-control/tcp/read"]);
    expect(calls[0].body).toEqual({ address: device.address, gateway, binding: { serverIncarnation: "devices-browser", projectIncarnation: 3, revision: 1 } });
    const factsText = m["serviceControl.actualConnection"].replace("{transport}", "TCP").replace("{address}", "1.1.250").replace("{skipped}", "2");
    await expect(panel.locator(".service-control-connection")).toHaveText(factsText);
    await shot(page, `tcp-read-${language}-${width}`);
    await panel.locator("button.service-control-review").click();
    await expect(panel.locator(".service-control-frozen-transport")).toHaveText(m["serviceControl.reviewedTransport"].replace("{transport}", "TCP"));
    await panel.locator('input[name="confirmation"]').fill(phrase);
    await shot(page, `tcp-review-${language}-${width}`);
    await panel.locator(".service-control-write").click();
    await expect(panel.locator(".service-control-result")).toBeVisible();
    expect(calls.map((c) => `${c.method} ${c.path}`)).toEqual(["POST /api/device/service-control/tcp/read", "POST /api/device/service-control/tcp"]);
    expect(calls[1].body).toEqual({ address: device.address, gateway, enable: true, confirmation: phrase, binding: { serverIncarnation: "devices-browser", projectIncarnation: 3, revision: 1 } });
    await expect(panel.locator(".service-control-connection")).toHaveText(factsText);
    await expect(panel.getByRole("alert")).toHaveCount(0);
    // Each fresh operation starts from UDP again.
    await expect(select).toHaveValue("udp");
    expect(state.unexpected).toEqual([]);
    await shot(page, `tcp-result-${language}-${width}`); await noOverflow(page);
  });
  test(`${language} explicit TCP peer EOF shows the failure reason and facts at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    const { state, calls } = await serve(page, language, "eof");
    const panel = await openPanel(page, language);
    await panel.locator('select[name="transport"]').selectOption("tcp"); await panel.locator(".service-control-read").click();
    await expect(panel.getByRole("alert")).toContainText("the connection was broken down");
    await expect(panel.locator(".service-control-connection")).toContainText("1.1.250"); await expect(panel.locator(".service-control-connection")).toContainText("4");
    await expect(panel.locator(".service-control-observation")).toHaveCount(0);
    expect(calls).toHaveLength(1); expect(state.unexpected).toEqual([]);
    await shot(page, `tcp-eof-${language}-${width}`); await noOverflow(page);
  });
  test(`${language} explicit TCP unavailable facts are not zero at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    const { state } = await serve(page, language, "unavailable");
    const panel = await openPanel(page, language);
    await panel.locator('select[name="transport"]').selectOption("tcp"); await panel.locator(".service-control-read").click();
    await expect(panel.locator(".service-control-connection")).toHaveText(m["serviceControl.connectionUnavailable"]);
    await expect(panel.getByRole("alert")).toBeVisible(); expect(state.unexpected).toEqual([]);
    await shot(page, `tcp-unavailable-${language}-${width}`); await noOverflow(page);
  });
  for (const mode of ["legacy", "missing", "wrong"] as const) {
    test(`${language} explicit TCP ${mode} answer is a refusal without UDP retry at ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 900 });
      const { state, calls } = await serve(page, language, mode);
      const panel = await openPanel(page, language);
      await panel.locator('select[name="transport"]').selectOption("tcp"); await panel.locator(".service-control-read").click();
      await expect(panel.getByRole("alert")).toHaveText(m[mode === "legacy" ? "serviceControl.tcpUnsupported" : "serviceControl.tcpMetadata"]);
      await expect(panel.locator(".service-control-observation")).toHaveCount(0); await expect(panel.locator(".service-control-result")).toHaveCount(0);
      expect(calls.map((c) => `${c.method} ${c.path}`)).toEqual(["POST /api/device/service-control/tcp/read"]);
      expect(state.unexpected).toEqual([]);
      await shot(page, `tcp-${mode}-${language}-${width}`); await noOverflow(page);
    });
  }
  test(`${language} UDP stays the default path with no TCP route or facts at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    const { state, calls } = await serve(page, language, "udp");
    const panel = await openPanel(page, language);
    await panel.locator(".service-control-read").click(); await expect(panel.locator(".service-control-observation")).toBeVisible();
    await panel.locator("button.service-control-review").click(); await panel.locator('input[name="confirmation"]').fill(phrase);
    await panel.locator(".service-control-write").click(); await expect(panel.locator(".service-control-result")).toBeVisible();
    expect(calls.map((c) => `${c.method} ${c.path}`)).toEqual(["GET /api/device/service-control", "POST /api/device/service-control"]);
    expect(calls[1].body).toEqual({ address: device.address, gateway, enable: true, confirmation: phrase });
    await expect(panel.locator(".service-control-connection, .service-control-frozen-transport")).toHaveCount(0);
    expect(state.unexpected).toEqual([]);
  });
}
