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
import { afterEach, expect, it, vi } from "vitest";

const apiMock = vi.hoisted(() => ({ serverVersion: vi.fn() }));
vi.mock("./api", () => ({ ...apiMock }));

import AboutDialog from "./AboutDialog";
import { messages as en } from "./messages/en";
import { messages as de } from "./messages/de";

let host: HTMLDivElement | undefined;
let root: Root | undefined;
const onClose = vi.fn();

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
