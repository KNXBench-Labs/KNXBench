/** Owns automatic release-check lifetime separately from project and bus operations. */
import { useCallback, useEffect, useRef, useState } from "react";
import { serverVersion } from "./api";
import { fetchGitHubReleases } from "./githubReleases";
import { isReleaseVersion, selectReleaseUpdate, type ReleaseUpdate } from "./releaseUpdates";
import { getAcknowledgedSettings, getSetting, useSettingsRevision, useSettingsState } from "./settingsStore";

export const UPDATE_CHECK_INTERVAL_MS = 60 * 60 * 1000;
export { UPDATE_CHECK_SETTING } from "./releaseUpdatePreference";
import { UPDATE_CHECK_SETTING, releaseChecksEnabled } from "./releaseUpdatePreference";
export type UpdateStatus = "idle" | "checking" | "current" | "available" | "unavailable";
export interface ReleaseUpdateControls {
  status: UpdateStatus;
  update: ReleaseUpdate | null;
  check: () => Promise<void>;
}

export function useReleaseUpdates(notify: (update: ReleaseUpdate) => void): ReleaseUpdateControls {
  const { hydration } = useSettingsState();
  useSettingsRevision();
  const enabled = releaseChecksEnabled(getSetting(UPDATE_CHECK_SETTING));
  const ready = hydration === "hydrated" && getAcknowledgedSettings([UPDATE_CHECK_SETTING]).ok;
  const [state, setState] = useState<{ status: UpdateStatus; update: ReleaseUpdate | null }>({ status: "idle", update: null });
  const notifyRef = useRef(notify); notifyRef.current = notify;
  const seen = useRef(new Set<string>());
  const active = useRef<AbortController | null>(null);
  const lastAttempt = useRef<number | null>(null);

  const run = useCallback(async (manual: boolean) => {
    if (!ready || active.current || (!manual && (!enabled || document.visibilityState === "hidden"))) return;
    const now = Date.now();
    if (!manual && lastAttempt.current !== null && now >= lastAttempt.current && now - lastAttempt.current < UPDATE_CHECK_INTERVAL_MS) return;
    lastAttempt.current = now;
    const controller = new AbortController(); active.current = controller;
    // Includes the same-origin version read, not only the external request.
    const deadline = setTimeout(() => controller.abort(), 10_000);
    setState(previous => ({ ...previous, status: "checking" }));
    try {
      const current = (await serverVersion(controller.signal)).version;
      if (controller.signal.aborted) return;
      if (!isReleaseVersion(current)) throw new Error("Unknown running version");
      const rows = await fetchGitHubReleases(controller.signal);
      if (controller.signal.aborted) return;
      const update = selectReleaseUpdate(current, rows);
      setState({ status: update ? "available" : "current", update });
      if (update && !seen.current.has(update.version)) {
        seen.current.add(update.version);
        notifyRef.current(update);
      }
    } catch {
      if (active.current === controller) setState(previous => ({ ...previous, status: "unavailable" }));
    } finally {
      clearTimeout(deadline);
      if (active.current === controller) active.current = null;
    }
  }, [enabled, ready]);

  useEffect(() => {
    if (!ready || !enabled) return;
    const automatic = () => { void run(false); };
    automatic();
    const timer = setInterval(automatic, UPDATE_CHECK_INTERVAL_MS);
    window.addEventListener("focus", automatic);
    window.addEventListener("online", automatic);
    document.addEventListener("visibilitychange", automatic);
    return () => {
      clearInterval(timer);
      window.removeEventListener("focus", automatic);
      window.removeEventListener("online", automatic);
      document.removeEventListener("visibilitychange", automatic);
      active.current?.abort(); active.current = null;
      // StrictMode's canceled first mount must not swallow the real startup check.
      lastAttempt.current = null;
    };
  }, [enabled, ready, run]);

  // Also cancel an explicitly requested check on unmount/disable/hydration loss.
  useEffect(() => {
    if (!enabled || !ready) {
      setState(previous => previous.status === "checking"
        ? { ...previous, status: previous.update ? "available" : "idle" } : previous);
    }
    return () => { active.current?.abort(); active.current = null; };
  }, [enabled, ready]);
  return { ...state, check: useCallback(() => run(true), [run]) };
}
