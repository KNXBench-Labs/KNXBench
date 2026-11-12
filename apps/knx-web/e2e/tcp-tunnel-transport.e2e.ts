/** Exercise explicit TCP monitoring through the full app with strict offline HTTP fixtures. */
import { expect, test, type Page } from "@playwright/test";
import { isDeepStrictEqual } from "node:util";
import type { BusMonitorStartResponse, BusMonitorTelegramsResponse } from "../src/api";
import type { ActivitySnapshot } from "../src/liveActivity";
import type { IpTunnelTransport } from "../src/serialTransport";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { dismissToasts, intercept, openProject } from "./communication-fixture";

const GATEWAY = "192.0.2.10:3671";
const INCARNATION = "tcp-fixture";

interface Session { id: number; transport: IpTunnelTransport }
async function serve(page: Page, language: "en" | "de", options: { preference: IpTunnelTransport; session?: Session; tcpRoute?: "present" | "missing" }) {
  const state = await intercept(page, language);
  const settings: Record<string, unknown> = {
    uiLanguage: language, theme: "porcelain", preferredGateway: GATEWAY, preferredIpTunnelTransport: options.preference, releaseChecksEnabled: false,
  };
  const tcpStarts: unknown[] = [], udpStarts: unknown[] = [], stops: string[] = [], reads: unknown[] = [], writes: unknown[] = [], requests: string[] = [];
  const control = { session: options.session ?? null as Session | null, polledTransport: "as-session" as "as-session" | "missing" | "udp" | "malformed", skipped: 0, omitCounter: false };
  page.on("request", (request) => { const url = new URL(request.url()); if (url.pathname.startsWith("/api/")) requests.push(`${request.method()} ${url.pathname}`); });
  await page.route("https://api.github.com/**", (route) => { state.unexpected.push("external release check"); return route.abort(); });
  await page.route("**/api/legacy-password", (route) => route.fulfill({ json: { available: true, remembered: false, problem: null } }));
  await page.route("**/api/settings", async (route) => {
    if (route.request().method() === "PUT") {
      const patch: unknown = route.request().postDataJSON();
      if (patch && typeof patch === "object" && "settings" in patch && patch.settings && typeof patch.settings === "object") Object.assign(settings, patch.settings);
    } else if (route.request().method() !== "GET") state.unexpected.push("settings method");
    return route.fulfill({ json: { schemaVersion: 1, status: "ok", settings } });
  });
  await page.route("**/api/bus/activity", (route) => {
    const sessions: ActivitySnapshot["sessions"] = control.session
      ? [{ kind: "busMonitor", id: control.session.id, gateway: GATEWAY, state: "active", transport: control.session.transport }] : [];
    if (control.polledTransport === "missing" && sessions[0]?.kind === "busMonitor") delete sessions[0].transport;
    if (control.polledTransport === "udp" && sessions[0]?.kind === "busMonitor") sessions[0].transport = "udp";
    const snapshot: ActivitySnapshot = { serverIncarnation: INCARNATION, coverage: "partial", historyState: "disabled", sessions, oneShot: [], oneShotDropped: 0, busyLocks: [], untracked: [] };
    return route.fulfill({ json: snapshot });
  });
  await page.route("**/api/bus/monitor/start-tcp", (route) => {
    if (options.tcpRoute === "missing") return route.fulfill({ status: 404, json: { error: "not found" } });
    const payload: unknown = route.request().postDataJSON(); tcpStarts.push(payload);
    if (route.request().method() !== "POST" || !isDeepStrictEqual(payload, { gateway: GATEWAY })) {
      state.unexpected.push("invalid tcp start"); return route.fulfill({ status: 400, json: { error: "invalid" } });
    }
    control.session = { id: 21, transport: "tcp" };
    const response: BusMonitorStartResponse = { sessionId: 21, serverIncarnation: INCARNATION, assignedAddress: "1.1.250", transport: "tcp", skippedFrames: control.skipped };
    return route.fulfill({ json: response });
  });
  await page.route("**/api/bus/monitor/start", (route) => { udpStarts.push(route.request().postDataJSON()); return route.fulfill({ status: 502, json: { error: "synthetic UDP unavailable" } }); });
  await page.route("**/api/bus/monitor/telegrams**", (route) => {
    if (!control.session) return route.fulfill({ status: 404, json: { error: "no session" } });
    const base = { sessionId: control.session.id, serverIncarnation: INCARNATION, status: "active", contextStatus: "current", projectOpen: true, nextSince: 0, droppedBefore: 0, telegrams: [] } as const;
    const body: BusMonitorTelegramsResponse | Record<string, unknown> =
      control.polledTransport === "missing" ? base
      : control.polledTransport === "udp" ? { ...base, transport: "udp" }
      : control.polledTransport === "malformed" ? { ...base, transport: "tls" }
      : control.session.transport === "tcp"
        ? { ...base, transport: "tcp", ...(control.omitCounter ? {} : { skippedFrames: control.skipped }) }
        : { ...base, transport: control.session.transport };
    return route.fulfill({ json: body });
  });
  await page.route("**/api/bus/monitor/stop-bound**", (route) => {
    stops.push(new URL(route.request().url()).search);
    const id = control.session?.id ?? 0; control.session = null;
    return route.fulfill({ json: { sessionId: id, serverIncarnation: INCARNATION, telegramCount: 0, droppedCount: 0, skippedFrames: control.skipped } });
  });
  await page.route("**/api/bus/read", (route) => {
    const payload: unknown = route.request().postDataJSON(); reads.push(payload);
    if (!isDeepStrictEqual(payload, { sessionId: 21, serverIncarnation: INCARNATION, destination: "1/1/1" })) {
      state.unexpected.push("invalid bound group read"); return route.fulfill({ status: 400, json: { error: "invalid" } });
    }
    return route.fulfill({ json: { service: "GroupValueRead", transport: "tcp" } });
  });
  await page.route("**/api/bus/write-bound", (route) => {
    const payload: unknown = route.request().postDataJSON(); writes.push(payload);
    if (!isDeepStrictEqual(payload, { sessionId: 21, serverIncarnation: INCARNATION, destination: "1/1/1", dpt: "DPST-1-1", inputFormat: null, value: "on" })) {
      state.unexpected.push("invalid bound group write"); return route.fulfill({ status: 400, json: { error: "invalid" } });
    }
    return route.fulfill({ json: { service: "GroupValueWrite", encodedPayload: "0x01 (6-bit)", decodedEcho: { kind: "value", dpt: "DPST-1-1", text: "on" }, transport: "tcp" } });
  });
  await page.route("**/api/bus/write", (route) => { state.unexpected.push("legacy unbound write over TCP"); return route.fulfill({ status: 409, json: { error: "binding required" } }); });
  return { state, settings, tcpStarts, udpStarts, stops, reads, writes, requests, control };
}
async function command(page: Page, label: string) {
  await page.keyboard.press("Control+Shift+P"); await page.getByRole("combobox").fill(label); await page.getByRole("combobox").press("Enter");
}
const noOverflow = async (page: Page) =>
  expect(await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth)).toBeLessThanOrEqual(1);

