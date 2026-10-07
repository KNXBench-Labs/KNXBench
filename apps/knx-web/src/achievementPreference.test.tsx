/** Unit tests for the achievements on/off preference. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it } from "vitest";
import { loadAchievementsEnabled, saveAchievementsEnabled, useAchievementsEnabled } from "./achievementPreference";
import { resetSettingsForTests, settingsStorage } from "./settingsStore";

afterEach(() => resetSettingsForTests());

function storage(value: string | null) {
  return { getItem: () => value };
}

describe("loadAchievementsEnabled", () => {
  it("is on unless the stored value is exactly \"false\"", () => {
    expect(loadAchievementsEnabled(storage(null))).toBe(true);
    expect(loadAchievementsEnabled(storage("true"))).toBe(true);
    expect(loadAchievementsEnabled(storage("garbage"))).toBe(true);
    expect(loadAchievementsEnabled(storage("false"))).toBe(false);
  });
});

describe("saveAchievementsEnabled", () => {
  it("round-trips through the settings record", () => {
    saveAchievementsEnabled(settingsStorage, false);
    expect(settingsStorage.getItem("achievementsEnabled")).toBe("false");
    expect(loadAchievementsEnabled(settingsStorage)).toBe(false);
    saveAchievementsEnabled(settingsStorage, true);
    expect(loadAchievementsEnabled(settingsStorage)).toBe(true);
  });
});

describe("useAchievementsEnabled", () => {
  it("follows a change made elsewhere", async () => {
    const seen: boolean[] = [];
    function Probe() {
      const [enabled] = useAchievementsEnabled();
      seen.push(enabled);
      return null;
    }
    const host = document.createElement("div");
    const root = createRoot(host);
    await act(async () => root.render(<Probe />));
    await act(async () => saveAchievementsEnabled(settingsStorage, false));
    expect(seen[0]).toBe(true);
    expect(seen.at(-1)).toBe(false);
    root.unmount();
  });
});
