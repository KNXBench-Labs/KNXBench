/** The App-level achievement hook: one tracker per window, fed by events and keyboard. */
// ADR-0089. Mounted once, in `App.tsx`. It owns the tracker's lifetime,
// loads the record once (a StrictMode double effect does not load twice),
// subscribes the tracker to `emitAchievementEvent` and listens for the
// Konami code. `onUnlocked` is read through a ref so the caller may pass a
// fresh closure every render (it translates with the current language).
import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import type { AchievementDefinition } from "./achievementCatalog";
import { emitAchievementEvent, subscribeAchievementEvents } from "./achievementEvents";
import { loadAchievementsEnabled } from "./achievementPreference";
import {
  createAchievementTracker,
  httpAchievementsRequest,
  type AchievementTracker,
  type TrackerDeps,
  type TrackerSnapshot,
} from "./achievementTracker";
import { installKonamiListener } from "./konami";
import { settingsStorage } from "./settingsStore";

export function useAchievements(
  onUnlocked: (definitions: AchievementDefinition[]) => void,
  request: TrackerDeps["request"] = httpAchievementsRequest,
): { tracker: AchievementTracker; snapshot: TrackerSnapshot } {
  const onUnlockedRef = useRef(onUnlocked);
  onUnlockedRef.current = onUnlocked;
  const [tracker] = useState(() =>
    createAchievementTracker({
      request,
      now: () => new Date(),
      isEnabled: () => loadAchievementsEnabled(settingsStorage),
      onUnlocked: (definitions) => onUnlockedRef.current(definitions),
    }),
  );
  const snapshot = useSyncExternalStore(tracker.subscribe, tracker.snapshot);

  const loadStarted = useRef(false);
  useEffect(() => {
    if (loadStarted.current) return;
    loadStarted.current = true;
    void tracker.load();
  }, [tracker]);

  useEffect(() => subscribeAchievementEvents((event) => tracker.report(event)), [tracker]);

  useEffect(() => installKonamiListener(document, () => emitAchievementEvent({ type: "konamiCode" })), []);

  return { tracker, snapshot };
}
