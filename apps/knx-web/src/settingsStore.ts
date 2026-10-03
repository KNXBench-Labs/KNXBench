/** The client half of the versioned settings file: a synchronous cache over the server's record. */
// Every preference the application remembers lives in one JSON document
// in the server's data directory (`apps/knx-server/src/settings.rs`).
// This module is the only thing in the frontend that talks to it.
//
// **The server's file is the record; what follows is a cache.** The whole
// document is mirrored into one `localStorage` key
// (`SETTINGS_CACHE_KEY`), for one reason: `index.html`'s pre-mount
// bootstrap and the first React render both need a theme *now*, and a
// network round trip is not now. The cache is therefore deliberately
// shaped so it cannot be mistaken for the record — one opaque key holding
// a whole document, not the eight hand-editable `knx-desktop:` keys this
// replaced. `initSettings()` hydrates it from the server while preserving
// explicit edits made after that authoritative read started.
//
// Three `knx-desktop:` keys stay in `localStorage` and are none of this
// module's business: `busContext.ts`'s `project-context`,
// `bus-session-context` and `context-changed` are per-window session
// state, not preferences. Moving them here would make two windows fight
// over one record.
//
// Mount-time preference effects run before the parent bootstrap effect and
// only restate cache-derived defaults. They update the cache but do not
// outrank the server. Writes after `initSettings()` starts are journalled
// per key and flushed once hydration completes.

import { useSyncExternalStore } from "react";
import { canonicalJson } from "./canonicalJson";

/** The one `localStorage` key this module owns: a cache of the document. */
export const SETTINGS_CACHE_KEY = "knx-desktop:settings-cache";

/**
 * Set once the browser's old per-key preferences have been handed to the
 * server. Belt and braces with the server's own "adopt only when there is
 * no file" rule: that rule already stops a second browser overwriting the
 * record, and this stops the *same* browser trying again every load.
 */
export const SETTINGS_ADOPTED_KEY = "knx-desktop:settings-adopted";

/**
 * Where each preference used to live, before there was a file. Read
 * exactly once, by `adoptionPayload()`, and deleted once the server has
 * them — a user who set a theme last week must not be reset to defaults
 * by this change, and must not be left with two copies of it either.
 *
 * The value side is the document key; the key side is the `localStorage`
 * key a build before this one wrote. Adding a preference does *not* mean
 * adding an entry here: this table is a closed historical record of the
 * browser era, not a registry of preferences.
 */
const BROWSER_ERA_KEYS: Readonly<Record<string, string>> = {
  "knx-desktop:theme": "theme",
  "knx-desktop:accent": "accent",
  "knx-desktop:density": "density",
  "knx-desktop:motion-level": "motionLevel",
  "knx-desktop:motion-style": "motionStyle",
  "knx-desktop:ui-language": "uiLanguage",
  "knx-desktop:ui-language-packs": "uiLanguagePacks",
  "knx-desktop:product-language": "productLanguage",
};

/** The one preference the browser era stored as a JSON string rather than
 * a plain value: the installed language packs. It becomes a real JSON
 * object in the document, so `settings.json` stays readable instead of
 * carrying an escaped blob. */
const BROWSER_ERA_JSON_KEYS = new Set(["uiLanguagePacks"]);

/** The schema version the browser era's preferences are handed over as —
 * `BROWSER_ERA_SCHEMA_VERSION` in `apps/knx-server/src/settings.rs`. The
 * server's migration chain normalizes them from there. */
const BROWSER_ERA_SCHEMA_VERSION = 0;

/** What `GET /api/settings` answers with. Mirrors `SettingsDto` in
 * `apps/knx-server/src/settings_routes.rs`. */
export interface SettingsResponse {
  conditionalPatchVersion?: number;
  schemaVersion: number;
  settings: Record<string, unknown>;
  status: "ok" | "absent" | "migrated" | "refusedNewer" | "quarantined";
  fileSchemaVersion?: number;
  movedTo?: string;
  diagnostic?: SettingsDiagnostic;
  message?: string;
}

export type SettingsQuarantineReason =
  | "unreadable"
  | "invalidJson"
  | "notObject"
  | "missingSchemaVersion"
  | "settingsNotObject";

