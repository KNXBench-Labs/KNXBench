/** Cross-window records of project and bus-session context, and the stale-lock they decide. */
// apps/knx-web/src/busContext.ts
//
// Why this module exists at all.
//
// `apps/knx-server/src/bus.rs:601-618` states it plainly: a
// `GroupAddressContext` "computes once, at start, from the project open in
// `AppState` at that moment", and is "never re-resolved mid-session". The
// snapshot is taken in `bus_routes.rs:120-123`, stored on the session
// (`bus.rs:1020`), and used for two separate things — decoding every
// incoming telegram (`drain_task`'s clone) and resolving the DPT of every
// outgoing write (`BusSession::resolve_write_dpt`, `bus.rs:1135-1141`).
//
// So the moment the project's group addresses, their names or their DPTs
// change after a session started, every decoded value and every
// DPT-resolved send in that session describes a project that no longer
// exists. The server does not notice, and it has no way to tell a client:
// there is no `WebSocket` and no `EventSource` anywhere in this frontend
// (`docs/KNOWN_LIMITATIONS.md` §63, point 3), and no `GET` route that
// returns the project tree at all — a `ProjectTree` only ever arrives as
// the *response* to a mutation. A second window therefore cannot ask the
// server what the project looks like now.
//
// What it can do is read what the editing window last saw. Both windows
// are same-origin (the desktop shell loads an ordinary `http://` URL —
// `apps/knx-desktop/src-tauri/src/lib.rs`), so they share one
// `localStorage`. The editing window publishes a fingerprint of the
// decode-relevant project state on every tree change; whoever starts a bus
// session records which fingerprint was current when it started. Comparing
// the two is the whole mechanism.
//
// This is a lock, not a sync: nothing here repairs a stale session, and
// nothing here silently carries on. A mismatch disables sending and says
// so, because a companion window quietly showing values decoded against a
// project that has since moved is worse than one showing nothing.

import type { ProjectTree } from "./bindings/ProjectTree";

/// What the editing window last published about the open project.
export const PROJECT_CONTEXT_KEY = "knx-desktop:project-context";
/// What the window that started the current bus session recorded about it.
export const SESSION_CONTEXT_KEY = "knx-desktop:bus-session-context";
/// Same-window notification: the `storage` event deliberately does not fire
/// in the window that wrote the value, so a window that has both the editor
/// and a monitor on screen would otherwise never hear its own publication.
export const CONTEXT_EVENT = "knx-desktop:context-changed";

export interface ProjectContextRecord {
  /// Fingerprint of everything `GroupAddressContext` freezes — see
  /// `fingerprintProjectContext`.
  fingerprint: string;
  /// `Date.now()` at publication. Used only to order a project record
  /// against a session record, never displayed.
  at: number;
}

export interface SessionContextRecord {
  sessionId: number;
  /// The project fingerprint current when this session started, or `null`
  /// when no project record existed then — which is not the same as "no
  /// project": a freshly reloaded window holds no tree yet even though the
  /// server may still have one open. `contextLock` treats the two cases
  /// differently on purpose.
  fingerprint: string | null;
  at: number;
}

/// Three-valued because two values would be a lie. `"synced"` means the
/// project state behind this session provably has not moved; `"stale"`
/// means it provably has; `"unverified"` means this client cannot tell —
/// it did not start the session, or the record it would compare against is
/// gone. `"unverified"` is shown, never silently folded into either of the
/// other two.
export type ContextLock = "synced" | "stale" | "unverified";

/// FNV-1a, 32-bit. Chosen because it is four lines, has no dependency and
/// needs no cryptographic property: this only ever answers "is this the
/// same string as last time", never "what was the original".
function fnv1a(text: string): string {
  let hash = 0x811c9dc5;
  for (let i = 0; i < text.length; i += 1) {
    hash ^= text.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193);
  }
  return (hash >>> 0).toString(16).padStart(8, "0");
}

/// Exactly the project facts a `GroupAddressContext` freezes, and nothing
/// else: for every group address, its formatted address (which carries the
/// project's `GroupAddressStyle` — `bus.rs`'s `format_destination`), its
/// name (`GroupAddressContext::name`) and its DPTs (`dpts`, produced by the
/// same `group_address_dpt_from` rule as `resolve_group_address_dpt` — see
/// `bindings/GroupAddressNode.ts`). Device names, parameters, buildings and
/// topology are deliberately absent: renaming a room cannot change how a
/// telegram decodes, and a fingerprint that moved on such an edit would
/// lock a session for no reason and train users to ignore the lock.
///
/// `null` — no project open in this window — fingerprints as `"none"` so
/// the value is always a string; whether that string is ever *published* is
/// `publishProjectContext`'s decision, not this function's.
export function fingerprintProjectContext(tree: ProjectTree | null): string {
  if (tree === null) return "none";
  const parts: string[] = [`v${tree.schema_version}`];
  let count = 0;
  for (const installation of tree.installations) {
    for (const address of installation.group_addresses) {
      count += 1;
      parts.push(`${address.address}${address.name}${address.dpts.join(",")}`);
    }
  }
  return `${count}-${fnv1a(parts.join(""))}`;
}

function readRecord<T>(storage: Pick<Storage, "getItem">, key: string): T | null {
  let raw: string | null = null;
  try {
    raw = storage.getItem(key);
  } catch {
    return null; // Storage can be unavailable (private mode, blocked origin).
  }
  if (raw === null) return null;
  try {
    return JSON.parse(raw) as T;
  } catch {
    // A hand-edited or half-written record is treated as absent rather than
    // trusted: "cannot tell" is a state this module already models.
    return null;
  }
}

