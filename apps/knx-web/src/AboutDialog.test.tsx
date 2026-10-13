/** Tests that About states the running build, the licence, and claims nothing it has not earned. */
// @vitest-environment happy-dom
//
// T28/F5. Half of this is ordinary dialog behaviour; the other half is a
// legal boundary. KNXBench is not certified by the KNX Association and is
// not affiliated with it, and "ETS" is somebody else's trademark — so the
// wording that says so is pinned here, in both catalogues, rather than
// left to whoever next tidies up the message file.
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

const apiMock = vi.hoisted(() => ({ serverVersion: vi.fn() }));
vi.mock("./api", () => ({ ...apiMock }));
// B16: each test chooses the bundled Hall of Fame it ships with.
const bundled = vi.hoisted(() => ({ result: null as unknown }));
vi.mock("./bundledHallOfFame", () => ({ get BUNDLED_HALL_OF_FAME() { return bundled.result; } }));

import AboutDialog from "./AboutDialog";
import { HALL_OF_FAME_BOARD_URL, parseHallOfFame } from "./hallOfFame";
import { messages as en } from "./messages/en";
import { messages as de } from "./messages/de";

let host: HTMLDivElement | undefined;
let root: Root | undefined;
const onClose = vi.fn();

const board = (contributors: unknown[]) => parseHallOfFame(JSON.stringify({
  schema_version: 1, repository: "KNXBench-Labs/KNXBench", generated_at: "2026-10-13T21:14:07Z",
  policy: {}, ranks: [], resolution_awards: {}, contributors,
}));
const person = (login: string, position: number, xp: number, title: string, medals: string[] = []) =>
  ({ login, issues: 15, replies: 12, resolutions: 8, xp, rank: { minimum_xp: 0, title }, medals, position });

beforeEach(() => {
  bundled.result = board([person("thengsty", 1, 167, "Parameter Poltergeist Hunter", ["Golden Bus Probe"]),
    person("hagbardZ", 2, 31, "Bus Bug Scout")]);
});

afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  root = undefined;
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
});

async function mount() {
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  await act(async () => root!.render(<AboutDialog onClose={onClose} />));
  return host.querySelector<HTMLElement>(".about-dialog")!;
}

it("shows the version the server reports, not one baked into the frontend", async () => {
  apiMock.serverVersion.mockResolvedValue({ version: "0.1.0-alpha.1+g4cde085" });
  const panel = await mount();

  expect(panel.querySelector(".about-dialog-version")?.textContent).toBe("0.1.0-alpha.1+g4cde085");
  expect(apiMock.serverVersion).toHaveBeenCalledTimes(1);
  // The heading names the application, not the crate that answered.
  expect(panel.querySelector("#about-dialog-title")?.textContent).toContain("KNXBench");
  expect(panel.textContent).not.toContain("knx-server");
});

// A version a user is about to paste into a bug report must never be a
// guess, so a silent server gets said out loud rather than papered over.
it("says the version is unknown when the server does not answer", async () => {
  apiMock.serverVersion.mockRejectedValue(new Error("connection refused"));
  const panel = await mount();

  expect(panel.querySelector(".about-dialog-version")?.textContent)
    .toBe(en["about.versionUnknown"]);
});

it("names the licence and denies certification, affiliation and any ETS claim", async () => {
  apiMock.serverVersion.mockResolvedValue({ version: "0.1.0-alpha.1" });
  const panel = await mount();
  const text = panel.textContent ?? "";

  expect(text).toContain("AGPL-3.0-or-later");
  expect(text).toContain("not certified by the KNX Association");
  expect(text).toContain("not affiliated with it");
  expect(text).toContain("ETS is a trademark of the KNX Association");
  // The one sentence this dialog must never grow: no compatibility claim,
  // in any of its usual phrasings. The lookbehind spares the denial three
  // lines up, which is the only "certified" allowed in this dialog.
  expect(text).not.toMatch(/ETS[- ]compatible|compatible with ETS|(?<!not )certified|fully supports/i);
});

// The catalogues are checked for the same *keys*, never for the same
// meaning — a German sentence that quietly dropped the denial would pass
// every other test in this repository.
it("denies the same things in German", () => {
  expect(de["about.independence"]).toMatch(/nicht zertifiziert/);
  expect(de["about.independence"]).toMatch(/keine[rn]? Verbindung|in keiner Verbindung/);
  expect(de["about.trademark"]).toMatch(/Marke der KNX Association/);
  expect(de["about.licenceValue"]).toBe(en["about.licenceValue"]);
  expect(de["about.independence"]).not.toMatch(/zertifiziert von|ETS-kompatibel/);
});

it("closes on Escape and on its own button", async () => {
  apiMock.serverVersion.mockResolvedValue({ version: "0.1.0-alpha.1" });
  const panel = await mount();

  await act(async () => {
    panel.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
  });
  expect(onClose).toHaveBeenCalledTimes(1);

  const close = panel.querySelector<HTMLButtonElement>(".about-dialog-footer button")!;
  expect(close.textContent).toBe(en["about.close"]);
  await act(async () => close.dispatchEvent(new MouseEvent("click", { bubbles: true })));
  expect(onClose).toHaveBeenCalledTimes(2);
});

