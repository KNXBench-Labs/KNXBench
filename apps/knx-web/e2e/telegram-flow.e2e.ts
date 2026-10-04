/** U20: the real monitor panel draws intercepted synthetic traffic as a flow, read-only. */
import { expect, test, type Page, type Route } from "@playwright/test";

const SWITCH = 0x1101;
const LIGHT = 0x0801;
const flags = { communication: true, read: null, write: true, transmit: null, update: null, readOnInit: null };

function snapshot(sessionId: number, generation: string, members: number[]) {
  return {
    serverIncarnation: "fixture", sessionId, generation, status: "current", groupAddressStyle: "ThreeLevel",
    devices: [
      { deviceId: 1, installationId: 1, name: "Switch", individualAddressRaw: SWITCH },
      { deviceId: 2, installationId: 1, name: "Dimmer", individualAddressRaw: 0x1102 },
      { deviceId: 3, installationId: 1, name: "Blind", individualAddressRaw: 0x1103 },
    ],
    groups: [{
      gaRaw: LIGHT, gaId: 10, installationId: 1, name: "Light", dpt: "1.001",
      members: [1, ...members].map((deviceId) => ({
        deviceId, comObjectId: deviceId * 100, direction: deviceId === 1 ? "Send" : "Receive", active: true, flags,
      })),
    }],
    diagnostics: {
      duplicateIndividualAddresses: [], ambiguousGroupAddresses: [], ambiguousDevices: [],
      danglingLinks: [], unknownDevices: [], objectsWithoutFlags: [],
    },
    truncated: { devices: 0, groups: 0, members: 0, diagnostics: 0 },
  };
}

function telegram(seq: number, generation: string, value: string, service = "GroupValueWrite") {
  return {
    seq, timestamp: "2026-10-04T12:00:00Z", source: "1.1.1", destination: "1/0/1", destinationName: "Light",
    service, rawPayload: "0x01", decoded: service === "GroupValueRead" ? null : { kind: "value", dpt: "DPST-1-1", text: value },
    control: null, sourceRaw: SWITCH, destinationRaw: LIGHT, observedAgeMs: 0, flowGeneration: generation,
  };
}

/** A tiny stateful fake server: rows are added by the test; every other path is refused and recorded. */
async function fakeServer(page: Page) {
  const state = {
    sessionId: 1,
    generation: "1",
    rows: [] as ReturnType<typeof telegram>[],
    snapshots: [] as string[],
    unexpected: [] as string[],
    members: { "1": [2], "2": [3] } as Record<string, number[]>,
  };
  const json = (route: Route, body: unknown, status = 200) =>
    route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
  await page.route("**/api/**", (route) => {
    const url = new URL(route.request().url());
    const method = route.request().method();
    if (method === "GET" && url.pathname === "/api/bus/monitor/telegrams") {
      const since = Number(url.searchParams.get("since") ?? "0");
      const rows = url.searchParams.get("contextOnly") ? [] : state.rows.filter((row) => row.seq >= since);
      const next = Math.max(since, ...state.rows.map((row) => row.seq + 1));
      return json(route, {
        sessionId: state.sessionId, serverIncarnation: "fixture", contextStatus: "current", projectOpen: true,
        status: "active", nextSince: next, droppedBefore: 0, telegrams: rows, flowGeneration: state.generation,
      });
    }
    if (method === "GET" && url.pathname === "/api/bus/monitor/flow-snapshot") {
      const generation = url.searchParams.get("generation")!;
      state.snapshots.push(`${url.searchParams.get("sessionId")}:${generation}`);
      return json(route, snapshot(Number(url.searchParams.get("sessionId")), generation, state.members[generation] ?? []));
    }
    if (method === "POST" && url.pathname === "/api/bus/discover") return json(route, { interfaces: [] });
    state.unexpected.push(`${method} ${url.pathname}`);
    return json(route, { error: "local fixture only" }, 404);
  });
  return state;
}

const node = (page: Page, name: string) => page.locator(`g.flow-node[aria-label^="${name}."]`);

async function openFlow(page: Page, language = "en", theme = "porcelain") {
  await page.goto(`/e2e/telegram-flow-fixture.html?lang=${language}&theme=${theme}`);
  await page.getByRole("tab", { name: language === "de" ? "Fluss" : "Flow" }).click();
}

