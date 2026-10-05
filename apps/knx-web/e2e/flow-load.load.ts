/** U21 measurements: dense burst and long session of the real flow view; study only. */
// Run: npx playwright test -c playwright.load.config.ts (production build, see vite.study.config.ts)
// Synthetic traffic through intercepted routes; headless Chromium on this
// machine. Writes docs/design/2026-10-04-telegram-flow-u21/measurements.json.
import { expect, test, type CDPSession, type Page, type Route } from "@playwright/test";
import { cpus } from "node:os";
import { mkdirSync, writeFileSync } from "node:fs";

const OUT = new URL("../../../docs/design/2026-10-04-telegram-flow-u21/", import.meta.url);
const flags = { communication: true, read: null, write: true, transmit: null, update: null, readOnInit: null };

interface Scenario {
  name: string;
  devices: number;
  groups: number;
  ratePerSecond: number;
  seconds: number;
  motion: boolean;
}

function project(devices: number, groups: number) {
  const lap = (g: number) => Math.floor(g / devices);
  // Two extra devices carry only the marker group 0. A device shows at most
  // three current values, so a marker on a device that also takes part in
  // the traffic is pushed out by newer values at high rates and was never
  // seen (AR21 review: no value lag at the §7 load).
  const markerPair = [devices, devices + 1];
  return {
    devices: Array.from({ length: devices + 2 }, (_, i) => ({
      deviceId: i + 1, installationId: 1, name: i < devices ? `Device ${i + 1}` : `Marker ${i - devices + 1}`,
      individualAddressRaw: 0x1100 + i + 1,
    })),
    groups: Array.from({ length: groups }, (_, g) => ({
      gaRaw: 0x0800 + g, gaId: g + 1, installationId: 1, name: `Group ${g}`, dpt: "1.001",
      // `lap` shifts the targets once groups outnumber devices, so the §7
      // load reaches its distinct pairs instead of repeating them; for
      // fewer groups than devices it is 0 and the U21 scenarios are unchanged.
      members: (g === 0 ? markerPair : [g % devices, (g * 7 + 3 + lap(g)) % devices, (g * 13 + 5 + 2 * lap(g)) % devices]).map((d, k) => ({
        deviceId: d + 1, comObjectId: (g + 1) * 10 + k, direction: k === 0 ? "Send" : "Receive", active: true, flags,
      })),
    })),
  };
}

/** Traffic made at poll time from the elapsed wall time; marker rows are injected by the test. */
async function trafficServer(page: Page, scenario: Scenario) {
  const { devices, groups } = project(scenario.devices, scenario.groups);
  const state = { seq: 1, rows: [] as Record<string, unknown>[], last: Date.now(), markers: new Map<string, number>(), stopped: false };
  const telegram = (seq: number, g: number, value: string) => {
    const sender = groups[g].members[0].deviceId;
    return {
      seq, timestamp: new Date().toISOString(), source: `1.1.${sender}`, destination: `1/0/${g}`, destinationName: null,
      service: "GroupValueWrite", rawPayload: "0x01", decoded: { kind: "value", dpt: "DPST-1-1", text: value }, control: null,
      sourceRaw: devices[sender - 1].individualAddressRaw, destinationRaw: groups[g].gaRaw, observedAgeMs: 0, flowGeneration: "1",
    };
  };
  const json = (route: Route, body: unknown) => route.fulfill({ contentType: "application/json", body: JSON.stringify(body) });
  await page.route("**/api/**", (route) => {
    const url = new URL(route.request().url());
    if (url.pathname === "/api/bus/monitor/telegrams") {
      const now = Date.now();
      if (!state.stopped) {
        const due = Math.round(((now - state.last) / 1000) * scenario.ratePerSecond);
        for (let i = 0; i < due; i += 1) {
          state.rows.push(telegram(state.seq, 1 + (state.seq % (groups.length - 1)), `v${state.seq}`));
          state.seq += 1;
        }
      }
      state.last = now;
      const since = Number(url.searchParams.get("since") ?? "0");
      const rows = state.rows.filter((row) => (row.seq as number) >= since);
      state.rows = rows.slice(-5_000);
      for (const row of rows) {
        const text = (row.decoded as { text: string }).text;
        if (text.startsWith("MARK") && !state.markers.has(text)) state.markers.set(text, now);
      }
      return json(route, {
        sessionId: 1, serverIncarnation: "load", contextStatus: "current", projectOpen: true, status: "active",
        nextSince: state.seq, droppedBefore: 0, telegrams: rows, flowGeneration: "1",
      });
    }
    if (url.pathname === "/api/bus/monitor/flow-snapshot") {
      return json(route, {
        serverIncarnation: "load", sessionId: 1, generation: "1", status: "current", groupAddressStyle: "ThreeLevel", devices, groups,
        diagnostics: { duplicateIndividualAddresses: [], ambiguousGroupAddresses: [], ambiguousDevices: [], danglingLinks: [], unknownDevices: [], objectsWithoutFlags: [] },
        truncated: { devices: 0, groups: 0, members: 0, diagnostics: 0 },
      });
    }
    if (url.pathname === "/api/bus/discover") return json(route, { interfaces: [] });
    return route.fulfill({ status: 404, contentType: "application/json", body: "{}" });
  });
  return {
    state,
    mark(k: number) {
      // Group 0 carries only markers, so no newer value replaces one in its batch.
      state.rows.push(telegram(state.seq, 0, `MARK${k}`));
      state.seq += 1;
    },
  };
}

