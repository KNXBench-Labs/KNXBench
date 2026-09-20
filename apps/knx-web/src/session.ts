/** The session seam: what the shell may do about a session, and who hears when one ends. */
//! Two small things that would otherwise force `api.ts` to import React or
//! `AuthGate.tsx` to import every caller of `api.ts`:
//!
//! - `SessionControls`, the handful of session facts the application shell
//!   needs in order to draw (or not draw) a logout control;
//! - a one-event notifier for "the server just answered 401", published by
//!   `api.ts`'s shared `request()` helper and subscribed to by `AuthGate`.
//!
//! Deliberately framework-free and dependency-free, so `api.ts` can publish
//! from inside a plain `fetch` wrapper and a test can subscribe without
//! rendering anything.

/** What the shell is told about the session it is running inside. */
export interface SessionControls {
  /**
   * Whether this server asks for a password at all (`GET /api/auth/status`'s
   * `required`). `false` on the desktop shell, where a logout control would
   * be nonsense — there is no session to end.
   */
  required: boolean;
  /** Ends the session and returns the user to the login screen. */
  logout: () => void;
}

type Listener = () => void;

const listeners = new Set<Listener>();

/**
 * Subscribes to "a request was refused for want of a session". Returns the
 * unsubscribe function, shaped for a bare `useEffect(() => subscribe(...), [])`.
 */
export function subscribeSessionExpired(listener: Listener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/**
 * Published by `api.ts` when an `/api/` call that is not itself an auth
 * endpoint comes back 401 — a session that idled past twelve hours, or a
 * server that was restarted and forgot every token it ever issued.
 *
 * A listener that throws must not stop the others hearing about it, and it
 * must certainly not turn a 401 into an unhandled rejection inside whatever
 * request happened to be the one refused, so each is called in its own
 * `try`.
 */
export function notifySessionExpired(): void {
  for (const listener of [...listeners]) {
    try {
      listener();
    } catch {
      /* a broken listener is not the refused request's problem */
    }
  }
}

/** Test helper: drops every subscription, so one suite cannot leak into the next. */
export function resetSessionListenersForTests(): void {
  listeners.clear();
}
