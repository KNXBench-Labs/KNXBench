/** Tests that the first-run guide opens once per stage, at start-up only, and never uninvited. */
// @vitest-environment happy-dom
//
// `onboardingGuide.test.ts` pins the rules; this file pins *when* they are
// asked: once, right after the settings record is first read from the
// server, with the state of the workbench at the moment the version
// answer lands.
import { act, StrictMode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

const apiMock = vi.hoisted(() => ({ serverVersion: vi.fn() }));
vi.mock("./api", () => ({ ...apiMock }));

import { ONBOARDING_GUIDE_KEY } from "./onboardingGuide";
import { getSetting, initSettings, resetSettingsForTests } from "./settingsStore";
import { useOnboardingGuide, type OnboardingGuideControls } from "./useOnboardingGuide";

let host: HTMLDivElement | undefined;
let root: Root | undefined;
let guide: OnboardingGuideControls | undefined;

function Harness(props: { projectOpen?: boolean; busy?: boolean }) {
  guide = useOnboardingGuide({ projectOpen: props.projectOpen ?? false, busy: props.busy ?? false });
  return null;
}

beforeEach(() => {
  resetSettingsForTests();
  vi.spyOn(console, "warn").mockImplementation(() => {});
  apiMock.serverVersion.mockResolvedValue({ version: "0.1.0-alpha.2+gabc1234" });
});

afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  root = undefined;
  host?.remove();
  host = undefined;
  guide = undefined;
  document.querySelectorAll("[data-test-dialog]").forEach((node) => node.remove());
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  vi.clearAllMocks();
  resetSettingsForTests();
});

/** A server whose settings record is `settings`; PUT patches are applied and recorded. */
function serve(settings: Record<string, unknown>, extra: Record<string, unknown> = { conditionalPatchVersion: 1 }) {
  const puts: Record<string, unknown>[] = [];
  vi.stubGlobal("fetch", vi.fn(async (_path: string, init?: RequestInit) => {
    if (init?.method === "PUT") {
      const body = JSON.parse(String(init.body)) as { settings: Record<string, unknown> };
      puts.push(body.settings);
      Object.assign(settings, body.settings);
    }
    return new Response(JSON.stringify({ schemaVersion: 1, status: "ok", settings, ...extra }), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    });
  }));
  return puts;
}

async function mount(element = <Harness />) {
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  await act(async () => root!.render(element));
}

/** Reads the settings record (as `main.tsx` does) and lets the version answer land. */
async function hydrate() {
  await act(async () => {
    await initSettings();
  });
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
  });
}

it("opens on the first start of an alpha build and remembers it as seen when closed", async () => {
  const puts = serve({});
  await mount();
  expect(guide!.open).toBeNull();
  await hydrate();
  expect(guide!.open).toEqual({ stage: "alpha", version: "0.1.0-alpha.2+gabc1234" });

  await act(async () => guide!.close());
  expect(guide!.open).toBeNull();
  expect(getSetting(ONBOARDING_GUIDE_KEY)).toEqual({ seenStage: "alpha", version: "0.1.0-alpha.2+gabc1234" });
  await act(async () => { await Promise.resolve(); });
  expect(puts).toContainEqual({ [ONBOARDING_GUIDE_KEY]: { seenStage: "alpha", version: "0.1.0-alpha.2+gabc1234" } });
});

it("stays closed once seen in this stage, and opens again for the next stage", async () => {
  serve({ [ONBOARDING_GUIDE_KEY]: { seenStage: "alpha", version: "0.1.0-alpha.1" } });
  await mount();
  await hydrate();
  expect(guide!.open).toBeNull();
  await act(async () => root!.unmount());
  root = undefined;

  resetSettingsForTests();
  serve({ [ONBOARDING_GUIDE_KEY]: { seenStage: "alpha", version: "0.1.0-alpha.1" } });
  apiMock.serverVersion.mockResolvedValue({ version: "0.1.0-beta.1" });
  await mount();
  await hydrate();
  expect(guide!.open?.stage).toBe("beta");
});

it("does not open over a project that is open when the version answer lands", async () => {
  serve({});
  await mount(<Harness projectOpen />);
  await hydrate();
  expect(guide!.open).toBeNull();
  // Closing the project later does not bring it up: one decision per start.
  await act(async () => root!.render(<Harness projectOpen={false} />));
  await act(async () => { await Promise.resolve(); });
  expect(guide!.open).toBeNull();
});

it("does not open during a project load or over another dialog", async () => {
  serve({});
  await mount(<Harness busy />);
  await hydrate();
  expect(guide!.open).toBeNull();
  await act(async () => root!.unmount());
  root = undefined;

  resetSettingsForTests();
  serve({});
  const dialog = document.createElement("div");
  dialog.setAttribute("role", "dialog");
  dialog.setAttribute("data-test-dialog", "");
  document.body.append(dialog);
  await mount();
  await hydrate();
  expect(guide!.open).toBeNull();
});

it("never asks for the version when the settings record cannot remember the guide", async () => {
  // An older server without conditional patches: the record is not acknowledged.
  serve({}, {});
  await mount();
  await hydrate();
  expect(guide!.open).toBeNull();
  expect(apiMock.serverVersion).not.toHaveBeenCalled();
});

it("stays closed when the settings record cannot be read at all", async () => {
  vi.stubGlobal("fetch", vi.fn(async () => { throw new Error("connection refused"); }));
  await mount();
  await hydrate();
  expect(guide!.open).toBeNull();
  expect(apiMock.serverVersion).not.toHaveBeenCalled();
});

it("stays closed when the server does not say which build it is", async () => {
  serve({});
  apiMock.serverVersion.mockRejectedValue(new Error("no answer"));
  await mount();
  await hydrate();
  expect(guide!.open).toBeNull();
});

it("opens exactly once under StrictMode's doubled effects", async () => {
  serve({});
  await mount(<StrictMode><Harness /></StrictMode>);
  await hydrate();
  expect(guide!.open?.stage).toBe("alpha");
});

it("opens on request even when already seen, and the request replaces the start-up decision", async () => {
  serve({ [ONBOARDING_GUIDE_KEY]: { seenStage: "alpha", version: null } });
  await mount();
  await act(async () => guide!.show());
  await act(async () => { await Promise.resolve(); await Promise.resolve(); });
  expect(guide!.open?.stage).toBe("alpha");
  await act(async () => guide!.close());
  await hydrate();
  expect(guide!.open).toBeNull();
});

it("an unnamed stage opened on request is shown but never remembered", async () => {
  serve({});
  apiMock.serverVersion.mockResolvedValue({ version: "1.0.0-nightly.1" });
  await mount();
  await act(async () => guide!.show());
  await act(async () => { await Promise.resolve(); await Promise.resolve(); });
  expect(guide!.open?.stage).toBe("preRelease");
  await act(async () => guide!.close());
  expect(getSetting(ONBOARDING_GUIDE_KEY)).toBeUndefined();
});
