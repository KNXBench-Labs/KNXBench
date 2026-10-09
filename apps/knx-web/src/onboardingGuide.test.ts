/** Tests when the first-run guide opens by itself and what it remembers. */
// @vitest-environment happy-dom
//
// The guide is advice, not a gate, so the rules pinned here lean towards
// staying quiet: it opens by itself only once per nameable release stage,
// only over an empty workbench, and never on a settings record the server
// has not acknowledged (a guide that cannot remember being seen would open
// on every start).
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { COMMANDS } from "./commandRegistry";
import type { CommandContext } from "./commandRegistry";
import {
  ONBOARDING_GUIDE_KEY,
  ONBOARDING_TASKS,
  rememberOnboardingGuideSeen,
  seenOnboardingStage,
  shouldOpenOnboardingGuide,
  type OnboardingStartup,
} from "./onboardingGuide";
import type { ReleaseStage } from "./programmingConsent";
import { getSetting, resetSettingsForTests } from "./settingsStore";

beforeEach(() => resetSettingsForTests());
afterEach(() => resetSettingsForTests());

function startup(overrides: Partial<OnboardingStartup> = {}): OnboardingStartup {
  return {
    stage: "alpha",
    settings: { ok: true, settings: { [ONBOARDING_GUIDE_KEY]: null } },
    projectOpen: false,
    busy: false,
    otherDialogOpen: false,
    ...overrides,
  };
}

describe("shouldOpenOnboardingGuide", () => {
  it("opens on a first start of a named stage", () => {
    expect(shouldOpenOnboardingGuide(startup())).toBe(true);
  });

  it("stays closed once the guide was seen in this stage", () => {
    const settings = { ok: true as const, settings: { [ONBOARDING_GUIDE_KEY]: { seenStage: "alpha" } } };
    expect(shouldOpenOnboardingGuide(startup({ settings }))).toBe(false);
  });

  it.each<[ReleaseStage, ReleaseStage]>([
    ["alpha", "beta"],
    ["beta", "releaseCandidate"],
    ["releaseCandidate", "stable"],
    // A downgrade is a different stage too; the guide says what *this* build is.
    ["beta", "alpha"],
  ])("opens again when the stage moves from %s to %s", (seen, running) => {
    const settings = { ok: true as const, settings: { [ONBOARDING_GUIDE_KEY]: { seenStage: seen } } };
    expect(shouldOpenOnboardingGuide(startup({ stage: running, settings }))).toBe(true);
  });

  it.each<ReleaseStage>(["unknown", "preRelease"])("never opens by itself for the %s stage", (stage) => {
    expect(shouldOpenOnboardingGuide(startup({ stage }))).toBe(false);
  });

  it.each(["notHydrated", "incompatibleSettings", "unsupportedServer", "pendingWrite", "unsupportedValue", "uncertain"] as const)(
    "never opens on unacknowledged settings (%s)",
    (reason) => {
      expect(shouldOpenOnboardingGuide(startup({ settings: { ok: false, reason } }))).toBe(false);
    },
  );

  it("never opens over a project, a load in progress or another dialog", () => {
    expect(shouldOpenOnboardingGuide(startup({ projectOpen: true }))).toBe(false);
    expect(shouldOpenOnboardingGuide(startup({ busy: true }))).toBe(false);
    expect(shouldOpenOnboardingGuide(startup({ otherDialogOpen: true }))).toBe(false);
  });
});

describe("seenOnboardingStage", () => {
  it("reads a stored nameable stage", () => {
    expect(seenOnboardingStage({ seenStage: "beta", version: "1.0.0-beta.1" })).toBe("beta");
  });

  it.each<unknown>([undefined, null, "alpha", [], {}, { seenStage: 3 }, { seenStage: "unknown" }, { seenStage: "gamma" }])(
    "treats %j as never seen",
    (value) => {
      expect(seenOnboardingStage(value)).toBeUndefined();
    },
  );
});

describe("rememberOnboardingGuideSeen", () => {
  it("stores the stage and, for the reader of settings.json, the version", () => {
    rememberOnboardingGuideSeen("alpha", "0.1.0-alpha.2+gabc1234");
    expect(getSetting(ONBOARDING_GUIDE_KEY)).toEqual({ seenStage: "alpha", version: "0.1.0-alpha.2+gabc1234" });
  });

  it.each<ReleaseStage>(["unknown", "preRelease"])("stores nothing for the %s stage", (stage) => {
    rememberOnboardingGuideSeen(stage, null);
    expect(getSetting(ONBOARDING_GUIDE_KEY)).toBeUndefined();
  });
});

describe("ONBOARDING_TASKS", () => {
  // The guide's buttons are the command palette's commands, not a second
  // way to do the same thing. A task naming a command that does not exist,
  // or one that is disabled on an empty workbench (where the guide opens),
  // would be a dead button.
  it("names only commands that exist and run with no project open", () => {
    const ran: string[] = [];
    const record = (name: string) => () => { ran.push(name); };
    const ctx: CommandContext = {
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
      openDevices: () => {},
      addDevice: record("addDevice"),
      openIntroduction: record("openIntroduction"),
      openAchievements: record("openAchievements"),
    };
    for (const task of ONBOARDING_TASKS) {
      const command = COMMANDS.find((entry) => entry.id === task.commandId);
      expect(command, task.commandId).toBeDefined();
      expect(command!.isEnabled(ctx), task.commandId).toBe(true);
      command!.run(ctx);
    }
    expect(ran).toEqual(["pickProject", "newProject", "openNativeProject", "openCatalog", "openBusMonitor"]);
  });

  it("has unique ids", () => {
    expect(new Set(ONBOARDING_TASKS.map((task) => task.id)).size).toBe(ONBOARDING_TASKS.length);
  });
});
