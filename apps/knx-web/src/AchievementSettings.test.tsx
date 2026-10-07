/** Tests for the achievements settings section: the switch and the non-destructive reset. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import AchievementSettings from "./AchievementSettings";
import type { AchievementTracker, AchievementsResponse } from "./achievementTracker";
import { loadAchievementsEnabled } from "./achievementPreference";
import { resetSettingsForTests, settingsStorage } from "./settingsStore";
import { messages as en } from "./messages/en";

let host: HTMLDivElement | undefined;
let root: Root | undefined;

afterEach(() => {
  act(() => root?.unmount());
  host?.remove();
  host = root = undefined;
  resetSettingsForTests();
});

function fakeTracker(reset: () => Promise<AchievementsResponse>): AchievementTracker {
  return {
    load: async () => {},
    report: () => {},
    reset: vi.fn(reset),
    snapshot: () => ({ status: "ready", record: { unlocked: {}, progress: {} } }),
    subscribe: () => () => {},
    settled: async () => {},
  };
}

async function render(tracker: AchievementTracker) {
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  await act(async () => root!.render(<AchievementSettings tracker={tracker} />));
  return host;
}

function button(text: string): HTMLButtonElement {
  return [...host!.querySelectorAll("button")].find((b) => b.textContent === text) as HTMLButtonElement;
}

const RESET_REPLY: AchievementsResponse = {
  schemaVersion: 1, unlocked: {}, progress: {}, status: "absent", movedTo: "achievements.reset-20261007T120000Z.json",
};

describe("AchievementSettings", () => {
  it("switches tracking off and on through the settings record", async () => {
    const view = await render(fakeTracker(async () => RESET_REPLY));
    const checkbox = view.querySelector("input[type=checkbox]") as HTMLInputElement;
    expect(checkbox.checked).toBe(true);
    await act(async () => checkbox.click());
    expect(loadAchievementsEnabled(settingsStorage)).toBe(false);
    await act(async () => checkbox.click());
    expect(loadAchievementsEnabled(settingsStorage)).toBe(true);
  });

  it("asks before resetting, and cancel resets nothing", async () => {
    const tracker = fakeTracker(async () => RESET_REPLY);
    const view = await render(tracker);
    await act(async () => button(en["settings.achievementsReset"]).click());
    expect(view.textContent).toContain(en["settings.achievementsResetConfirm"]);
    await act(async () => button(en["settings.achievementsResetCancel"]).click());
    expect(tracker.reset).not.toHaveBeenCalled();
    expect(view.textContent).not.toContain(en["settings.achievementsResetConfirm"]);
  });

  it("resets on confirmation and says where the old record went", async () => {
    const tracker = fakeTracker(async () => RESET_REPLY);
    const view = await render(tracker);
    await act(async () => button(en["settings.achievementsReset"]).click());
    await act(async () => button(en["settings.achievementsResetConfirmButton"]).click());
    expect(tracker.reset).toHaveBeenCalledTimes(1);
    expect(view.textContent).toContain("achievements.reset-20261007T120000Z.json");
  });

  it("says so when there was nothing to keep", async () => {
    const view = await render(fakeTracker(async () => ({ ...RESET_REPLY, movedTo: undefined })));
    await act(async () => button(en["settings.achievementsReset"]).click());
    await act(async () => button(en["settings.achievementsResetConfirmButton"]).click());
    expect(view.textContent).toContain(en["settings.achievementsResetNothing"]);
  });

  it("reports a failed reset", async () => {
    const view = await render(fakeTracker(async () => { throw new Error("nope"); }));
    await act(async () => button(en["settings.achievementsReset"]).click());
    await act(async () => button(en["settings.achievementsResetConfirmButton"]).click());
    expect(view.textContent).toContain(en["settings.achievementsResetFailed"]);
  });
});