async function instrument(page: Page) {
  await page.addInitScript(() => {
    const w = window as unknown as { __frameTimes: number[]; __longTasks: number[]; __markSeen: Record<string, number> };
    w.__frameTimes = [];
    w.__longTasks = [];
    w.__markSeen = {};
    const raf = window.requestAnimationFrame.bind(window);
    window.requestAnimationFrame = (callback) => raf((time) => {
      if (w.__frameTimes.length < 200_000) w.__frameTimes.push(time);
      callback(time);
    });
    new PerformanceObserver((list) => { for (const entry of list.getEntries()) w.__longTasks.push(entry.duration); })
      .observe({ type: "longtask", buffered: true });
    new MutationObserver(() => {
      for (const badge of document.querySelectorAll(".flow-badge")) {
        const text = badge.textContent ?? "";
        const match = /MARK\d+/.exec(text);
        if (match && !(match[0] in w.__markSeen)) w.__markSeen[match[0]] = Date.now();
      }
    }).observe(document, { subtree: true, childList: true, characterData: true });
  });
}

async function metric(cdp: CDPSession, name: string): Promise<number> {
  const { metrics } = await cdp.send("Performance.getMetrics");
  return metrics.find((m) => m.name === name)?.value ?? 0;
}

const quantile = (values: number[], q: number) => {
  if (values.length === 0) return null;
  const sorted = [...values].sort((a, b) => a - b);
  return Number(sorted[Math.min(sorted.length - 1, Math.floor(q * sorted.length))].toFixed(1));
};