export type SettingsDiagnostic =
  | { kind: "migrated"; fromVersion: number; toVersion: number }
  | { kind: "adopted"; fromVersion: number; toVersion: number }
  | { kind: "refusedNewer"; fileVersion: number; currentVersion: number }
  | { kind: "quarantined"; reason: SettingsQuarantineReason; movedTo: string };

export type SettingsHydrationState = "cached" | "hydrated" | "failed";

export interface SettingsState {
  hydration: SettingsHydrationState;
  diagnostic: SettingsDiagnostic | undefined;
  fallbackMessage: string | undefined;
}

interface CachedDocument {
  schemaVersion: number;
  settings: Record<string, unknown>;
}

let document_: CachedDocument | undefined;
/** Last server observation, not another store: optimistic/cache values are not acknowledgment. */
let acknowledged_: SettingsResponse | undefined;
let acknowledgmentUncertain = false;
let settingsEpoch = 0;
let synchronized = false;
let pendingBeforeHydration = new Map<string, unknown>();
let revision = 0;
const subscribers = new Set<() => void>();
const stateSubscribers = new Set<() => void>();
let state_: SettingsState = {
  hydration: "cached",
  diagnostic: undefined,
  fallbackMessage: undefined,
};

/** Writes are serialized through one promise chain, so two preferences
 * changed in the same tick reach the server in the order they were made
 * rather than racing each other into the same read-modify-write. */
let queue: Promise<void> = Promise.resolve();
let writeGeneration = 0;
const unpersisted = new Map<string, { value: unknown; generation: number }>();

function readCache(): CachedDocument {
  try {
    const raw = window.localStorage.getItem(SETTINGS_CACHE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as unknown;
      if (typeof parsed === "object" && parsed !== null && !Array.isArray(parsed)) {
        const settings = (parsed as CachedDocument).settings;
        if (typeof settings === "object" && settings !== null && !Array.isArray(settings)) {
          return { schemaVersion: (parsed as CachedDocument).schemaVersion ?? 0, settings };
        }
      }
    }
  } catch {
    // A damaged or unavailable cache is just a cache miss: the record is
    // on the server, and the next `initSettings()` refills this.
  }
  return { schemaVersion: 0, settings: {} };
}

function cache(): CachedDocument {
  if (document_ === undefined) document_ = readCache();
  return document_;
}

function writeCache(): boolean {
  try {
    window.localStorage.setItem(SETTINGS_CACHE_KEY, JSON.stringify(cache()));
    return true;
  } catch {
    // Private browsing, a full quota, a browser with storage switched off.
    // The session still works; only the next first paint is slower to
    // find the right theme.
    return false;
  }
}

function notify(): void {
  revision += 1;
  for (const onStoreChange of subscribers) onStoreChange();
}

/** Reads one preference as parsed JSON, or `undefined` when unset. */
export function getSetting(key: string): unknown {
  return cache().settings[key];
}

export type SettingsAcknowledgementReason = "notHydrated" | "incompatibleSettings" | "unsupportedServer" | "pendingWrite" | "unsupportedValue" | "uncertain";
export type SettingsAcknowledgedSnapshot = { ok: true; settings: Record<string, unknown> }
  | { ok: false; reason: SettingsAcknowledgementReason };
