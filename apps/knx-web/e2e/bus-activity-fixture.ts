/** Answers the footer's passive bus-activity read with an idle snapshot for whole-app fixtures. */
// WP-26: the status bar is always mounted and reads `GET /api/bus/activity`
// on its own, so a strict offline fixture has to own that one read. The
// answer is a fresh, schema-valid, truly idle snapshot: no session, no
// one-shot, no lock. Only the GET is answered; every other method or path
// stays `undefined`, so the spec's own request ledger keeps refusing it.
import type { ActivitySnapshot } from "../src/liveActivity";

/** A new idle snapshot on every call, so no fixture can mutate another's answer. */
export function idleBusActivity(): ActivitySnapshot {
  return {
    serverIncarnation: "fixture",
    coverage: "partial",
    historyState: "disabled",
    sessions: [],
    oneShot: [],
    oneShotDropped: 0,
    busyLocks: [],
    untracked: [],
  };
}

/** The idle snapshot for the passive read, or `undefined` for any other request. */
export function busActivityFixtureAnswer(method: string, path: string): ActivitySnapshot | undefined {
  return method === "GET" && path === "/api/bus/activity" ? idleBusActivity() : undefined;
}
