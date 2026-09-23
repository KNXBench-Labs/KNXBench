/** Cross-window records of project and bus-session context, and the stale-lock they decide. */
// apps/knx-web/src/busContext.ts
//
// Why this module exists at all.
//
// `apps/knx-server/src/bus.rs:601-618` states it plainly: a
// `GroupAddressContext` starts from the project open in `AppState` at that
// moment. The snapshot is stored on the session and used for two separate
// things — decoding every
// incoming telegram (`drain_task`'s clone) and resolving the DPT of every
// outgoing write. A confirmed group-address-style Set/Undo/Redo publication
// replaces that whole server snapshot in place; other project edits do not.
//
// So the moment the project's group addresses, their names or their DPTs
// change after a session started without one of those style publications,
// the running context is stale. There is no WebSocket/EventSource push; a
// window can fetch the current project with `GET /api/project`, but a second
// window is not automatically told to do so when another window edits it.
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
  /// Server-owned ordering. Absent legacy records are revision zero.
  snapshotRevision?: number;
  /// Opaque server-process lifetime that owns `snapshotRevision`. Missing
  /// means a legacy record and cannot supersede a modern one.
  serverIncarnation?: string;
  /// Process lifetimes this browser has already replaced. Keeping their
  /// opaque ids makes a delayed old response distinguishable from a third,
  /// genuinely new server lifetime.
  retiredServerIncarnations?: string[];
}