export class SettingsMutationError extends Error {
  constructor(readonly kind: SettingsAcknowledgementReason | "conflict" | "rejected" | "ambiguous" | "stale") { super(kind); }
}
function usableSettingsDocument(response: unknown): response is SettingsResponse {
  if (typeof response !== "object" || response === null || Array.isArray(response)) return false;
  const value = response as SettingsResponse;
  return value.schemaVersion === 1
    && ["ok", "absent", "migrated"].includes(value.status)
    && typeof value.settings === "object" && value.settings !== null && !Array.isArray(value.settings);
}
function usableAcknowledgment(response: unknown): response is SettingsResponse {
  return usableSettingsDocument(response) && response.conditionalPatchVersion === 1;
}
function adoptAcknowledgment(response: SettingsResponse): { cacheError: boolean } {
  acknowledged_ = JSON.parse(JSON.stringify(response)) as SettingsResponse;
  acknowledgmentUncertain = false;
  const settings = { ...response.settings };
  for (const [key, { value }] of unpersisted) {
    if (value === undefined || value === null) delete settings[key];
    else settings[key] = value;
  }
  document_ = { schemaVersion: response.schemaVersion, settings };
  const cacheError = !writeCache();
  notify();
  return { cacheError };
}
export async function patchAcknowledgedSettings(patchInput: Record<string, unknown>, expectedInput: Record<string, unknown>): Promise<{ cacheError: boolean }> {
  if (Object.keys(patchInput).some((key) => !Object.hasOwn(expectedInput, key) || patchInput[key] === undefined)) {
    throw new SettingsMutationError("unsupportedValue");
  }
  const patch = JSON.parse(JSON.stringify(patchInput)) as Record<string, unknown>;
  const expected = JSON.parse(JSON.stringify(expectedInput)) as Record<string, unknown>;
  const epoch = settingsEpoch;
  const operation = queue.then(async () => {
    if (settingsEpoch !== epoch) throw new SettingsMutationError("stale");
    const snapshot = getAcknowledgedSettings(Object.keys(expected));
    if (!snapshot.ok) throw new SettingsMutationError(snapshot.reason);
    if (canonicalJson(snapshot.settings) !== canonicalJson(expected)) throw new SettingsMutationError("conflict");
    let response: SettingsResponse;
    try {
      response = await requestJson<SettingsResponse>("/api/settings", {
        method: "PUT", body: JSON.stringify({ settings: patch, expectedSettings: expected }),
      });
      if (settingsEpoch !== epoch) throw new SettingsMutationError("stale");
      if (!usableAcknowledgment(response) || Object.keys(expected).some((key) => {
        const wanted = Object.hasOwn(patch, key) ? patch[key] : expected[key];
        return wanted === null ? Object.hasOwn(response.settings, key)
          : !Object.hasOwn(response.settings, key) || canonicalJson(response.settings[key]) !== canonicalJson(wanted);
      })) throw new SettingsMutationError("ambiguous");
    } catch (error) {
      if (settingsEpoch !== epoch) throw new SettingsMutationError("stale");
      if (error instanceof SettingsHttpError && error.status === 409) throw new SettingsMutationError("conflict");
      if (error instanceof SettingsHttpError && error.status >= 400 && error.status < 500) throw new SettingsMutationError("rejected");
      acknowledgmentUncertain = true;
      notify();
      try {
        const reread = await requestJson<SettingsResponse>("/api/settings");
        if (settingsEpoch !== epoch) throw new SettingsMutationError("stale");
        if (usableAcknowledgment(reread)) adoptAcknowledgment(reread);
      } catch { /* Keep last acknowledgment and require a fresh authoritative observation. */ }
      if (settingsEpoch !== epoch) throw new SettingsMutationError("stale");
      throw new SettingsMutationError("ambiguous");
    }
    return adoptAcknowledgment(response);
  });
  queue = operation.then(() => undefined, () => undefined);
  return operation;
}
export function getAcknowledgedSettings(keys: readonly string[]): SettingsAcknowledgedSnapshot {
  if (state_.hydration !== "hydrated" || !acknowledged_) return { ok: false, reason: "notHydrated" };
  if (acknowledgmentUncertain) return { ok: false, reason: "uncertain" };
  if (acknowledged_.schemaVersion !== 1 || !["ok", "absent", "migrated"].includes(acknowledged_.status)
      || typeof acknowledged_.settings !== "object" || acknowledged_.settings === null || Array.isArray(acknowledged_.settings)) {
    return { ok: false, reason: "incompatibleSettings" };
  }
  if (acknowledged_.conditionalPatchVersion !== 1) return { ok: false, reason: "unsupportedServer" };
  if (keys.some((key) => unpersisted.has(key) || pendingBeforeHydration.has(key))) return { ok: false, reason: "pendingWrite" };
  // The conditional wire contract uses null for absence. Do not conflate an
  // explicit damaged stored null with a key that can be expected absent.
  if (keys.some((key) => Object.hasOwn(acknowledged_!.settings, key) && acknowledged_!.settings[key] === null)) {
    return { ok: false, reason: "unsupportedValue" };
  }
  return { ok: true, settings: Object.fromEntries(keys.map((key) => [key,
    Object.hasOwn(acknowledged_!.settings, key) ? JSON.parse(JSON.stringify(acknowledged_!.settings[key])) as unknown : null])) };
}

