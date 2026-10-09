/** Tests the first-run guide's pages, its exits, and that its buttons run palette commands. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { CommandContext } from "./commandRegistry";
import { messages as de } from "./messages/de";
import { messages as en } from "./messages/en";
import OnboardingGuide from "./OnboardingGuide";
import type { ReleaseStage } from "./programmingConsent";
import { resetSettingsForTests } from "./settingsStore";
import { resetUiLanguageForTests } from "./uiLanguage";

let host: HTMLDivElement | undefined;
let root: Root | undefined;
let calls: string[] = [];
const onClose = vi.fn(() => { calls.push("close"); });

function ctx(): CommandContext {
  const record = (name: string) => () => { calls.push(name); };
  return {
    tree: null,
    newProject: record("newProject"),
    pickProject: record("pickProject"),
    openNativeProject: record("openNativeProject"),
    saveProject: record("saveProject"),
    saveProjectAs: record("saveProjectAs"),
    undo: record("undo"),
    redo: record("redo"),
    openSearch: record("openSearch"),
    openLog: record("openLog"),
    openBusMonitor: record("openBusMonitor"),
    openSettings: record("openSettings"),
    openCompanion: record("openCompanion"),
    openHelp: record("openHelp"),
    openCatalog: record("openCatalog"),
    openProjectHistory: () => {},
    openDevices: () => {},
    addDevice: record("addDevice"),
    openIntroduction: record("openIntroduction"),
    openAchievements: record("openAchievements"),
  };
}

beforeEach(() => {
  resetSettingsForTests();
  resetUiLanguageForTests();
  calls = [];
});

afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  root = undefined;
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
  resetSettingsForTests();
  resetUiLanguageForTests();
});

async function mount(stage: ReleaseStage = "alpha") {
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  await act(async () => root!.render(<OnboardingGuide stage={stage} ctx={ctx()} onClose={onClose} />));
  return document.querySelector<HTMLElement>(".onboarding-guide")!;
}

function button(panel: HTMLElement, label: string): HTMLButtonElement {
  const found = Array.from(panel.querySelectorAll("button")).find((entry) => entry.textContent?.includes(label));
  if (!found) throw new Error(`no button labelled ${label}`);
  return found;
}

async function click(target: HTMLElement) {
  await act(async () => target.click());
}

it("says what the build is first: its stage, what the stage means, and to keep backups", async () => {
  const panel = await mount("alpha");
  expect(panel.getAttribute("role")).toBe("dialog");
  expect(panel.querySelector("#onboarding-title")?.textContent).toBe(en["onboarding.title"]);
  expect(panel.querySelector(".onboarding-progress")?.textContent).toBe("Step 1 of 4");
  expect(panel.querySelector(".onboarding-stage")?.textContent).toBe(en["programmingConsent.stage.alpha"]);
  expect(panel.textContent).toContain(en["onboarding.stage.alpha"]);
  expect(panel.textContent).toContain(en["onboarding.step.about.trust"]);
  expect(panel.textContent).toContain("it is not ETS");
  // Focus starts on Next, so Enter pages forward rather than running anything.
  expect(document.activeElement?.textContent).toBe(en["onboarding.next"]);
});

it("pages forward and back through all four steps, and the last one finishes", async () => {
  const panel = await mount();
  for (const [index, title] of [
    "onboarding.step.scope.title",
    "onboarding.step.start.title",
    "onboarding.step.help.title",
  ].entries()) {
    await click(button(panel, en["onboarding.next"]));
    expect(panel.querySelector(".onboarding-progress")?.textContent).toBe(`Step ${index + 2} of 4`);
    expect(panel.querySelector("#onboarding-step-title")?.textContent).toBe(en[title as keyof typeof en]);
  }
  expect(panel.textContent).not.toContain(en["onboarding.skip"]);
  await click(button(panel, en["onboarding.back"]));
  expect(panel.querySelector(".onboarding-progress")?.textContent).toBe("Step 3 of 4");
  await click(button(panel, en["onboarding.back"]));
  await click(button(panel, en["onboarding.back"]));
  // Back is gone on the first page; focus must not fall out of the dialog.
  expect(panel.textContent).not.toContain(en["onboarding.back"]);
  expect(panel.contains(document.activeElement)).toBe(true);
  for (let i = 0; i < 3; i += 1) await click(button(panel, en["onboarding.next"]));
  await click(button(panel, en["onboarding.finish"]));
  expect(onClose).toHaveBeenCalledTimes(1);
});

it("closes on Skip and on Escape without asking anything", async () => {
  let panel = await mount();
  await click(button(panel, en["onboarding.skip"]));
  expect(onClose).toHaveBeenCalledTimes(1);
  await act(async () => root!.unmount());
  root = undefined;

  panel = await mount();
  await act(async () => {
    panel.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
  });
  expect(onClose).toHaveBeenCalledTimes(2);
});

it("lists what works and what does not, and links the limits help topic", async () => {
  const panel = await mount();
  await click(button(panel, en["onboarding.next"]));
  expect(panel.querySelectorAll(".onboarding-list-works li")).toHaveLength(5);
  expect(panel.querySelectorAll(".onboarding-list-not-yet li")).toHaveLength(3);
  const opened: string[] = [];
  const listener = (event: Event) => opened.push((event as CustomEvent<{ topicId: string }>).detail.topicId);
  window.addEventListener("knxbench:open-help", listener);
  await click(button(panel, en["onboarding.step.scope.more"]));
  window.removeEventListener("knxbench:open-help", listener);
  expect(calls).toEqual(["close"]);
  expect(opened).toEqual(["limits"]);
});

it.each([
  ["onboarding.task.importEts.title", "pickProject"],
  ["onboarding.task.newProject.title", "newProject"],
  ["onboarding.task.openNative.title", "openNativeProject"],
  ["onboarding.task.productData.title", "openCatalog"],
  ["onboarding.task.busMonitor.title", "openBusMonitor"],
] as const)("the task %s closes the guide, then runs its palette command", async (titleKey, action) => {
  const panel = await mount();
  await click(button(panel, en["onboarding.next"]));
  await click(button(panel, en["onboarding.next"]));
  expect(panel.querySelectorAll(".onboarding-task")).toHaveLength(5);
  await click(button(panel, en[titleKey]));
  expect(calls).toEqual(["close", action]);
});

it("the help page opens Help through the palette command", async () => {
  const panel = await mount();
  for (let i = 0; i < 3; i += 1) await click(button(panel, en["onboarding.next"]));
  await click(button(panel, en["onboarding.step.help.openHelp"]));
  expect(calls).toEqual(["close", "openHelp"]);
});

it("switches the interface language from the first page", async () => {
  const panel = await mount();
  const deutsch = button(panel, en["language.de"]);
  expect(deutsch.getAttribute("aria-pressed")).toBe("false");
  await click(deutsch);
  expect(panel.querySelector("#onboarding-title")?.textContent).toBe(de["onboarding.title"]);
  expect(button(panel, de["language.de"]).getAttribute("aria-pressed")).toBe("true");
  expect(panel.querySelector(".onboarding-progress")?.textContent).toBe("Schritt 1 von 4");
});

it.each<ReleaseStage>(["alpha", "beta", "releaseCandidate", "stable", "preRelease", "unknown"])(
  "names the %s stage and says what it means",
  async (stage) => {
    const panel = await mount(stage);
    expect(panel.querySelector(`.onboarding-stage-${stage}`)).not.toBeNull();
    expect(panel.querySelector(".onboarding-stage-meaning")?.textContent).toBe(en[`onboarding.stage.${stage}`]);
  },
);
