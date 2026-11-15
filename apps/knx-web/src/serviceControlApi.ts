/** Strict explicit-TCP service-control calls and their `connection` metadata admission. */
import { noteRefusal } from "./api";
import type { ServiceControlReading, ServiceControlWriteResponse } from "./api";
import { validTcpConnection, type TcpConnectionFacts } from "./tcpConnectionFacts";

/** The project snapshot a TCP operation is bound to; a changed one is refused by the server. */
export interface ServiceControlBinding { serverIncarnation: string; projectIncarnation: number; revision: number }
export type TcpServiceControlReading = ServiceControlReading & { connection: TcpConnectionFacts };
export type TcpServiceControlWrite = ServiceControlWriteResponse & { connection: TcpConnectionFacts };

/**
 * A refused or unusable TCP call. `status` 0 means the answer came but its metadata was not
 * admissible; `connection` carries the failure's facts when the server sent valid ones.
 */
export class ServiceControlTcpRefusal extends Error {
  constructor(readonly status: number, readonly detail: string | null, readonly connection: TcpConnectionFacts | null) {
    super(`service-control-tcp-refused (${status})`);
  }
}

async function call(path: string, body: unknown): Promise<Record<string, unknown>> {
  const response = await fetch(path, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body), cache: "no-store" });
  if (!response.ok) {
    noteRefusal(path, response.status);
    const parsed: unknown = await response.json().catch(() => null);
    const object = parsed && typeof parsed === "object" ? parsed as { error?: unknown; connection?: unknown } : {};
    // Failure facts must be well formed; anything else is dropped, never trusted.
    const connection = validTcpConnection(object.connection, false) ? object.connection : null;
    throw new ServiceControlTcpRefusal(response.status, typeof object.error === "string" ? object.error.slice(0, 512) : null, connection);
  }
  const value: unknown = await response.json().catch(() => null);
  // A 2xx claiming success must carry the key, with an opened tunnel: no tunnel cannot have read anything.
  if (!value || typeof value !== "object" || Array.isArray(value) || !("connection" in value) ||
      !validTcpConnection((value as { connection: unknown }).connection, true)) {
    throw new ServiceControlTcpRefusal(0, null, null);
  }
  return value as Record<string, unknown>;
}

/** Calls only the strict sibling: an older server answers 404/405, never a UDP read. */
export async function readServiceControlTcp(address: string, gateway: string, binding: ServiceControlBinding): Promise<TcpServiceControlReading> {
  return await call("/api/device/service-control/tcp/read", { address, gateway, binding }) as unknown as TcpServiceControlReading;
}

export async function writeServiceControlTcp(
  address: string, gateway: string, enable: boolean, confirmation: string, binding: ServiceControlBinding,
): Promise<TcpServiceControlWrite> {
  return await call("/api/device/service-control/tcp", { address, gateway, enable, confirmation, binding }) as unknown as TcpServiceControlWrite;
}
