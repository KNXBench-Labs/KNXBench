/** Owns lossless line-scan exclusion storage and dotted individual-address validation. */
import { getSetting, setSetting, useSettingsRevision } from "./settingsStore";

export const LINE_SCAN_EXCLUSIONS_KEY = "lineScanExclusions";

export function loadLineScanExclusions(): string[] {
  const value = getSetting(LINE_SCAN_EXCLUSIONS_KEY);
  return Array.isArray(value)
    ? value.filter((item): item is string => typeof item === "string")
    : [];
}

export function saveLineScanExclusions(values: readonly string[]): void {
  setSetting(LINE_SCAN_EXCLUSIONS_KEY, [...values]);
}

export function validateIndividualAddress(value: string): boolean {
  const match = /^(\d+)\.(\d+)\.(\d+)$/.exec(value);
  if (!match) return false;
  const [, area, line, device] = match.map(Number);
  return area! <= 15 && line! <= 15 && device! <= 255;
}

export function hasInvalidLineScanExclusions(values: readonly string[]): boolean {
  return values.some(
    (value, index) => !validateIndividualAddress(value) || values.indexOf(value) !== index,
  );
}

export function useLineScanExclusions(): string[] {
  useSettingsRevision();
  return loadLineScanExclusions();
}
