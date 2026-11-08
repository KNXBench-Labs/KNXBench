/** Validate serial targets without accessing preferences, devices or the network. */

export interface SerialTarget { path: string; source: string; mode: "passive" | "active" }
export type BusTransport = "ip" | "serial";
/** The IP tunnel transport of a KNXnet/IP session; UDP is the default, TCP always explicit. */
export type IpTunnelTransport = "udp" | "tcp";
/** An explicit TCP tunnel to a gateway `host:port`; a bare string stays the UDP endpoint. */
export interface TcpEndpoint { kind: "tcp"; gateway: string }
export type BusEndpoint = string | SerialTarget | TcpEndpoint;

export function isIpTunnelTransport(value: unknown): value is IpTunnelTransport {
  return value === "udp" || value === "tcp";
}
export function isTcpEndpoint(value: unknown): value is TcpEndpoint {
  return value !== null && typeof value === "object" && !Array.isArray(value)
    && Object.keys(value).length === 2 && "kind" in value && "gateway" in value
    && value.kind === "tcp" && typeof value.gateway === "string" && value.gateway.length > 0;
}
/** `gateway`, or `gateway · TCP` for an observed or selected TCP tunnel; UDP stays unmarked. */
export function gatewayWithTransport(gateway: string, transport: IpTunnelTransport | undefined): string {
  return transport === "tcp" ? `${gateway} · TCP` : gateway;
}

export function validSerialSource(source: string): boolean {
  if (!/^(0|[1-9][0-9]?)\.(0|[1-9][0-9]?)\.(0|[1-9][0-9]{0,2})$/.test(source)) return false;
  const [area, line, device] = source.split(".").map(Number);
  return area <= 15 && line <= 15 && device <= 255
    && !["0.0.0", "15.15.255", "1.1.220"].includes(source);
}
export function validSerialPath(path: string): boolean {
  return path.trim().length > 0 && path === path.trim() && !/[\u0000-\u001f\u007f-\u009f]/u.test(path);
}
export function validSerialTarget(value: unknown): value is SerialTarget {
  return value !== null && typeof value === "object" && !Array.isArray(value)
    && Object.keys(value).length === 3 && "path" in value && "source" in value && "mode" in value
    && typeof value.path === "string" && validSerialPath(value.path)
    && typeof value.source === "string" && validSerialSource(value.source)
    && (value.mode === "passive" || value.mode === "active");
}
export function serialLabel(target: SerialTarget): string {
  return `USB/serial TP-UART · ${target.path} · ${target.source} · ${target.mode}`;
}
