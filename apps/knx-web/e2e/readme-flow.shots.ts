/** Records the README's telegram-flow clip: the real monitor panel, synthetic traffic. */

// The clip shows the productive Flow tab of BusMonitorPanel (the same
// fixture page the U20/U21 browser checks use) under the bundled green-CRT
// theme. Playwright answers every API call: devices and group addresses come
// from the fictional manual sample house (tools/manual_sample_project.py) and
// telegrams follow the timetable below. No server, project, bus or hardware.
//
// Run: npx playwright test -c playwright.readme.config.ts
// PNG frames land in target/playwright/readme/; README_FLOW_GIF=<path>
// additionally assembles them into the README GIF with ffmpeg.
import { test, type Route } from "@playwright/test";
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";

const ia = (area: number, line: number, device: number) => (area << 12) | (line << 8) | device;
const ga = (main: number, middle: number, sub: number) => (main << 11) | (middle << 8) | sub;
const fmtIa = (raw: number) => `${raw >> 12}.${(raw >> 8) & 0xf}.${raw & 0xff}`;
const fmtGa = (raw: number) => `${raw >> 11}/${(raw >> 8) & 0x7}/${raw & 0xff}`;

const DEVICES = [
  [1, "Switch actuator ground floor", ia(1, 1, 1)],
  [2, "Blind actuator", ia(1, 1, 2)],
  [3, "Push button living room", ia(1, 1, 10)],
  [4, "Push button kitchen", ia(1, 1, 11)],
  [5, "Thermostat living room", ia(1, 1, 20)],
  [6, "Switch actuator first floor", ia(1, 2, 1)],
  [7, "Push button bedroom", ia(1, 2, 10)],
  [8, "Push button hall", ia(1, 2, 11)],
] as const;

// [group address, name, DPT, sender device id, receiver device ids]
const GROUPS = [
  [ga(0, 0, 1), "Living room ceiling light", "1.001", 3, [1]],
  [ga(0, 0, 2), "Kitchen light", "1.001", 4, [1]],
  [ga(0, 0, 3), "Hall light", "1.001", 8, [1]],
  [ga(0, 0, 4), "Bedroom light", "1.001", 7, [6]],
  [ga(0, 1, 1), "Living room ceiling light status", "1.001", 1, []],
  [ga(0, 1, 2), "Kitchen light status", "1.001", 1, []],
  [ga(0, 1, 4), "Bedroom light status", "1.001", 6, []],
  [ga(1, 0, 1), "Living room blind up/down", "1.008", 3, [2]],
  [ga(2, 0, 1), "Living room actual temperature", "9.001", 5, []],
  [ga(2, 1, 1), "Living room valve position", "5.001", 5, []],
] as const;

// [seconds after start, source device id, group address, DPST, decoded text, raw payload]
const TIMETABLE: [number, number, number, string, string, string][] = [
  [0.6, 3, ga(0, 0, 1), "DPST-1-1", "On", "0x01"],
  [1.0, 1, ga(0, 1, 1), "DPST-1-1", "On", "0x01"],
  [2.0, 4, ga(0, 0, 2), "DPST-1-1", "On", "0x01"],
  [2.4, 1, ga(0, 1, 2), "DPST-1-1", "On", "0x01"],
  [3.4, 7, ga(0, 0, 4), "DPST-1-1", "On", "0x01"],
  [3.8, 6, ga(0, 1, 4), "DPST-1-1", "On", "0x01"],
  [4.8, 5, ga(2, 0, 1), "DPST-9-1", "21.5 °C", "0x0C33"],
  [5.3, 5, ga(2, 1, 1), "DPST-5-1", "38 %", "0x61"],
  [6.3, 3, ga(1, 0, 1), "DPST-1-8", "Down", "0x01"],
  [7.3, 8, ga(0, 0, 3), "DPST-1-1", "On", "0x01"],
  [7.7, 1, ga(0, 1, 1), "DPST-1-1", "On", "0x01"],
  [8.7, 3, ga(0, 0, 1), "DPST-1-1", "Off", "0x00"],
  [9.1, 1, ga(0, 1, 1), "DPST-1-1", "Off", "0x00"],
  [10.2, 4, ga(0, 0, 2), "DPST-1-1", "Off", "0x00"],
  [10.6, 1, ga(0, 1, 2), "DPST-1-1", "Off", "0x00"],
  [11.6, 5, ga(2, 0, 1), "DPST-9-1", "21.6 °C", "0x0C38"],
];
const CLIP_SECONDS = 13;
const FRAME_MS = 100;

const flags = { communication: true, read: null, write: true, transmit: null, update: null, readOnInit: null };
const diagnostics = {
  duplicateIndividualAddresses: [], ambiguousGroupAddresses: [], ambiguousDevices: [],
  danglingLinks: [], unknownDevices: [], objectsWithoutFlags: [],
};