export interface SessionContextRecord {
  sessionId: number;
  /// Binds a reusable numeric id to the server process that issued it.
  /// Missing legacy records are deliberately never treated as verified.
  serverIncarnation?: string;
  /// Fingerprint from the session's last confirmed whole-context
  /// publication. Initially this is the matching project publication at
  /// connect time, or `null` when none existed; confirmed style
  /// Set/Undo/Redo responses may replace it later.
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
///
/// The width is the honest limit of this lock. Two unequal inputs collide
/// with probability 2^-32 per comparison, so `"synced"` is "almost
/// certainly unchanged", never "provably unchanged"; and FNV-1a is not
/// collision-resistant, so a *crafted* project could be made to collide
/// deliberately. Neither is defended against, because nothing here is a
/// security boundary — the lock is a decoding-staleness hint, and the
/// blast radius of a miss is one mislabelled telegram, not a bad write.
/// Documented in `KNOWN_LIMITATIONS.md` §82.
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
///
/// The two separators below are written as `` and `` escapes,
/// never as the literal bytes. They were literals once, and because
/// neither character renders, two successive reviewers read this line as a
/// bare concatenation and filed a collision — address `1/1/1` named
/// `0Foo` against address `1/1/10` named `Foo` — that the separators had
/// already prevented. Escapes cost nothing at runtime (identical strings,
/// identical fingerprints) and make the field boundaries visible to the
/// next reader. Do not "simplify" them back; `busContext.test.ts` pins
/// that exact pair.
///
/// What the separators do *not* survive is a name that itself contains
/// U+0001 or U+0002. No supported import can produce one — XML 1.0 forbids
/// both characters outright, so no `.knxproj` name can carry them — and no
/// keyboard types them, which is why the residual is documented
/// (`KNOWN_LIMITATIONS.md` §82) rather than defended against with a
/// length-prefixed encoding that would invalidate every stored
/// fingerprint.
export function fingerprintProjectContext(tree: ProjectTree | null): string {
  if (tree === null) return "none";
  const parts: string[] = [`v${tree.schema_version}`];
  let count = 0;
  for (const installation of tree.installations) {
    for (const address of installation.group_addresses) {
      count += 1;
      parts.push(`${address.address}\u0001${address.name}\u0001${address.dpts.join(",")}`);
    }
  }
  return `${count}-${fnv1a(parts.join("\u0002"))}`;
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
  return record &&
    typeof record.fingerprint === "string" &&
    typeof record.at === "number" &&
    (record.snapshotRevision === undefined || typeof record.snapshotRevision === "number") &&
    (record.serverIncarnation === undefined || typeof record.serverIncarnation === "string") &&
    (record.retiredServerIncarnations === undefined ||
      (Array.isArray(record.retiredServerIncarnations) &&
        record.retiredServerIncarnations.every((value) => typeof value === "string")))
    ? record
    : null;
}

export function readSessionContext(storage: Pick<Storage, "getItem">): SessionContextRecord | null {
  const record = readRecord<SessionContextRecord>(storage, SESSION_CONTEXT_KEY);
  return record &&
    typeof record.sessionId === "number" &&
    (record.serverIncarnation === undefined || typeof record.serverIncarnation === "string") &&
    (record.fingerprint === null || typeof record.fingerprint === "string") &&
    typeof record.at === "number"
    ? record
    : null;
}

export function writeProjectContext(
  storage: Pick<Storage, "getItem" | "setItem">,
  tree: ProjectTree,
): boolean {
  const revision = tree.snapshot_revision ?? 0;
  const incarnation = tree.server_incarnation;
  const current = readProjectContext(storage);
  let retired = current?.retiredServerIncarnations ?? [];
  if (incarnation === undefined) {
    // A legacy response has no lifetime evidence. It can still participate
    // in the old revision-only ordering while the stored record is also
    // legacy, but it may not replace a modern process identity.
    if (current?.serverIncarnation !== undefined) return false;
    if (revision < (current?.snapshotRevision ?? 0)) return false;
  } else if (current?.serverIncarnation === incarnation) {
    if (revision < (current.snapshotRevision ?? 0)) return false;
  } else if (current?.serverIncarnation !== undefined) {
    if (retired.includes(incarnation)) return false;
    retired = [...new Set([...retired, current.serverIncarnation])];
  } else {
    // A modern response establishes the first trustworthy lifetime after a
    // missing or legacy record. Unknown legacy provenance is not retired by
    // name because it had no name to compare later.
    retired = [];
  }
  writeRecord(storage, PROJECT_CONTEXT_KEY, {
    fingerprint: fingerprintProjectContext(tree),
    at: Date.now(),
    snapshotRevision: revision,
    serverIncarnation: incarnation,
    retiredServerIncarnations: retired,
  } satisfies ProjectContextRecord);
  return true;
}

export function writeSessionContext(
  storage: Pick<Storage, "getItem" | "setItem">,
  sessionId: number,
  serverIncarnation: string,
): void {
  const project = readProjectContext(storage);
  writeRecord(storage, SESSION_CONTEXT_KEY, {
    sessionId,
    serverIncarnation,
    fingerprint: project?.serverIncarnation === serverIncarnation ? project.fingerprint : null,
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

/// The whole decision, as a pure function of four inputs, so it can be
/// tested without a DOM, a session or a server.
///
/// `activeSessionId` is the session the caller is *currently attached to*
/// (`null`: none — there is nothing to be stale about yet).
export function contextLock(
  project: ProjectContextRecord | null,
  session: SessionContextRecord | null,
  activeSessionId: number | null,
  activeServerIncarnation?: string,
): ContextLock {
  if (activeSessionId === null) return "synced";
  if (activeServerIncarnation === undefined) return "unverified";
  // Either no window recorded a session start, or the recorded one is not
  // this one — somebody else's browser profile started it, or it was
  // started before this build shipped. Nothing to compare against.
  if (
    session === null ||
    session.sessionId !== activeSessionId ||
    session.serverIncarnation !== activeServerIncarnation
  ) return "unverified";
  if (project !== null && project.serverIncarnation !== activeServerIncarnation) {
    return "unverified";
  }
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
export function publishProjectContext(tree: ProjectTree): boolean {
  if (typeof window === "undefined") return true;
  const accepted = writeProjectContext(window.localStorage, tree);
  if (accepted) notifyContextChanged();
  return accepted;
}

/// Rebases only the exact active session named by an accepted server tree.
/// The marker is emitted only after the server replaced that session's whole
/// `GroupAddressContext`; ordinary name/DPT edits carry no marker and remain
/// fail-closed. A project record with a newer revision also blocks a delayed
/// publication from making a stale tree look synchronized.
export function rebasePublishedSessionContext(tree: ProjectTree): void {
  if (typeof window === "undefined") return;
  const sessionId = tree.group_address_context_session_id;
  const serverIncarnation = tree.server_incarnation;
  if (sessionId === undefined || serverIncarnation === undefined) return;
  const session = readSessionContext(window.localStorage);
  if (
    session?.sessionId !== sessionId ||
    session.serverIncarnation !== serverIncarnation
  ) return;
  const project = readProjectContext(window.localStorage);
  const revision = tree.snapshot_revision ?? 0;
  if (project?.serverIncarnation !== serverIncarnation) return;
  if ((project?.snapshotRevision ?? 0) > revision) return;
  if (
    project?.snapshotRevision === revision
    && project.fingerprint !== fingerprintProjectContext(tree)
  ) return;
  writeRecord(window.localStorage, SESSION_CONTEXT_KEY, {
    sessionId,
    serverIncarnation,
    fingerprint: fingerprintProjectContext(tree),
    at: Date.now(),
  } satisfies SessionContextRecord);
  notifyContextChanged();
}

/// Records which project fingerprint was current when `sessionId` started.
/// Called by whichever window pressed Connect; the other window reads it.
export function recordSessionContext(sessionId: number, serverIncarnation: string): void {
  if (typeof window === "undefined") return;
  writeSessionContext(window.localStorage, sessionId, serverIncarnation);
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
export function readContextLock(
  activeSessionId: number | null,
  activeServerIncarnation?: string,
): ContextLock {
  if (typeof window === "undefined") return "unverified";
  return contextLock(
    readProjectContext(window.localStorage),
    readSessionContext(window.localStorage),
    activeSessionId,
    activeServerIncarnation,
  );
}

/// Whether any window has published a project. Companion views use this
/// synchronous hint instead of issuing a current-project GET on every render.
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
