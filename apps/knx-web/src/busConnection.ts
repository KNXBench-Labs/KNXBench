/** Shared bus-connection controller: activity-derived view, one action token, bound stop. */
import { isTcpEndpoint, validSerialTarget, type BusEndpoint, type IpTunnelTransport, type SerialTarget } from "./serialTransport";
import { useSyncExternalStore } from "react";
import * as api from "./api";
import type { BoundBusSession, BusMonitorStartResponse, BusMonitorStopResponse } from "./api";
import { emitAchievementEvent } from "./achievementEvents";
import { HistoryContractError } from "./activityHistory";
import { forgetSessionContext, readSessionContext, recordSessionContext, subscribeContextChanges } from "./busContext";
import type { TunnelHolder } from "./gatewayRefusal";
import type { Translate } from "./i18n";
import type { ActivitySnapshot } from "./liveActivity";

/**
 * What this window may say about the one shared bus session (WP-26).
 *
 * `connected` only on an admitted `busMonitor` session in state `active`;
 * `closed` is `lost` (the gateway ended it, the retained slot still blocks
 * other users until released). `disconnected` means no KNXBench session or
 * held reservation was seen; the snapshot's coverage is partial, so it is
 * never proof of an idle bus. A held session lock without a visible session
 * is `indeterminate`; a failed, refused, malformed or hidden read is
 * `unknown` and never a stale answer.
 */
export type BusConnectionView =
  | { phase: "unknown"; reason: "loading" | "unavailable" | "malformed" | "hidden" }
  | { phase: "disconnected" }
  | { phase: "connected" | "lost"; sessionId: number; serverIncarnation: string; gateway?: string; serial?: SerialTarget; transport?: IpTunnelTransport }
  | { phase: "occupied"; holder: TunnelHolder }
  | { phase: "indeterminate" };

export interface BusConnectionState {
  view: BusConnectionView;
  /** This window's one action in flight, owned until its request settles. */
  action: null | "connecting" | "disconnecting";
  /** The server incarnation changed since an earlier read in this window. */
  restarted: boolean;
}

export type BusActionResult<T> =
  | { ok: true; value: T }
  | { ok: false; reason: "inFlight" | "stale" | "invalidIdentity" }
  | { ok: false; reason: "failed"; error: unknown };

/** Visible polling cadence; the same two seconds as the live activity view. */
const POLL_MS = 2000;

/**
 * The holder a snapshot names, in the server's own priority: an admitted
 * System-2 run first, even next to a retained monitor (the bound stop refuses
 * it as busy too). A monitor is connected/lost only with an identity the
 * bound stop can name exactly; anything else is malformed, never repaired.
 */
export function connectionFromActivity(snapshot: ActivitySnapshot): BusConnectionView {
  if (snapshot.busyLocks.includes("system2Operation")) {
    return { phase: "occupied", holder: { kind: "connecting", session: "system2Operation" } };
  }
  for (const session of snapshot.sessions) {
    if (session.kind === "busMonitor") {
      const identity = { sessionId: session.id, serverIncarnation: snapshot.serverIncarnation };
      if (!api.isBoundBusSession(identity)) return { phase: "unknown", reason: "malformed" };
      const view = { phase: session.state === "active" ? "connected" as const : "lost" as const, ...identity };
      return {
        ...view,
        ...(session.gateway === undefined ? {} : { gateway: session.gateway }),
        ...(session.serial === undefined ? {} : { serial: session.serial }),
        ...(session.transport === undefined ? {} : { transport: session.transport }),
      };
    }
  }
  for (const session of snapshot.sessions) {
    if (session.kind === "deviceDownload" && session.state === "running") {
      return { phase: "occupied", holder: { kind: "deviceDownload", address: session.address, state: "running" } };
    }
    if (session.kind === "addressProgramming" && (session.state === "waiting" || session.state === "programming")) {
      return { phase: "occupied", holder: { kind: "addressProgramming", address: session.address, state: session.state } };
    }
    if (session.kind === "lineScan" && session.state === "running") {
      return {
        phase: "occupied",
        holder: { kind: "lineScan", sessionId: session.id, probed: session.probed, total: session.total },
      };
    }
  }
  const lock = snapshot.busyLocks.find((held) => held !== "busMonitorOrGroupWrite");
  if (lock !== undefined) return { phase: "occupied", holder: { kind: "connecting", session: lock } };
  return snapshot.busyLocks.length > 0 ? { phase: "indeterminate" } : { phase: "disconnected" };
}

