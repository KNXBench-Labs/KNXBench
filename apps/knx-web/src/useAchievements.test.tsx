/** Tests for useAchievements: the event channel and the Konami listener reach the tracker. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it } from "vitest";
import { useAchievements } from "./useAchievements";
import { emitAchievementEvent } from "./achievementEvents";
import type { AchievementDefinition } from "./achievementCatalog";
import type { AchievementsResponse, TrackerSnapshot } from "./achievementTracker";
import { KONAMI_SEQUENCE } from "./konami";
import { resetSettingsForTests } from "./settingsStore";

let root: Root | undefined;

afterEach(() => {
  act(() => root?.unmount());
  root = undefined;
  resetSettingsForTests();
});

const EMPTY: AchievementsResponse = { schemaVersion: 1, unlocked: {}, progress: {}, status: "absent" };

async function mount() {
  const unlocked: string[] = [];
  const paths: string[] = [];
  let latest: TrackerSnapshot | undefined;
  const request = async (path: string, init?: RequestInit): Promise<AchievementsResponse> => {
    paths.push(path);
    if (init?.body) {
      const body = JSON.parse(String(init.body));
      return { ...EMPTY, status: "ok", unlocked: body.unlocked, progress: body.progress };
    }
    return EMPTY;
  };
  function Probe() {
    const { snapshot } = useAchievements((defs: AchievementDefinition[]) => unlocked.push(...defs.map((d) => d.id)), request);
    latest = snapshot;
    return null;
  }
  root = createRoot(document.createElement("div"));
  await act(async () => root!.render(<Probe />));
  return { unlocked, paths, snapshot: () => latest! };
}

describe("useAchievements", () => {
  it("loads the record once and becomes ready", async () => {
    const view = await mount();
    expect(view.paths).toEqual(["/api/achievements"]);
    expect(view.snapshot().status).toBe("ready");
  });

  it("turns an emitted event into an unlock and a saved record", async () => {
    const view = await mount();
    await act(async () => emitAchievementEvent({ type: "projectCreated" }));
    expect(view.unlocked).toEqual(["foundation"]);
    expect(view.paths).toContain("/api/achievements/record");
    expect(view.snapshot().record.unlocked.foundation).toBeTruthy();
  });

  it("unlocks the Konami achievement from the keyboard", async () => {
    const view = await mount();
    await act(async () => {
      for (const key of KONAMI_SEQUENCE) document.body.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
    });
    expect(view.unlocked).toEqual(["konami"]);
  });

  it("stops listening when unmounted", async () => {
    const view = await mount();
    act(() => root!.unmount());
    root = undefined;
    emitAchievementEvent({ type: "projectCreated" });
    expect(view.unlocked).toEqual([]);
  });
});
