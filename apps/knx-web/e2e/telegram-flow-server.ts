/** U20/U21 e2e helpers: a stateful fake monitor server and flow-view locators. */
import type { Page, Route } from "@playwright/test";

const SWITCH = 0x1101;
const LIGHT = 0x0801;
const flags = { communication: true, read: null, write: true, transmit: null, update: null, readOnInit: null };

export function snapshot(sessionId: number, generation: string, members: number[]) {
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

export function telegram(seq: number, generation: string, value: string, service = "GroupValueWrite") {
  return {
    seq, timestamp: "2026-10-04T12:00:00Z", source: "1.1.1", destination: "1/0/1", destinationName: "Light",
    service, rawPayload: "0x01", decoded: service === "GroupValueRead" ? null : { kind: "value", dpt: "DPST-1-1", text: value },
    control: null, sourceRaw: SWITCH, destinationRaw: LIGHT, observedAgeMs: 0, flowGeneration: generation,
  };
}

/** A tiny stateful fake server: rows are added by the test; every other path is refused and recorded. */
export async function fakeServer(page: Page) {
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

export const node = (page: Page, name: string) => page.locator(`g.flow-node[aria-label^="${name}."]`);

export async function openFlow(page: Page, language = "en", theme = "porcelain") {
  await page.goto(`/e2e/telegram-flow-fixture.html?lang=${language}&theme=${theme}`);
  await page.getByRole("tab", { name: language === "de" ? "Fluss" : "Flow" }).click();
}