/**
 * Writes one preference and, once `initSettings()` has run, sends it to
 * the server as a one-key patch. An unchanged value writes nothing at all
 * — which is what keeps the hooks below from re-sending every preference
 * to the server on every mount, since each of them persists its current
 * value in the effect that applies it.
 */
export function setSetting(key: string, value: unknown): void {
  write(key, value, false);
}

/** Safety-sensitive booleans use the server record, never the optimistic cache. */
export async function readPersistedBooleanSetting(key: string): Promise<boolean> {
  await initSettings();
  if (state_.hydration !== "hydrated") throw new Error("settings are not available from the server");
  await queue;
  const response = await requestJson<SettingsResponse>("/api/settings");
  if (response.status === "refusedNewer" || response.status === "quarantined") {
    throw new Error("settings file cannot be used for a debug action");
  }
  return response.settings?.[key] === true;
}

/** Patch and read back a safety setting before reflecting it in the cache.
 * Keep the write in the store's serialization queue alongside ordinary
 * preferences; a rejected or contradictory answer leaves the UI disabled. */
export async function setPersistedBooleanSetting(key: string, value: boolean): Promise<void> {
  await initSettings();
  if (state_.hydration !== "hydrated") throw new Error("settings are not available from the server");
  const operation = queue.then(async () => {
    const updated = await requestJson<SettingsResponse>("/api/settings", {
      method: "PUT",
      body: JSON.stringify({ settings: { [key]: value } }),
    });
    if (updated.status === "refusedNewer" || updated.status === "quarantined" ||
        updated.settings?.[key] !== value) throw new Error("settings write was not confirmed");
    const readBack = await requestJson<SettingsResponse>("/api/settings");
    if (readBack.status === "refusedNewer" || readBack.status === "quarantined" ||
        readBack.settings?.[key] !== value) {
      throw new Error("settings readback did not confirm the debug preference");
    }
    cache().settings[key] = value;
    writeCache();
    notify();
  });
  queue = operation.then(() => undefined, () => undefined);
  return operation;
}

/**
 * [`setSetting`], but a failed cache write is rolled back and rethrown
 * instead of shrugged off.
 *
 * One caller: `languagePack.ts`, the only store here that accepts
 * arbitrary user-supplied JSON of unbounded size and therefore the only
 * one whose write can realistically hit a storage quota. It treats its
 * in-memory mutation and its persist as one transaction, and a persist
 * that quietly fails would let a rejected import ride into storage on the
 * next successful one.
 */
export function setSettingOrThrow(key: string, value: unknown): void {
  write(key, value, true);
}

function write(key: string, value: unknown, strict: boolean): void {
  const settings = cache().settings;
  if (JSON.stringify(settings[key]) === JSON.stringify(value)) return;
  const had = Object.prototype.hasOwnProperty.call(settings, key);
  const previous = settings[key];
  if (value === undefined || value === null) delete settings[key];
  else settings[key] = value;

  if (strict) {
    try {
      window.localStorage.setItem(SETTINGS_CACHE_KEY, JSON.stringify(cache()));
    } catch (error) {
      if (had) settings[key] = previous;
      else delete settings[key];
      throw error;
    }
  } else {
    writeCache();
  }

  notify();
  // Hooks persist their cache-derived defaults during their first effect,
  // before the parent bootstrap effect starts. Those are initialization,
  // not user edits, and must not outrank the server record. Once bootstrap
  // has started, however, any write happened while the authoritative read
  // was in flight and must survive it.
  if (!synchronized && started !== undefined) {
    pendingBeforeHydration.set(key, value === undefined ? null : value);
  }
  // `null`, not "absent": the server reads a null in a patch as "remove
  // this key", which is the only way to unset a preference remotely.
  push({ [key]: value === undefined ? null : value });
}

/**
 * A `Storage`-shaped view of the record, for the preference modules that
 * were written against `localStorage` and validate their own values
 * (`theme.ts`, `appearance.ts`, `motion.ts`, `uiLanguage.ts`,
 * `productLanguage.ts`). Their key constants name document keys now, not
 * `localStorage` keys; everything else about them is unchanged, including
 * that a value they do not recognise falls back to a default rather than
 * throwing.
 *
 * `getItem` returns `null` for a preference whose stored value is not a
 * string — the honest answer for a caller whose whole vocabulary is
 * `string | null`, and one its "unknown value means the default" rule
 * already handles.
 */
