/** Tests that programming is never answered "yes" without an explicit or remembered consent. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

const apiMock = vi.hoisted(() => ({ serverVersion: vi.fn() }));
vi.mock("./api", () => ({ ...apiMock }));

import { useProgrammingConsent, type ProgrammingConsent } from "./useProgrammingConsent";
import {
  PROGRAMMING_CONSENT_KEY,
  rememberProgrammingConsent,
} from "./programmingConsent";
import { getSetting, resetSettingsForTests } from "./settingsStore";
import { messages as en } from "./messages/en";
import { messages as de } from "./messages/de";

let host: HTMLDivElement | undefined;
let root: Root | undefined;
let consent: ProgrammingConsent | undefined;

function Harness() {
  consent = useProgrammingConsent();
  return <>{consent.dialog}</>;
}

beforeEach(() => resetSettingsForTests());

afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  root = undefined;
  host?.remove();
  host = undefined;
  consent = undefined;
  vi.clearAllMocks();
  resetSettingsForTests();
});

async function mount() {
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  await act(async () => root!.render(<Harness />));
}

/**
 * Starts a request and lets the version fetch settle, so the dialog (if
 * any) is on screen. Returns the pending answer wrapped, because an async
 * function returning a promise would otherwise make `await ask()` wait
 * for the user's answer itself.
 */
async function ask(target = "1.1.67 — Switch actuator"): Promise<{ answer: Promise<boolean> }> {
  let answer: Promise<boolean> | undefined;
  await act(async () => {
    answer = consent!.request(target);
  });
  return { answer: answer! };
}

function dialog() {
  return document.querySelector<HTMLElement>(".programming-consent-dialog");
}

function button(label: string) {
  return [...dialog()!.querySelectorAll("button")].find((b) => b.textContent === label)!;
}

it("names the alpha stage, the build and the target before anything is programmed", async () => {
  apiMock.serverVersion.mockResolvedValue({ version: "0.1.0-alpha.1+g4cde085" });
  await mount();
  await ask();

  const panel = dialog()!;
  expect(panel).not.toBeNull();
  expect(panel.querySelector(".programming-consent-stage")?.textContent).toBe("Alpha");
  expect(panel.querySelector(".programming-consent-version")?.textContent).toBe("0.1.0-alpha.1+g4cde085");
  expect(panel.textContent).toContain("1.1.67 — Switch actuator");
  expect(panel.textContent).toContain(en["programmingConsent.risk.alpha"]);
});

it("resolves true only after the user confirms, and remembers nothing unasked", async () => {
  apiMock.serverVersion.mockResolvedValue({ version: "0.1.0-alpha.1" });
  await mount();
  const { answer } = await ask();

  await act(async () => button(en["programmingConsent.confirm"]).click());
  await expect(answer).resolves.toBe(true);
  expect(dialog()).toBeNull();
  expect(getSetting(PROGRAMMING_CONSENT_KEY)).toBeUndefined();
});

it.each([
  ["Cancel", () => button(en["programmingConsent.cancel"]).click()],
  [
    "Escape",
    () => dialog()!.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })),
  ],
  ["the backdrop", () => document.querySelector<HTMLElement>(".search-overlay")!.click()],
])("%s means no", async (_label, dismiss) => {
  apiMock.serverVersion.mockResolvedValue({ version: "0.1.0-alpha.1" });
  await mount();
  const { answer } = await ask();

  await act(async () => dismiss());
  await expect(answer).resolves.toBe(false);
  expect(dialog()).toBeNull();
});

it("does not put initial focus on the confirm button", async () => {
  apiMock.serverVersion.mockResolvedValue({ version: "0.1.0-alpha.1" });
  await mount();
  await ask();
  expect(document.activeElement?.textContent).toBe(en["programmingConsent.cancel"]);
});

it("persists 'don't ask again' for the stage and skips the dialog next time", async () => {
  apiMock.serverVersion.mockResolvedValue({ version: "0.1.0-alpha.1" });
  await mount();
  const { answer: first } = await ask();

  const checkbox = dialog()!.querySelector<HTMLInputElement>(".programming-consent-remember input")!;
  await act(async () => checkbox.click());
  await act(async () => button(en["programmingConsent.confirm"]).click());
  await expect(first).resolves.toBe(true);
  expect(getSetting(PROGRAMMING_CONSENT_KEY)).toEqual({ stage: "alpha", version: "0.1.0-alpha.1" });

  const { answer: second } = await ask();
  expect(dialog()).toBeNull();
  await expect(second).resolves.toBe(true);
});

it("a ticked checkbox with Cancel remembers nothing", async () => {
  apiMock.serverVersion.mockResolvedValue({ version: "0.1.0-alpha.1" });
  await mount();
  const { answer } = await ask();

  const checkbox = dialog()!.querySelector<HTMLInputElement>(".programming-consent-remember input")!;
  await act(async () => checkbox.click());
  await act(async () => button(en["programmingConsent.cancel"]).click());
  await expect(answer).resolves.toBe(false);
  expect(getSetting(PROGRAMMING_CONSENT_KEY)).toBeUndefined();
});

it("asks again once the build moves from alpha to beta", async () => {
  rememberProgrammingConsent("alpha", "0.1.0-alpha.1");
  apiMock.serverVersion.mockResolvedValue({ version: "0.2.0-beta.1" });
  await mount();
  await ask();

  expect(dialog()).not.toBeNull();
  expect(dialog()!.querySelector(".programming-consent-stage")?.textContent).toBe("Beta");
});

it("asks every time, without a 'don't ask again' box, when the server does not answer", async () => {
  apiMock.serverVersion.mockRejectedValue(new Error("connection refused"));
  await mount();
  const { answer } = await ask();

  const panel = dialog()!;
  expect(panel.querySelector(".programming-consent-stage")?.textContent).toBe("Unknown");
  expect(panel.querySelector(".programming-consent-version")?.textContent).toBe(
    en["programmingConsent.versionUnknown"],
  );
  expect(panel.querySelector(".programming-consent-remember")).toBeNull();
  await act(async () => button(en["programmingConsent.confirm"]).click());
  await expect(answer).resolves.toBe(true);
  expect(getSetting(PROGRAMMING_CONSENT_KEY)).toBeUndefined();
});

it("refuses a second request while one question is open", async () => {
  apiMock.serverVersion.mockResolvedValue({ version: "0.1.0-alpha.1" });
  await mount();
  const { answer: first } = await ask("1.1.1");
  const { answer: second } = await ask("1.1.2");

  await expect(second).resolves.toBe(false);
  expect(dialog()!.textContent).toContain("1.1.1");
  await act(async () => button(en["programmingConsent.cancel"]).click());
  await expect(first).resolves.toBe(false);
});

it("an unmount with the question open is a no", async () => {
  apiMock.serverVersion.mockResolvedValue({ version: "0.1.0-alpha.1" });
  await mount();
  const { answer } = await ask();

  await act(async () => root!.unmount());
  root = undefined;
  await expect(answer).resolves.toBe(false);
});

it("every message key exists in both catalogues", () => {
  const keys = Object.keys(en).filter((k) => k.startsWith("programmingConsent.") || k.startsWith("settings.programmingConsent"));
  expect(keys.length).toBeGreaterThan(20);
  for (const key of keys) expect(de).toHaveProperty(key);
});