function snapshot() {
  return {
    serverIncarnation: "readme", sessionId: 1, generation: "1", status: "current", groupAddressStyle: "ThreeLevel",
    devices: DEVICES.map(([deviceId, name, raw]) => ({ deviceId, installationId: 1, name, individualAddressRaw: raw })),
    groups: GROUPS.map(([gaRaw, name, dpt, sender, receivers], i) => ({
      gaRaw, gaId: i + 1, installationId: 1, name, dpt,
      members: [sender, ...receivers].map((deviceId, k) => ({
        deviceId, comObjectId: (i + 1) * 100 + k, direction: k === 0 ? "Send" : "Receive", active: true, flags,
      })),
    })),
    diagnostics,
    truncated: { devices: 0, groups: 0, members: 0, diagnostics: 0 },
  };
}

test("records the telegram flow of the fictional sample house", async ({ page }) => {
  test.setTimeout(60_000);
  let start = 0;
  const json = (route: Route, body: unknown, status = 200) =>
    route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
  await page.route("**/api/**", (route) => {
    const url = new URL(route.request().url());
    if (url.pathname === "/api/bus/monitor/telegrams") {
      const since = Number(url.searchParams.get("since") ?? "0");
      const elapsed = start ? (Date.now() - start) / 1000 : -1;
      const due = TIMETABLE.map((row, i) => ({ row, seq: i + 1 })).filter(({ row }) => row[0] <= elapsed);
      const telegrams = url.searchParams.get("contextOnly") ? [] : due.filter(({ seq }) => seq >= since).map(({ row, seq }) => {
        const [, source, dest, dpt, text, rawPayload] = row;
        const sourceRaw = DEVICES.find(([id]) => id === source)![2];
        return {
          seq, timestamp: new Date().toISOString(), source: fmtIa(sourceRaw), destination: fmtGa(dest),
          destinationName: GROUPS.find(([g]) => g === dest)![1], service: "GroupValueWrite", rawPayload,
          decoded: { kind: "value", dpt, text }, control: null, sourceRaw, destinationRaw: dest,
          observedAgeMs: 0, flowGeneration: "1",
        };
      });
      return json(route, {
        sessionId: 1, serverIncarnation: "readme", contextStatus: "current", projectOpen: true, status: "active",
        nextSince: due.length + 1, droppedBefore: 0, telegrams, flowGeneration: "1",
      });
    }
    if (url.pathname === "/api/bus/monitor/flow-snapshot") return json(route, snapshot());
    if (url.pathname === "/api/bus/discover") return json(route, { interfaces: [] });
    return json(route, { error: "readme recording only" }, 404);
  });

  await page.goto("/e2e/telegram-flow-fixture.html?lang=en&theme=graphite");
  // The bundled CRT pack, applied through the app's own validated path.
  const crt = JSON.parse(readFileSync(new URL("../themes/modern-retro-green-crt.knx-theme.json", import.meta.url), "utf8"));
  await page.evaluate(async (pack) => {
    const { applyThemePack } = await import("/src/themePackDom.ts");
    const applied = applyThemePack(document.documentElement, pack, "default");
    if (!applied.ok) throw new Error(JSON.stringify(applied.diagnostic));
  }, crt);
  await page.getByRole("tab", { name: "Flow" }).click();
  // The drawing appears with the first telegram. Frames are lossless element
  // screenshots, not the page video: a lossy video's noise on the black CRT
  // background changes every pixel of every frame and triples the GIF.
  start = Date.now();
  const drawing = page.locator(".flow-svg").first();
  await drawing.waitFor();
  const frames = test.info().outputPath("frames");
  mkdirSync(frames, { recursive: true });
  const shots: number[] = [];
  while (Date.now() - start < CLIP_SECONDS * 1000) {
    const due = start + 1000 + shots.length * FRAME_MS;
    if (Date.now() < due) await page.waitForTimeout(due - Date.now());
    await drawing.screenshot({ path: `${frames}/${String(shots.length).padStart(4, "0")}.png`, animations: "allow" });
    shots.push(Date.now());
  }

  const out = process.env.README_FLOW_GIF;
  if (out) {
    // Each frame lasts until the next one was taken, so the clip keeps real time.
    const list = shots.map((t, i) => `file '${frames}/${String(i).padStart(4, "0")}.png'\nduration ${(((shots[i + 1] ?? t + FRAME_MS) - t) / 1000).toFixed(3)}`);
    writeFileSync(`${frames}/list.txt`, list.join("\n") + "\n");
    const filter = "fps=10,scale=900:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=48:stats_mode=diff[p];[b][p]paletteuse=dither=none:diff_mode=rectangle";
    execFileSync("ffmpeg", ["-y", "-loglevel", "error", "-f", "concat", "-safe", "0", "-i", `${frames}/list.txt`, "-vf", filter, "-loop", "0", out]);
  }
});
