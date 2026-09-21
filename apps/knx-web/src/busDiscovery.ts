/** Module-level store for the KNXnet/IP interface search: one result, shared by every reader. */
import { useSyncExternalStore } from "react";
import * as api from "./api";
import type { BusDiscoveredInterface } from "./api";

/**
 * Where the one search this application runs has got to.
 *
 * - `"idle"` — no search has been started yet in this window.
 * - `"searching"` — a `POST /api/bus/discover` is in flight. The server's
 *   search window is fixed (Core v01.06.02 AS §5.2.4's `SEARCH_TIMEOUT`),
 *   so this lasts a noticeable number of seconds and the button that
 *   starts it must be disabled for the duration: clicking four times buys
 *   four timeouts, not a faster answer.
 * - `"done"` — a search finished. `interfaces` is what it found, and an
 *   empty array here is a *result*, not a failure.
 * - `"failed"` — the search could not be performed (the server answered
 *   `502`, or the request never got there). Deliberately not an error
 *   state anything shouts about: a laptop with no KNX installation on its
 *   network is the common case, not an exception.
 */
export type BusDiscoveryPhase = "idle" | "searching" | "done" | "failed";

export interface BusDiscoveryState {
  phase: BusDiscoveryPhase;
  interfaces: BusDiscoveredInterface[];
  /**
   * Whatever the failed attempt reported, for the muted detail line —
   * never a modal, never a toast. `null` in every other phase. The text
   * comes from the server and is therefore English regardless of UI
   * language; it sits next to a translated sentence that carries the
   * meaning, so a reader who cannot use it loses nothing.
   */
  error: string | null;
}

// One module-level value, broadcast through `useSyncExternalStore` —
// the same shape `productLanguage.ts` uses, for the same reason: two
// independent readers (`App.tsx`'s startup effect and `BusMonitorPanel`)
// must never hold two different answers to "what did the search find".
// A `useState` per component would give the panel an empty list while the
// startup search it did not fire had already finished.
let state: BusDiscoveryState = { phase: "idle", interfaces: [], error: null };
const subscribers = new Set<() => void>();

function getSnapshot(): BusDiscoveryState {
  return state;
}

function subscribe(onStoreChange: () => void): () => void {
  subscribers.add(onStoreChange);
  return () => subscribers.delete(onStoreChange);
}

function publish(next: BusDiscoveryState): void {
  state = next;
  for (const onStoreChange of subscribers) onStoreChange();
}

/**
 * Runs a search and publishes the result. Never rejects and never throws:
 * the failure *is* a published phase, so no caller has to decide what to
 * do about a rejected promise on a path the user did not ask for.
 *
 * A search already in flight is not doubled — the second caller just
 * awaits nothing and returns, leaving the first one's answer to arrive on
 * its own.
 *
 * On failure the previously found interfaces are kept. They are the last
 * thing this application actually knows about the network, and a search
 * that could not run says nothing about whether they are still reachable;
 * dropping them would take working options away from a user who was about
 * to click one.
 */
export async function searchBusInterfaces(): Promise<void> {
  if (state.phase === "searching") return;
  publish({ ...state, phase: "searching", error: null });
  try {
    const response = await api.discoverBusInterfaces();
    publish({ phase: "done", interfaces: response.interfaces, error: null });
  } catch (e) {
    publish({ ...state, phase: "failed", error: api.errorMessage(e) });
  }
}

/**
 * Starts the one search that happens without anybody asking — at
 * application start, and again if a component that needs the result
 * mounts before that ever ran (a second window, a test rendering the
 * panel alone). Does nothing once a search has been started, so it is
 * safe to call from as many mount effects as want it.
 *
 * Deliberately not `await`ed by its callers: this must never block first
 * paint, never delay a project from loading, and never make the user wait
 * on a network with nothing on it.
 */
export function ensureBusDiscovery(): void {
  if (state.phase !== "idle") return;
  void searchBusInterfaces();
}

export function useBusDiscovery(): BusDiscoveryState {
  return useSyncExternalStore(subscribe, getSnapshot);
}

/**
 * Drops the module-level result and forgets every subscriber. Module
 * state outlives a component and a test file's modules are shared across
 * its tests, so without this one test's finished search would decide the
 * next test's first render — and a test that unmounts its host without
 * `root.unmount()` (as `BusMonitorPanel.test.tsx` does, to keep each test
 * to one root) never triggers `subscribe`'s own cleanup, so its callback
 * would sit in the `Set` past the end of the test and re-render a
 * detached tree on every later `publish()`. Both halves are the same
 * kind of leftover — a value and a listener nobody will read again — so
 * both are cleared together.
 */
export function resetBusDiscoveryForTests(): void {
  state = { phase: "idle", interfaces: [], error: null };
  subscribers.clear();
}

/**
 * The current value, without a React render to observe it through. Only
 * the store's own tests use this; components read `useBusDiscovery`.
 */
export function readBusDiscoveryForTests(): BusDiscoveryState {
  return state;
}