// The compact, centred layout must not cost a single fact: build, licence,
// the update notice with its link and pre-release warning, both legal
// sentences and the close button all stay, in reading order.
it("keeps every section, including the update notice, in reading order", async () => {
  apiMock.serverVersion.mockResolvedValue({ version: "0.1.0-alpha.9+g1234567" });
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  const updates = { status: "available" as const, check: vi.fn(async () => {}),
    update: { version: "0.1.0-alpha.10", url: "https://github.com/KNXBench-Labs/KNXBench/releases/tag/v0.1.0-alpha.10", prerelease: true } };
  await act(async () => root!.render(<AboutDialog onClose={onClose} updates={updates} />));
  const panel = host.querySelector<HTMLElement>(".about-dialog")!;
  const text = panel.textContent ?? "";
  const order = [en["about.title"], "0.1.0-alpha.9+g1234567", en["about.licenceValue"],
    en["updates.available"].replace("{version}", "0.1.0-alpha.10"), en["updates.releaseDetails"], en["updates.preview"],
    en["updates.check"], en["about.independence"], en["about.trademark"], en["about.close"]];
  const positions = order.map((part) => text.indexOf(part));
  expect(positions.every((position) => position >= 0), JSON.stringify(positions)).toBe(true);
  expect([...positions].sort((a, b) => a - b)).toEqual(positions);
  expect(panel.querySelector(".about-dialog-footer button")?.textContent).toBe(en["about.close"]);
});

// B16: the community board as bundled with this build — shown, never scored, never fetched.
it("shows every participant of the bundled Hall of Fame exactly as generated", async () => {
  apiMock.serverVersion.mockResolvedValue({ version: "0.1.0-alpha.1" });
  const fetchSpy = vi.spyOn(globalThis, "fetch");
  const panel = await mount();
  const section = panel.querySelector<HTMLElement>("[data-hall-of-fame]")!;
  const rows = [...section.querySelectorAll<HTMLElement>("[data-hall-of-fame-entry]")];
  expect(rows.map((row) => row.dataset.hallOfFameEntry)).toEqual(["thengsty", "hagbardZ"]);
  // The list numbers itself from the generator's positions; About computes none.
  expect(rows.map((row) => row.getAttribute("value"))).toEqual(["1", "2"]);
  expect(rows[0].textContent).toContain("Parameter Poltergeist Hunter");
  expect(rows[0].textContent).toContain("167 XP");
  expect(rows[0].textContent).toContain("Golden Bus Probe");
  expect(rows[0].textContent).toContain(en["about.hallOfFame.counters"]
    .replace("{issues}", "15").replace("{replies}", "12").replace("{resolutions}", "8"));
  const profile = rows[0].querySelector<HTMLAnchorElement>("a")!;
  expect([profile.href, profile.target, profile.rel]).toEqual(["https://github.com/thengsty", "_blank", "noopener noreferrer"]);
  // Bundled, dated, and honest about it; the live board is one link away.
  expect(section.querySelector("time")?.getAttribute("datetime")).toBe("2026-10-13T21:14:07Z");
  expect(section.textContent).toContain(en["about.hallOfFame.notLive"]);
  expect(section.querySelector<HTMLAnchorElement>("[data-hall-of-fame-board]")?.href).toBe(HALL_OF_FAME_BOARD_URL);
  expect(fetchSpy).not.toHaveBeenCalled();
  fetchSpy.mockRestore();
});

it("shows a hostile handle as plain text and links it nowhere", async () => {
  bundled.result = board([person("<img src=x onerror=alert(1)>", 1, 5, "Telegram Trainee")]);
  apiMock.serverVersion.mockResolvedValue({ version: "0.1.0-alpha.1" });
  const panel = await mount();
  const row = panel.querySelector<HTMLElement>("[data-hall-of-fame-entry]")!;
  expect(row.textContent).toContain("<img src=x onerror=alert(1)>");
  expect(row.querySelector("img, a")).toBeNull();
});

it("says so when the bundled board cannot be read, and still links the live one", async () => {
  bundled.result = parseHallOfFame("{ not json");
  apiMock.serverVersion.mockResolvedValue({ version: "0.1.0-alpha.1" });
  const panel = await mount();
  const section = panel.querySelector<HTMLElement>("[data-hall-of-fame]")!;
  expect(section.textContent).toContain(en["about.hallOfFame.unavailable"]);
  expect(section.querySelector("[data-hall-of-fame-entry]")).toBeNull();
  expect(section.querySelector<HTMLAnchorElement>("[data-hall-of-fame-board]")?.href).toBe(HALL_OF_FAME_BOARD_URL);
});

it("says so when nobody is on the board yet", async () => {
  bundled.result = board([]);
  apiMock.serverVersion.mockResolvedValue({ version: "0.1.0-alpha.1" });
  const panel = await mount();
  expect(panel.querySelector("[data-hall-of-fame]")?.textContent).toContain(en["about.hallOfFame.empty"]);
});

it("keeps the thank-you a thank-you in German too", () => {
  expect(de["about.hallOfFame.intro"]).toMatch(/Dankeschön/);
  expect(de["about.hallOfFame.intro"]).toMatch(/keine Bewertung/);
  expect(en["about.hallOfFame.intro"]).toMatch(/not a (rating|measure)/);
});