const INITIAL: BusConnectionState = { view: { phase: "unknown", reason: "loading" }, action: null, restarted: false };
let state: BusConnectionState = INITIAL;
let serialObservation: { identity: BoundBusSession; target: SerialTarget } | null = null;
/** Last admitted TCP provenance for one immutable identity; UDP needs no memory (it is the legacy contract). */
let tcpObservation: BoundBusSession | null = null;
let observedIdentity: BoundBusSession | null = null;
let observationVersion = 0;

/** Synchronous observation token for a pending reattach, never a connection request. */
export function busObservationVersion(): string { return `${epoch}:${observationVersion}`; }

function rememberIdentity(identity: BoundBusSession): void {
  if (observedIdentity?.sessionId !== identity.sessionId || observedIdentity.serverIncarnation !== identity.serverIncarnation) observationVersion += 1;
  observedIdentity = {sessionId:identity.sessionId,serverIncarnation:identity.serverIncarnation};
}

/** Last admitted serial provenance for this exact immutable identity, even across unknown reads. */
export function knownSerialSession(identity: BoundBusSession): SerialTarget | undefined {
  return serialObservation?.identity.sessionId === identity.sessionId
    && serialObservation.identity.serverIncarnation === identity.serverIncarnation ? serialObservation.target : undefined;
}

/** Whether this exact identity was admitted as a TCP session, even across unknown reads. */
export function knownTcpSession(identity: BoundBusSession): boolean {
  return tcpObservation?.sessionId === identity.sessionId && tcpObservation.serverIncarnation === identity.serverIncarnation;
}

function rememberTcp(identity: BoundBusSession): void {
  tcpObservation = { sessionId: identity.sessionId, serverIncarnation: identity.serverIncarnation };
}

/** A fresh poll cannot override a different currently observed session or process. */
export function acceptsObservedSession(identity: BoundBusSession): boolean {
  if (lastIncarnation !== null && lastIncarnation !== identity.serverIncarnation) return false;
  return observedIdentity === null || (observedIdentity.sessionId === identity.sessionId
    && observedIdentity.serverIncarnation === identity.serverIncarnation);
}

const subscribers = new Set<() => void>();
/** Bumped by every action start/settle and hide: an older read answer is dropped. */
let readGeneration = 0;
/** Bumped when the server incarnation changes: an older action publishes nothing. */
let epoch = 0;
let lastIncarnation: string | null = null;
let reading: Promise<void> | null = null;
let readAgain = false;
let pollers = 0;
let stopPolling: (() => void) | null = null;
/** Sessions the activity view named while this window's action ran; kept through later unknown reads. */
let seenDuringAction: BoundBusSession[] = [];
/** The session record as it was when the action started (another window may publish one meanwhile). */
let recordAtActionStart: ReturnType<typeof readSessionContext> = null;

function publish(next: Partial<BusConnectionState>): void {
  const view = next.view;
  if (view?.phase === "connected" || view?.phase === "lost") {
    rememberIdentity(view);
    if (view.serial) serialObservation = {identity:{sessionId:view.sessionId,serverIncarnation:view.serverIncarnation},target:view.serial};
    if (view.transport === "tcp") rememberTcp(view);
  }
  state = { ...state, ...next };
  for (const listener of subscribers) listener();
}

function subscribe(listener: () => void): () => void {
  subscribers.add(listener);
  return () => subscribers.delete(listener);
}

const visible = () => typeof document === "undefined" || document.visibilityState === "visible";

async function readOnce(): Promise<void> {
  const generation = readGeneration;
  let view: BusConnectionView;
  let incarnation: string | null = null;
  try {
    const snapshot = await api.busActivity();
    incarnation = snapshot.serverIncarnation;
    view = connectionFromActivity(snapshot);
    // Known TCP never reads back as the legacy UDP contract: missing or other metadata fails closed.
    if ((view.phase === "connected" || view.phase === "lost") && knownTcpSession(view) && view.transport !== "tcp") {
      view = { phase: "unknown", reason: "malformed" };
    }
  } catch (error) {
    view = { phase: "unknown", reason: error instanceof HistoryContractError ? "malformed" : "unavailable" };
  }
  if (generation !== readGeneration) {
    readAgain = true;
    return;
  }
  let restarted = state.restarted;
  // A malformed view is no evidence of a server lifetime.
  if (incarnation !== null && !(view.phase === "unknown" && view.reason === "malformed")) {
    if (lastIncarnation !== null && lastIncarnation !== incarnation) {
      restarted = true;
      epoch += 1;
    }
    lastIncarnation = incarnation;
  }
  publish({ view, restarted });
  if (state.action !== null && (view.phase === "connected" || view.phase === "lost")) {
    seenDuringAction.push({ sessionId: view.sessionId, serverIncarnation: view.serverIncarnation });
  }
}