export const settingsStorage: Pick<Storage, "getItem" | "setItem" | "removeItem"> = {
  getItem(key: string): string | null {
    const value = getSetting(key);
    return typeof value === "string" ? value : null;
  },
  setItem(key: string, value: string): void {
    setSetting(key, value);
  },
  removeItem(key: string): void {
    setSetting(key, null);
  },
};

/** Subscribes to record changes — the server's answer landing, or another
 * component writing a preference. Paired with `settingsRevision()` for
 * `useSyncExternalStore`. */
export function subscribeToSettings(onStoreChange: () => void): () => void {
  subscribers.add(onStoreChange);
  return () => subscribers.delete(onStoreChange);
}

/** A counter that changes whenever the record does. A number rather than
 * the document itself, because `useSyncExternalStore` compares snapshots
 * with `Object.is` and a fresh object every call would loop it. */
export function settingsRevision(): number {
  return revision;
}

/**
 * Re-renders a component whenever the record changes. What the hooks
 * built on `useState` (`useThemeId`, `useAppearance`, `useMotion`) use to
 * notice the server's answer arriving after they have already mounted —
 * which is the normal case, since `initSettings()` is a round trip and
 * the first paint is not going to wait for it.
 */
export function useSettingsRevision(): number {
  return useSyncExternalStore(subscribeToSettings, settingsRevision, settingsRevision);
}

export function getSettingsState(): SettingsState {
  return state_;
}

export function subscribeToSettingsState(onStoreChange: () => void): () => void {
  stateSubscribers.add(onStoreChange);
  return () => stateSubscribers.delete(onStoreChange);
}

export function useSettingsState(): SettingsState {
  return useSyncExternalStore(subscribeToSettingsState, getSettingsState, getSettingsState);
}

function setSettingsState(next: SettingsState): void {
  state_ = next;
  for (const onStoreChange of stateSubscribers) onStoreChange();
}

/** Clears the in-memory document and the cache, and disarms server
 * synchronization again. Tests need this for the same reason every other
 * module-level store here has a reset: clearing `localStorage` does not
 * un-seed a module that has already read it. */
export function resetSettingsForTests(): void {
  document_ = undefined;
  acknowledged_ = undefined;
  acknowledgmentUncertain = false;
  settingsEpoch += 1;
  synchronized = false;
  pendingBeforeHydration = new Map();
  state_ = { hydration: "cached", diagnostic: undefined, fallbackMessage: undefined };
  started = undefined;
  queue = Promise.resolve();
  writeGeneration += 1;
  unpersisted.clear();
  try {
    window.localStorage.removeItem(SETTINGS_CACHE_KEY);
    window.localStorage.removeItem(SETTINGS_ADOPTED_KEY);
  } catch {
    // Nothing to clear if storage is unavailable.
  }
  notify();
}

class SettingsHttpError extends Error {
  constructor(readonly status: number, message: string) { super(message); }
}
async function requestJson<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    headers: init?.body ? { "Content-Type": "application/json" } : undefined,
    ...init,
  });
  if (!response.ok) {
    const body = (await response.json().catch(() => null)) as { error?: string } | null;
    throw new SettingsHttpError(response.status, body?.error ?? `${response.status} ${response.statusText}`);
  }
  return (await response.json()) as T;
}