async function run(page: Page, scenario: Scenario) {
  await instrument(page);
  const server = await trafficServer(page, scenario);
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("Performance.enable");
  await page.goto(`/e2e/telegram-flow-fixture.html?lang=en&theme=graphite${scenario.motion ? "" : "&motion=off"}`);
  await page.getByRole("tab", { name: "Flow" }).click();
  const level = await page.evaluate(() => document.documentElement.getAttribute("data-motion-level"));
  expect(level === "off").toBe(!scenario.motion);
  await expect(page.locator("g.flow-node").first()).toBeAttached();
  await cdp.send("HeapProfiler.collectGarbage");
  const heapStart = await metric(cdp, "JSHeapUsedSize");
  const taskStart = await metric(cdp, "TaskDuration");
  const wallStart = Date.now();
  const marks = 5;
  const heapSamples: number[] = [];
  for (let k = 0; k < marks; k += 1) {
    await page.waitForTimeout((scenario.seconds * 1000) / marks);
    server.mark(k);
    await cdp.send("HeapProfiler.collectGarbage");
    heapSamples.push(Number(((await metric(cdp, "JSHeapUsedSize")) / 1048576).toFixed(1)));
  }
  await page.waitForTimeout(2_000);
  server.state.stopped = true;
  const wall = (Date.now() - wallStart) / 1000;
  const task = (await metric(cdp, "TaskDuration")) - taskStart;
  await cdp.send("HeapProfiler.collectGarbage");
  const heapEnd = await metric(cdp, "JSHeapUsedSize");
  const page_ = await page.evaluate(() => {
    const w = window as unknown as { __frameTimes: number[]; __longTasks: number[]; __markSeen: Record<string, number> };
    return {
      frameTimes: w.__frameTimes, longTasks: w.__longTasks, markSeen: w.__markSeen,
      nodes: document.querySelectorAll("g.flow-node").length, edges: document.querySelectorAll("g.flow-edge").length,
      svgElements: document.querySelector(".flow-svg")?.querySelectorAll("*").length ?? 0,
      reduced: document.querySelector(".flow-reduced")?.textContent ?? null,
    };
  });
  const deltas = page_.frameTimes.slice(1).map((t, i) => t - page_.frameTimes[i]).filter((d) => d < 1_000);
  const lags = [...server.state.markers].map(([mark, fulfilled]) => (page_.markSeen[mark] ?? NaN) - fulfilled);
  return {
    scenario,
    telegrams: server.state.seq - 1,
    wallSeconds: Number(wall.toFixed(1)),
    mainThreadBusyShare: Number((task / wall).toFixed(3)),
    heapMiB: { start: Number((heapStart / 1048576).toFixed(1)), samplesAfterGc: heapSamples, endAfterGc: Number((heapEnd / 1048576).toFixed(1)) },
    frames: { count: page_.frameTimes.length, intervalP50: quantile(deltas, 0.5), intervalP95: quantile(deltas, 0.95), intervalMax: quantile(deltas, 1) },
    longTasks: { count: page_.longTasks.length, totalMs: Number(page_.longTasks.reduce((a, b) => a + b, 0).toFixed(0)), maxMs: Number(Math.max(0, ...page_.longTasks).toFixed(0)) },
    markerLagMs: { values: lags.map((l) => (Number.isFinite(l) ? l : null)), max: Math.max(...lags.filter(Number.isFinite)) },
    dom: { nodes: page_.nodes, edges: page_.edges, svgElements: page_.svgElements },
    reducedRenderingNote: page_.reduced,
  };
}

const SCENARIOS: Scenario[] = [
  { name: "dense-burst-motion", devices: 300, groups: 120, ratePerSecond: 200, seconds: 15, motion: true },
  { name: "dense-burst-motion-off", devices: 300, groups: 120, ratePerSecond: 200, seconds: 15, motion: false },
  { name: "long-session-motion", devices: 60, groups: 40, ratePerSecond: 10, seconds: 180, motion: true },
  // §7 starting load (AR21 finding 1): 500 devices, ~2,500 directed pairs, 1,000 telegrams/s.
  { name: "target-load-motion", devices: 500, groups: 1250, ratePerSecond: 1000, seconds: 15, motion: true },
  { name: "target-load-motion-off", devices: 500, groups: 1250, ratePerSecond: 1000, seconds: 15, motion: false },
  // The same load for a minute: the first ~12 s are the initial layout
  // settling; the rest shows the steady state local reheat is for.
  { name: "target-load-motion-60s", devices: 500, groups: 1250, ratePerSecond: 1000, seconds: 60, motion: true },
];

const results: unknown[] = [];

for (const scenario of SCENARIOS) {
  test(scenario.name, async ({ page, browser }) => {
    test.setTimeout(scenario.seconds * 1000 + 120_000);
    const result = await run(page, scenario);
    results.push({ ...result, environment: { chromium: browser.version(), cpu: cpus()[0]?.model, cores: cpus().length, headless: true, build: "production (vite build)" } });
    console.log(JSON.stringify(result));
  });
}

test.afterAll(() => {
  mkdirSync(OUT, { recursive: true });
  // FLOW_LOAD_FILE keeps a partial run (e.g. `--grep target-load`) from
  // overwriting the full measurement file.
  writeFileSync(new URL(process.env.FLOW_LOAD_FILE ?? "measurements.json", OUT), `${JSON.stringify({ measuredAt: new Date().toISOString(), results }, null, 2)}\n`);
});