/**
 * One read of `GET /api/bus/activity`; never starts, stops or retries a
 * session. A call while a read is in flight asks for one follow-up read
 * instead of a second concurrent one. Hidden: no request, `unknown`.
 */
export function refreshBusConnection(): Promise<void> {
  if (!visible()) {
    readGeneration += 1;
    publish({ view: { phase: "unknown", reason: "hidden" } });
    return Promise.resolve();
  }
  if (reading) {
    readAgain = true;
    return reading;
  }
  reading = (async () => {
    try {
      do {
        readAgain = false;
        await readOnce();
      } while (readAgain && visible());
    } finally {
      reading = null;
    }
  })();
  return reading;
}

/** Polls every two seconds while visible; reference-counted, returns its release. */
export function acquireBusConnectionPolling(): () => void {
  pollers += 1;
  if (pollers === 1) {
    const read = () => void refreshBusConnection();
    const interval = window.setInterval(() => { if (visible()) read(); }, POLL_MS);
    const unsubscribeContext = subscribeContextChanges(read);
    window.addEventListener("focus", read);
    document.addEventListener("visibilitychange", read);
    stopPolling = () => {
      window.clearInterval(interval);
      unsubscribeContext();
      window.removeEventListener("focus", read);
      document.removeEventListener("visibilitychange", read);
    };
    read();
  }
  let released = false;
  return () => {
    if (released) return;
    released = true;
    pollers -= 1;
    if (pollers === 0) {
      stopPolling?.();
      stopPolling = null;
    }
  };
}

function takeToken(action: "connecting" | "disconnecting"): number | null {
  if (state.action !== null) return null;
  readGeneration += 1;
  seenDuringAction = [];
  recordAtActionStart = readRecord();
  publish({ action });
  return epoch;
}

function releaseToken(): void {
  readGeneration += 1;
  publish({ action: null });
  void refreshBusConnection();
}

/**
 * Whether a session identity still belongs to the server lifetime this
 * window observed last. With no admitted read yet there is no evidence
 * against it (a first start may legitimately precede every read).
 */
function ofObservedLifetime(identity: BoundBusSession): boolean {
  return lastIncarnation === null || lastIncarnation === identity.serverIncarnation;
}

type SessionIdentity = { sessionId: number; serverIncarnation?: string };
const sameSession = (a: SessionIdentity, b: SessionIdentity) =>
  a.sessionId === b.sessionId && a.serverIncarnation === b.serverIncarnation;

function readRecord() {
  return typeof window === "undefined" ? null : readSessionContext(window.localStorage);
}

/**
 * Whether another session than `identity` became known while the action
 * ran: a session record published meanwhile (one already there before the
 * request is not evidence), or any session the activity view named, even if
 * a later read failed or the window was hidden. A larger id alone means nothing.
 */
function otherSessionAppeared(identity: BoundBusSession): boolean {
  const record = readRecord();
  const published = record !== null && !sameSession(identity, record)
    && (recordAtActionStart === null || !sameSession(record, recordAtActionStart) || record.at !== recordAtActionStart.at);
  return published || seenDuringAction.some((seen) => !sameSession(identity, seen));
}

/**
 * Whether anything newer than `identity` was seen while its stop ran: the
 * window's session record names another session, or the activity view shows
 * another one. The same process alone is not ownership.
 */
function supersededDuringStop(identity: BoundBusSession): boolean {
  const record = readRecord();
  if (record !== null && !sameSession(identity, record)) return true;
  const { view } = state;
  return ((view.phase === "connected" || view.phase === "lost") && !sameSession(identity, view))
    || otherSessionAppeared(identity);
}

/** Worded failure of an explicit TCP start: a server without the route says so; UDP was never tried. */
export function tcpStartErrorMessage(error: unknown, t: Translate): string {
  const status = api.errorStatus(error);
  if (status === 404 || status === 405) return t("busTransport.ipTunnel.unsupported");
  return api.errorMessage(error);
}

