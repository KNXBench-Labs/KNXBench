/** The achievements on/off switch: one key in the versioned settings record, on by default. */
// ADR-0089. Same pattern as `autosaveSettings.ts`: the record lives in
// `apps/knx-server/src/settings.rs`, `settingsStorage` is the synchronous
// cache over it. Off means off: the tracker asks `loadAchievementsEnabled`
// on every event and counts nothing while it says no. What was already
// unlocked stays in `achievements.json`; switching off is not a reset.
import { useEffect, useState } from "react";
import { settingsStorage, useSettingsRevision } from "./settingsStore";

const ENABLED_KEY = "achievementsEnabled";

/** Anything but the literal `"false"` reads as on, including no value. */
export function loadAchievementsEnabled(storage: Pick<Storage, "getItem">): boolean {
  return storage.getItem(ENABLED_KEY) !== "false";
}

export function saveAchievementsEnabled(storage: Pick<Storage, "setItem">, enabled: boolean): void {
  storage.setItem(ENABLED_KEY, enabled ? "true" : "false");
}

/** The switch as React state, re-read whenever the settings record changes. */
export function useAchievementsEnabled(): [boolean, (enabled: boolean) => void] {
  const revision = useSettingsRevision();
  const [enabled, setEnabledState] = useState(() => loadAchievementsEnabled(settingsStorage));
  useEffect(() => {
    setEnabledState(loadAchievementsEnabled(settingsStorage));
  }, [revision]);
  function setEnabled(value: boolean) {
    setEnabledState(value);
    saveAchievementsEnabled(settingsStorage, value);
  }
  return [enabled, setEnabled];
}
