/** Explicit per-device matrix object inventory via canonical device-detail reads. */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import * as api from "./api";
import type { ComObjectNode } from "./bindings/ComObjectNode";
import type { DeviceNode } from "./bindings/DeviceNode";
import type { InstallationNode } from "./bindings/InstallationNode";
import { matrixColumnKey, type DeviceObjectInventory } from "./groupAddressMatrix";
import { useProductLanguage } from "./productLanguage";
import { collectInstallationDevices } from "./treeUtils";

/**
 * `source` is the project snapshot (plus product language) a read belongs to.
 * A loaded entry from another source is outdated: it is re-read, never shown.
 */
export type ObjectInventoryEntry =
  | { status: "loading"; source: object; previous: ComObjectNode[] | null }
  | { status: "loaded"; source: object; objects: ComObjectNode[] }
  | { status: "failed"; source: object; message: string | null; mismatch?: boolean };
export type ObjectInventoryState = "not-loaded" | "loading" | "loaded" | "failed";
export interface ObjectInventorySummary { total: number; loaded: number; loading: number; failed: number; notLoaded: number }

export function objectInventoryState(entry: ObjectInventoryEntry | undefined, source: object): ObjectInventoryState {
  if (!entry) return "not-loaded";
  if (entry.status === "failed") return "failed";
  return entry.status === "loading" || entry.source !== source ? "loading" : "loaded";
}

/** Devices with objects to load; a device without objects is not "missing". */
export const loadableDevices = (devices: readonly DeviceNode[]) => devices.filter(device => device.com_object_count > 0);

export function currentObjectInventory(
  devices: readonly DeviceNode[], entries: ReadonlyMap<number, ObjectInventoryEntry>, source: object,
): Map<number, DeviceObjectInventory> {
  const inventory = new Map<number, DeviceObjectInventory>();
  for (const device of devices) {
    const entry = entries.get(device.id);
    if (entry?.status === "loaded" && entry.source === source) inventory.set(device.id, { device, objects: entry.objects });
  }
  return inventory;
}

export function summarizeObjectInventory(
  devices: readonly DeviceNode[], entries: ReadonlyMap<number, ObjectInventoryEntry>, source: object,
): ObjectInventorySummary {
  const summary: ObjectInventorySummary = { total: 0, loaded: 0, loading: 0, failed: 0, notLoaded: 0 };
  for (const device of loadableDevices(devices)) {
    summary.total++;
    const state = objectInventoryState(entries.get(device.id), source);
    if (state === "not-loaded") summary.notLoaded++; else if (state === "failed") summary.failed++;
    else if (state === "loading") summary.loading++; else summary.loaded++;
  }
  return summary;
}

/** Column keys that are only temporarily absent while their device is re-read. */
export function pendingInventoryColumnKeys(entries: ReadonlyMap<number, ObjectInventoryEntry>, source: object): Set<string> {
  const keys = new Set<string>();
  for (const [deviceId, entry] of entries) {
    const objects = entry.status === "loading" ? entry.previous : entry.status === "loaded" && entry.source !== source ? entry.objects : null;
    for (const object of objects ?? []) keys.add(matrixColumnKey(deviceId, object.id));
  }
  return keys;
}

export interface GroupAddressObjectInventory {
  devices: DeviceNode[];
  entries: ReadonlyMap<number, ObjectInventoryEntry>;
  source: object;
  current: Map<number, DeviceObjectInventory>;
  summary: ObjectInventorySummary;
  pendingKeys: Set<string>;
  load: (deviceId: number) => void;
}

/**
 * Reads only on explicit request. Replies are applied only for the request's
 * own generation, snapshot and project; loaded devices are re-read after a
 * project update or language change, and a project switch forgets everything.
 */
export function useGroupAddressObjectInventory(installation: InstallationNode, projectKey: string): GroupAddressObjectInventory {
  const devices = useMemo(() => [...collectInstallationDevices(installation).values()], [installation]);
  return useDeviceObjectInventory(devices, installation, projectKey);
}

/**
 * The same explicit, snapshot-bound reads for any device list — the
 * group-address member picker uses it for the devices the link rule admits.
 * `snapshot` is the project data the list was derived from: a new snapshot
 * makes loaded entries outdated, so they are re-read rather than shown.
 */
export function useDeviceObjectInventory(devices: DeviceNode[], snapshot: object, projectKey: string): GroupAddressObjectInventory {
  const [language] = useProductLanguage();
  // A fresh identity per snapshot, language and project: the staleness token.
  const source = useMemo(() => ({}), [snapshot, language, projectKey]);
  const [entries, setEntries] = useState<ReadonlyMap<number, ObjectInventoryEntry>>(() => new Map());
  const entriesRef = useRef(entries); entriesRef.current = entries;
  const live = useRef({ source, language, projectKey, mounted: true, generations: new Map<number, number>() });

  const load = useCallback((deviceId: number) => {
    const l = live.current;
    const generation = (l.generations.get(deviceId) ?? 0) + 1;
    l.generations.set(deviceId, generation);
    const requestSource = l.source;
    setEntries(previous => {
      const old = previous.get(deviceId);
      const kept = old?.status === "loaded" ? old.objects : old?.status === "loading" ? old.previous : null;
      return new Map(previous).set(deviceId, { status: "loading", source: requestSource, previous: kept });
    });
    const settle = (entry: ObjectInventoryEntry) => {
      if (!l.mounted || l.generations.get(deviceId) !== generation || l.source !== requestSource) return;
      setEntries(previous => new Map(previous).set(deviceId, entry));
    };
    api.deviceDetail(deviceId, l.language).then(
      detail => settle(detail.id === deviceId
        ? { status: "loaded", source: requestSource, objects: detail.com_objects }
        : { status: "failed", source: requestSource, message: null, mismatch: true }),
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
    if (switched) {
      for (const id of l.generations.keys()) forget(id);
      setEntries(new Map());
      return;
    }
    const present = new Set(devices.map(device => device.id));
    const gone = [...entriesRef.current.keys()].filter(id => !present.has(id));
    if (gone.length > 0) {
      gone.forEach(forget);
      setEntries(previous => { const next = new Map(previous); gone.forEach(id => next.delete(id)); return next; });
    }
    for (const [id, entry] of entriesRef.current) {
      if (present.has(id) && entry.status !== "failed" && entry.source !== source) load(id);
    }
  }, [source, language, projectKey, devices, load]);

  const current = useMemo(() => currentObjectInventory(devices, entries, source), [devices, entries, source]);
  const summary = useMemo(() => summarizeObjectInventory(devices, entries, source), [devices, entries, source]);
  const pendingKeys = useMemo(() => pendingInventoryColumnKeys(entries, source), [entries, source]);
  return { devices, entries, source, current, summary, pendingKeys, load };
}
