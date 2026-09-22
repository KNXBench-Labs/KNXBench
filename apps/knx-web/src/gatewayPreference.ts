/** Owns the passive preferred-gateway value stored in the shared settings record. */
import { useMemo } from "react";
import { settingsStorage, useSettingsRevision } from "./settingsStore";

export const PREFERRED_GATEWAY_KEY = "preferredGateway";

export function loadPreferredGateway(): string {
  return settingsStorage.getItem(PREFERRED_GATEWAY_KEY)?.trim() ?? "";
}

export function savePreferredGateway(value: string): void {
  const normalized = value.trim();
  if (normalized === "") settingsStorage.removeItem(PREFERRED_GATEWAY_KEY);
  else settingsStorage.setItem(PREFERRED_GATEWAY_KEY, normalized);
}

export function usePreferredGateway(): readonly [string, (value: string) => void] {
  const revision = useSettingsRevision();
  const gateway = useMemo(loadPreferredGateway, [revision]);
  return [gateway, savePreferredGateway] as const;
}
