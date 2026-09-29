/** Separates gateway fields while preserving the server's IPv4 endpoint contract. */
export const DEFAULT_KNX_PORT = "3671";

export interface GatewayFields {
  host: string;
  port: string;
}

/** Read an existing single-field preference or a discovered control endpoint. */
export function splitGatewayEndpoint(value: string): GatewayFields {
  const endpoint = value.trim();
  if (endpoint.startsWith("[")) {
    const close = endpoint.indexOf("]");
    if (close > 0 && (endpoint.length === close + 1 || endpoint[close + 1] === ":")) {
      return {
        host: endpoint.slice(1, close),
        port: endpoint.slice(close + 2).trim() || DEFAULT_KNX_PORT,
      };
    }
  }
  const firstColon = endpoint.indexOf(":");
  if (firstColon !== -1 && firstColon === endpoint.lastIndexOf(":")) {
    return {
      host: endpoint.slice(0, firstColon).trim(),
      port: endpoint.slice(firstColon + 1).trim() || DEFAULT_KNX_PORT,
    };
  }
  // An unbracketed IPv6 literal is ambiguous as an endpoint. Retain it as
  // entered rather than mistaking its final segment for a decimal port.
  return { host: endpoint, port: DEFAULT_KNX_PORT };
}

export type GatewayValidation =
  | { ok: true; endpoint: string }
  | { ok: false; reason: "hostRequired" | "unsupportedHost" | "invalidPort" };

/** The monitor route and KNXnet/IP tunnel currently accept SocketAddrV4 only. */
export function validateGatewayFields(fields: GatewayFields): GatewayValidation {
  const host = fields.host.trim();
  if (!host) return { ok: false, reason: "hostRequired" };
  const octets = host.split(".");
  if (
    octets.length !== 4 ||
    octets.some((octet) => !/^(0|[1-9][0-9]{0,2})$/.test(octet) || Number(octet) > 255)
  ) {
    return { ok: false, reason: "unsupportedHost" };
  }
  const portInput = fields.port.trim() || DEFAULT_KNX_PORT;
  if (!/^[0-9]+$/.test(portInput)) return { ok: false, reason: "invalidPort" };
  const port = Number(portInput);
  if (!Number.isSafeInteger(port) || port < 1 || port > 65535) {
    return { ok: false, reason: "invalidPort" };
  }
  return { ok: true, endpoint: `${host}:${port}` };
}
