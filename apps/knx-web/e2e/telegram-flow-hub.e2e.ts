/** AR21 finding 2: a busy hub settles with no node, name or value block over another. */
import { expect, test, type Page, type Route } from "@playwright/test";

const flags = { communication: true, read: null, write: true, transmit: null, update: null, readOnInit: null };
const GROUPS = 12;
const HUB = 0x1101;
// The hub sends on every group; each group has two receivers of its own.
const devices = Array.from({ length: 1 + GROUPS * 2 }, (_, i) => ({
  deviceId: i + 1, installationId: 1, name: i === 0 ? "Hub" : `Device ${i + 1}`, individualAddressRaw: HUB + i,
}));
const groups = Array.from({ length: GROUPS }, (_, g) => ({
  gaRaw: 0x0800 + g, gaId: g + 1, installationId: 1, name: `Group ${g}`, dpt: "1.001",
  members: [0, 1 + 2 * g, 2 + 2 * g].map((d, k) => ({
    deviceId: d + 1, comObjectId: (g + 1) * 10 + k, direction: k === 0 ? "Send" : "Receive", active: true, flags,
  })),
}));
const diagnostics = {
  duplicateIndividualAddresses: [], ambiguousGroupAddresses: [], ambiguousDevices: [],
  danglingLinks: [], unknownDevices: [], objectsWithoutFlags: [],
};

/** Six hub telegrams per poll, round-robin over the groups; everything else is refused. */
async function hubServer(page: Page) {
  let seq = 1;
  const json = (route: Route, body: unknown, status = 200) =>
    route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
  await page.route("**/api/**", (route) => {
    const url = new URL(route.request().url());
    if (url.pathname === "/api/bus/monitor/telegrams") {
      const telegrams = Array.from({ length: 6 }, () => {
        const g = seq % GROUPS;
        const row = {
          seq, timestamp: new Date().toISOString(), source: "1.1.1", destination: `1/0/${g}`, destinationName: null,
          service: "GroupValueWrite", rawPayload: "0x01", decoded: { kind: "value", dpt: "DPST-1-1", text: seq % 2 ? "On" : "Off" },
          control: null, sourceRaw: HUB, destinationRaw: 0x0800 + g, observedAgeMs: 0, flowGeneration: "1",
        };
        seq += 1;
        return row;
      });
      return json(route, {
        sessionId: 1, serverIncarnation: "hub", contextStatus: "current", projectOpen: true, status: "active",
        nextSince: seq, droppedBefore: 0, telegrams, flowGeneration: "1",
      });
    }
    if (url.pathname === "/api/bus/monitor/flow-snapshot") {
      return json(route, {
        serverIncarnation: "hub", sessionId: 1, generation: "1", status: "current", groupAddressStyle: "ThreeLevel",
        devices, groups, diagnostics, truncated: { devices: 0, groups: 0, members: 0, diagnostics: 0 },
      });
    }
    return json(route, {}, 404);
  });
}

interface Box { left: number; top: number; right: number; bottom: number }

/** Every node's circle, and the box around its name and value lines, in page pixels. */
function nodeBoxes(page: Page) {
  return page.locator("g.flow-node").evaluateAll((nodes) =>
    nodes.map((node) => {
      const box = (elements: Element[]) => {
        const rects = elements.map((element) => element.getBoundingClientRect()).filter((rect) => rect.width > 0);
        return {
          left: Math.min(...rects.map((r) => r.left)), top: Math.min(...rects.map((r) => r.top)),
          right: Math.max(...rects.map((r) => r.right)), bottom: Math.max(...rects.map((r) => r.bottom)),
        };
      };
      return {
        name: node.querySelector(".flow-node-label")?.textContent ?? "",
        circle: box([...node.querySelectorAll("circle")]),
        text: box([...node.querySelectorAll(".flow-node-label, .flow-badge")]),
      };
    }));
}

const overlap = (p: Box, q: Box) =>
  Math.min(Math.min(p.right, q.right) - Math.max(p.left, q.left), Math.min(p.bottom, q.bottom) - Math.max(p.top, q.top));

test("a busy hub settles with every circle, name and value block clear of its neighbours", async ({ page }) => {
  await hubServer(page);
  await page.setViewportSize({ width: 1400, height: 1000 });
  await page.goto("/e2e/telegram-flow-fixture.html?lang=en&theme=graphite");
  await page.getByRole("tab", { name: "Flow" }).click();
  await expect(page.locator("g.flow-node")).toHaveCount(devices.length);
  // The hub's value block is full (three values and an overflow line) …
  await expect(page.locator("g.flow-node", { hasText: "Hub" }).locator(".flow-badge")).toHaveCount(4, { timeout: 20_000 });
  // … and the layout has come to rest: no node moved for two seconds.
  const transforms = () => page.locator("g.flow-node").evaluateAll((nodes) => nodes.map((n) => n.getAttribute("transform")).join("|"));
  await expect.poll(async () => {
    const before = await transforms();
    await page.waitForTimeout(2_000);
    return before === (await transforms());
  }, { timeout: 60_000, intervals: [0] }).toBe(true);
  await page.getByRole("button", { name: "Freeze" }).click();

  const boxes = await nodeBoxes(page);
  const svg = (await page.locator(".flow-svg").first().boundingBox())!;
  expect(boxes).toHaveLength(devices.length);
  for (const a of boxes) {
    // Circle and name stay inside the drawing area.
    expect(a.circle.top, a.name).toBeGreaterThanOrEqual(svg.y - 1);
    expect(a.text.top, a.name).toBeGreaterThanOrEqual(svg.y - 1);
    expect(a.circle.left, a.name).toBeGreaterThanOrEqual(svg.x - 1);
    expect(a.circle.right, a.name).toBeLessThanOrEqual(svg.x + svg.width + 1);
    for (const b of boxes) {
      if (a === b) continue;
      expect(overlap(a.circle, b.circle), `${a.name} circle / ${b.name} circle`).toBeLessThan(1);
      expect(overlap(a.circle, b.text), `${a.name} circle / ${b.name} text`).toBeLessThan(1);
      expect(overlap(a.text, b.text), `${a.name} text / ${b.name} text`).toBeLessThan(1);
    }
  }
  await page.locator(".flow-svg").first().screenshot({ path: test.info().outputPath("busy-hub.png") });
});