for (const language of ["en", "de"] as const) for (const width of [1440, 400]) {
  const m = language === "en" ? en : de;
  const setup = async (page: Page, options: Parameters<typeof serve>[2]) => {
    await page.setViewportSize({ width, height: 1000 }); await page.emulateMedia({ reducedMotion: "reduce" });
    const fixture = await serve(page, language, options); await openProject(page, language); await dismissToasts(page, language);
    return fixture;
  };

  test(`${language} ${width}: saved TCP preference starts only on the TCP route and the observed TCP drives footer, monitor and bound group traffic`, { tag: "@exclusive" }, async ({ page }) => {
    const fixture = await setup(page, { preference: "tcp" });
    await expect(page.locator(".bus-connection-state")).toContainText(m["busConnection.state.disconnected"]);
    await page.waitForTimeout(500);
    expect(fixture.tcpStarts).toEqual([]); expect(fixture.udpStarts).toEqual([]); // The preference alone never connects.
    await page.locator(".bus-connection-trigger").click();
    const panel = page.locator(".bus-connection-panel");
    const tunnel = panel.getByRole("combobox", { name: m["busTransport.ipTunnel.label"], exact: true });
    await expect(tunnel).toHaveValue("tcp");
    await expect(panel).toContainText(m["busTransport.ipTunnel.note"]);
    await expect(panel.locator(".bus-transport-experimental")).toHaveText(m["transport.experimentalNote"]);
    await tunnel.selectOption("udp"); await expect(panel.locator(".bus-transport-experimental")).toHaveCount(0);
    await tunnel.selectOption("tcp");
    await expect.poll(() => fixture.settings.preferredIpTunnelTransport).toBe("tcp");
    expect(fixture.tcpStarts).toEqual([]); expect(fixture.udpStarts).toEqual([]);
    await page.locator(".bus-connection-action").click();
    await expect.poll(() => fixture.tcpStarts).toEqual([{ gateway: GATEWAY }]);
    await expect(page.locator(".bus-connection-state")).toContainText(`${GATEWAY} · TCP`);
    await expect(page.locator(".bus-connection-state .transport-experimental-badge")).toHaveText(m["transport.experimental"]);
    expect(fixture.udpStarts).toEqual([]);
    await page.keyboard.press("Escape");
    await command(page, m["toolbar.busMonitor"]);
    const monitor = page.locator(".bus-monitor-panel");
    await expect(monitor).toBeVisible(); await dismissToasts(page, language);
    await expect(monitor.locator(".bus-monitor-session")).toContainText(m["busTransport.ipTunnel.observed"].replace("{transport}", "TCP"));
    await expect(monitor.getByRole("combobox", { name: m["busTransport.ipTunnel.label"], exact: true })).toBeDisabled();
    await monitor.locator(".bus-compose-destination").fill("1/1/1");
    await monitor.getByRole("button", { name: m["busTransport.read"], exact: true }).click();
    await expect.poll(() => fixture.reads).toEqual([{ sessionId: 21, serverIncarnation: INCARNATION, destination: "1/1/1" }]);
    await monitor.locator(".bus-compose-dpt").selectOption("DPST-1-1");
    await monitor.locator(".bus-compose-value").fill("on");
    await monitor.getByRole("button", { name: m["busCompose.send"], exact: true }).click();
    await expect.poll(() => fixture.writes).toEqual([{ sessionId: 21, serverIncarnation: INCARNATION, destination: "1/1/1", dpt: "DPST-1-1", inputFormat: null, value: "on" }]);
    await expect(monitor).toContainText(m["busTransport.ipTunnel.sendNote"]);
    await monitor.locator(".bus-transport-fields").scrollIntoViewIfNeeded();
    await monitor.locator(".bus-transport-fields").screenshot({ path: test.info().outputPath("tcp-monitor-fields.png") });
    await noOverflow(page);
    // Skipped TCP frames: absent while zero, then the exact cumulative count, apart from dropped rows.
    await expect(monitor.locator(".bus-monitor-skipped-frames")).toHaveCount(0);
    fixture.control.skipped = 3;
    await expect(monitor.locator(".bus-monitor-skipped-frames")).toHaveText(m["busMonitor.skippedFrames.other"].replace("{count}", "3"), { timeout: 8000 });
    await expect(monitor.locator(".bus-monitor-gap-notice")).toHaveCount(0);
    await noOverflow(page);
    await monitor.getByRole("button", { name: m["busMonitor.disconnect"], exact: true }).click();
    await expect(monitor.locator(".bus-monitor-stop-skipped")).toHaveText(m["busMonitor.skippedFramesAtStop.other"].replace("{count}", "3"));
    await expect.poll(() => fixture.stops).toEqual([`?sessionId=21&serverIncarnation=${INCARNATION}`]);
    expect(fixture.requests.filter((request) => request === "POST /api/bus/write")).toEqual([]);
    expect(fixture.requests.filter((request) => request === "POST /api/bus/monitor/start")).toEqual([]);
    expect(fixture.udpStarts).toEqual([]); expect(fixture.state.unexpected).toEqual([]);
  });

  test(`${language} ${width}: an older server without the TCP route fails once with the unsupported text and no UDP request`, { tag: "@exclusive" }, async ({ page }) => {
    const fixture = await setup(page, { preference: "tcp", tcpRoute: "missing" });
    await page.locator(".bus-connection-action").click();
    await expect(page.locator(".bus-connection-error")).toHaveText(m["busTransport.ipTunnel.unsupported"]);
    await page.waitForTimeout(500);
    expect(fixture.requests.filter((request) => request === "POST /api/bus/monitor/start-tcp")).toHaveLength(1);
    expect(fixture.requests.filter((request) => request === "POST /api/bus/monitor/start")).toEqual([]);
    expect(fixture.udpStarts).toEqual([]);
    await expect(page.locator(".bus-connection-state")).toContainText(m["busConnection.state.disconnected"]);
    await expect(page.locator(".bus-connection-action")).toBeEnabled();
    await noOverflow(page);
  });

  test(`${language} ${width}: a UDP session is shown as UDP even while the saved preference says TCP`, async ({ page }) => {
    const fixture = await setup(page, { preference: "tcp", session: { id: 22, transport: "udp" } });
    await expect(page.locator(".bus-connection-state")).toContainText(GATEWAY);
    await expect(page.locator(".bus-connection-state")).not.toContainText("TCP");
    await command(page, m["toolbar.busMonitor"]);
    const monitor = page.locator(".bus-monitor-panel");
    await expect(monitor.locator(".bus-monitor-session")).toContainText(m["busTransport.ipTunnel.observed"].replace("{transport}", "UDP"));
    await expect(monitor.locator(".bus-monitor-session")).not.toContainText("TCP");
    expect(fixture.tcpStarts).toEqual([]); expect(fixture.udpStarts).toEqual([]); expect(fixture.state.unexpected).toEqual([]);
  });

  test(`${language} ${width}: a poll that drops or alters the TCP metadata locks group actions instead of reading as UDP`, async ({ page }) => {
    const fixture = await setup(page, { preference: "udp", session: { id: 21, transport: "tcp" } });
    await expect(page.locator(".bus-connection-state")).toContainText("· TCP");
    await command(page, m["toolbar.busMonitor"]);
    const monitor = page.locator(".bus-monitor-panel");
    await dismissToasts(page, language);
    await expect(monitor.locator(".bus-monitor-session")).toContainText(m["busTransport.ipTunnel.observed"].replace("{transport}", "TCP"));
    await expect(monitor.locator(".bus-compose-destination")).toBeEnabled();
    for (const lacking of ["missing", "udp"] as const) {
      fixture.control.polledTransport = lacking;
      await expect(monitor).toContainText(m["busTransport.ipTunnel.unverified"], { timeout: 8000 });
      await expect(monitor.locator(".bus-compose-destination")).toBeDisabled();
      await expect(monitor.locator(".bus-monitor-session")).not.toContainText("UDP");
      // Each fault starts from a healthy session: restore it and wait until it is usable again.
      fixture.control.polledTransport = "as-session";
      await expect(monitor.locator(".bus-compose-destination")).toBeEnabled({ timeout: 8000 });
      await expect(monitor).not.toContainText(m["busTransport.ipTunnel.unverified"]);
    }
    // A TCP poll without its counter is malformed: refused, never shown as zero skipped frames.
    fixture.control.omitCounter = true;
    await expect(monitor).toContainText("Malformed skipped-frame count", { timeout: 8000 });
    await expect(monitor.locator(".bus-compose-destination")).toBeDisabled();
    await expect(monitor.locator(".bus-monitor-skipped-frames")).toHaveCount(0);
    fixture.control.omitCounter = false;
    await expect(monitor.locator(".bus-compose-destination")).toBeEnabled({ timeout: 8000 });
    // Malformed metadata is refused by the API layer, never displayed as a transport.
    fixture.control.polledTransport = "malformed";
    await expect(monitor).toContainText("Malformed session transport", { timeout: 8000 });
    await expect(monitor.locator(".bus-monitor-session")).not.toContainText("UDP");
    fixture.control.polledTransport = "as-session";
    await expect(monitor.locator(".bus-compose-destination")).toBeEnabled({ timeout: 8000 });
    await expect(monitor).not.toContainText(m["busTransport.ipTunnel.unverified"]);
    await noOverflow(page);
    expect(fixture.writes).toEqual([]); expect(fixture.udpStarts).toEqual([]); expect(fixture.state.unexpected).toEqual([]);
  });
}
