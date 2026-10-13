/** Issue #10: words the structured gateway refusals; unknown shapes keep the server text. */
import { translate, type Translate, type TranslatableKey } from "./i18n";
import { BUSY_LOCKS, type ActivitySnapshot, type BusyLock } from "./liveActivity";

/**
 * Who holds the gateway's one tunnel (`409`, `kind: "gatewayBusy"`). Only
 * KNXBench's own sessions are named; another client such as ETS shows up as
 * the gateway's own refusal (`gatewayRefused`) instead.
 */
export type TunnelHolder =
  | { kind: "deviceDownload"; address: string; state: "running" }
  /** `sessionId` is known only from the activity snapshot (to stop it). */
  | { kind: "addressProgramming"; address: string; state: "waiting" | "programming"; sessionId?: number }
  /** `gateway` is missing only from an older server's activity snapshot. */
  | { kind: "busMonitor"; sessionId: number; gateway?: string; state: "active" | "closed" }
  | { kind: "lineScan"; sessionId: number; probed: number; total: number }
  | { kind: "connecting"; session: BusyLock };

export type GatewayRefusal =
  | { kind: "gatewayBusy"; holder: TunnelHolder }
  /** `gatewayError` is the specification's name, `null` for an undefined code. */
  | { kind: "gatewayRefused"; gateway: string; gatewayStatus: number; gatewayError: string | null }
  | { kind: "gatewayTimeout"; gateway: string }
  | { kind: "unsupportedByteOrder"; address: string; order: string }
  /** Goal A3 (KL-163): stop admission refused a new tunnel; nothing was sent. */
  | { kind: "serverStopping" };

type Fields = Record<string, unknown>;

/** Exactly these keys, no more, no fewer: an unexpected shape is not worded. */
function exactly(value: unknown, keys: readonly string[]): Fields | null {
  if (value === null || typeof value !== "object" || Array.isArray(value)) return null;
  const actual = Object.keys(value);
  return actual.length === keys.length && keys.every((key) => Object.hasOwn(value, key)) ? value as Fields : null;
}
const count = (value: unknown): value is number => typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
const positive = (value: unknown): value is number => count(value) && value >= 1;
const text = (value: unknown): value is string => typeof value === "string" && value.length > 0 && value.length <= 256;
const individualAddress = (value: unknown) => typeof value === "string" && /^\d{1,2}\.\d{1,2}\.\d{1,3}$/.test(value);
const endpoint = (value: unknown) => typeof value === "string" && /^\d{1,3}(\.\d{1,3}){3}:\d{1,5}$/.test(value);
const oneOf = (value: unknown, allowed: readonly string[]) => typeof value === "string" && allowed.includes(value);

function admitHolder(value: unknown): TunnelHolder | null {
  const kind = (value as Fields | null)?.kind;
  let h: Fields | null;
  switch (kind) {
    case "deviceDownload":
      h = exactly(value, ["kind", "address", "state"]);
      return h && individualAddress(h.address) && h.state === "running" ? h as TunnelHolder : null;
    case "addressProgramming":
      h = exactly(value, ["kind", "address", "state"]);
      return h && individualAddress(h.address) && oneOf(h.state, ["waiting", "programming"]) ? h as TunnelHolder : null;
    case "busMonitor":
      h = exactly(value, ["kind", "sessionId", "gateway", "state"]);
      return h && positive(h.sessionId) && endpoint(h.gateway) && oneOf(h.state, ["active", "closed"])
        ? h as TunnelHolder : null;
    case "lineScan":
      h = exactly(value, ["kind", "sessionId", "probed", "total"]);
      return h && positive(h.sessionId) && count(h.probed) && count(h.total) && h.probed <= h.total
        ? h as TunnelHolder : null;
    case "connecting":
      h = exactly(value, ["kind", "session"]);
      return h && oneOf(h.session, BUSY_LOCKS) ? h as TunnelHolder : null;
    default:
      return null;
  }
}

/**
 * The structured refusal on an error `api.ts`'s `request()` threw, or `null`
 * for anything else, including a malformed or unexpected body (whose own
 * `error` text is then shown as before).
 */
export function admitGatewayRefusal(error: unknown): GatewayRefusal | null {
  if (!(error instanceof Error)) return null;
  const { status, body } = error as Error & { status?: unknown; body?: unknown };
  const kind = (body as Fields | null | undefined)?.kind;
  let b: Fields | null;
  switch (kind) {
    case "gatewayBusy": {
      b = exactly(body, ["error", "kind", "holder"]);
      const holder = b && status === 409 ? admitHolder(b.holder) : null;
      return holder ? { kind, holder } : null;
    }
    case "gatewayRefused":
      b = exactly(body, ["error", "kind", "gateway", "gatewayStatus", "gatewayError"]);
      return b && status === 502 && endpoint(b.gateway) && count(b.gatewayStatus) && b.gatewayStatus <= 0xff
        && (b.gatewayError === null || text(b.gatewayError))
        ? { kind, gateway: b.gateway as string, gatewayStatus: b.gatewayStatus, gatewayError: b.gatewayError as string | null }
        : null;
    case "gatewayTimeout":
      b = exactly(body, ["error", "kind", "gateway"]);
      return b && status === 502 && endpoint(b.gateway) ? { kind, gateway: b.gateway as string } : null;
    case "serverStopping":
      return exactly(body, ["error", "kind"]) && status === 503 ? { kind } : null;
    case "unsupportedByteOrder":
      b = exactly(body, ["error", "kind", "address", "order"]);
      return b && status === 422 && individualAddress(b.address) && text(b.order)
        ? { kind, address: b.address as string, order: b.order } : null;
    default:
      return null;
  }
}

