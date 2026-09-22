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

function writeCache(): void {
  try {
    window.localStorage.setItem(SETTINGS_CACHE_KEY, JSON.stringify(cache()));
  } catch {
    // Private browsing, a full quota, a browser with storage switched off.
    // The session still works; only the next first paint is slower to
    // find the right theme.
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
  synchronized = false;
  pendingBeforeHydration = new Map();
  state_ = { hydration: "cached", diagnostic: undefined, fallbackMessage: undefined };
  started = undefined;
  queue = Promise.resolve();
  try {
    window.localStorage.removeItem(SETTINGS_CACHE_KEY);
    window.localStorage.removeItem(SETTINGS_ADOPTED_KEY);
  } catch {
    // Nothing to clear if storage is unavailable.
  }
  notify();
}

async function requestJson<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    headers: init?.body ? { "Content-Type": "application/json" } : undefined,
    ...init,
  });
  if (!response.ok) {
    const body = (await response.json().catch(() => null)) as { error?: string } | null;
    throw new Error(body?.error ?? `${response.status} ${response.statusText}`);
  }
  return (await response.json()) as T;
}

function push(patch: Record<string, unknown>): void {
  if (!synchronized) return;
  queue = queue
    .then(() => requestJson<SettingsResponse>("/api/settings", {
      method: "PUT",
      body: JSON.stringify({ settings: patch }),
    }))
    .then(() => undefined)
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
 * place and the session running on it. It is called once, from
 * `main.tsx`, inside the authentication gate — every `/api/` route needs
 * a session, this one included.
 */
export function initSettings(): Promise<void> {
  // Once per page, even under `StrictMode`'s deliberate double-mount: the
  // second call would otherwise race the first through adoption, and one
  // of the two would lose to its own 409.
  started ??= loadFromServer();
  return started;
}

let started: Promise<void> | undefined;

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