function push(patch: Record<string, unknown>): void {
  if (!synchronized) return;
  const epoch = settingsEpoch;
  const generation = ++writeGeneration;
  for (const [key, value] of Object.entries(patch)) unpersisted.set(key, { value, generation });
  queue = queue
    .then(() => settingsEpoch === epoch ? requestJson<SettingsResponse>("/api/settings", {
      method: "PUT",
      body: JSON.stringify({ settings: patch }),
    }) : undefined)
    .then((response) => {
      if (settingsEpoch !== epoch) return;
      // A new server observation also revokes stale capability claims.
      acknowledged_ = JSON.parse(JSON.stringify(response)) as SettingsResponse;
      // Ordinary preferences still work with older servers, but only an
      // actual matching acknowledgment clears their outstanding intent.
      const confirmed = usableSettingsDocument(response) && Object.entries(patch).every(([key, value]) =>
        value === null ? !Object.hasOwn(response.settings, key)
          : Object.hasOwn(response.settings, key) && canonicalJson(response.settings[key]) === canonicalJson(value));
      if (confirmed) {
        acknowledgmentUncertain = false;
      }
      for (const key of Object.keys(patch)) {
        if (confirmed && unpersisted.get(key)?.generation === generation) unpersisted.delete(key);
      }
      if (confirmed) notify();
    })
    .catch((error: unknown) => {
      // A refused write (a settings file from a newer build) or an
      // unreachable server. The session keeps the change in memory; the
      // record does not. Saying so is better than pretending it saved.
      console.warn("KNXBench: a preference could not be saved to the settings file", error);
    });
}

/** Everything the browser era left behind, as a document the server can
 * adopt — or `undefined` when there is nothing to adopt. */
function adoptionPayload(): Record<string, unknown> | undefined {
  const settings: Record<string, unknown> = {};
  for (const [legacyKey, key] of Object.entries(BROWSER_ERA_KEYS)) {
    let raw: string | null = null;
    try {
      raw = window.localStorage.getItem(legacyKey);
    } catch {
      return undefined;
    }
    if (raw === null) continue;
    if (BROWSER_ERA_JSON_KEYS.has(key)) {
      try {
        settings[key] = JSON.parse(raw) as unknown;
      } catch {
        // A hand-edited or truncated pack store. Dropping it here is the
        // same verdict `languagePack.ts` already reaches on read ("no
        // packs installed"), and the original string stays in
        // `localStorage` because adoption only clears what it carried.
      }
      continue;
    }
    settings[key] = raw;
  }
  return Object.keys(settings).length > 0 ? settings : undefined;
}

function forgetBrowserEraKeys(adopted: Record<string, unknown>): void {
  for (const [legacyKey, key] of Object.entries(BROWSER_ERA_KEYS)) {
    if (!(key in adopted)) continue;
    try {
      window.localStorage.removeItem(legacyKey);
    } catch {
      // Leaving a stale copy behind is untidy, not lossy: nothing reads
      // these keys any more.
    }
  }
}

function apply(response: SettingsResponse): void {
  const settings = { ...response.settings };
  for (const [key, value] of pendingBeforeHydration) {
    if (value === null || value === undefined) delete settings[key];
    else settings[key] = value;
  }
  document_ = { schemaVersion: response.schemaVersion, settings };
  writeCache();
  notify();
}

/**
 * Loads the record from the server and makes it the truth for this
 * session, adopting the browser's old keys on the way if the server has
 * no file yet.
 *
 * Never throws and never blocks the application: a server that cannot be
 * reached (or refuses, because the session expired) leaves the cache in
 * place and the session running on it. Normally called once from
 * `main.tsx`, inside the authentication gate; an explicit safety-setting
 * re-check may retry only after a failed first load. Every `/api/` route
 * needs a session, this one included.
 */
export function initSettings(): Promise<void> {
  // A failed first read must not permanently lock a safety preference to
  // stale cache data. A later explicit re-check may retry hydration, still
  // sharing one in-flight promise with simultaneous callers.
  if (state_.hydration === "failed") {
    started = undefined;
    setSettingsState({ hydration: "cached", diagnostic: undefined, fallbackMessage: undefined });
  }
  started ??= loadFromServer();
  return started;
}

let started: Promise<void> | undefined;

/**
 * Re-read the same server record in every authenticated window, including
 * separate browser profiles and the native companion. No peer cache is
 * authoritative. Cleanup invalidates pending reads as well as the timer.
 */
