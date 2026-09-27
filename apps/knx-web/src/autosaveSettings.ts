/** Autosave preferences: enabled by default, five-minute interval by default. */
// Same versioned-settings-document pattern as `motion.ts`: the record
// lives in `apps/knx-server/src/settings.rs`, `settingsStorage` is the
// synchronous cache over it, and this module owns exactly two keys.
import { useEffect, useState } from "react";
import { settingsStorage, useSettingsRevision } from "./settingsStore";

// Keys inside the settings document the server keeps, not `localStorage`
// keys — `settingsStorage` is what resolves them against the record.
const ENABLED_KEY = "autosaveEnabled";
const INTERVAL_MINUTES_KEY = "autosaveIntervalMinutes";

/** ISSUE-04: "autosave should default to five minutes". */
export const DEFAULT_AUTOSAVE_INTERVAL_MINUTES = 5;
/** The plan's five-second warning toast before an autosave actually runs. */
export const AUTOSAVE_COUNTDOWN_SECONDS = 5;

const MIN_INTERVAL_MINUTES = 1;
const MAX_INTERVAL_MINUTES = 120;

/**
 * Reads the persisted enabled flag. Anything that isn't the literal
 * string `"false"` reads as enabled — including an absent key, which is
 * what "enabled by default" means for a session that has never touched
 * this preference.
 */
export function loadAutosaveEnabled(storage: Pick<Storage, "getItem">): boolean {
  return storage.getItem(ENABLED_KEY) !== "false";
}

export function saveAutosaveEnabled(storage: Pick<Storage, "setItem">, enabled: boolean): void {
  storage.setItem(ENABLED_KEY, enabled ? "true" : "false");
}

/**
 * Reads the persisted interval in minutes. Anything unparsable, out of
 * range, or absent resolves to the default — the same
 * always-safe-default philosophy as `loadMotionLevel`.
 */
export function loadAutosaveIntervalMinutes(storage: Pick<Storage, "getItem">): number {
  const raw = storage.getItem(INTERVAL_MINUTES_KEY);
  if (raw === null) return DEFAULT_AUTOSAVE_INTERVAL_MINUTES;
  const parsed = Number(raw);
  if (!Number.isFinite(parsed) || parsed < MIN_INTERVAL_MINUTES || parsed > MAX_INTERVAL_MINUTES) {
    return DEFAULT_AUTOSAVE_INTERVAL_MINUTES;
  }
  return parsed;
}

export function saveAutosaveIntervalMinutes(storage: Pick<Storage, "setItem">, minutes: number): void {
  storage.setItem(INTERVAL_MINUTES_KEY, String(minutes));
}

/**
 * Reads the persisted autosave preferences on mount and re-reads them
 * whenever the settings record changes — exactly the `useMotion`/
 * `useAppearance` pattern, so a change made from another window (or the
 * settings panel in this one) reaches the running autosave timer without
 * a reload.
 */
export function useAutosaveSettings(): {
  enabled: boolean;
  setEnabled: (value: boolean) => void;
  intervalMinutes: number;
  setIntervalMinutes: (value: number) => void;
} {
  const revision = useSettingsRevision();
  const [enabled, setEnabled] = useState<boolean>(() => loadAutosaveEnabled(settingsStorage));
  const [intervalMinutes, setIntervalMinutes] = useState<number>(() =>
    loadAutosaveIntervalMinutes(settingsStorage),
  );

  useEffect(() => {
    setEnabled(loadAutosaveEnabled(settingsStorage));
    setIntervalMinutes(loadAutosaveIntervalMinutes(settingsStorage));
  }, [revision]);

  useEffect(() => {
    saveAutosaveEnabled(settingsStorage, enabled);
  }, [enabled]);

  useEffect(() => {
    saveAutosaveIntervalMinutes(settingsStorage, intervalMinutes);
  }, [intervalMinutes]);

  return { enabled, setEnabled, intervalMinutes, setIntervalMinutes };
}