function writeRecord(storage: Pick<Storage, "setItem">, key: string, value: unknown): void {
  try {
    storage.setItem(key, JSON.stringify(value));
  } catch {
    /* Storage can be unavailable; the lock degrades to "unverified". */
  }
}

export function readProjectContext(storage: Pick<Storage, "getItem">): ProjectContextRecord | null {
  const record = readRecord<ProjectContextRecord>(storage, PROJECT_CONTEXT_KEY);
  return record && typeof record.fingerprint === "string" && typeof record.at === "number"
    ? record
    : null;
}

export function readSessionContext(storage: Pick<Storage, "getItem">): SessionContextRecord | null {
  const record = readRecord<SessionContextRecord>(storage, SESSION_CONTEXT_KEY);
  return record &&
    typeof record.sessionId === "number" &&
    (record.fingerprint === null || typeof record.fingerprint === "string") &&
    typeof record.at === "number"
    ? record
    : null;
}

export function writeProjectContext(storage: Pick<Storage, "setItem">, tree: ProjectTree): void {
  writeRecord(storage, PROJECT_CONTEXT_KEY, {
    fingerprint: fingerprintProjectContext(tree),
    at: Date.now(),
  } satisfies ProjectContextRecord);
}

export function writeSessionContext(storage: Pick<Storage, "getItem" | "setItem">, sessionId: number): void {
  writeRecord(storage, SESSION_CONTEXT_KEY, {
    sessionId,
    fingerprint: readProjectContext(storage)?.fingerprint ?? null,
    at: Date.now(),
  } satisfies SessionContextRecord);
}

export function clearSessionContext(storage: Pick<Storage, "removeItem">): void {
  try {
    storage.removeItem(SESSION_CONTEXT_KEY);
  } catch {
    /* See `writeRecord`. */
  }
}

/// The whole decision, as a pure function of three inputs, so it can be
/// tested without a DOM, a session or a server.
///
/// `activeSessionId` is the session the caller is *currently attached to*
/// (`null`: none — there is nothing to be stale about yet).
export function contextLock(
  project: ProjectContextRecord | null,
  session: SessionContextRecord | null,
  activeSessionId: number | null,
): ContextLock {
  if (activeSessionId === null) return "synced";
  // Either no window recorded a session start, or the recorded one is not
  // this one — somebody else's browser profile started it, or it was
  // started before this build shipped. Nothing to compare against.
  if (session === null || session.sessionId !== activeSessionId) return "unverified";
  if (session.fingerprint === null) {
    // No project record existed when the session started. If none exists
    // now either, nothing has moved. If one appeared since, a project was
    // opened or edited under a session that froze a different (possibly
    // empty) context — that is exactly the case this lock is for.
    if (project === null) return "synced";
    return project.at > session.at ? "stale" : "unverified";
  }
  if (project === null) return "unverified";
  return project.fingerprint === session.fingerprint ? "synced" : "stale";
}

function notifyContextChanged(): void {
  if (typeof window === "undefined") return;
  window.dispatchEvent(new Event(CONTEXT_EVENT));
}

/// Publishes the current project fingerprint. Called from `App.tsx` on
/// every tree change — and deliberately never with `null`: a window that
/// has just reloaded holds no tree, while the server it talks to may still
/// have the same project open as before, so publishing `"none"` there would
/// invent a project change that never happened and lock a perfectly valid
/// session.
export function publishProjectContext(tree: ProjectTree): void {
  if (typeof window === "undefined") return;
  writeProjectContext(window.localStorage, tree);
  notifyContextChanged();
}

/// Records which project fingerprint was current when `sessionId` started.
/// Called by whichever window pressed Connect; the other window reads it.
export function recordSessionContext(sessionId: number): void {
  if (typeof window === "undefined") return;
  writeSessionContext(window.localStorage, sessionId);
  notifyContextChanged();
}

/// Called after a deliberate disconnect. The session is gone, so the record
/// describing it is too — leaving it behind would make the *next* session
/// briefly look verified against a predecessor's fingerprint.
export function forgetSessionContext(): void {
  if (typeof window === "undefined") return;
  clearSessionContext(window.localStorage);
  notifyContextChanged();
}

/// The lock for `activeSessionId`, read from this window's `localStorage`.
export function readContextLock(activeSessionId: number | null): ContextLock {
  if (typeof window === "undefined") return "unverified";
  return contextLock(
    readProjectContext(window.localStorage),
    readSessionContext(window.localStorage),
    activeSessionId,
  );
}

/// Whether any window has published a project. This is the only signal a
/// companion window has for "a project is open" — it has no tree of its own
/// and, as the module comment says, no route to ask for one.
export function projectContextKnown(): boolean {
  if (typeof window === "undefined") return false;
  const record = readProjectContext(window.localStorage);
  return record !== null && record.fingerprint !== "none";
}

/// Fires on both cross-window (`storage`) and same-window (`CONTEXT_EVENT`)
/// changes to either record. Cross-window delivery depends on the platform
/// actually dispatching `storage` between windows; nothing here *relies* on
/// it, because every consumer re-reads the lock on its own poll tick as
/// well (see `BusMonitorPanel.tsx`).
export function subscribeContextChanges(listener: () => void): () => void {
  if (typeof window === "undefined") return () => {};
  function onStorage(event: StorageEvent) {
    if (event.key === null || event.key === PROJECT_CONTEXT_KEY || event.key === SESSION_CONTEXT_KEY) {
      listener();
    }
  }
  window.addEventListener("storage", onStorage);
  window.addEventListener(CONTEXT_EVENT, listener);
  return () => {
    window.removeEventListener("storage", onStorage);
    window.removeEventListener(CONTEXT_EVENT, listener);
  };
}