export function startSettingsRefresh(): () => void {
  const interval = 5000;
  let active = true;
  let inFlight = false;
  let timer: number | undefined;

  function schedule() {
    if (!active) return;
    window.clearTimeout(timer);
    timer = window.setTimeout(() => { void refresh(); }, interval);
  }

  async function refresh() {
    if (!active || inFlight) return;
    window.clearTimeout(timer);
    inFlight = true;
    try {
      if (document.visibilityState === "hidden") return;
      if (!synchronized) {
        await initSettings();
        return;
      }
      const writes = queue;
      await writes;
      if (!active || queue !== writes) return;
      const generation = writeGeneration;
      const response = await requestJson<SettingsResponse>("/api/settings");
      if (!active || queue !== writes || writeGeneration !== generation) return;
      acknowledged_ = JSON.parse(JSON.stringify(response)) as SettingsResponse;
      if (usableAcknowledgment(response)) acknowledgmentUncertain = false;
      const settings = { ...response.settings };
      // A failed PUT stays a visible local edit, not a lost edit on the
      // next read. Successful newer patches clear only their own generation.
      for (const [key, { value }] of unpersisted) {
        if (value === null || value === undefined) delete settings[key];
        else settings[key] = value;
      }
      apply({ ...response, settings });
      setSettingsState({ hydration: "hydrated", diagnostic: response.diagnostic, fallbackMessage: response.message });
    } catch (error) {
      if (active) console.warn("KNXBench: settings refresh could not read the server record", error);
    } finally {
      inFlight = false;
      schedule();
    }
  }

  function onVisible() {
    if (document.visibilityState !== "hidden") void refresh();
  }
  window.addEventListener("focus", onVisible);
  document.addEventListener("visibilitychange", onVisible);
  void initSettings().then(schedule);
  return () => {
    active = false;
    window.clearTimeout(timer);
    window.removeEventListener("focus", onVisible);
    document.removeEventListener("visibilitychange", onVisible);
  };
}

async function loadFromServer(): Promise<void> {
  let response: SettingsResponse;
  try {
    response = await requestJson<SettingsResponse>("/api/settings");
  } catch (error) {
    console.warn("KNXBench: settings could not be read from the server", error);
    setSettingsState({ hydration: "failed", diagnostic: undefined, fallbackMessage: undefined });
    return;
  }

  if (response.status === "absent" && !hasAdopted()) {
    const payload = adoptionPayload();
    if (payload) {
      try {
        response = await requestJson<SettingsResponse>("/api/settings/adopt", {
          method: "POST",
          body: JSON.stringify({
            schemaVersion: BROWSER_ERA_SCHEMA_VERSION,
            settings: payload,
          }),
        });
        forgetBrowserEraKeys(payload);
        // The POST is the only thing that proves the handover happened.
        markAdopted();
      } catch (error) {
        // Most likely a 409: another window adopted first, and the record
        // now exists. Re-read rather than guess.
        console.warn("KNXBench: browser preferences could not be adopted", error);
        try {
          response = await requestJson<SettingsResponse>("/api/settings");
        } catch {
          setSettingsState({
            hydration: "failed",
            diagnostic: undefined,
            fallbackMessage: undefined,
          });
          return;
        }
        // A record exists, so somebody adopted; this browser has nothing
        // left to hand over. Still `absent` means the POST failed for its
        // own reasons — a dropped connection, a 500, an expired session —
        // and the preferences are still sitting in this browser. Leave the
        // flag unset so the next load tries again, rather than marking a
        // handover that never happened and losing the lot.
        if (response.status !== "absent") {
          forgetBrowserEraKeys(payload);
          markAdopted();
        }
      }
    }
  }

  if (response.message) {
    // Also in the session log, server-side, where the Log panel shows it.
    console.warn(`KNXBench: ${response.message}`);
  }
  acknowledged_ = JSON.parse(JSON.stringify(response)) as SettingsResponse;
  apply(response);
  synchronized = true;
  setSettingsState({
    hydration: "hydrated",
    diagnostic: response.diagnostic,
    fallbackMessage: response.message,
  });
  if (pendingBeforeHydration.size > 0) {
    const patch = Object.fromEntries(pendingBeforeHydration);
    push(patch);
    pendingBeforeHydration.clear();
  }
}

function hasAdopted(): boolean {
  try {
    return window.localStorage.getItem(SETTINGS_ADOPTED_KEY) !== null;
  } catch {
    return false;
  }
}

function markAdopted(): void {
  try {
    window.localStorage.setItem(SETTINGS_ADOPTED_KEY, new Date().toISOString());
  } catch {
    // The server's "adopt only into an empty file" rule is the real
    // guard; this key only saves a pointless request.
  }
}