/** Plain reasons for the CONNECT_RESPONSE codes the server names (Core §7.8.2, Tunnelling §2.2.2/§5.4.3.1). */
const REFUSAL_REASON: Readonly<Record<string, TranslatableKey>> = {
  E_CONNECTION_TYPE: "gatewayRefusal.reason.connectionType",
  E_CONNECTION_OPTION: "gatewayRefusal.reason.connectionOption",
  E_NO_MORE_CONNECTIONS: "gatewayRefusal.reason.noMoreConnections",
  E_NO_MORE_UNIQUE_CONNECTIONS: "gatewayRefusal.reason.noMoreUniqueConnections",
  E_AUTHORISATION_ERROR: "gatewayRefusal.reason.authorisation",
  E_TUNNELLING_LAYER: "gatewayRefusal.reason.tunnellingLayer",
  E_NO_TUNNELLING_ADDRESS: "gatewayRefusal.reason.noTunnellingAddress",
  E_CONNECTION_IN_USE: "gatewayRefusal.reason.connectionInUse",
};

const CONNECTING: Readonly<Record<BusyLock, TranslatableKey>> = {
  deviceDownload: "gatewayRefusal.busy.connecting.deviceDownload",
  managementOperation: "gatewayRefusal.busy.connecting.managementOperation",
  busMonitorOrGroupWrite: "gatewayRefusal.busy.connecting.busMonitorOrGroupWrite",
  lineScan: "gatewayRefusal.busy.connecting.lineScan",
};

const hex = (status: number) => `0x${status.toString(16).toUpperCase().padStart(2, "0")}`;

/**
 * What holds the gateway, read from the volatile activity snapshot so the
 * download page can say so before anything is prepared. The page's own
 * download (and its lock) is not a foreign holder; a finished scan or
 * programming holds nothing. A held lock without a session is named by its
 * lock only.
 */
export function holderFromActivity(snapshot: ActivitySnapshot): TunnelHolder | null {
  for (const session of snapshot.sessions) {
    if (session.kind === "busMonitor") {
      return session.gateway === undefined
        ? { kind: "busMonitor", sessionId: session.id, state: session.state }
        : { kind: "busMonitor", sessionId: session.id, gateway: session.gateway, state: session.state };
    }
    if (session.kind === "lineScan" && session.state === "running") {
      return { kind: "lineScan", sessionId: session.id, probed: session.probed, total: session.total };
    }
    if (session.kind === "addressProgramming" && (session.state === "waiting" || session.state === "programming")) {
      return { kind: "addressProgramming", sessionId: session.id, address: session.address, state: session.state };
    }
  }
  const lock = snapshot.busyLocks.find((held) => held !== "deviceDownload");
  return lock === undefined ? null : { kind: "connecting", session: lock };
}

export function describeHolder(holder: TunnelHolder, t: Translate): string {
  switch (holder.kind) {
    case "deviceDownload":
      return t("gatewayRefusal.busy.deviceDownload", { address: holder.address });
    case "addressProgramming":
      return holder.state === "waiting"
        ? t("gatewayRefusal.busy.addressProgrammingWaiting", { address: holder.address })
        : t("gatewayRefusal.busy.addressProgramming", { address: holder.address });
    case "busMonitor":
      if (holder.gateway === undefined) return t("gatewayRefusal.busy.busMonitorRunning");
      return holder.state === "active"
        ? t("gatewayRefusal.busy.busMonitor", { gateway: holder.gateway })
        : t("gatewayRefusal.busy.busMonitorClosed", { gateway: holder.gateway });
    case "lineScan":
      return t("gatewayRefusal.busy.lineScan", { probed: holder.probed, total: holder.total });
    case "connecting":
      return t(CONNECTING[holder.session]);
  }
}

export function describeGatewayRefusal(refusal: GatewayRefusal, t: Translate): string {
  switch (refusal.kind) {
    case "gatewayBusy":
      return describeHolder(refusal.holder, t);
    case "gatewayRefused": {
      const reason = refusal.gatewayError === null ? undefined : REFUSAL_REASON[refusal.gatewayError];
      const status = hex(refusal.gatewayStatus);
      if (reason) {
        return t("gatewayRefusal.refused", {
          gateway: refusal.gateway, reason: t(reason), constant: refusal.gatewayError as string, status,
        });
      }
      return refusal.gatewayError === null
        ? t("gatewayRefusal.refusedUnknown", { gateway: refusal.gateway, status })
        : t("gatewayRefusal.refusedNamed", { gateway: refusal.gateway, constant: refusal.gatewayError, status });
    }
    case "gatewayTimeout":
      return t("gatewayRefusal.timeout", { gateway: refusal.gateway });
    case "serverStopping":
      return t("gatewayRefusal.serverStopping");
    case "unsupportedByteOrder":
      return t("gatewayRefusal.byteOrder", { address: refusal.address, order: refusal.order });
  }
}

/** The worded refusal in the active UI language, or `null` to keep the error's own text. */
export function gatewayRefusalMessage(error: unknown): string | null {
  const refusal = admitGatewayRefusal(error);
  return refusal ? describeGatewayRefusal(refusal, translate) : null;
}
