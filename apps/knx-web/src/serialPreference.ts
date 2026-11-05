/** Own the persisted serial drafts and transport selection without connecting. */
import { useMemo } from "react";
import { getSetting, setSetting, useSettingsRevision } from "./settingsStore";
import type { BusTransport, SerialTarget } from "./serialTransport";

export function useBusTransport(): readonly [BusTransport, (value: BusTransport) => void] {
  const revision = useSettingsRevision();
  const value = useMemo(() => getSetting("preferredBusTransport") === "serial" ? "serial" : "ip", [revision]);
  return [value, (next) => setSetting("preferredBusTransport", next)];
}
export function loadSerialTarget(): SerialTarget {
  const path = getSetting("serialPath"), source = getSetting("serialSource"), mode = getSetting("serialMode");
  return { path: typeof path === "string" ? path : "", source: typeof source === "string" ? source : "", mode: mode === "active" ? "active" : "passive" };
}
export function useSerialTarget(): readonly [SerialTarget, (value: SerialTarget) => void] {
  const revision = useSettingsRevision();
  const value = useMemo(loadSerialTarget, [revision]);
  return [value, (next) => {
    setSetting("serialPath", next.path); setSetting("serialSource", next.source); setSetting("serialMode", next.mode);
  }];
}