test("draws senders and configured members, opens the Inspector from the keyboard and writes nothing", async ({ page }) => {
  const server = await fakeServer(page);
  server.rows.push(telegram(1, "1", "On"));
  await openFlow(page);
  await expect(node(page, "Switch")).toBeVisible();
  await expect(node(page, "Dimmer").locator(".flow-badge-inferred")).toHaveText("◇ 1/0/1 On");
  await expect(node(page, "Blind")).toHaveCount(0);
  await node(page, "Dimmer").focus();
  await page.keyboard.press("Enter");
  const inspector = page.locator(".flow-inspector");
  await expect(inspector.locator("h3")).toHaveText("Dimmer");
  await expect(inspector).toContainText("configured member (inferred from the project)");
  await expect(inspector).toContainText("From Switch · configured in the project");
  await page.keyboard.press("ArrowRight");
  await expect(node(page, "Switch")).toBeFocused();
  expect(server.unexpected).toEqual([]);
  expect(server.snapshots).toEqual(["1:1"]);
});

test("expires a value seven seconds after it was observed and keeps the map", async ({ page }) => {
  await page.clock.install();
  const server = await fakeServer(page);
  server.rows.push(telegram(1, "1", "On"));
  await openFlow(page);
  const badge = node(page, "Dimmer").locator(".flow-badge");
  await expect(badge).toHaveText("◇ 1/0/1 On");
  await page.clock.runFor(6_000);
  await expect(badge).toHaveCount(1);
  await page.clock.runFor(1_500);
  await expect(badge).toHaveCount(0);
  await expect(node(page, "Dimmer")).toBeVisible();
});

test("a read neither sets nor renews a value", async ({ page }) => {
  await page.clock.install();
  const server = await fakeServer(page);
  server.rows.push(telegram(1, "1", "On"));
  await openFlow(page);
  const badge = node(page, "Dimmer").locator(".flow-badge");
  await expect(badge).toHaveText("◇ 1/0/1 On");
  await page.clock.runFor(5_000);
  server.rows.push(telegram(2, "1", "", "GroupValueRead"));
  await page.clock.runFor(1_000);
  await page.clock.runFor(1_500);
  await expect(badge).toHaveCount(0);
});

test("resolves each telegram with its own generation after a project change", async ({ page }) => {
  const server = await fakeServer(page);
  server.rows.push(telegram(1, "1", "On"));
  await openFlow(page);
  await expect(node(page, "Dimmer")).toBeVisible();
  server.generation = "2";
  server.rows.push(telegram(2, "2", "Off"));
  await expect(node(page, "Blind").locator(".flow-badge-inferred")).toHaveText("◇ 1/0/1 Off");
  await expect(node(page, "Dimmer")).toBeVisible();
  await node(page, "Dimmer").click();
  await expect(page.locator(".flow-inspector")).toContainText("1/0/1: observed 1× (project evidence from interpretation 1)");
  expect(server.snapshots).toEqual(["1:1", "1:2"]);
});

test("starts a new map when another session answers", async ({ page }) => {
  const server = await fakeServer(page);
  server.rows.push(telegram(1, "1", "On"));
  await openFlow(page);
  await expect(node(page, "Switch")).toBeVisible();
  server.sessionId = 2;
  server.rows = [];
  await expect(page.getByText("No group telegrams observed in this session yet.")).toBeVisible();
  await expect(node(page, "Switch")).toHaveCount(0);
});

test("fits a narrow window in German", async ({ page }) => {
  await page.setViewportSize({ width: 360, height: 800 });
  const server = await fakeServer(page);
  server.rows.push(telegram(1, "1", "Ein"));
  await openFlow(page, "de");
  await expect(node(page, "Dimmer").locator(".flow-badge-inferred")).toHaveText("◇ 1/0/1 Ein");
  await expect(page.getByRole("button", { name: "Vergrößern" })).toBeVisible();
  const layout = await page.evaluate(() => ({ viewport: innerWidth, document: document.documentElement.scrollWidth }));
  expect(layout.document, JSON.stringify(layout)).toBeLessThanOrEqual(layout.viewport);
});

test("draws lines and text in the theme's colours, and follows a theme change", async ({ page }) => {
  const server = await fakeServer(page);
  server.rows.push(telegram(1, "1", "On"));
  await openFlow(page, "en", "porcelain");
  const edge = page.locator(".flow-edge path").first();
  await expect(edge).toBeVisible();
  const read = () => page.evaluate(() => {
    const path = document.querySelector(".flow-edge path")!;
    const muted = getComputedStyle(document.documentElement).getPropertyValue("--knx-muted").trim();
    const probe = document.createElement("span");
    probe.style.color = muted;
    document.body.append(probe);
    const expected = getComputedStyle(probe).color;
    probe.remove();
    return { stroke: getComputedStyle(path).stroke, expected, halo: getComputedStyle(document.querySelector(".flow-badge")!).paintOrder };
  });
  const light = await read();
  expect(light.stroke).toBe(light.expected);
  expect(light.halo).toContain("stroke");
  await page.evaluate(() => { document.documentElement.dataset.theme = "graphite"; });
  const dark = await read();
  expect(dark.stroke).toBe(dark.expected);
  expect(dark.stroke).not.toBe(light.stroke);
});
