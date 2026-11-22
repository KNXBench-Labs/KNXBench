/** Per-device DeviceDetail.last_download reads for the group-address views (guarded). */
// Reuses the object-inventory pattern: one snapshot-bound read per device, replies
// applied only for their own generation, project and snapshot, so a late reply for
// another project or device never reaches the view. The read is gated on an explicit
// Settings preference ("Last-download marks", off by default; see WP-48.5), so a profile
// that never enabled it issues no per-device call.
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import * as api from "./api";
import type { DeviceNode } from "./bindings/DeviceNode";
import type { ObjectLastDownload } from "./bindings/ObjectLastDownload";
import { useProductLanguage } from "./productLanguage";
import { settingsStorage, useSettingsRevision } from "./settingsStore";

/** The preference key; absent means the group-address last-download marks stay off. */
export const GA_LAST_DOWNLOAD_KEY = "gaLastDownloadMarks";

/** Whether the preference is on. Off by default: it reads every device's detail. Only `"true"`/`"1"` enable it. */
export function gaLastDownloadMarksEnabled(storage: Pick<Storage, "getItem"> = settingsStorage): boolean {
  try { const value = storage.getItem(GA_LAST_DOWNLOAD_KEY); return value === "true" || value === "1"; } catch { return false; }
}

/** Writes the preference as the literal `"true"`/`"false"`, like the other boolean preferences. */
export function saveGaLastDownloadMarks(storage: Pick<Storage, "setItem">, enabled: boolean): void {
  storage.setItem(GA_LAST_DOWNLOAD_KEY, enabled ? "true" : "false");
}

/** The preference as React state, re-read whenever the settings record changes. */
export function useGaLastDownloadMarks(): [boolean, (enabled: boolean) => void] {
  const revision = useSettingsRevision();
  const [enabled, setEnabledState] = useState(() => gaLastDownloadMarksEnabled());
  useEffect(() => { setEnabledState(gaLastDownloadMarksEnabled()); }, [revision]);
  return [enabled, (value: boolean) => { setEnabledState(value); saveGaLastDownloadMarks(settingsStorage, value); }];
}

export type LastDownloadEntry =
  | { status: "loading"; source: object; previous: ObjectLastDownload | null }
  | { status: "loaded"; source: object; lastDownload: ObjectLastDownload | undefined }
  | { status: "failed"; source: object; message: string | null };

export interface DeviceLastDownloads {
  entries: ReadonlyMap<number, LastDownloadEntry>;
  source: object;
  /** The current snapshot's comparison per device; absent entry means not yet read. */
  current: Map<number, ObjectLastDownload | undefined>;
  /** The preference state this read set was built under. */
  enabled: boolean;
  load: (deviceId: number) => void;
}

/**
 * One `GET /api/device/{id}` per device of the installation, re-read after a
 * project update or language change. The device detail is the only carrier of
 * `last_download` (ADR-0174 §5); edit responses omit it.
 */
export function useDeviceLastDownloads(devices: DeviceNode[], snapshot: object, projectKey: string): DeviceLastDownloads {
  const [language] = useProductLanguage();
  useSettingsRevision(); // re-render on a preference change; only the preference itself restarts the reads
  const enabled = gaLastDownloadMarksEnabled();
  const source = useMemo(() => ({}), [snapshot, language, projectKey, enabled]);
  const [entries, setEntries] = useState<ReadonlyMap<number, LastDownloadEntry>>(() => new Map());
  const entriesRef = useRef(entries); entriesRef.current = entries;
  const live = useRef({ source, language, projectKey, mounted: true, generations: new Map<number, number>() });

  const load = useCallback((deviceId: number) => {
    const l = live.current;
    const generation = (l.generations.get(deviceId) ?? 0) + 1;
    l.generations.set(deviceId, generation);
    const requestSource = l.source;
    setEntries(previous => {
      const old = previous.get(deviceId);
      const kept = old?.status === "loaded" ? old.lastDownload ?? null : old?.status === "loading" ? old.previous : null;
      return new Map(previous).set(deviceId, { status: "loading", source: requestSource, previous: kept });
    });
    const settle = (entry: LastDownloadEntry) => {
      if (!l.mounted || l.generations.get(deviceId) !== generation || l.source !== requestSource) return;
      setEntries(previous => new Map(previous).set(deviceId, entry));
    };
    // The read is optional for the marks: a missing or failed comparison marks nothing.
    Promise.resolve(api.deviceDetail(deviceId, l.language)).then(
      detail => settle(detail.id === deviceId
        ? { status: "loaded", source: requestSource, lastDownload: detail.last_download }
        : { status: "failed", source: requestSource, message: null }),
      error => settle({ status: "failed", source: requestSource, message: api.errorMessage(error) }),
    );
  }, []);

  useEffect(() => {
    const l = live.current;
    l.mounted = true;
    return () => { l.mounted = false; };
  }, []);

  useEffect(() => {
    const l = live.current;
    const switched = l.projectKey !== projectKey;
    l.source = source; l.language = language; l.projectKey = projectKey;
    const forget = (id: number) => l.generations.set(id, (l.generations.get(id) ?? 0) + 1);
    if (switched || !enabled) {
      for (const id of l.generations.keys()) forget(id);
      if (entriesRef.current.size > 0) setEntries(new Map());
      return;
    }
    const present = new Set(devices.map(device => device.id));
    const gone = [...entriesRef.current.keys()].filter(id => !present.has(id));
    if (gone.length > 0) {
      gone.forEach(forget);
      setEntries(previous => { const next = new Map(previous); gone.forEach(id => next.delete(id)); return next; });
    }
    // The group-address views read every device's comparison up front, but only
    // when the opt-in preference is on: unlike the matrix's opt-in object lists,
    // the marks need no per-device button.
    if (!enabled) return;
    for (const device of devices) {
      const entry = entriesRef.current.get(device.id);
      if (entry === undefined || (entry.status !== "failed" && entry.source !== source)) load(device.id);
    }
  }, [source, language, projectKey, devices, load, enabled]);

  const current = useMemo(() => {
    const map = new Map<number, ObjectLastDownload | undefined>();
    for (const device of devices) {
      const entry = entries.get(device.id);
      if (entry?.status === "loaded" && entry.source === source) map.set(device.id, entry.lastDownload);
    }
    return map;
  }, [devices, entries, source]);

  return { entries, source, current, enabled, load };
}
