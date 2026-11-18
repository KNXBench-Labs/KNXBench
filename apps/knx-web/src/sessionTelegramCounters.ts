/** Per-session bus telegram counters shared by the monitor and the status bar. */
import { useSyncExternalStore } from "react";

export interface SessionIdentity {
  sessionId: number;
  serverIncarnation: string;
}

export interface SessionTelegramCounters {
  identity: SessionIdentity;
  received: number;
  sent: number;
}

// One entry at a time, exactly like `AppState.bus_session` (D3/D6): the
// counters always describe the one current session, and a new session's
// first event resets them instead of adding onto the previous session's.
let current: SessionTelegramCounters | null = null;
const listeners = new Set<() => void>();

function publish(next: SessionTelegramCounters | null): void {
  current = next;
  for (const listener of listeners) listener();
}

function sameIdentity(a: SessionIdentity | null, b: SessionIdentity | null): boolean {
  return a !== null && b !== null && a.sessionId === b.sessionId && a.serverIncarnation === b.serverIncarnation;
}

/** A session became current (or none is): its counters start at zero. */
export function resetSessionTelegramCounters(identity: SessionIdentity | null): void {
  if (sameIdentity(current?.identity ?? null, identity)) return;
  publish(identity === null ? null : { identity, received: 0, sent: 0 });
}

/** Wire telegrams observed for the current session (SessionClosed markers are local status, not counted). */
export function recordReceivedTelegrams(identity: SessionIdentity, count: number): void {
  if (!sameIdentity(current?.identity ?? null, identity) || count <= 0) return;
  publish({ identity, received: current!.received + count, sent: current!.sent });
}

/** One telegram actually sent from this session (compose/send path, after the server accepted it). */
export function recordSentTelegram(identity: SessionIdentity): void {
  if (!sameIdentity(current?.identity ?? null, identity)) return;
  publish({ identity, received: current!.received, sent: current!.sent + 1 });
}

export function readSessionTelegramCounters(): SessionTelegramCounters | null {
  return current;
}

export function useSessionTelegramCounters(): SessionTelegramCounters | null {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => current,
  );
}