/** Each endpoint kind has exactly one start route; an unrecognised shape opens nothing. */
function startEndpoint(endpoint: BusEndpoint): Promise<BusMonitorStartResponse> {
  if (typeof endpoint === "string") return api.startBusMonitor(endpoint);
  if (isTcpEndpoint(endpoint)) return api.startTcpBusMonitor(endpoint.gateway);
  if (validSerialTarget(endpoint)) return api.startSerialBusMonitor(endpoint);
  return Promise.reject(new Error("Invalid bus endpoint; nothing was sent."));
}

/**
 * Explicit connect: the start route, then, only for the current epoch and
 * the server lifetime observed now, the achievement, the caller's synchronous
 * `beforePublish` and the session record, in that order. A stale answer
 * publishes nothing and is never stopped. A refusal is returned unmodified;
 * nothing retries.
 */
export async function connectBusSession(
  endpoint: BusEndpoint,
  hooks: { beforePublish?: (started: BusMonitorStartResponse) => void; stillOwned?: (started: BusMonitorStartResponse) => boolean } = {},
): Promise<BusActionResult<BusMonitorStartResponse>> {
  const token = takeToken("connecting");
  if (token === null) return { ok: false, reason: "inFlight" };
  try {
    let started: BusMonitorStartResponse;
    try {
      started = await startEndpoint(endpoint);
    } catch (error) {
      return { ok: false, reason: "failed", error };
    }
    if (!api.isBoundBusSession(started)) {
      return { ok: false, reason: "failed", error: new Error("The server answered with a malformed bus session identity.") };
    }
    if (token !== epoch || !ofObservedLifetime(started) || otherSessionAppeared(started)
      || hooks.stillOwned?.(started) === false) {
      return { ok: false, reason: "stale" };
    }
    rememberIdentity(started);
    if (started.serial) serialObservation = {identity:{sessionId:started.sessionId,serverIncarnation:started.serverIncarnation},target:started.serial};
    if (started.transport === "tcp") rememberTcp(started);
    emitAchievementEvent({ type: "busMonitorStarted" });
    hooks.beforePublish?.(started);
    recordSessionContext(started.sessionId, started.serverIncarnation);
    return { ok: true, value: started };
  } finally {
    releaseToken();
  }
}

/**
 * Explicit disconnect of exactly `identity` on the bound route. On success
 * the caller's synchronous `afterStop` runs before the session record is
 * forgotten, but only while nothing newer owns the client state: same epoch
 * and lifetime, no other session in the record or the view, and the caller's
 * `stillOwned` (e.g. a panel that adopted another session) agrees. Otherwise
 * the stop of `identity` is reported `stale` with no side effect. A refusal
 * (changed session, busy, missing route) is returned unmodified: no forget,
 * no retry, no legacy stop.
 */
export async function disconnectBusSession(
  identity: BoundBusSession,
  hooks: { afterStop?: (summary: BusMonitorStopResponse) => void; stillOwned?: () => boolean } = {},
): Promise<BusActionResult<BusMonitorStopResponse>> {
  if (!api.isBoundBusSession(identity)) return { ok: false, reason: "invalidIdentity" };
  const token = takeToken("disconnecting");
  if (token === null) return { ok: false, reason: "inFlight" };
  try {
    let summary: BusMonitorStopResponse;
    try {
      summary = await api.stopBoundBusMonitor(identity);
    } catch (error) {
      return { ok: false, reason: "failed", error };
    }
    if (token !== epoch || !ofObservedLifetime(identity) || supersededDuringStop(identity)
      || hooks.stillOwned?.() === false) {
      return { ok: false, reason: "stale" };
    }
    hooks.afterStop?.(summary);
    // Only A's record (or none) is cleared; never one published meanwhile.
    const record = readRecord();
    if (record === null || sameSession(identity, record)) forgetSessionContext();
    return { ok: true, value: summary };
  } finally {
    releaseToken();
  }
}

export function useBusConnection(): BusConnectionState {
  return useSyncExternalStore(subscribe, () => state);
}

export function readBusConnectionForTests(): BusConnectionState {
  return state;
}

/** Drops state, listeners and polling; module state outlives a test otherwise. */
export function resetBusConnectionForTests(): void {
  stopPolling?.();
  stopPolling = null;
  pollers = 0;
  state = INITIAL;
  serialObservation = null;
  tcpObservation = null;
  observedIdentity = null;
  observationVersion = 0;
  subscribers.clear();
  readGeneration += 1;
  epoch += 1;
  lastIncarnation = null;
  seenDuringAction = [];
  recordAtActionStart = null;
  reading = null;
  readAgain = false;
}
